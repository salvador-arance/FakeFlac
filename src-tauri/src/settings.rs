//! RF-13: the persisted language preference (`specs/contracts/settings.md`, ADR-0049).
//!
//! A small adapter of the app shell: a versioned JSON file in Tauri's `app_config_dir()`, loaded
//! once per process and saved through a temporary sibling and an atomic replacement. All file
//! access goes through [`SettingsFs`] so tests can inject failures. The Windows user's language
//! list ([`windows_preferred_languages`]) is a separate native source, injected into
//! [`SettingsState`] and only consulted when no settings file exists.

use crate::dto::{
    AppError, AppErrorCode, Language, Settings, SettingsLoadStatus, SettingsSnapshot,
};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// The settings file inside the app configuration directory.
pub const FILE_NAME: &str = "settings.json";

/// The schema this release reads and writes.
const SCHEMA_VERSION: u64 = 1;

/// Attempts to find an unused temporary name before giving up.
const TEMP_ATTEMPTS: u32 = 16;

/// The file operations of a load and a save, all on absolute paths (section 4).
pub trait SettingsFs: Send + Sync {
    /// Reads a whole file; `NotFound` means there is no file.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    /// Creates a directory and its missing parents.
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;
    /// Creates a new file exclusively (fails if it exists). The writer's `flush` is the flush
    /// before the replacement.
    fn create_new(&self, path: &Path) -> io::Result<Box<dyn Write + Send>>;
    /// Atomically replaces `to` with `from`.
    fn replace(&self, from: &Path, to: &Path) -> io::Result<()>;
    /// Removes a file (best-effort cleanup of a temporary file).
    fn remove_file(&self, path: &Path) -> io::Result<()>;
}

/// The standard file system.
pub struct StdSettingsFs;

/// A file whose `flush` also asks the OS to write it to disk before it is published.
struct SyncedFile(File);

impl Write for SyncedFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()?;
        self.0.sync_all()
    }
}

impl SettingsFs for StdSettingsFs {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        std::fs::create_dir_all(path)
    }

    fn create_new(&self, path: &Path) -> io::Result<Box<dyn Write + Send>> {
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        Ok(Box::new(SyncedFile(file)))
    }

    fn replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        // Replaces an existing destination atomically on Windows and Unix.
        std::fs::rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }
}

type PreferredLanguages = Box<dyn FnOnce() -> Option<Vec<String>> + Send>;

struct Inner {
    /// `None` until the first `get` or `set`.
    snapshot: Option<SettingsSnapshot>,
    /// The Windows list source, taken (and called at most once) during initialization.
    preferred_languages: Option<PreferredLanguages>,
    /// The initial load saw a newer schema: saves are refused for the whole process.
    read_only: bool,
}

/// The one backend owner of the settings of this process. Calls are serialized.
pub struct SettingsState {
    config_dir: Option<PathBuf>,
    fs: Arc<dyn SettingsFs>,
    inner: Mutex<Inner>,
}

impl SettingsState {
    /// `config_dir` is Tauri's `app_config_dir()` (`None` if it could not be resolved);
    /// `preferred_languages` is the Windows user's ordered language list (`None` = unavailable).
    /// Constructing does no I/O.
    pub fn new(
        config_dir: Option<PathBuf>,
        preferred_languages: impl FnOnce() -> Option<Vec<String>> + Send + 'static,
    ) -> SettingsState {
        SettingsState::with_fs(config_dir, preferred_languages, Arc::new(StdSettingsFs))
    }

    /// As [`SettingsState::new`], with the given file system.
    pub fn with_fs(
        config_dir: Option<PathBuf>,
        preferred_languages: impl FnOnce() -> Option<Vec<String>> + Send + 'static,
        fs: Arc<dyn SettingsFs>,
    ) -> SettingsState {
        SettingsState {
            config_dir,
            fs,
            inner: Mutex::new(Inner {
                snapshot: None,
                preferred_languages: Some(Box::new(preferred_languages)),
                read_only: false,
            }),
        }
    }

