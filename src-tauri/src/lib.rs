mod commands;
mod discovery;
mod manager;
mod models;

use manager::AppState;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};
use tauri_plugin_autostart::MacosLauncher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .setup(|app| {
            let state = AppState::load(app.handle())?;
            app.manage(state.clone());

            let open_item = MenuItem::with_id(
                app,
                "open",
                "Open Vibe Manager / Vibe Managerを開く",
                true,
                None::<&str>,
            )?;
            let quit_item = MenuItem::with_id(
                app,
                "quit",
                "Quit Vibe Manager / Vibe Managerを終了",
                true,
                None::<&str>,
            )?;
            let menu = Menu::with_items(app, &[&open_item, &quit_item])?;

            TrayIconBuilder::with_id("main")
                .icon(
                    app.default_window_icon()
                        .expect("default window icon is required")
                        .clone(),
                )
                .tooltip("Vibe Manager")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_main_window(tray.app_handle());
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_main_window(app),
                    "quit" => {
                        app.state::<AppState>().prepare_for_exit(app);
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            if let Some(window) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        let state = app_handle.state::<AppState>();
                        if !state.settings().onboarding_complete {
                            return;
                        }
                        api.prevent_close();
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.hide();
                        }
                    }
                });
            }

            let launched_automatically = std::env::args().any(|arg| arg == "--autostart");
            if launched_automatically {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            state.start_monitor(app.handle().clone());
            state.start_discovery_monitor(app.handle().clone());
            state.auto_restore(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::save_settings,
            commands::create_project,
            commands::update_project,
            commands::delete_project,
            commands::start_project,
            commands::stop_project,
            commands::restart_project,
            commands::restore_all,
            commands::get_project_logs,
            commands::clear_project_logs,
            commands::refresh_discovery,
            commands::ignore_discovery_candidate,
            commands::clear_ignored_discovery_candidates,
            commands::open_discovered_url,
            commands::open_project_directory,
            commands::open_project_url,
            commands::open_project_editor,
            commands::open_project_terminal,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Vibe Manager");

    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
            app.state::<AppState>().prepare_for_exit(app);
        }
    });
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn toggle_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) && window.is_focused().unwrap_or(false) {
            let _ = window.hide();
        } else {
            show_main_window(app);
        }
    }
}
