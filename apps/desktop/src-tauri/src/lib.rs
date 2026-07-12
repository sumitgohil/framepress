//! TinyDrop desktop — Tauri v2 shell.
//!
//! All business logic lives in `tinydrop-core`. This crate wires the
//! [`AdaptiveOptimizer`] (and the queue/history/settings modules added in
//! later branches) into Tauri commands that the Svelte frontend invokes
//! through the typed IPC layer.

pub mod commands;
pub mod context;

use tauri::Manager;

use crate::context::AppContext;

/// Application entry point. Called from `main.rs`.
pub fn run() {
    // Initialize structured logging. RUST_LOG controls the filter.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("tinydrop=info,warn")),
        )
        .with_target(false)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Build the AppContext once, store it under the well-known state
            // handle so Tauri commands can fetch it via `app.state::<>()`.
            let ctx = AppContext::build().map_err(|e| {
                Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error>
            })?;
            let queue = ctx.queue();
            app.manage(ctx);

            // `setup` runs before Tauri enters its async runtime, so defer
            // the queue worker until the runtime is available.
            tauri::async_runtime::spawn(async move {
                queue.start();
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::version,
            commands::optimize_paths,
            commands::optimize_one,
            commands::cancel_job,
            commands::pause_queue,
            commands::resume_queue,
            commands::queue_snapshot,
            commands::queue_stats,
            commands::recent_history,
            commands::stats_snapshot,
            commands::get_active_preset,
            commands::set_active_preset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
