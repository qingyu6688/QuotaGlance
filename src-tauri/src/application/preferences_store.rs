use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use serde_json::Value;

use super::{Preferences, PreferencesRecovery};

const PREFERENCES_SCHEMA_VERSION: u64 = 1;
const MAX_PREFERENCES_BYTES: usize = 256 * 1024;

pub(crate) struct LoadedPreferences {
    pub preferences: Preferences,
    pub recovery: Option<PreferencesRecovery>,
}

pub(crate) struct PreferencesStore {
    directory: PathBuf,
    current_valid: bool,
    writes_blocked: bool,
}

#[derive(Debug)]
pub(crate) enum PreferencesStoreError {
    Io,
    Invalid,
    VersionUnsupported,
}

enum ReadFailure {
    Missing,
    Io,
    Corrupted,
    VersionUnsupported,
}

impl PreferencesStore {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            current_valid: false,
            writes_blocked: false,
        }
    }

    pub fn load(&mut self) -> LoadedPreferences {
        match read_preferences(&self.current_path()) {
            Ok(preferences) => {
                self.current_valid = true;
                LoadedPreferences {
                    preferences,
                    recovery: None,
                }
            }
            Err(ReadFailure::VersionUnsupported) => {
                self.writes_blocked = true;
                recovered_defaults("PREFERENCES_VERSION_UNSUPPORTED")
            }
            Err(current_failure) => match read_preferences(&self.backup_path()) {
                Ok(preferences) => LoadedPreferences {
                    preferences,
                    recovery: Some(PreferencesRecovery {
                        source: "backup".to_owned(),
                        reason_code: "PREFERENCES_CORRUPTED".to_owned(),
                    }),
                },
                Err(ReadFailure::VersionUnsupported) => {
                    self.writes_blocked = true;
                    recovered_defaults("PREFERENCES_VERSION_UNSUPPORTED")
                }
                Err(_) if matches!(current_failure, ReadFailure::Missing) => LoadedPreferences {
                    preferences: Preferences::default(),
                    recovery: None,
                },
                Err(_) => recovered_defaults("PREFERENCES_CORRUPTED"),
            },
        }
    }

    pub fn save(&mut self, preferences: &Preferences) -> Result<(), PreferencesStoreError> {
        if self.writes_blocked {
            return Err(PreferencesStoreError::VersionUnsupported);
        }
        validate_preferences(preferences).map_err(|()| PreferencesStoreError::Invalid)?;

        let mut bytes =
            serde_json::to_vec_pretty(preferences).map_err(|_| PreferencesStoreError::Invalid)?;
        bytes.push(b'\n');
        if bytes.len() > MAX_PREFERENCES_BYTES {
            return Err(PreferencesStoreError::Invalid);
        }

        fs::create_dir_all(&self.directory).map_err(|_| PreferencesStoreError::Io)?;
        let temporary = self.temporary_path();
        write_synced(&temporary, &bytes).map_err(|_| PreferencesStoreError::Io)?;

        if self.current_valid {
            let current = fs::read(self.current_path()).map_err(|_| PreferencesStoreError::Io)?;
            if current.len() > MAX_PREFERENCES_BYTES {
                let _ = fs::remove_file(&temporary);
                return Err(PreferencesStoreError::Invalid);
            }
            write_synced(&self.backup_path(), &current).map_err(|_| PreferencesStoreError::Io)?;
        }

        if let Err(error) = fs::rename(&temporary, self.current_path()) {
            if !self.current_path().exists() {
                let _ = fs::remove_file(&temporary);
                return Err(PreferencesStoreError::Io);
            }
            fs::remove_file(self.current_path()).map_err(|_| PreferencesStoreError::Io)?;
            if fs::rename(&temporary, self.current_path()).is_err() {
                let _ = restore_backup(&self.backup_path(), &self.current_path());
                let _ = fs::remove_file(&temporary);
                let _ = error;
                return Err(PreferencesStoreError::Io);
            }
        }

        sync_directory(&self.directory);
        self.current_valid = true;
        Ok(())
    }

    fn current_path(&self) -> PathBuf {
        self.directory.join("preferences.json")
    }

    fn backup_path(&self) -> PathBuf {
        self.directory.join("preferences.json.bak")
    }

    fn temporary_path(&self) -> PathBuf {
        self.directory.join("preferences.json.tmp")
    }
}

fn recovered_defaults(reason_code: &str) -> LoadedPreferences {
    LoadedPreferences {
        preferences: Preferences::default(),
        recovery: Some(PreferencesRecovery {
            source: "defaults".to_owned(),
            reason_code: reason_code.to_owned(),
        }),
    }
}

fn read_preferences(path: &Path) -> Result<Preferences, ReadFailure> {
    let metadata = fs::metadata(path).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => ReadFailure::Missing,
        _ => ReadFailure::Io,
    })?;
    let length = usize::try_from(metadata.len()).map_err(|_| ReadFailure::Corrupted)?;
    if length == 0 || length > MAX_PREFERENCES_BYTES {
        return Err(ReadFailure::Corrupted);
    }

    let bytes = fs::read(path).map_err(|_| ReadFailure::Io)?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| ReadFailure::Corrupted)?;
    let schema_version = value
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .ok_or(ReadFailure::Corrupted)?;
    if schema_version > PREFERENCES_SCHEMA_VERSION {
        return Err(ReadFailure::VersionUnsupported);
    }
    if schema_version != PREFERENCES_SCHEMA_VERSION {
        return Err(ReadFailure::Corrupted);
    }

    let preferences: Preferences =
        serde_json::from_value(value).map_err(|_| ReadFailure::Corrupted)?;
    validate_preferences(&preferences).map_err(|()| ReadFailure::Corrupted)?;
    Ok(preferences)
}

