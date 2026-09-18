//! Application error type and its HTTP mapping.
//!
//! Every failure surfaces as `{ "error": { "code", "message", "source"? } }`
//! with a status code that actually reflects what happened.

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Unsupported(String),
    #[error("unknown source `{0}`")]
    UnknownSource(String),
    #[error("source `{0}` is not implemented yet")]
    SourceNotImplemented(String),
    #[error("{message}")]
    Upstream {
        site: Option<String>,
        message: String,
    },
    #[error("{site} did not respond in time")]
    UpstreamTimeout { site: String },
    /// The source is adult-oriented and the request did not opt into adult content.
    #[error("{0} is an adult source; enable adult content to search it")]
    AdultHidden(String),
    /// Cloudflare served a challenge no client in this build can pass.
    #[error(
        "{site} is behind a Cloudflare challenge that needs a real browser; this build cannot pass it"
    )]
    Blocked { site: String },
    /// The client started too many downloads; retry after this many seconds.
    #[error("too many downloads started from your address; try again in {retry_after_s}s")]
    RateLimited { retry_after_s: u64 },
    #[error("{site} returned data we could not parse: {message}")]
    Parse { site: String, message: String },
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Internal(anyhow::Error::new(e).context("filesystem error"))
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorDetail {
    /// Stable machine-readable code.
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "VALIDATION",
            Self::NotFound(_) => "NOT_FOUND",
            Self::Conflict(_) => "CONFLICT",
            Self::Unsupported(_) => "UNSUPPORTED",
            Self::UnknownSource(_) => "UNKNOWN_SOURCE",
            Self::SourceNotImplemented(_) => "SOURCE_NOT_IMPLEMENTED",
            Self::Upstream { .. } => "UPSTREAM_FAILURE",
            Self::UpstreamTimeout { .. } => "UPSTREAM_TIMEOUT",
            Self::Blocked { .. } => "SOURCE_BLOCKED",
            Self::AdultHidden(_) => "ADULT_HIDDEN",
            Self::Parse { .. } => "UPSTREAM_PARSE",
            Self::RateLimited { .. } => "RATE_LIMITED",
            Self::Internal(_) => "INTERNAL",
        }
    }

    pub fn status(&self) -> StatusCode {
        match self {
            Self::Validation(_) | Self::UnknownSource(_) => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Unsupported(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::SourceNotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            Self::Upstream { .. } | Self::Parse { .. } => StatusCode::BAD_GATEWAY,
            Self::UpstreamTimeout { .. } => StatusCode::GATEWAY_TIMEOUT,
            Self::Blocked { .. } => StatusCode::SERVICE_UNAVAILABLE,
            Self::AdultHidden(_) => StatusCode::FORBIDDEN,
            Self::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn source_name(&self) -> Option<String> {
        match self {
            Self::Upstream { site, .. } => site.clone(),
            Self::UpstreamTimeout { site } | Self::Blocked { site } | Self::Parse { site, .. } => {
                Some(site.clone())
            }
            Self::UnknownSource(s) | Self::SourceNotImplemented(s) | Self::AdultHidden(s) => {
                Some(s.clone())
            }
            _ => None,
        }
    }

    pub fn detail(&self) -> ErrorDetail {
        let message = match self {
            // Never leak internal error chains to clients.
            Self::Internal(e) => {
                tracing::error!(error = ?e, "internal error");
                "internal server error".to_string()
            }
            other => other.to_string(),
        };
        ErrorDetail {
            code: self.code(),
            message,
            source: self.source_name(),
        }
    }

    /// Map a reqwest failure into an upstream error for `source`.
    pub fn from_reqwest(source: &str, err: reqwest::Error) -> Self {
        if err.is_timeout() {
            return Self::UpstreamTimeout {
                site: source.to_string(),
            };
        }
        let message = if let Some(status) = err.status() {
            format!("{source} responded with HTTP {status}")
        } else if err.is_connect() {
            format!("could not connect to {source}")
        } else if err.is_decode() {
            return Self::Parse {
                site: source.to_string(),
                message: err.to_string(),
            };
        } else {
            format!("request to {source} failed")
        };
        Self::Upstream {
            site: Some(source.to_string()),
            message,
        }
    }

    /// Map a wreq (impersonating client) failure into an upstream error.
    pub fn from_wreq(source: &str, err: wreq::Error) -> Self {
        if err.is_timeout() {
            return Self::UpstreamTimeout {
                site: source.to_string(),
            };
        }
        let message = if err.is_connect() || err.is_connection_reset() {
            format!("could not connect to {source}")
        } else {
            format!("request to {source} failed: {err}")
        };
        Self::Upstream {
            site: Some(source.to_string()),
            message,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let retry_after = match &self {
            Self::RateLimited { retry_after_s } => Some(*retry_after_s),
            _ => None,
        };
        let body = ErrorBody {
            error: self.detail(),
        };
        let mut resp = (status, Json(body)).into_response();
        if let Some(secs) = retry_after {
            resp.headers_mut()
                .insert(axum::http::header::RETRY_AFTER, secs.into());
        }
        resp
    }
}

pub type AppResult<T> = Result<T, AppError>;
