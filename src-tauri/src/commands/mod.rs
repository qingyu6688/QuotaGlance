use tauri::{AppHandle, Emitter, State};
use tauri_plugin_autostart::ManagerExt;

use crate::{
    application::{
        refresh_quota_runtime, AppServerStatus, AppState, IpcError, PreferencesEnvelope,
        PreferencesStoreError, RefreshReason, RefreshReceipt, Theme, WidgetMode, WindowState,
    },
    domain::{ErrorCode, QuotaSnapshot},
    platform::{apply_always_on_top, apply_click_through, apply_widget_mode},
};

#[tauri::command]
pub fn get_quota_snapshot(state: State<'_, AppState>) -> Result<QuotaSnapshot, IpcError> {
    state
        .snapshot
        .read()
        .map(|snapshot| snapshot.clone())
        .map_err(|_| state_unavailable_error())
}

#[tauri::command]
pub async fn refresh_quota(app: AppHandle) -> Result<RefreshReceipt, IpcError> {
    refresh_quota_runtime(&app, RefreshReason::Manual, true).await
}

#[tauri::command]
pub fn get_app_server_status(state: State<'_, AppState>) -> Result<AppServerStatus, IpcError> {
    state
        .app_server_status
        .read()
        .map(|status| status.clone())
        .map_err(|_| state_unavailable_error())
}

#[tauri::command]
pub fn get_preferences(state: State<'_, AppState>) -> Result<PreferencesEnvelope, IpcError> {
    let preferences = state
        .preferences
        .read()
        .map(|preferences| preferences.clone())
        .map_err(|_| state_unavailable_error())?;
    let recovery = state
        .preferences_recovery
        .read()
        .map(|recovery| recovery.clone())
        .map_err(|_| state_unavailable_error())?;
    Ok(PreferencesEnvelope {
        preferences,
        recovery,
    })
}

#[tauri::command]
pub fn set_theme(
    theme: Theme,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<PreferencesEnvelope, IpcError> {
    let revision = state.next_revision();
    let payload = {
        let mut preferences = state
            .preferences
            .write()
            .map_err(|_| state_unavailable_error())?;
        let mut next = preferences.clone();
        next.revision = revision;
        next.theme = theme;
        state
            .preferences_store
            .lock()
            .map_err(|_| state_unavailable_error())?
            .save(&next)
            .map_err(preferences_store_error)?;
        *preferences = next;
        if let Ok(mut recovery) = state.preferences_recovery.write() {
            *recovery = None;
        }
        PreferencesEnvelope {
            preferences: preferences.clone(),
            recovery: None,
        }
    };

    let _ = app.emit_to("widget", "preferences://changed", payload.clone());
    Ok(payload)
}

#[tauri::command]
pub fn set_widget_mode(
    mode: WidgetMode,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<WindowState, IpcError> {
    apply_widget_mode(&app, &state, mode)
}

#[tauri::command]
pub fn set_always_on_top(
    enabled: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<WindowState, IpcError> {
    apply_always_on_top(&app, &state, enabled)
}

#[tauri::command]
pub fn set_click_through(
    enabled: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<WindowState, IpcError> {
    apply_click_through(&app, &state, enabled)
}

#[tauri::command]
pub fn set_launch_at_login(
    enabled: bool,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<PreferencesEnvelope, IpcError> {
    let manager = app.autolaunch();
    let payload = {
        let mut preferences = state
            .preferences
            .write()
            .map_err(|_| state_unavailable_error())?;
        let mut preferences_store = state
            .preferences_store
            .lock()
            .map_err(|_| state_unavailable_error())?;
        let previously_enabled = manager
            .is_enabled()
            .map_err(|_| startup_operation_error())?;
        let revision = state.next_revision();
        let mut next = preferences.clone();
        next.revision = revision;
        next.startup.launch_at_login = enabled;

        if previously_enabled != enabled {
            let result = if enabled {
                manager.enable()
            } else {
                manager.disable()
            };
            result.map_err(|_| startup_operation_error())?;
        }

        let save_result = preferences_store.save(&next);
        if let Err(error) = save_result {
            let rollback_result = if previously_enabled != enabled {
                if previously_enabled {
                    manager.enable()
                } else {
                    manager.disable()
                }
            } else {
                Ok(())
            };
            if rollback_result.is_err() {
                return Err(startup_operation_error());
            }
            return Err(preferences_store_error(error));
        }
        *preferences = next;
        if let Ok(mut recovery) = state.preferences_recovery.write() {
            *recovery = None;
        }
        PreferencesEnvelope {
            preferences: preferences.clone(),
            recovery: None,
        }
    };

    let _ = app.emit_to("widget", "preferences://changed", payload.clone());
    Ok(payload)
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

fn state_unavailable_error() -> IpcError {
    IpcError::new(
        ErrorCode::ServiceUnavailable,
        "error.internalStateUnavailable",
        true,
        None,
    )
}

fn startup_operation_error() -> IpcError {
    IpcError::new(
        ErrorCode::StartupOperationFailed,
        "error.startupOperationFailed",
        true,
        None,
    )
}

fn preferences_store_error(error: PreferencesStoreError) -> IpcError {
    match error {
        PreferencesStoreError::VersionUnsupported => IpcError::new(
            ErrorCode::PreferencesVersionUnsupported,
            "error.preferencesVersionUnsupported",
            false,
            None,
        ),
        PreferencesStoreError::Io | PreferencesStoreError::Invalid => IpcError::new(
            ErrorCode::PreferencesWriteFailed,
            "error.preferencesWriteFailed",
            true,
            None,
        ),
    }
}
