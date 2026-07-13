//! Optimization Tauri commands. Branch 5 wires the real async queue with
//! progress events; Branch 4's synchronous version is preserved for the
//! one-shot path used by tests and the early integration layer.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::task;

use framepress_core::queue::QueueItem;
use framepress_core::{
    optimizer::detect_format, AdaptiveOptimizer, CompressionPreset, ImageFormat,
};

use crate::context::AppContext;

/// Args for `optimize_paths` command. Matches the TS shape.
#[derive(Debug, Clone, Deserialize)]
pub struct OptimizePathsArgs {
    /// File paths to enqueue.
    pub paths: Vec<String>,
    /// Active preset.
    pub preset: CompressionPreset,
}

/// Args for the one-shot `optimize_one` command.
#[derive(Debug, Clone, Deserialize)]
pub struct OptimizeOneArgs {
    pub input_path: String,
    pub preset: CompressionPreset,
    pub output_path: String,
}

/// Details of an explicitly requested WebP copy.
#[derive(Debug, Clone, Serialize)]
pub struct WebpCopyDto {
    pub output_path: String,
    pub optimized_bytes: u64,
}

/// Expand dropped files and folders into a stable, de-duplicated image list.
///
/// Folder traversal intentionally happens in the desktop process rather than
/// the frontend: native drag-and-drop supplies filesystem paths directly, and
/// this keeps file picking and dropping on the same path. Symlinked
/// directories are skipped to prevent directory cycles.
fn expand_image_paths(paths: impl IntoIterator<Item = PathBuf>) -> Result<Vec<PathBuf>, String> {
    let mut pending: Vec<PathBuf> = paths.into_iter().collect();
    let mut images = BTreeSet::new();

    while let Some(path) = pending.pop() {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("Could not access {}: {error}", path.display()))?;

        if metadata.file_type().is_symlink() {
            continue;
        }

        if metadata.is_dir() {
            let entries = fs::read_dir(&path)
                .map_err(|error| format!("Could not read folder {}: {error}", path.display()))?;
            for entry in entries {
                let entry = entry.map_err(|error| {
                    format!("Could not read an item in {}: {error}", path.display())
                })?;
                pending.push(entry.path());
            }
            continue;
        }

        if metadata.is_file()
            && ImageFormat::from_path(&path).is_some()
            && !path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| stem.ends_with("-framepress"))
        {
            images.insert(path);
        }
    }

    Ok(images.into_iter().collect())
}

/// Frontend-facing mirror of [`framepress_core::ScoredCandidate`].
#[derive(Debug, Clone, Serialize)]
pub struct ScoredCandidateDto {
    pub engine: String,
    pub output_path: String,
    pub format: ImageFormat,
    pub original_bytes: u64,
    pub optimized_bytes: u64,
    pub dssim: Option<f64>,
    pub duration_ms: u64,
    pub passed_quality_gate: bool,
    pub margin_pct_vs_runner_up: Option<f64>,
}

impl From<framepress_core::ScoredCandidate> for ScoredCandidateDto {
    fn from(s: framepress_core::ScoredCandidate) -> Self {
        Self {
            engine: s.result.engine,
            output_path: s.result.output_path.to_string_lossy().to_string(),
            format: s.result.format,
            original_bytes: s.result.original_bytes,
            optimized_bytes: s.result.optimized_bytes,
            dssim: s.result.dssim,
            duration_ms: s.result.duration_ms,
            passed_quality_gate: s.passed_quality_gate,
            margin_pct_vs_runner_up: s.margin_pct_vs_runner_up,
        }
    }
}

/// Enqueue paths and return their IDs. Each item runs through the
/// [`QueueProcessor`] which emits `queue:item_updated` events as it
/// progresses.
pub async fn optimize_paths(
    args: OptimizePathsArgs,
    app: AppHandle,
    ctx: &AppContext,
) -> Result<Vec<String>, String> {
    ctx.set_active_preset(args.preset).await;
    let preset = args.preset;
    let queue = ctx.queue();

    let paths = expand_image_paths(args.paths.into_iter().map(PathBuf::from))?;
    if paths.is_empty() {
        return Err("No supported images were found in the selected files or folders.".to_string());
    }

    let mut ids = Vec::with_capacity(paths.len());
    for path in paths {
        let id = queue.enqueue(path, preset).map_err(|e| format!("{e}"))?;
        spawn_queue_poller(app.clone(), id.clone(), queue.clone());
        ids.push(id);
    }

    Ok(ids)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::{expand_image_paths, webp_copy_output_path};

    #[test]
    fn expands_nested_folders_and_skips_unsupported_or_generated_files() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("nested");
        fs::create_dir(&nested).unwrap();

        let top_level = dir.path().join("cover.PNG");
        let nested_image = nested.join("photo.jpeg");
        fs::write(&top_level, []).unwrap();
        fs::write(&nested_image, []).unwrap();
        fs::write(dir.path().join("notes.txt"), []).unwrap();
        fs::write(nested.join("photo-framepress.webp"), []).unwrap();

        let paths = expand_image_paths([dir.path().to_path_buf()]).unwrap();

        assert_eq!(paths, vec![top_level, nested_image]);
    }

    #[test]
    fn de_duplicates_files_selected_directly_and_through_a_folder() {
        let dir = tempfile::tempdir().unwrap();
        let image = dir.path().join("cover.png");
        fs::write(&image, []).unwrap();

        let paths = expand_image_paths([dir.path().to_path_buf(), image.clone()]).unwrap();

        assert_eq!(paths, vec![image]);
    }

    #[test]
    fn webp_copy_path_is_a_sibling_with_a_webp_extension() {
        assert_eq!(
            webp_copy_output_path(Path::new("/images/banner.png")),
            Path::new("/images/banner-framepress.webp")
        );
    }
}

