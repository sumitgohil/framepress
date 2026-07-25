//! Tauri command handlers. Each command is a thin async function that pulls
//! the [`AppContext`] out of Tauri state and delegates to it.

mod history;
mod optimize;
mod settings;

use tauri::{Manager, State};

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

/// Reveal and focus the main dashboard window. Used by the compact tray widget.
#[tauri::command]
pub fn show_main_window(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window is unavailable".to_string())?;
    window.show().map_err(|error| error.to_string())?;
    window.set_focus().map_err(|error| error.to_string())
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
) -> Result<Vec<framepress_core::queue::QueueItem>, String> {
    optimize::queue_snapshot(&ctx).await
}

#[tauri::command]
pub async fn queue_stats(
    ctx: State<'_, AppContext>,
) -> Result<framepress_core::queue::QueueStats, String> {
    optimize::queue_stats(&ctx).await
}

#[tauri::command]
pub async fn optimize_one(
    args: optimize::OptimizeOneArgs,
    ctx: State<'_, AppContext>,
) -> Result<optimize::ScoredCandidateDto, String> {
    optimize::optimize_one(args, &ctx).await
}

/// Create an explicitly requested WebP copy alongside an existing PNG/JPEG.
#[tauri::command]
pub async fn export_webp_copy(
    input_path: String,
    preset: framepress_core::CompressionPreset,
    app: tauri::AppHandle,
    ctx: State<'_, AppContext>,
) -> Result<optimize::WebpCopyDto, String> {
    optimize::export_webp_copy(input_path, preset, app, &ctx).await
}

/// Locate a previously created WebP sibling for a source file.
#[tauri::command]
pub fn existing_webp_copy(input_path: String) -> Option<optimize::WebpCopyDto> {
    optimize::existing_webp_copy(input_path)
}

#[tauri::command]
pub async fn recent_history(
    limit: u32,
    ctx: State<'_, AppContext>,
) -> Result<Vec<framepress_core::history::HistoryEntry>, String> {
    history::recent_history_inner(limit, ctx).await
}

#[tauri::command]
pub async fn stats_snapshot(
    ctx: State<'_, AppContext>,
) -> Result<framepress_core::history::StatsSnapshot, String> {
    history::stats_snapshot_inner(ctx).await
}

/// Statistics page payload for the requested local time range.
#[tauri::command]
pub async fn analytics_snapshot(
    range: framepress_core::history::AnalyticsRange,
    ctx: State<'_, AppContext>,
) -> Result<framepress_core::history::AnalyticsSnapshot, String> {
    history::analytics_snapshot_inner(range, ctx).await
}

#[tauri::command]
pub async fn get_active_preset(
    ctx: State<'_, AppContext>,
) -> Result<framepress_core::CompressionPreset, String> {
    settings::get_active_preset_inner(ctx).await
}

#[tauri::command]
pub async fn set_active_preset(
    preset: framepress_core::CompressionPreset,
    ctx: State<'_, AppContext>,
) -> Result<framepress_core::CompressionPreset, String> {
    settings::set_active_preset_inner(preset, ctx).await
}

/// Read the local MCP server configuration (the token is masked in the UI).
#[tauri::command]
pub async fn mcp_config(
    ctx: State<'_, AppContext>,
) -> Result<crate::mcp::AgentAccessConfig, String> {
    Ok(ctx.agent_access().config().await)
}

/// Enable or disable FramePress's loopback-only MCP endpoint.
#[tauri::command]
pub async fn set_mcp_enabled(
    enabled: bool,
    ctx: State<'_, AppContext>,
) -> Result<crate::mcp::McpServerStatus, String> {
    ctx.agent_access().set_enabled(enabled).await
}

/// Return endpoint state for Settings and the connection test.
#[tauri::command]
pub async fn mcp_status(ctx: State<'_, AppContext>) -> Result<crate::mcp::McpServerStatus, String> {
    Ok(ctx.agent_access().status().await)
}

/// Change local MCP configuration. The server restarts if its port changed.
#[tauri::command]
pub async fn update_mcp_config(
    config: crate::mcp::AgentAccessConfig,
    ctx: State<'_, AppContext>,
) -> Result<crate::mcp::AgentAccessConfig, String> {
    let was_running = ctx.agent_access().status().await.running;
    if was_running {
        ctx.agent_access().stop().await;
    }
    let next = ctx.agent_access().update_config(config).await?;
    if next.enabled {
        ctx.agent_access().start().await?;
    }
    Ok(next)
}

/// Generate a fresh local bearer token and restart the endpoint.
#[tauri::command]
pub async fn rotate_mcp_token(
    ctx: State<'_, AppContext>,
) -> Result<crate::mcp::AgentAccessConfig, String> {
    ctx.agent_access().rotate_token().await
}