    /// The current snapshot, loading the settings on first use.
    pub fn get(&self) -> SettingsSnapshot {
        let mut inner = self.lock();
        self.initialized(&mut inner)
    }

    /// Saves `settings` and makes them current; the snapshot is only updated after the file was
    /// replaced. Errors: `SETTINGS_READ_ONLY` (a newer file), `SETTINGS_WRITE_FAILED`.
    pub fn set(&self, settings: Settings) -> Result<SettingsSnapshot, AppError> {
        let mut inner = self.lock();
        self.initialized(&mut inner);
        if inner.read_only {
            return Err(read_only(
                "the settings file was written by a newer FakeFlac version",
            ));
        }
        let dir = self.config_dir.as_deref().ok_or_else(|| {
            write_failed("the app configuration directory could not be resolved".into())
        })?;
        self.save(dir, settings.language)?;
        let snapshot = SettingsSnapshot {
            settings,
            load_status: SettingsLoadStatus::Loaded,
        };
        inner.snapshot = Some(snapshot);
        Ok(snapshot)
    }

    /// A poisoned lock still holds a consistent snapshot: it is only replaced after a save.
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn initialized(&self, inner: &mut Inner) -> SettingsSnapshot {
        if let Some(snapshot) = inner.snapshot {
            return snapshot;
        }
        let (language, load_status) = self.load(inner);
        inner.read_only = load_status == SettingsLoadStatus::NewerVersion;
        let snapshot = SettingsSnapshot {
            settings: Settings { language },
            load_status,
        };
        inner.snapshot = Some(snapshot);
        snapshot
    }

    /// The initial load (section 3). Never modifies the file system.
    fn load(&self, inner: &mut Inner) -> (Language, SettingsLoadStatus) {
        let Some(dir) = self.config_dir.as_deref() else {
            return (Language::En, SettingsLoadStatus::Unavailable);
        };
        match self.fs.read(&dir.join(FILE_NAME)) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let list = inner.preferred_languages.take().and_then(|source| source());
                (
                    detect_language(list.as_deref()),
                    SettingsLoadStatus::Default,
                )
            }
            Err(_) => (Language::En, SettingsLoadStatus::Unavailable),
            Ok(bytes) => match parse(&bytes) {
                Parsed::Valid(language) => (language, SettingsLoadStatus::Loaded),
                Parsed::Newer => (Language::En, SettingsLoadStatus::NewerVersion),
                Parsed::Invalid => (Language::En, SettingsLoadStatus::Invalid),
            },
        }
    }

    /// Section 4 steps 1-2: the pre-save check, then a temporary sibling replacing the file.
    fn save(&self, dir: &Path, language: Language) -> Result<(), AppError> {
        let dest = dir.join(FILE_NAME);
        match self.fs.read(&dest) {
            Ok(bytes) => {
                if matches!(parse(&bytes), Parsed::Newer) {
                    return Err(read_only(
                        "the settings file was replaced by a newer FakeFlac version",
                    ));
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(write_failed(format!(
                    "cannot inspect {} before saving: {e}",
                    dest.display()
                )))
            }
        }
        self.fs
            .create_dir_all(dir)
            .map_err(|e| write_failed(format!("cannot create {}: {e}", dir.display())))?;

        let bytes = serde_json::to_vec(&FileV1Out {
            schema_version: SCHEMA_VERSION,
            language,
        })
        .map_err(|e| write_failed(format!("cannot serialize the settings: {e}")))?;

        let (temp, mut writer) = self.create_temp(dir)?;
        let written = writer.write_all(&bytes).and_then(|()| writer.flush());
        drop(writer);
        if let Err(e) = written {
            self.cleanup(&temp);
            return Err(write_failed(format!(
                "cannot write {}: {e}",
                temp.display()
            )));
        }
        if let Err(e) = self.fs.replace(&temp, &dest) {
            self.cleanup(&temp);
            return Err(write_failed(format!(
                "cannot replace {}: {e}",
                dest.display()
            )));
        }
        Ok(())
    }

    /// A new, uniquely named sibling of the settings file, created exclusively.
    fn create_temp(&self, dir: &Path) -> Result<(PathBuf, Box<dyn Write + Send>), AppError> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let mut last = None;
        for _ in 0..TEMP_ATTEMPTS {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0);
            let temp = dir.join(format!(
                ".{FILE_NAME}.{}-{n}-{nanos}.tmp",
                std::process::id()
            ));
            match self.fs.create_new(&temp) {
                Ok(writer) => return Ok((temp, writer)),
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => last = Some(e),
                Err(e) => {
                    return Err(write_failed(format!(
                        "cannot create {}: {e}",
                        temp.display()
                    )))
                }
            }
        }
        Err(write_failed(format!(
            "cannot create a temporary settings file in {}: {}",
            dir.display(),
            last.map_or_else(|| "no attempt".to_string(), |e| e.to_string())
        )))
    }

    /// Best-effort removal of this attempt's temporary file.
    fn cleanup(&self, temp: &Path) {
        let _ = self.fs.remove_file(temp);
    }
}

