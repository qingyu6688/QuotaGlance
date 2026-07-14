pub mod application;
pub mod commands;
pub mod domain;
pub mod infrastructure;
pub mod platform;
pub mod providers;

use application::{
    spawn_refresh_scheduler, spawn_startup_refresh, AppState, PreferencesStore, RefreshReason,
    WidgetMode,
};
use platform::{apply_widget_mode, restore_window_preferences, setup_tray};
use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
    Manager, WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let application = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let state = app.state::<AppState>();
            let _ = apply_widget_mode(app, &state, WidgetMode::Card);
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            let mut preferences_store = PreferencesStore::new(app.path().app_config_dir()?);
            let mut loaded_preferences = preferences_store.load();
            if let Ok(enabled) = app.autolaunch().is_enabled() {
                loaded_preferences.preferences.startup.launch_at_login = enabled;
            }
            app.manage(AppState::new(
                env!("CARGO_PKG_VERSION"),
                preferences_store,
                loaded_preferences,
            ));

            let state = app.state::<AppState>();
            let _ = restore_window_preferences(app.handle(), &state);

            if setup_tray(app).is_err() {
                if let Some(window) = app.get_webview_window("widget") {
                    let _ = window.set_skip_taskbar(false);
                }
            }

            spawn_startup_refresh(app.handle().clone());
            spawn_refresh_scheduler(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_quota_snapshot,
            commands::refresh_quota,
            commands::get_app_server_status,
            commands::get_preferences,
            commands::set_theme,
            commands::set_widget_mode,
            commands::set_always_on_top,
            commands::set_click_through,
            commands::set_launch_at_login,
            commands::quit_app,
        ])
        .on_tray_icon_event(|app, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let state = app.state::<AppState>();
                let _ = apply_widget_mode(app, &state, WidgetMode::Card);
            }
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let app = window.app_handle();
                let state = app.state::<AppState>();
                let _ = apply_widget_mode(app, &state, WidgetMode::Hidden);
            }
        })
        .build(tauri::generate_context!());

    let Ok(application) = application else {
        eprintln!("QuotaGlance 初始化失败");
        return;
    };

    application.run(|app, event| {
        if matches!(&event, tauri::RunEvent::ExitRequested { .. }) {
            tauri::async_runtime::block_on(application::shutdown_app_server(app));
        }
        if matches!(&event, tauri::RunEvent::Resumed) {
            let handle = app.clone();
            tauri::async_runtime::spawn(async move {
                let _ =
                    application::refresh_quota_runtime(&handle, RefreshReason::Resume, false).await;
            });
        }
    });
}
