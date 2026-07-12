//! Tauri command handlers. Each command is a thin async function that pulls
//! the [`AppContext`] out of Tauri state and delegates to it.

mod history;
mod optimize;
mod settings;

use tauri::State;

use crate::context::AppContext;

/// Liveness check. Returns `"pong"`.
#[tauri::command]
pub fn ping() -> &'static str {
    "pong"
}

/// Application version string for the frontend to display.
#[tauri::command]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Enqueue file paths for optimization.
#[tauri::command]
pub async fn optimize_paths(
    args: optimize::OptimizePathsArgs,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<Vec<String>, String> {
    optimize::optimize_paths(args, app, &ctx).await
}

#[tauri::command]
pub async fn cancel_job(job_id: String, ctx: State<'_, AppContext>) -> Result<(), String> {
    optimize::cancel_job(job_id, &ctx).await
}

#[tauri::command]
pub async fn pause_queue(ctx: State<'_, AppContext>) -> Result<(), String> {
    optimize::pause_queue(&ctx).await
}

#[tauri::command]
pub async fn resume_queue(ctx: State<'_, AppContext>) -> Result<(), String> {
    optimize::resume_queue(&ctx).await
}

#[tauri::command]
pub async fn queue_snapshot(
    ctx: State<'_, AppContext>,
) -> Result<Vec<tinydrop_core::queue::QueueItem>, String> {
    optimize::queue_snapshot(&ctx).await
}

#[tauri::command]
pub async fn queue_stats(
    ctx: State<'_, AppContext>,
) -> Result<tinydrop_core::queue::QueueStats, String> {
    optimize::queue_stats(&ctx).await
}

#[tauri::command]
pub async fn optimize_one(
    args: optimize::OptimizeOneArgs,
    ctx: State<'_, AppContext>,
) -> Result<optimize::ScoredCandidateDto, String> {
    optimize::optimize_one(args, &ctx).await
}

#[tauri::command]
pub async fn recent_history(
    limit: u32,
    ctx: State<'_, AppContext>,
) -> Result<Vec<tinydrop_core::history::HistoryEntry>, String> {
    history::recent_history_inner(limit, ctx).await
}

#[tauri::command]
pub async fn stats_snapshot(
    ctx: State<'_, AppContext>,
) -> Result<tinydrop_core::history::StatsSnapshot, String> {
    history::stats_snapshot_inner(ctx).await
}

#[tauri::command]
pub async fn get_active_preset(
    ctx: State<'_, AppContext>,
) -> Result<tinydrop_core::CompressionPreset, String> {
    settings::get_active_preset_inner(ctx).await
}

#[tauri::command]
pub async fn set_active_preset(
    preset: tinydrop_core::CompressionPreset,
    ctx: State<'_, AppContext>,
) -> Result<tinydrop_core::CompressionPreset, String> {
    settings::set_active_preset_inner(preset, ctx).await
}
