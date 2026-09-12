//! In-process download task engine. Replaces Celery + Redis.
//!
//! A task moves PENDING -> PROGRESS -> SUCCESS | FAILURE | CANCELLED. Status is
//! published on a `watch` channel so polling and SSE read the same value.
//! Every task owns a work directory under `DOWNLOAD_DIR/<task_id>` that is
//! deleted on failure, on cancel, or by the sweeper once the retention window
//! closes. A finished archive is NOT deleted when it is downloaded: the browser
//! may lose the transfer on the way, and a second attempt has to be able to
//! fetch the same file instead of rebuilding it.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::{Semaphore, watch};
use tokio_util::sync::CancellationToken;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    model::{FetchTier, Format},
    pipeline::{self, ChapterDir, Packaged, images, safe_label},
    state::SharedState,
};

/// Seconds allowed per chapter before the tier multiplier.
const SECONDS_PER_CHAPTER: u64 = 240;
const MIN_TIMEOUT: Duration = Duration::from_secs(600);
/// How long a finished archive stays downloadable, counted from completion.
/// Downloading it does not shorten this; the window is the only lifetime.
const SUCCESS_RETENTION: Duration = Duration::from_secs(900);
/// How long failed and cancelled entries stay visible to status polls.
const TERMINAL_RETENTION: Duration = Duration::from_secs(600);
const SWEEP_INTERVAL: Duration = Duration::from_secs(60);
pub const MAX_CHAPTERS_PER_TASK: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChapterRef {
    pub id: String,
    /// Chapter label used for file names ("12", "12.5").
    pub number: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DownloadRequest {
    /// Source id or slug.
    pub source: String,
    #[serde(default = "default_title")]
    pub comic_title: String,
    #[serde(default = "default_format")]
    pub format: Format,
    pub chapters: Vec<ChapterRef>,
    #[serde(default)]
    pub lang: Option<String>,
}

fn default_title() -> String {
    "Chapters".into()
}
fn default_format() -> Format {
    Format::Pdf
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskState {
    Pending,
    Progress,
    Success,
    Failure,
    Cancelled,
}

impl TaskState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Success | Self::Failure | Self::Cancelled)
    }
}

/// Snapshot of a task. Superset of the shape the old client polls.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TaskStatus {
    pub task_id: Uuid,
    pub state: TaskState,
    /// Human readable progress message.
    pub status: String,
    /// 0-100.
    pub progress: u8,
    pub chapter_current: usize,
    pub chapter_total: usize,
    /// Legacy alias of `chapter_total`.
    pub total_chapters: usize,
    pub comic_title: String,
    pub source: String,
    pub format: Format,
    /// Unix seconds.
    pub created_at: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    /// Legacy alias: the old client never used it, but the old API returned it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Chapters that were skipped because their pages could not be resolved.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub warnings: Vec<String>,
}

struct TaskEntry {
    status: watch::Sender<TaskStatus>,
    cancel: CancellationToken,
    workdir: PathBuf,
    packaged: Mutex<Option<Packaged>>,
    finished_at: Mutex<Option<Instant>>,
}

impl TaskEntry {
    fn update(&self, f: impl FnOnce(&mut TaskStatus)) {
        self.status.send_modify(f);
    }
    fn finish(&self) {
        *self.finished_at.lock().unwrap() = Some(Instant::now());
    }
}

pub struct TaskEngine {
    tasks: DashMap<Uuid, Arc<TaskEntry>>,
    semaphore: Arc<Semaphore>,
    download_dir: PathBuf,
    image_concurrency: usize,
    pub rar_available: bool,
}

/// What `/download/file` needs to stream an archive.
pub struct Collectable {
    pub packaged: Packaged,
    pub comic_title: String,
}

impl TaskEngine {
    pub async fn new(
        download_dir: PathBuf,
        max_concurrent: usize,
        image_concurrency: usize,
    ) -> anyhow::Result<Self> {
        tokio::fs::create_dir_all(&download_dir).await?;
        let rar_available = pipeline::cbr::rar_available().await;
        Ok(Self {
            tasks: DashMap::new(),
            semaphore: Arc::new(Semaphore::new(max_concurrent.max(1))),
            download_dir,
            image_concurrency,
            rar_available,
        })
    }

