//! Optimization Tauri commands. Branch 5 wires the real async queue with
//! progress events; Branch 4's synchronous version is preserved for the
//! one-shot path used by tests and the early integration layer.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::task;

use tinydrop_core::queue::QueueItem;
use tinydrop_core::{optimizer::detect_format, AdaptiveOptimizer, CompressionPreset, ImageFormat};

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

/// Frontend-facing mirror of [`tinydrop_core::ScoredCandidate`].
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

impl From<tinydrop_core::ScoredCandidate> for ScoredCandidateDto {
    fn from(s: tinydrop_core::ScoredCandidate) -> Self {
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

    let mut ids = Vec::with_capacity(args.paths.len());
    for raw in args.paths {
        let path = PathBuf::from(raw);
        let id = queue.enqueue(path, preset).map_err(|e| format!("{e}"))?;
        spawn_queue_poller(app.clone(), id.clone(), queue.clone());
        ids.push(id);
    }

    Ok(ids)
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
pub async fn queue_stats(ctx: &AppContext) -> Result<tinydrop_core::queue::QueueStats, String> {
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
    queue: std::sync::Arc<tinydrop_core::QueueProcessor>,
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
            use tinydrop_core::queue::JobStatus;
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
