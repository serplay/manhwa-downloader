//! CBR output via the proprietary `rar` CLI, mirroring the old server. Only
//! offered when the binary is on PATH (see `available_formats`).

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use tokio::process::Command;

use super::{ChapterDir, Packaged, Progress, chapter_stem};
use crate::error::{AppError, AppResult};

/// `true` when `rar` can be executed.
pub async fn rar_available() -> bool {
    Command::new("rar")
        .arg("-?")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success() || s.code() == Some(7))
        .unwrap_or(false)
}

async fn rar(cwd: &Path, archive: &str, files: &[String]) -> AppResult<()> {
    let status = Command::new("rar")
        .arg("a")
        .arg("-ep1")
        .arg("-idq")
        .arg(archive)
        .args(files)
        .current_dir(cwd)
        .status()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to run rar: {e}")))?;
    if !status.success() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "rar exited with {status}"
        )));
    }
    Ok(())
}

pub async fn package(
    workdir: &Path,
    chapters: Vec<ChapterDir>,
    progress: Progress<'_>,
) -> AppResult<Packaged> {
    progress("Creating CBR...");
    let mut taken = HashSet::new();
    let mut cbrs = Vec::with_capacity(chapters.len());
    for chapter in &chapters {
        let files: Vec<String> = chapter
            .pages
            .iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
            .collect();
        let name = format!("{}.cbr", chapter_stem(&chapter.number, &mut taken));
        let archive = workdir.join(&name);
        rar(&chapter.path, archive.to_str().unwrap_or(&name), &files).await?;
        cbrs.push(name);
        let _ = tokio::fs::remove_dir_all(&chapter.path).await;
    }
    rar(workdir, "Chapters.rar", &cbrs).await?;
    for c in &cbrs {
        let _ = tokio::fs::remove_file(workdir.join(c)).await;
    }
    let out: PathBuf = workdir.join("Chapters.rar");
    Ok(Packaged {
        path: out,
        content_type: "application/vnd.rar",
        extension: "rar",
    })
}
