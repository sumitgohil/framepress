//! FramePress desktop — Tauri v2 shell.
//!
//! All business logic lives in `framepress-core`. This crate wires the
//! [`AdaptiveOptimizer`] (and the queue/history/settings modules added in
//! later branches) into Tauri commands that the Svelte frontend invokes
//! through the typed IPC layer.

pub mod commands;
pub mod context;
pub mod mcp;

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

#[cfg(not(target_os = "macos"))]
use tauri::{
    menu::PredefinedMenuItem,
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
};

use crate::context::AppContext;

/// Application entry point. Called from `main.rs`.
pub fn run() {
    // Initialize structured logging. RUST_LOG controls the filter.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("framepress=info,warn")),
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
            let ctx = AppContext::build(app.handle().clone()).map_err(|e| {
                Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error>
            })?;
            let queue = ctx.queue();
            let mcp = ctx.agent_access();
            app.manage(ctx);

            // `setup` runs before Tauri enters its async runtime, so defer
            // the queue worker until the runtime is available.
            tauri::async_runtime::spawn(async move {
                queue.start();
            });
            tauri::async_runtime::spawn(async move {
                if mcp.config().await.enabled {
                    if let Err(error) = mcp.start().await {
                        tracing::warn!(%error, "could not start configured MCP server");
                    }
                }
            });

            setup_tray(app.handle())?;
            #[cfg(target_os = "macos")]
            keep_running_in_menu_bar(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::version,
            commands::show_main_window,
            commands::optimize_paths,
            commands::optimize_one,
            commands::export_webp_copy,
            commands::existing_webp_copy,
            commands::cancel_job,
            commands::pause_queue,
            commands::resume_queue,
            commands::queue_snapshot,
            commands::queue_stats,
            commands::recent_history,
            commands::stats_snapshot,
            commands::analytics_snapshot,
            commands::get_active_preset,
            commands::set_active_preset,
            commands::mcp_config,
            commands::mcp_status,
            commands::set_mcp_enabled,
            commands::update_mcp_config,
            commands::rotate_mcp_token,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Reveal and focus the main dashboard window. No-op if the window has
/// already been destroyed (e.g., during shutdown).
fn show_main_window(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        window.set_focus()?;
    }
    Ok(())
}

/// Configure the macOS menu-bar control for the local MCP service.
///
/// Includes an "Open Dashboard" item so users can reopen the main window
/// after it has been hidden by the close button. The window is normally
/// hidden rather than quit on macOS, so without this entry there is no way
/// to bring the dashboard back once the X is clicked.
#[cfg(target_os = "macos")]
fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let open_dashboard =
        MenuItem::with_id(app, "open-dashboard", "Open Dashboard", true, None::<&str>)?;
    let toggle_mcp = MenuItem::with_id(app, "toggle-mcp", "Start MCP", true, None::<&str>)?;
    let exit = MenuItem::with_id(app, "exit", "Exit FramePress", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_dashboard, &toggle_mcp, &exit])?;

    // The bundled app icon is the full-colour brand artwork. Keep the
    // monochrome artwork exclusively for the compact menu-bar presentation.
    let icon = tauri::include_image!("icons/tray-icon.png");
    let menu_item = toggle_mcp.clone();
    let mut tray = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .tooltip("FramePress")
        // Use the supplied monochrome mark as a macOS template image so it
        // stays legible in both the light and dark menu-bar appearances.
        .icon_as_template(true)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "open-dashboard" => {
                let _ = show_main_window(app);
            }
            "toggle-mcp" => {
                let app = app.clone();
                let menu_item = menu_item.clone();
                tauri::async_runtime::spawn(async move {
                    let manager = app.state::<AppContext>().agent_access();
                    let should_start = !manager.status().await.running;
                    match manager.set_enabled(should_start).await {
                        Ok(status) => {
                            let label = if status.running { "Stop MCP" } else { "Start MCP" };
                            let _ = menu_item.set_text(label);
                        }
                        Err(error) => tracing::warn!(%error, "could not toggle MCP server from menu bar"),
                    }
                });
            }
            "exit" => app.exit(0),
            _ => {}
        });
    tray = tray.icon(icon);
    tray.build(app)?;

    // The server may have been enabled in a previous session. Reflect its
    // actual runtime state once startup has completed.
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let status = app.state::<AppContext>().agent_access().status().await;
        let _ = toggle_mcp.set_text(if status.running { "Stop MCP" } else { "Start MCP" });
    });
    Ok(())
}

/// Closing the main window hides it on macOS so an active MCP server remains
/// available from the menu bar. Selecting "Exit FramePress" ends the process.
#[cfg(target_os = "macos")]
fn keep_running_in_menu_bar(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let window_to_hide = window.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window_to_hide.hide();
            }
        });
    }
}

#[cfg(not(target_os = "macos"))]
fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let open_main = MenuItem::with_id(app, "open-main", "Open FramePress", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit FramePress", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open_main, &separator, &quit])?;

    let icon = app.default_window_icon().cloned();
    let mut tray = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .tooltip("FramePress")
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
