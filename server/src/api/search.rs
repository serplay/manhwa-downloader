use std::{collections::BTreeMap, sync::Arc};

use axum::{
    Json,
    extract::{Query, State},
};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::{
    cache::Caches,
    error::{AppError, AppResult, ErrorDetail},
    model::Comic,
    sources::{SearchOptions, Source},
    state::SharedState,
};

pub const MAX_QUERY_LEN: usize = 200;

#[derive(Deserialize, IntoParams)]
pub struct SearchParams {
    /// Title to search for.
    pub q: String,
    /// Source id, slug, or `all`.
    #[serde(default = "default_source")]
    pub source: String,
    /// Preferred language code (only some sources honor it).
    pub lang: Option<String>,
    /// Include adult titles and adult-oriented sources. Off by default.
    #[serde(default)]
    pub adult: bool,
}

fn default_source() -> String {
    "all".into()
}

#[derive(Serialize, ToSchema)]
pub struct SearchResponse {
    /// Results keyed by source slug.
    pub results: BTreeMap<String, Vec<Comic>>,
    /// Sources that failed, keyed by slug. A partial failure is still a 200.
    pub errors: BTreeMap<String, ErrorDetail>,
}

pub fn validate_query(q: &str) -> AppResult<String> {
    let q = q.trim();
    if q.is_empty() {
        return Err(AppError::Validation(
            "search query must not be empty".into(),
        ));
    }
    if q.chars().count() > MAX_QUERY_LEN {
        return Err(AppError::Validation(format!(
            "search query must be at most {MAX_QUERY_LEN} characters"
        )));
    }
    Ok(q.to_string())
}

/// Cached search against one source.
pub async fn search_source(
    state: &SharedState,
    source: &Arc<dyn Source>,
    query: &str,
    lang: Option<&str>,
    opts: SearchOptions,
) -> AppResult<Arc<Vec<Comic>>> {
    let meta = source.meta();
    if meta.adult && !opts.include_adult {
        return Err(AppError::AdultHidden(meta.name.to_string()));
    }
    let key = Caches::search_key(meta.slug, query, lang, opts.include_adult);
    if let Some(hit) = state.caches.search.get(&key).await {
        return Ok(hit);
    }
    let ctx = state.ctx();
    // Sources filter upstream where they can; this guarantees the contract
    // regardless of how thorough a given source is.
    let comics: Vec<Comic> = source
        .search(&ctx, query, lang, &opts)
        .await?
        .into_iter()
        .filter(|c| opts.include_adult || !c.adult)
        .collect();
    let comics = Arc::new(comics);
    // Only cache non-empty answers; an empty result is often a transient upstream hiccup.
    if !comics.is_empty() {
        state.caches.search.insert(key, Arc::clone(&comics)).await;
    }
    Ok(comics)
}

/// Search every implemented source concurrently. Failures are reported per source.
pub async fn search_all(
    state: &SharedState,
    query: &str,
    lang: Option<&str>,
    opts: SearchOptions,
) -> (BTreeMap<String, Vec<Comic>>, BTreeMap<String, ErrorDetail>) {
    let futures = state
        .registry
        .implemented()
        .filter(|s| opts.include_adult || !s.meta().adult)
        .map(|source| {
            let source = Arc::clone(source);
            async move {
                let slug = source.meta().slug.to_string();
                let outcome = search_source(state, &source, query, lang, opts).await;
                (slug, outcome)
            }
        });
    let mut results = BTreeMap::new();
    let mut errors = BTreeMap::new();
    for (slug, outcome) in join_all(futures).await {
        match outcome {
            Ok(comics) => {
                results.insert(slug, (*comics).clone());
            }
            Err(err) => {
                tracing::warn!(source = %slug, error = %err, "search failed");
                errors.insert(slug, err.detail());
            }
        }
    }
    (results, errors)
}

#[utoipa::path(get, path = "/search", tag = "catalogue", params(SearchParams),
    responses(
        (status = 200, body = SearchResponse),
        (status = 400, description = "Empty query or unknown source", body = crate::error::ErrorBody),
        (status = 501, description = "Source not ported yet", body = crate::error::ErrorBody),
        (status = 502, description = "Source failed", body = crate::error::ErrorBody)))]
pub async fn search(
    State(state): State<SharedState>,
    Query(params): Query<SearchParams>,
) -> AppResult<Json<SearchResponse>> {
    let query = validate_query(&params.q)?;
    let lang = params.lang.as_deref();
    let opts = SearchOptions {
        include_adult: params.adult,
    };
    if params.source.eq_ignore_ascii_case("all") {
        let (results, errors) = search_all(&state, &query, lang, opts).await;
        return Ok(Json(SearchResponse { results, errors }));
    }
    let source = state.registry.get(&params.source)?;
    let comics = search_source(&state, &source, &query, lang, opts).await?;
    let mut results = BTreeMap::new();
    results.insert(source.meta().slug.to_string(), (*comics).clone());
    Ok(Json(SearchResponse {
        results,
        errors: BTreeMap::new(),
    }))
}
