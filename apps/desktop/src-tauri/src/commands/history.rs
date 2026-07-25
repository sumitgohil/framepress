//! History + stats Tauri commands. Phase 1: read-only from the SQLite store.

use tauri::State;

use crate::context::AppContext;

pub async fn recent_history_inner(
    limit: u32,
    ctx: State<'_, AppContext>,
) -> Result<Vec<framepress_core::history::HistoryEntry>, String> {
    ctx.history().recent(limit).map_err(|e| format!("{e}"))
}

pub async fn stats_snapshot_inner(
    ctx: State<'_, AppContext>,
) -> Result<framepress_core::history::StatsSnapshot, String> {
    ctx.history().stats().map_err(|e| format!("{e}"))
}

pub async fn analytics_snapshot_inner(
    range: framepress_core::history::AnalyticsRange,
    ctx: State<'_, AppContext>,
) -> Result<framepress_core::history::AnalyticsSnapshot, String> {
    ctx.history().analytics(range).map_err(|e| format!("{e}"))
}