/// Cancel a queued or running job.
pub async fn cancel_job(job_id: String, ctx: &AppContext) -> Result<(), String> {
    ctx.queue().cancel(&job_id);
    Ok(())
}

/// Pause processing. In-flight jobs run to completion; new items wait.
pub async fn pause_queue(ctx: &AppContext) -> Result<(), String> {
    ctx.queue().pause();
    Ok(())
}

/// Resume processing.
pub async fn resume_queue(ctx: &AppContext) -> Result<(), String> {
    ctx.queue().resume();
    Ok(())
}

/// Snapshot the queue for the UI.
pub async fn queue_snapshot(ctx: &AppContext) -> Result<Vec<QueueItem>, String> {
    Ok(ctx.queue_snapshot())
}

/// Aggregate queue stats.
pub async fn queue_stats(ctx: &AppContext) -> Result<framepress_core::queue::QueueStats, String> {
    Ok(ctx.queue_stats())
}

/// One-shot optimize call. Used by tests and by the early integration path.
pub async fn optimize_one(
    args: OptimizeOneArgs,
    ctx: &AppContext,
) -> Result<ScoredCandidateDto, String> {
    let optimizer = ctx.optimizer().clone();
    let input = PathBuf::from(args.input_path);
    let output = PathBuf::from(args.output_path);
    let preset = args.preset;

    let result = task::spawn_blocking(move || optimizer.optimize(&input, preset, &output))
        .await
        .map_err(|e| format!("worker panicked: {e}"))?
        .map_err(|e| format!("{e}"))?;

    Ok(result.into())
}

/// Create a WebP sibling only after the user explicitly requests it.
pub async fn export_webp_copy(
    input_path: String,
    preset: CompressionPreset,
    app: AppHandle,
    ctx: &AppContext,
) -> Result<WebpCopyDto, String> {
    let input = PathBuf::from(input_path);
    let format = detect_format(&input).map_err(|error| error.to_string())?;
    if !matches!(format, ImageFormat::Png | ImageFormat::Jpeg) {
        return Err("WebP copies can be created from PNG or JPEG files only.".to_string());
    }

    let output = webp_copy_output_path(&input);
    if let Some(copy) = existing_webp_copy(input.to_string_lossy().to_string()) {
        return Ok(copy);
    }
    let settings = ctx.optimizer().resolve_settings(preset, format);
    let optimizer = ctx.optimizer().clone();
    let input_for_task = input.clone();
    let output_for_task = output.clone();

    let result = task::spawn_blocking(move || {
        optimizer.run_single("webp", &input_for_task, &output_for_task, &settings)
    })
    .await
    .map_err(|error| format!("WebP export worker panicked: {error}"))?
    .map_err(|error| error.to_string())?;

    let copy = WebpCopyDto {
        output_path: result.output_path.to_string_lossy().to_string(),
        optimized_bytes: result.optimized_bytes,
    };
    let queue_item = ctx.queue().record_completed_export(input, preset, result);
    let _ = app.emit("queue:item_updated", queue_item);

    Ok(copy)
}

/// Return an already-created WebP sibling, if it still exists on disk.
pub fn existing_webp_copy(input_path: String) -> Option<WebpCopyDto> {
    let output = webp_copy_output_path(Path::new(&input_path));
    let metadata = std::fs::metadata(&output).ok()?;
    if !metadata.is_file() {
        return None;
    }
    Some(WebpCopyDto {
        output_path: output.to_string_lossy().to_string(),
        optimized_bytes: metadata.len(),
    })
}

fn webp_copy_output_path(input: &Path) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("image");
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{stem}-framepress.webp"))
}

#[allow(dead_code)]
fn run_one(
    optimizer: &AdaptiveOptimizer,
    input: &Path,
    preset: CompressionPreset,
) -> anyhow::Result<ScoredCandidateDto> {
    let output = AdaptiveOptimizer::plan_output_path(input, detect_format(input)?);
    let result = optimizer.optimize(input, preset, &output)?;
    Ok(result.into())
}

/// Polls a queue item and emits an event for its initial, running, and
/// terminal states. The initial event is important: a fast job can otherwise
/// finish before the Queue page has a chance to render it.
fn spawn_queue_poller(
    app: AppHandle,
    job_id: String,
    queue: std::sync::Arc<framepress_core::QueueProcessor>,
) {
    tokio::spawn(async move {
        let mut last = None;
        loop {
            let snap = queue.snapshot();
            let Some(item) = snap.into_iter().find(|i| i.id == job_id) else {
                // Item removed (shouldn't happen in v1); stop polling.
                break;
            };

            // Emit immediately, then whenever state or completion time changes.
            let key = (item.status, item.completed_at);
            if last.as_ref() != Some(&key) {
                let _ = app.emit("queue:item_updated", &item);
                last = Some(key);
            }

            // Stop when the item reaches a terminal state.
            use framepress_core::queue::JobStatus;
            if matches!(
                item.status,
                JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
            ) {
                break;
            }

            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        }
    });
}