/// Section 3: the first supported entry of the Windows list by primary subtag, else English.
fn detect_language(list: Option<&[String]>) -> Language {
    list.unwrap_or_default()
        .iter()
        .find_map(|tag| {
            let primary = tag.split('-').next().unwrap_or_default();
            if primary.eq_ignore_ascii_case("es") {
                Some(Language::Es)
            } else if primary.eq_ignore_ascii_case("en") {
                Some(Language::En)
            } else {
                None
            }
        })
        .unwrap_or(Language::En)
}

enum Parsed {
    Valid(Language),
    Newer,
    Invalid,
}

/// Schema 1, strict: exactly these fields, no duplicates, the right types.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FileV1In {
    schema_version: u64,
    language: Language,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileV1Out {
    schema_version: u64,
    language: Language,
}

/// Classifies the bytes of a settings file (section 3). A readable JSON object with a positive
/// integer `schemaVersion > 1` is newer, whatever its other fields; everything else that is not
/// valid schema 1 is invalid.
fn parse(bytes: &[u8]) -> Parsed {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Parsed::Invalid;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Parsed::Invalid;
    };
    let version = value
        .as_object()
        .and_then(|o| o.get("schemaVersion"))
        .and_then(serde_json::Value::as_u64);
    if version.is_some_and(|v| v > SCHEMA_VERSION) {
        return Parsed::Newer;
    }
    // Typed parsing of the text (not the value) also rejects duplicate fields.
    match serde_json::from_str::<FileV1In>(text) {
        Ok(file) if file.schema_version == SCHEMA_VERSION => Parsed::Valid(file.language),
        _ => Parsed::Invalid,
    }
}

fn write_failed(message: String) -> AppError {
    AppError {
        code: AppErrorCode::SettingsWriteFailed,
        message,
    }
}

fn read_only(message: &str) -> AppError {
    AppError {
        code: AppErrorCode::SettingsReadOnly,
        message: message.to_string(),
    }
}

/// The Windows user's ordered preferred languages (Settings › Language), from
/// `Windows.System.UserProfile.GlobalizationPreferences.Languages()` (ADR-0049 amendment).
/// `None` when the list cannot be read.
#[cfg(windows)]
pub fn windows_preferred_languages() -> Option<Vec<String>> {
    use windows::System::UserProfile::GlobalizationPreferences;
    let list = GlobalizationPreferences::Languages().ok()?;
    let size = list.Size().ok()?;
    (0..size)
        .map(|i| list.GetAt(i).ok().map(|tag| tag.to_string()))
        .collect()
}

/// Not Windows: the source is unavailable (English fallback).
#[cfg(not(windows))]
pub fn windows_preferred_languages() -> Option<Vec<String>> {
    None
}
