use tauri::{AppHandle, Emitter, LogicalSize, Manager, Size};

use crate::{
    application::{
        AppState, IpcError, PreferencesEnvelope, PreferencesStoreError, WidgetMode, WindowState,
    },
    domain::ErrorCode,
};

const CARD_WIDTH: f64 = 320.0;
const CARD_HEIGHT: f64 = 320.0;
const ORB_SIZE: f64 = 136.0;

pub fn apply_widget_mode(
    app: &AppHandle,
    state: &AppState,
    mode: WidgetMode,
) -> Result<WindowState, IpcError> {
    let window = app
        .get_webview_window("widget")
        .ok_or_else(window_operation_error)?;
    apply_mode_to_window(&window, mode)?;

    update_window_state(app, state, |window_state| {
        window_state.mode = mode;
        window_state.visible = mode != WidgetMode::Hidden;
    })
}

pub fn restore_window_preferences(app: &AppHandle, state: &AppState) -> Result<(), IpcError> {
    let preferences = state
        .preferences
        .read()
        .map(|preferences| preferences.clone())
        .map_err(|_| state_unavailable_error())?;
    let window = app
        .get_webview_window("widget")
        .ok_or_else(window_operation_error)?;

    apply_mode_to_window(&window, preferences.widget.mode)?;
    window
        .set_always_on_top(preferences.widget.always_on_top)
        .map_err(|_| window_operation_error())?;
    window
        .set_ignore_cursor_events(preferences.widget.click_through)
        .map_err(|_| window_operation_error())?;
    Ok(())
}

fn apply_mode_to_window(window: &tauri::WebviewWindow, mode: WidgetMode) -> Result<(), IpcError> {
    match mode {
        WidgetMode::Card => {
            window
                .set_size(Size::Logical(LogicalSize::new(CARD_WIDTH, CARD_HEIGHT)))
                .map_err(|_| window_operation_error())?;
            window.show().map_err(|_| window_operation_error())?;
        }
        WidgetMode::Orb => {
            window
                .set_size(Size::Logical(LogicalSize::new(ORB_SIZE, ORB_SIZE)))
                .map_err(|_| window_operation_error())?;
            window.show().map_err(|_| window_operation_error())?;
        }
        WidgetMode::Hidden => window.hide().map_err(|_| window_operation_error())?,
    }
    Ok(())
}

pub fn apply_always_on_top(
    app: &AppHandle,
    state: &AppState,
    enabled: bool,
) -> Result<WindowState, IpcError> {
    let window = app
        .get_webview_window("widget")
        .ok_or_else(window_operation_error)?;
    window
        .set_always_on_top(enabled)
        .map_err(|_| window_operation_error())?;

    update_window_state(app, state, |window_state| {
        window_state.always_on_top = enabled;
    })
}

pub fn apply_click_through(
    app: &AppHandle,
    state: &AppState,
    enabled: bool,
) -> Result<WindowState, IpcError> {
    let window = app
        .get_webview_window("widget")
        .ok_or_else(window_operation_error)?;
    window
        .set_ignore_cursor_events(enabled)
        .map_err(|_| window_operation_error())?;

    update_window_state(app, state, |window_state| {
        window_state.click_through = enabled;
    })
}

pub fn current_window_state(state: &AppState) -> Result<WindowState, IpcError> {
    state
        .window_state
        .read()
        .map(|window_state| window_state.clone())
        .map_err(|_| state_unavailable_error())
}

fn update_window_state<F>(
    app: &AppHandle,
    state: &AppState,
    update: F,
) -> Result<WindowState, IpcError>
where
    F: FnOnce(&mut WindowState),
{
    let revision = state.next_revision();
    let mut current = state
        .window_state
        .write()
        .map_err(|_| state_unavailable_error())?;
    update(&mut current);
    current.revision = revision;
    let payload = current.clone();
    drop(current);
    let _ = app.emit_to("widget", "window://state-changed", payload.clone());

    let preferences_payload = {
        let mut preferences = state
            .preferences
            .write()
            .map_err(|_| state_unavailable_error())?;
        let mut next = preferences.clone();
        next.revision = revision;
        next.widget.mode = payload.mode;
        next.widget.always_on_top = payload.always_on_top;
        next.widget.click_through = payload.click_through;
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

    let _ = app.emit_to("widget", "preferences://changed", preferences_payload);
    Ok(payload)
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

fn window_operation_error() -> IpcError {
    IpcError::new(
        ErrorCode::WindowOperationFailed,
        "error.windowOperationFailed",
        true,
        None,
    )
}

fn state_unavailable_error() -> IpcError {
    IpcError::new(
        ErrorCode::ServiceUnavailable,
        "error.internalStateUnavailable",
        true,
        None,
    )
}
