use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex, RwLock,
};
use std::time::Instant;

use crate::{domain::QuotaSnapshot, infrastructure::AppServerSession};

use super::{
    refresh_policy::RefreshPolicy, AppServerStatus, LoadedPreferences, Preferences,
    PreferencesRecovery, PreferencesStore, RefreshState, UpdateStatus, WindowState,
};

pub struct AppState {
    pub snapshot: RwLock<QuotaSnapshot>,
    pub last_good_snapshot: RwLock<Option<QuotaSnapshot>>,
    pub app_server_status: RwLock<AppServerStatus>,
    pub refresh_state: RwLock<RefreshState>,
    pub preferences: RwLock<Preferences>,
    pub preferences_recovery: RwLock<Option<PreferencesRecovery>>,
    pub window_state: RwLock<WindowState>,
    pub update_status: RwLock<UpdateStatus>,
    pub refresh_guard: tokio::sync::Mutex<()>,
    pub app_server_session: tokio::sync::Mutex<Option<AppServerSession>>,
    pub last_manual_refresh: Mutex<Option<Instant>>,
    pub refresh_policy: Mutex<RefreshPolicy>,
    pub(crate) preferences_store: Mutex<PreferencesStore>,
    revision: AtomicU64,
}

impl AppState {
    pub(crate) fn new(
        application_version: &str,
        preferences_store: PreferencesStore,
        loaded_preferences: LoadedPreferences,
    ) -> Self {
        let initial_revision = loaded_preferences.preferences.revision;
        let initial_window = WindowState {
            revision: initial_revision,
            mode: loaded_preferences.preferences.widget.mode,
            visible: loaded_preferences.preferences.widget.mode != super::WidgetMode::Hidden,
            always_on_top: loaded_preferences.preferences.widget.always_on_top,
            click_through: loaded_preferences.preferences.widget.click_through,
            bounds: None,
        };
        Self {
            snapshot: RwLock::new(QuotaSnapshot::loading(0)),
            last_good_snapshot: RwLock::new(None),
            app_server_status: RwLock::new(AppServerStatus::default()),
            refresh_state: RwLock::new(RefreshState::default()),
            preferences: RwLock::new(loaded_preferences.preferences),
            preferences_recovery: RwLock::new(loaded_preferences.recovery),
            window_state: RwLock::new(initial_window),
            update_status: RwLock::new(UpdateStatus::idle(application_version)),
            refresh_guard: tokio::sync::Mutex::new(()),
            app_server_session: tokio::sync::Mutex::new(None),
            last_manual_refresh: Mutex::new(None),
            refresh_policy: Mutex::new(RefreshPolicy::default()),
            preferences_store: Mutex::new(preferences_store),
            revision: AtomicU64::new(initial_revision),
        }
    }

    pub fn next_revision(&self) -> u64 {
        self.revision.fetch_add(1, Ordering::Relaxed) + 1
    }
}
