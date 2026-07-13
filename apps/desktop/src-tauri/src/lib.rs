//! TinyDrop desktop — Tauri v2 shell.
//!
//! All business logic lives in `tinydrop-core`. This crate wires the
//! [`AdaptiveOptimizer`] (and the queue/history/settings modules added in
//! later branches) into Tauri commands that the Svelte frontend invokes
//! through the typed IPC layer.

pub mod commands;
pub mod context;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

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

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init());

    builder
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

            setup_tray(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::version,
            commands::show_main_window,
            commands::optimize_paths,
            commands::optimize_one,
            commands::export_webp_copy,
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

fn show_main_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let open_main = MenuItem::with_id(app, "open-main", "Open TinyDrop", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit TinyDrop", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open_main, &separator, &quit])?;

    let icon = app.default_window_icon().cloned();
    let mut tray = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .tooltip("TinyDrop")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open-main" => {
                let _ = show_main_window(app);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = icon {
        tray = tray.icon(icon);
    }
    tray.build(app)?;
    Ok(())
}
