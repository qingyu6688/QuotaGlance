use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Manager,
};

use crate::{
    application::{refresh_quota_runtime, AppState, RefreshReason, WidgetMode},
    platform::{apply_click_through, apply_widget_mode},
};

pub fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show_card = MenuItem::with_id(app, "show-card", "显示额度卡片", true, None::<&str>)?;
    let show_orb = MenuItem::with_id(app, "show-orb", "显示浮球", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, "refresh", "立即刷新", true, None::<&str>)?;
    let unlock = MenuItem::with_id(app, "unlock", "关闭鼠标穿透", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "退出 QuotaGlance", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&show_card, &show_orb, &refresh, &unlock, &separator, &quit],
    )?;

    let mut builder = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .tooltip("QuotaGlance");
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder
        .on_menu_event(|app, event| {
            let state = app.state::<AppState>();
            match event.id.as_ref() {
                "show-card" => {
                    let _ = apply_widget_mode(app, &state, WidgetMode::Card);
                }
                "show-orb" => {
                    let _ = apply_widget_mode(app, &state, WidgetMode::Orb);
                }
                "refresh" => {
                    let handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = refresh_quota_runtime(&handle, RefreshReason::Manual, true).await;
                    });
                }
                "unlock" => {
                    let _ = apply_click_through(app, &state, false);
                }
                "quit" => app.exit(0),
                _ => {}
            }
        })
        .build(app)?;
    Ok(())
}
