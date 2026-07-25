//! Settings Tauri commands. Phase 1: in-memory only, persisted via the
//! frontend's localStorage. Branch 7 wires the OS app-config JSON file.

use tauri::State;

use framepress_core::CompressionPreset;

use crate::context::AppContext;

pub async fn get_active_preset_inner(
    ctx: State<'_, AppContext>,
) -> Result<CompressionPreset, String> {
    Ok(ctx.active_preset().await)
}

pub async fn set_active_preset_inner(
    preset: CompressionPreset,
    ctx: State<'_, AppContext>,
) -> Result<CompressionPreset, String> {
    Ok(ctx.set_active_preset(preset).await)
}