fn validate_preferences(preferences: &Preferences) -> Result<(), ()> {
    if u64::from(preferences.schema_version) != PREFERENCES_SCHEMA_VERSION
        || preferences.revision == u64::MAX
        || preferences.updates.channel != "stable"
        || !is_percentage(preferences.notifications.warning_remaining_percent)
        || !is_percentage(preferences.notifications.critical_remaining_percent)
        || preferences.notifications.critical_remaining_percent
            > preferences.notifications.warning_remaining_percent
    {
        return Err(());
    }

    match &preferences.widget.selected_quota.limit_id {
        Some(limit_id) if limit_id.is_empty() || limit_id.chars().count() > 128 => return Err(()),
        None if preferences.widget.selected_quota.slot.is_some() => return Err(()),
        _ => {}
    }

    for bounds in [
        preferences.widget.bounds_by_mode.orb.as_ref(),
        preferences.widget.bounds_by_mode.card.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if !bounds.x.is_finite()
            || !bounds.y.is_finite()
            || !bounds.width.is_finite()
            || bounds.width <= 0.0
            || !bounds.height.is_finite()
            || bounds.height <= 0.0
            || !bounds.scale_factor_at_save.is_finite()
            || !(0.5..=8.0).contains(&bounds.scale_factor_at_save)
            || bounds
                .monitor_id
                .as_ref()
                .is_some_and(|monitor_id| monitor_id.chars().count() > 256)
        {
            return Err(());
        }
    }

    Ok(())
}

fn is_percentage(value: f64) -> bool {
    value.is_finite() && (0.0..=100.0).contains(&value)
}

fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.flush()?;
    file.sync_all()
}

fn restore_backup(backup: &Path, current: &Path) -> io::Result<()> {
    if backup.exists() && !current.exists() {
        fs::copy(backup, current)?;
    }
    Ok(())
}

fn sync_directory(directory: &Path) {
    if let Ok(directory_file) = File::open(directory) {
        let _ = directory_file.sync_all();
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use crate::application::Theme;

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    fn test_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir().join(format!(
            "quota-glance-preferences-{nonce}-{}",
            NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn missing_file_loads_defaults_without_recovery_warning() {
        let directory = test_directory();
        let mut store = PreferencesStore::new(directory.clone());

        let loaded = store.load();

        assert_eq!(loaded.preferences, Preferences::default());
        assert!(loaded.recovery.is_none());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn save_round_trip_and_backup_recovery_are_supported() {
        let directory = test_directory();
        let mut store = PreferencesStore::new(directory.clone());
        let _ = store.load();
        let mut first = Preferences {
            revision: 1,
            ..Preferences::default()
        };
        let legacy_light = serde_json::from_str::<Theme>(r#""light""#);
        let legacy_dark = serde_json::from_str::<Theme>(r#""dark""#);
        assert!(matches!(legacy_light, Ok(Theme::Aurora)));
        assert!(matches!(legacy_dark, Ok(Theme::Graphite)));
        assert!(matches!(
            serde_json::from_str::<Theme>(r#""sunset""#),
            Ok(Theme::Sunset)
        ));
        assert!(matches!(
            serde_json::from_str::<Theme>(r#""honey""#),
            Ok(Theme::Honey)
        ));
        assert!(matches!(
            serde_json::from_str::<Theme>(r#""rose""#),
            Ok(Theme::Rose)
        ));

        first.theme = Theme::Rose;
        first.widget.always_on_top = false;
        assert!(store.save(&first).is_ok());

        let mut second = first.clone();
        second.revision = 2;
        second.widget.click_through = true;
        assert!(store.save(&second).is_ok());
        assert!(fs::write(directory.join("preferences.json"), b"{broken").is_ok());

        let mut reloaded_store = PreferencesStore::new(directory.clone());
        let loaded = reloaded_store.load();
        assert_eq!(loaded.preferences, first);
        assert_eq!(
            loaded
                .recovery
                .as_ref()
                .map(|recovery| recovery.source.as_str()),
            Some("backup")
        );
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn future_schema_blocks_writes_and_preserves_the_file() {
        let directory = test_directory();
        assert!(fs::create_dir_all(&directory).is_ok());
        let future = br#"{"schemaVersion":99,"revision":7}"#;
        assert!(fs::write(directory.join("preferences.json"), future).is_ok());
        let mut store = PreferencesStore::new(directory.clone());

        let loaded = store.load();
        let result = store.save(&Preferences::default());

        assert_eq!(
            loaded
                .recovery
                .as_ref()
                .map(|recovery| recovery.reason_code.as_str()),
            Some("PREFERENCES_VERSION_UNSUPPORTED")
        );
        assert!(matches!(
            result,
            Err(PreferencesStoreError::VersionUnsupported)
        ));
        assert_eq!(
            fs::read(directory.join("preferences.json")).ok().as_deref(),
            Some(future.as_slice())
        );
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn invalid_thresholds_are_rejected_before_writing() {
        let directory = test_directory();
        let mut store = PreferencesStore::new(directory.clone());
        let _ = store.load();
        let mut preferences = Preferences::default();
        preferences.notifications.warning_remaining_percent = 10.0;
        preferences.notifications.critical_remaining_percent = 20.0;

        assert!(matches!(
            store.save(&preferences),
            Err(PreferencesStoreError::Invalid)
        ));
        assert!(!directory.join("preferences.json").exists());
        let _ = fs::remove_dir_all(directory);
    }
}