    /// Delete work directories left behind by a previous process.
    pub async fn remove_orphans(&self) {
        let Ok(mut entries) = tokio::fs::read_dir(&self.download_dir).await else {
            return;
        };
        let mut removed = 0;
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name();
            let is_task_dir = name
                .to_str()
                .and_then(|n| Uuid::parse_str(n).ok())
                .is_some();
            if is_task_dir && tokio::fs::remove_dir_all(entry.path()).await.is_ok() {
                removed += 1;
            }
        }
        if removed > 0 {
            tracing::info!(removed, "removed orphaned download directories");
        }
    }

    pub fn active_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|e| !e.status.borrow().state.is_terminal())
            .count()
    }

    pub fn list(&self) -> Vec<TaskStatus> {
        let mut v: Vec<TaskStatus> = self
            .tasks
            .iter()
            .map(|e| e.status.borrow().clone())
            .collect();
        v.sort_by_key(|s| std::cmp::Reverse(s.created_at));
        v
    }

    pub fn status(&self, id: Uuid) -> AppResult<TaskStatus> {
        self.entry(id).map(|e| e.status.borrow().clone())
    }

    pub fn subscribe(&self, id: Uuid) -> AppResult<watch::Receiver<TaskStatus>> {
        self.entry(id).map(|e| e.status.subscribe())
    }

    fn entry(&self, id: Uuid) -> AppResult<Arc<TaskEntry>> {
        self.tasks
            .get(&id)
            .map(|e| Arc::clone(&e))
            .ok_or_else(|| AppError::NotFound(format!("task {id} not found")))
    }

    pub fn cancel(&self, id: Uuid) -> AppResult<()> {
        let entry = self.entry(id)?;
        if entry.status.borrow().state.is_terminal() {
            return Err(AppError::Conflict(format!(
                "task {id} has already finished"
            )));
        }
        entry.cancel.cancel();
        Ok(())
    }

    /// Resolve the archive of a finished task. 409 while it is still running,
    /// 404 once the retention window has closed and the task is gone.
    pub fn collectable(&self, id: Uuid) -> AppResult<Collectable> {
        let entry = self.entry(id)?;
        let status = entry.status.borrow().clone();
        match status.state {
            TaskState::Success => {}
            TaskState::Pending | TaskState::Progress => {
                return Err(AppError::Conflict(format!(
                    "task {id} is still running ({}%)",
                    status.progress
                )));
            }
            TaskState::Failure | TaskState::Cancelled => {
                return Err(AppError::NotFound(format!(
                    "task {id} did not produce a file"
                )));
            }
        }
        let packaged = entry
            .packaged
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| AppError::NotFound(format!("task {id} has no file")))?;
        Ok(Collectable {
            packaged,
            comic_title: status.comic_title,
        })
    }

    /// Validate and queue a download. Returns the initial status.
    pub fn spawn(&self, state: SharedState, req: DownloadRequest) -> AppResult<TaskStatus> {
        if req.chapters.is_empty() {
            return Err(AppError::Validation("select at least one chapter".into()));
        }
        if req.chapters.len() > MAX_CHAPTERS_PER_TASK {
            return Err(AppError::Validation(format!(
                "at most {MAX_CHAPTERS_PER_TASK} chapters per download"
            )));
        }
        if req.comic_title.chars().count() > 200 {
            return Err(AppError::Validation("comic title is too long".into()));
        }
        if !pipeline::format_supported(req.format, self.rar_available) {
            return Err(AppError::Unsupported(
                "CBR output needs the `rar` binary, which is not installed on this server".into(),
            ));
        }
        let source = state.registry.get(&req.source)?;
        if !source.meta().capabilities.download {
            return Err(AppError::SourceNotImplemented(
                source.meta().slug.to_string(),
            ));
        }

        let id = Uuid::new_v4();
        let workdir = self.download_dir.join(id.to_string());
        let initial = TaskStatus {
            task_id: id,
            state: TaskState::Pending,
            status: "Task is waiting in the queue...".into(),
            progress: 0,
            chapter_current: 0,
            chapter_total: req.chapters.len(),
            total_chapters: req.chapters.len(),
            comic_title: req.comic_title.clone(),
            source: source.meta().slug.to_string(),
            format: req.format,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            file_size: None,
            content_type: None,
            file_name: None,
            zip_path: None,
            error: None,
            warnings: Vec::new(),
        };
        let (tx, _rx) = watch::channel(initial.clone());
        let entry = Arc::new(TaskEntry {
            status: tx,
            cancel: CancellationToken::new(),
            workdir: workdir.clone(),
            packaged: Mutex::new(None),
            finished_at: Mutex::new(None),
        });
        self.tasks.insert(id, Arc::clone(&entry));

        let multiplier = match source.meta().tier {
            FetchTier::Plain => 1.0,
            FetchTier::Impersonate => 1.3,
            FetchTier::Browser => 1.5,
        };
        let timeout = Duration::from_secs_f64(
            (SECONDS_PER_CHAPTER * req.chapters.len() as u64) as f64 * multiplier,
        )
        .max(MIN_TIMEOUT);

        let semaphore = Arc::clone(&self.semaphore);
        let image_concurrency = self.image_concurrency;
        tokio::spawn(async move {
            let span = tracing::info_span!("download_task", %id, source = source.meta().slug, chapters = req.chapters.len());
            let _guard = span.enter();
            drop(_guard);
            let cancel = entry.cancel.clone();
            let work = run(
                Arc::clone(&state),
                Arc::clone(&entry),
                &req,
                source,
                semaphore,
                image_concurrency,
            );
            let outcome = tokio::select! {
                res = tokio::time::timeout(timeout, work) => match res {
                    Ok(r) => r,
                    Err(_) => Err(AppError::UpstreamTimeout { site: "the download".into() }),
                },
                _ = cancel.cancelled() => {
                    entry.update(|s| {
                        s.state = TaskState::Cancelled;
                        s.status = "Cancelled".into();
                    });
                    entry.finish();
                    let _ = tokio::fs::remove_dir_all(&workdir).await;
                    tracing::info!(%id, "task cancelled");
                    return;
                }
            };
            match outcome {
                Ok(packaged) => {
                    let size = tokio::fs::metadata(&packaged.path)
                        .await
                        .map(|m| m.len())
                        .unwrap_or(0);
                    let file_name = pipeline::ascii_filename(&req.comic_title, packaged.extension);
                    *entry.packaged.lock().unwrap() = Some(packaged.clone());
                    entry.update(|s| {
                        s.state = TaskState::Success;
                        s.status = "Completed".into();
                        s.progress = 100;
                        s.chapter_current = s.chapter_total;
                        s.file_size = Some(size);
                        s.content_type = Some(packaged.content_type.to_string());
                        s.file_name = Some(file_name);
                        s.zip_path = Some(packaged.path.display().to_string());
                    });
                    entry.finish();
                    tracing::info!(%id, size, "task finished");
                }
                Err(err) => {
                    let message = err.detail().message;
                    tracing::warn!(%id, error = %message, "task failed");
                    entry.update(|s| {
                        s.state = TaskState::Failure;
                        s.status = "Error".into();
                        s.error = Some(message);
                    });
                    entry.finish();
                    let _ = tokio::fs::remove_dir_all(&workdir).await;
                }
            }
        });
        Ok(initial)
    }

    /// Periodic cleanup. Runs for the life of the process.
    pub async fn sweep_forever(self: Arc<Self>) {
        let mut interval = tokio::time::interval(SWEEP_INTERVAL);
        loop {
            interval.tick().await;
            let now = Instant::now();
            let mut expired = Vec::new();
            for e in self.tasks.iter() {
                let state = e.status.borrow().state;
                if !state.is_terminal() {
                    continue;
                }
                let finished = e.finished_at.lock().unwrap().unwrap_or(now);
                let age = now.duration_since(finished);
                let retention = if state == TaskState::Success {
                    SUCCESS_RETENTION
                } else {
                    TERMINAL_RETENTION
                };
                if age >= retention {
                    expired.push((*e.key(), e.workdir.clone()));
                }
            }
            for (id, dir) in expired {
                self.tasks.remove(&id);
                let _ = tokio::fs::remove_dir_all(&dir).await;
                tracing::debug!(%id, "swept task");
            }
        }
    }
}

async fn run(
    state: SharedState,
    entry: Arc<TaskEntry>,
    req: &DownloadRequest,
    source: Arc<dyn crate::sources::Source>,
    semaphore: Arc<Semaphore>,
    image_concurrency: usize,
) -> AppResult<Packaged> {
    let _permit = semaphore
        .acquire()
        .await
        .map_err(|_| AppError::Internal(anyhow::anyhow!("task semaphore closed")))?;
    entry.update(|s| {
        s.state = TaskState::Progress;
        s.status = "Starting download...".into();
    });
    tokio::fs::create_dir_all(&entry.workdir).await?;

    let ctx = state.ctx();
    let total = req.chapters.len();
    let mut dirs: Vec<ChapterDir> = Vec::with_capacity(total);
    for (i, ch) in req.chapters.iter().enumerate() {
        entry.update(|s| {
            s.status = format!("Downloading chapter {}/{}", i + 1, total);
            s.progress = ((i * 90) / total) as u8;
            s.chapter_current = i;
        });
        let pages = match source.chapter_pages(&ctx, &ch.id).await {
            Ok(p) => p,
            Err(e) => {
                let msg = format!("Chapter {} skipped: {}", ch.number, e.detail().message);
                tracing::warn!(chapter = %ch.number, error = %e, "chapter skipped");
                entry.update(|s| s.warnings.push(msg));
                continue;
            }
        };
        let dir = entry
            .workdir
            .join(format!("{:04}_{}", i + 1, safe_label(&ch.number)));
        match images::download_chapter(
            &state.fetcher,
            source.meta().tier,
            &pages,
            &dir,
            image_concurrency,
        )
        .await
        {
            Ok(files) => dirs.push(ChapterDir {
                number: ch.number.clone(),
                path: dir,
                pages: files,
            }),
            Err(e) => {
                let msg = format!("Chapter {} skipped: {}", ch.number, e.detail().message);
                tracing::warn!(chapter = %ch.number, error = %e, "chapter skipped");
                entry.update(|s| s.warnings.push(msg));
                let _ = tokio::fs::remove_dir_all(&dir).await;
            }
        }
    }
    if dirs.is_empty() {
        return Err(AppError::Upstream {
            site: Some(source.meta().slug.to_string()),
            message: "no chapter could be downloaded".into(),
        });
    }

    let progress_entry = Arc::clone(&entry);
    let on_progress = move |msg: &str| {
        progress_entry.update(|s| {
            s.status = msg.to_string();
            s.progress = s.progress.max(92);
            s.chapter_current = s.chapter_total;
        });
    };
    pipeline::package(
        req.format,
        &entry.workdir,
        dirs,
        &req.comic_title,
        &on_progress,
    )
    .await
}
