//! Language settings storage of the Tauri adapter (@RF-13, @RNF-04, @RNF-05, @RNF-09):
//! `specs/contracts/settings.md` sections 2-4, ADR-0049, acceptance L2 of
//! `specs/ui/language-settings.md`. Scenarios of `specs/features/language-settings.feature`
//! (backend side): "First launch detects Windows language without creating a file", "Restore a
//! saved language before enabling the main screen", "An explicit Save survives a new app process",
//! "A failed save preserves the old preference and permits retry", "Invalid persisted settings
//! fall back without modifying the file", "An unreadable configuration does not prevent analysis",
//! "A newer settings version is preserved", "A newer file appearing after startup is not
//! overwritten", "Invalid IPC input cannot write a language preference" and "Settings writes are
//! confined to the Unicode app configuration root".
//!
//! Interface of the adapter these tests fix (module `fakeflac_lib::settings`; the DTOs are in
//! `fakeflac_lib::dto`):
//! - `settings::FILE_NAME` = `"settings.json"`, the file inside the app configuration directory.
//! - `SettingsState::new(config_dir: Option<PathBuf>, preferred_languages)` with the standard
//!   file system, and `SettingsState::with_fs(config_dir, preferred_languages, fs:
//!   Arc<dyn SettingsFs>)`. `config_dir` is Tauri's `app_config_dir()`, `None` when it could not
//!   be resolved. `preferred_languages: impl FnOnce() -> Option<Vec<String>> + Send + 'static` is
//!   the Windows user's ordered preferred UI languages (`None` = the source is unavailable); it
//!   is the injected test input of section 3, called at most once, during initialization.
//! - Constructing does no I/O. `get(&self) -> SettingsSnapshot` initializes on first use and then
//!   returns the current snapshot; `set(&self, Settings) -> Result<SettingsSnapshot, AppError>`
//!   initializes first if necessary. Both take `&self` (one owner serializes them; the state is
//!   `Send + Sync`, managed behind an `Arc` as the other states).
//! - `SettingsFs` (`Send + Sync`) is the narrow seam of the five operations of section 4, all on
//!   absolute paths: `read(path) -> io::Result<Vec<u8>>` (the initial load and the pre-save
//!   check; `NotFound` = no file), `create_dir_all(path)`, `create_new(path) ->
//!   io::Result<Box<dyn Write + Send>>` (exclusive creation of the temporary sibling; the
//!   writer's `flush` is the flush before replacing), `replace(from, to)` (atomic replacement)
//!   and `remove_file(path)` (best-effort cleanup of this attempt's temporary file).
//! - `windows_preferred_languages() -> Option<Vec<String>>`: the native source (Windows only).
//!
//! Readings chosen where the spec is open (reported in the test-stage report):
//! - "Directory creation" can only fail when the directory is missing, so that example starts
//!   without a configuration directory (active English from the Windows list, no previous file)
//!   instead of a saved English file; the previous state (no file, no directory) must be kept.
//! - Best-effort cleanup is asserted to succeed when `remove_file` works: after a failed save the
//!   configuration directory holds exactly what it held before.
//! - A `NotFound` from the pre-save `read` means "no destination"; the injected read failure is
//!   `PermissionDenied`.
//!
//! Everything lives in a repository-local temporary root (`CARGO_TARGET_TMPDIR`) whose path has
//! spaces and non-ASCII characters; never `%APPDATA%`, the corpus or a real audio file.
//! Protected after the `test(RF-13)` commit.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use fakeflac_lib::dto::{AppErrorCode, Language, Settings, SettingsLoadStatus, SettingsSnapshot};
use fakeflac_lib::settings::{SettingsFs, SettingsState, FILE_NAME};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const APP_DIR: &str = "io.github.fakeflac.app";
const EN_FILE: &[u8] = br#"{"schemaVersion":1,"language":"en"}"#;
const ES_FILE: &[u8] = br#"{"schemaVersion":1,"language":"es"}"#;
const NEWER_FILE: &[u8] = br#"{"schemaVersion":2,"language":"es","theme":"dark"}"#;

// --- Temporary roots ----------------------------------------------------------------------------

/// `<target tmp>/settings store/Ñandú 日本語 config/<name>`; the configuration directory
/// `<root>/io.github.fakeflac.app` is not created unless a test does it.
struct Root(PathBuf);

impl Root {
    fn new(name: &str) -> Root {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("settings store")
            .join("Ñandú 日本語 config")
            .join(name);
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("create root");
        Root(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn config_dir(&self) -> PathBuf {
        self.0.join(APP_DIR)
    }

    fn file(&self) -> PathBuf {
        self.config_dir().join(FILE_NAME)
    }

    /// Writes the settings file (creating the configuration directory).
    fn write_settings(&self, bytes: &[u8]) {
        std::fs::create_dir_all(self.config_dir()).unwrap();
        std::fs::write(self.file(), bytes).unwrap();
    }

    /// Every entry below the root: relative path -> file bytes (`None` for a directory).
    fn tree(&self) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
        fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                let rel = path.strip_prefix(base).unwrap().to_path_buf();
                if path.is_dir() {
                    out.insert(rel, None);
                    walk(base, &path, out);
                } else {
                    out.insert(rel, Some(std::fs::read(&path).unwrap()));
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(&self.0, &self.0, &mut out);
        out
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// --- Inputs and expectations --------------------------------------------------------------------

/// The Windows list of an example: `unavailable` = no list, `empty` = an empty list, otherwise
/// comma-separated tags in preference order.
fn prefs(list: &str) -> Option<Vec<String>> {
    match list {
        "unavailable" => None,
        "empty" => Some(Vec::new()),
        _ => Some(list.split(',').map(str::to_string).collect()),
    }
}

fn owner(root: &Root, list: &str) -> SettingsState {
    let list = prefs(list);
    SettingsState::new(Some(root.config_dir()), move || list)
}

fn owner_with(root: &Root, list: &str, fs: Arc<dyn SettingsFs>) -> SettingsState {
    let list = prefs(list);
    SettingsState::with_fs(Some(root.config_dir()), move || list, fs)
}

fn snap(language: Language, load_status: SettingsLoadStatus) -> SettingsSnapshot {
    SettingsSnapshot {
        settings: Settings { language },
        load_status,
    }
}

fn settings(language: Language) -> Settings {
    Settings { language }
}

fn code(language: Language) -> &'static str {
    match language {
        Language::En => "en",
        Language::Es => "es",
    }
}

/// The file holds valid schema 1 JSON for `language`: exactly the two fields, UTF-8, no BOM.
fn assert_schema1_file(root: &Root, language: Language) {
    let bytes = std::fs::read(root.file()).expect("the settings file exists");
    assert!(
        !bytes.starts_with(&[0xEF, 0xBB, 0xBF]),
        "a UTF-8 BOM is not written"
    );
    let text = std::str::from_utf8(&bytes).expect("UTF-8");
    let value: serde_json::Value = serde_json::from_str(text).expect("valid JSON");
    assert_eq!(
        value,
        serde_json::json!({"schemaVersion": 1, "language": code(language)}),
        "{text}"
    );
}

/// Names in the configuration directory (empty when it does not exist).
fn config_names(root: &Root) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(root.config_dir()) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

// --- A file system with one injected failure ----------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Read,
    CreateDir,
    TempWrite,
    TempFlush,
    Replace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Call {
    Read(PathBuf),
    CreateDir(PathBuf),
    CreateNew(PathBuf),
    Replace(PathBuf, PathBuf),
    Remove(PathBuf),
}

/// The standard file system, except the armed operation, which fails with `PermissionDenied`.
#[derive(Default)]
struct FaultyFs {
    fail: Mutex<Option<Op>>,
    calls: Mutex<Vec<Call>>,
}

impl FaultyFs {
    fn arm(&self, op: Option<Op>) {
        *self.fail.lock().unwrap() = op;
    }

    fn fails(&self, op: Op) -> bool {
        *self.fail.lock().unwrap() == Some(op)
    }

    fn log(&self, call: Call) {
        self.calls.lock().unwrap().push(call);
    }

    fn calls(&self) -> Vec<Call> {
        self.calls.lock().unwrap().clone()
    }
}

fn injected() -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, "injected failure")
}

struct FaultyFile {
    file: File,
    fail_write: bool,
    fail_flush: bool,
}

impl Write for FaultyFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.fail_write {
            return Err(injected());
        }
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.fail_flush {
            return Err(injected());
        }
        self.file.sync_all()
    }
}

impl SettingsFs for FaultyFs {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.log(Call::Read(path.to_path_buf()));
        if self.fails(Op::Read) {
            return Err(injected());
        }
        std::fs::read(path)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        self.log(Call::CreateDir(path.to_path_buf()));
        if self.fails(Op::CreateDir) {
            return Err(injected());
        }
        std::fs::create_dir_all(path)
    }

    fn create_new(&self, path: &Path) -> io::Result<Box<dyn Write + Send>> {
        self.log(Call::CreateNew(path.to_path_buf()));
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        Ok(Box::new(FaultyFile {
            file,
            fail_write: self.fails(Op::TempWrite),
            fail_flush: self.fails(Op::TempFlush),
        }))
    }

    fn replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.log(Call::Replace(from.to_path_buf(), to.to_path_buf()));
        if self.fails(Op::Replace) {
            return Err(injected());
        }
        std::fs::rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.log(Call::Remove(path.to_path_buf()));
        std::fs::remove_file(path)
    }
}

// ================================================================================================
// Scenario Outline: First launch detects Windows language without creating a file
// ================================================================================================

fn check_first_launch(case: &str, list: &str, expected: Language) {
    let root = Root::new(&format!("first-launch-{case}"));
    let before = root.tree();
    let calls = Arc::new(AtomicUsize::new(0));
    let state = {
        let calls = Arc::clone(&calls);
        let list = prefs(list);
        SettingsState::new(Some(root.config_dir()), move || {
            calls.fetch_add(1, Ordering::SeqCst);
            list
        })
    };
    assert_eq!(
        state.get(),
        snap(expected, SettingsLoadStatus::Default),
        "{list}"
    );
    assert_eq!(
        state.get(),
        snap(expected, SettingsLoadStatus::Default),
        "{list}"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "the Windows list is inspected once, during initialization"
    );
    assert_eq!(root.tree(), before, "initialization creates nothing");
    assert!(!root.config_dir().exists(), "no configuration directory");
}

// @RF-13 Examples: | es-ES,en-US | es |
#[test]
fn rf13_first_launch_es_es_then_en_us_is_spanish() {
    check_first_launch("1", "es-ES,en-US", Language::Es);
}

// @RF-13 Examples: | en-GB,es-MX | en |
#[test]
fn rf13_first_launch_en_gb_then_es_mx_is_english() {
    check_first_launch("2", "en-GB,es-MX", Language::En);
}

// @RF-13 Examples: | fr-FR,es-MX,en-US | es | (unsupported entries are skipped)
#[test]
fn rf13_first_launch_skips_unsupported_entries() {
    check_first_launch("3", "fr-FR,es-MX,en-US", Language::Es);
}

// @RF-13 Examples: | ES-mx | es | (the primary subtag matches case-insensitively)
#[test]
fn rf13_first_launch_matches_the_primary_subtag_case_insensitively() {
    check_first_launch("4", "ES-mx", Language::Es);
}

// @RF-13 Examples: | en | en |
#[test]
fn rf13_first_launch_bare_en_is_english() {
    check_first_launch("5", "en", Language::En);
}

// @RF-13 Examples: | es | es |
#[test]
fn rf13_first_launch_bare_es_is_spanish() {
    check_first_launch("6", "es", Language::Es);
}

// @RF-13 Examples: | fr-FR,ja-JP | en | (wholly unsupported list)
#[test]
fn rf13_first_launch_wholly_unsupported_list_falls_back_to_english() {
    check_first_launch("7", "fr-FR,ja-JP", Language::En);
}

// @RF-13 Examples: | empty | en |
#[test]
fn rf13_first_launch_empty_list_falls_back_to_english() {
    check_first_launch("8", "empty", Language::En);
}

// @RF-13 Examples: | unavailable | en |
#[test]
fn rf13_first_launch_unavailable_list_falls_back_to_english() {
    check_first_launch("9", "unavailable", Language::En);
}

// Section 3: "Match each language tag's primary subtag": other regional and script subtags.
#[test]
fn rf13_first_launch_matches_other_regional_and_script_tags() {
    check_first_launch("10", "es-419", Language::Es);
    check_first_launch("11", "eu-ES,en-US", Language::En);
    check_first_launch("12", "zh-Hans-CN,es-ES", Language::Es);
}

// Section 3: "No file or no config directory yet": an existing directory without the file.
#[test]
fn rf13_an_existing_directory_without_the_file_is_a_missing_file() {
    let root = Root::new("first-launch-dir");
    std::fs::create_dir_all(root.config_dir()).unwrap();
    std::fs::write(root.config_dir().join("other.json"), b"{}").unwrap();
    let before = root.tree();
    let state = owner(&root, "es-MX");
    assert_eq!(state.get(), snap(Language::Es, SettingsLoadStatus::Default));
    assert_eq!(root.tree(), before);
}

// Section 3: an orphan temporary file must not be loaded as the settings file.
#[test]
fn rf13_an_orphan_temporary_file_is_not_loaded() {
    let root = Root::new("orphan");
    std::fs::create_dir_all(root.config_dir()).unwrap();
    for name in [
        "settings.json.tmp",
        "settings.json.1234.tmp",
        ".settings.json~",
    ] {
        std::fs::write(root.config_dir().join(name), ES_FILE).unwrap();
    }
    let before = root.tree();
    let state = owner(&root, "en-US");
    assert_eq!(state.get(), snap(Language::En, SettingsLoadStatus::Default));
    assert_eq!(root.tree(), before);
}

// ================================================================================================
// Scenario: Restore a saved language before enabling the main screen (backend side)
// ================================================================================================

// @RF-13 the settings file contains schemaVersion 1 and language "es"; Windows says "en-US"
#[test]
fn rf13_a_valid_saved_language_takes_priority_over_windows() {
    let root = Root::new("restore-es");
    root.write_settings(ES_FILE);
    let before = root.tree();
    let calls = Arc::new(AtomicUsize::new(0));
    let state = {
        let calls = Arc::clone(&calls);
        SettingsState::new(Some(root.config_dir()), move || {
            calls.fetch_add(1, Ordering::SeqCst);
            prefs("en-US")
        })
    };
    assert_eq!(state.get(), snap(Language::Es, SettingsLoadStatus::Loaded));
    assert!(calls.load(Ordering::SeqCst) <= 1);
    assert_eq!(root.tree(), before, "loading never modifies anything");
}

// L2: "saved-language precedence after an OS language change"
#[test]
fn rf13_saved_english_stays_after_windows_changes_to_spanish() {
    let root = Root::new("restore-en");
    root.write_settings(EN_FILE);
    assert_eq!(
        owner(&root, "es-ES").get(),
        snap(Language::En, SettingsLoadStatus::Loaded)
    );
}

// Section 3: "key order and whitespace are not semantic"
#[test]
fn rf13_key_order_and_whitespace_are_not_semantic() {
    let root = Root::new("restore-order");
    root.write_settings(b"\r\n  { \"language\" : \"es\" ,\n\t\"schemaVersion\" : 1 }  \r\n");
    let before = root.tree();
    assert_eq!(
        owner(&root, "en-US").get(),
        snap(Language::Es, SettingsLoadStatus::Loaded)
    );
    assert_eq!(root.tree(), before);
}

// ================================================================================================
// Scenario Outline: An explicit Save survives a new app process
// ================================================================================================

fn check_save_survives(case: &str, before: Language, after: Language) {
    let root = Root::new(&format!("save-{case}"));
    // "the initialized app uses language <before>": English from Windows with no file, or a
    // saved Spanish file.
    let state = match before {
        Language::En => owner(&root, "en-US"),
        Language::Es => {
            root.write_settings(ES_FILE);
            owner(&root, "en-US")
        }
    };
    assert_eq!(state.get().settings.language, before);

    let saved = state.set(settings(after)).expect("the save succeeds");
    assert_eq!(saved, snap(after, SettingsLoadStatus::Loaded));
    assert_eq!(state.get(), snap(after, SettingsLoadStatus::Loaded));
    assert_schema1_file(&root, after);
    assert_eq!(
        config_names(&root),
        [FILE_NAME],
        "no temporary file is left"
    );

    // "a new app process reads language <after> with loadStatus loaded", whatever Windows says
    let opposite = match after {
        Language::En => "es-ES",
        Language::Es => "en-US",
    };
    assert_eq!(
        owner(&root, opposite).get(),
        snap(after, SettingsLoadStatus::Loaded)
    );
}

// @RF-13 Examples: | en | es |
#[test]
fn rf13_save_english_to_spanish_survives_a_new_process() {
    check_save_survives("en-es", Language::En, Language::Es);
}

// @RF-13 Examples: | es | en |
#[test]
fn rf13_save_spanish_to_english_survives_a_new_process() {
    check_save_survives("es-en", Language::Es, Language::En);
}

// @RF-13 Examples: | en | en | (saving the active language creates the first file)
#[test]
fn rf13_save_of_the_active_language_creates_the_first_file() {
    check_save_survives("en-en", Language::En, Language::En);
}

// Section 4 step 2: a temporary sibling with exclusive creation, flushed, then atomically
// replacing the destination; never written, truncated or deleted in place.
#[test]
fn rf13_a_save_publishes_through_a_temporary_sibling_and_an_atomic_replace() {
    let root = Root::new("atomic");
    root.write_settings(EN_FILE);
    let fs = Arc::new(FaultyFs::default());
    let state = owner_with(&root, "en-US", fs.clone());
    assert_eq!(state.get(), snap(Language::En, SettingsLoadStatus::Loaded));
    let start = fs.calls().len();

    state.set(settings(Language::Es)).expect("save");
    let calls = fs.calls()[start..].to_vec();
    let file = root.file();
    let temps: Vec<PathBuf> = calls
        .iter()
        .filter_map(|c| match c {
            Call::CreateNew(p) => Some(p.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(temps.len(), 1, "one temporary file: {calls:?}");
    let temp = &temps[0];
    assert_eq!(
        temp.parent(),
        Some(root.config_dir().as_path()),
        "a sibling"
    );
    assert_ne!(temp, &file, "the destination is never created in place");
    assert!(
        calls.contains(&Call::Replace(temp.clone(), file.clone())),
        "the temporary file replaces the destination: {calls:?}"
    );
    assert!(
        !calls.contains(&Call::Remove(file.clone())),
        "the destination is never deleted first"
    );
    let read_pos = calls.iter().position(|c| c == &Call::Read(file.clone()));
    let replace_pos = calls
        .iter()
        .position(|c| matches!(c, Call::Replace(..)))
        .unwrap();
    assert!(
        read_pos.is_some_and(|r| r < replace_pos),
        "the destination is inspected before it is replaced: {calls:?}"
    );
    assert_schema1_file(&root, Language::Es);
}

// Section 3: the directory is created by the first explicit Save only, inside the Unicode root.
#[test]
fn rf13_the_first_save_creates_the_configuration_directory() {
    let root = Root::new("create-dir");
    let state = owner(&root, "en-US");
    assert_eq!(state.get(), snap(Language::En, SettingsLoadStatus::Default));
    assert!(!root.config_dir().exists());
    assert_eq!(
        state.set(settings(Language::Es)).unwrap(),
        snap(Language::Es, SettingsLoadStatus::Loaded)
    );
    assert_schema1_file(&root, Language::Es);
}

// Section 2: "`set_settings` initializes it first if necessary".
#[test]
fn rf13_set_before_get_initializes_first() {
    let root = Root::new("set-first");
    root.write_settings(EN_FILE);
    let state = owner(&root, "en-US");
    assert_eq!(
        state.set(settings(Language::Es)).unwrap(),
        snap(Language::Es, SettingsLoadStatus::Loaded)
    );
    assert_eq!(state.get(), snap(Language::Es, SettingsLoadStatus::Loaded));
}

// Section 2: "Calls serialize through one backend-owned settings state".
#[test]
fn rf13_concurrent_saves_through_one_owner_are_serialized() {
    let root = Root::new("serialized");
    let state = owner(&root, "en-US");
    state.get();
    std::thread::scope(|s| {
        for i in 0..16 {
            let state = &state;
            s.spawn(move || {
                let language = if i % 2 == 0 {
                    Language::Es
                } else {
                    Language::En
                };
                assert_eq!(
                    state.set(settings(language)).unwrap(),
                    snap(language, SettingsLoadStatus::Loaded)
                );
            });
        }
    });
    let current = state.get();
    assert_eq!(current.load_status, SettingsLoadStatus::Loaded);
    assert_schema1_file(&root, current.settings.language);
    assert_eq!(config_names(&root), [FILE_NAME]);
}

// Section 4: "No file watcher or cross-process live synchronization"; the last replacement wins.
#[test]
fn rf13_independent_owners_do_not_synchronize_and_the_last_save_wins() {
    let root = Root::new("two-owners");
    let a = owner(&root, "en-US");
    let b = owner(&root, "en-US");
    assert_eq!(a.get(), snap(Language::En, SettingsLoadStatus::Default));
    assert_eq!(b.get(), snap(Language::En, SettingsLoadStatus::Default));
    a.set(settings(Language::Es)).unwrap();
    assert_eq!(b.get(), snap(Language::En, SettingsLoadStatus::Default));
    b.set(settings(Language::En)).unwrap();
    assert_eq!(a.get(), snap(Language::Es, SettingsLoadStatus::Loaded));
    assert_eq!(
        owner(&root, "es-ES").get(),
        snap(Language::En, SettingsLoadStatus::Loaded)
    );
}

// ================================================================================================
// Scenario Outline: A failed save preserves the old preference and permits retry (@RNF-04)
// ================================================================================================

fn check_failed_save(case: &str, op: Op) {
    let root = Root::new(&format!("failed-{case}"));
    let fs = Arc::new(FaultyFs::default());
    // See the header: directory creation fails only when the directory is missing.
    let expected_before = if op == Op::CreateDir {
        snap(Language::En, SettingsLoadStatus::Default)
    } else {
        root.write_settings(EN_FILE);
        snap(Language::En, SettingsLoadStatus::Loaded)
    };
    let state = owner_with(&root, "en-US", fs.clone());
    assert_eq!(state.get(), expected_before);
    let before = root.tree();

    fs.arm(Some(op));
    let error = state
        .set(settings(Language::Es))
        .expect_err("the save fails");
    assert_eq!(error.code, AppErrorCode::SettingsWriteFailed, "{op:?}");
    assert_eq!(
        root.tree(),
        before,
        "{op:?}: previous bytes kept, no file left"
    );
    assert_eq!(state.get(), expected_before, "{op:?}: previous snapshot");

    // "When I retry and the save succeeds"
    fs.arm(None);
    assert_eq!(
        state.set(settings(Language::Es)).unwrap(),
        snap(Language::Es, SettingsLoadStatus::Loaded)
    );
    assert_eq!(state.get(), snap(Language::Es, SettingsLoadStatus::Loaded));
    assert_schema1_file(&root, Language::Es);
    assert_eq!(config_names(&root), [FILE_NAME]);
    assert_eq!(
        owner(&root, "en-US").get(),
        snap(Language::Es, SettingsLoadStatus::Loaded)
    );
}

// @RF-13 @RNF-04 Examples: | directory creation |
#[test]
fn rf13_a_failed_directory_creation_keeps_the_previous_state() {
    check_failed_save("dir", Op::CreateDir);
}

// @RF-13 @RNF-04 Examples: | read before write |
#[test]
fn rf13_a_failed_read_before_write_keeps_the_previous_file() {
    check_failed_save("read", Op::Read);
}

// @RF-13 @RNF-04 Examples: | temporary file write |
#[test]
fn rf13_a_failed_temporary_write_keeps_the_previous_file() {
    check_failed_save("write", Op::TempWrite);
}

// @RF-13 @RNF-04 Examples: | temporary file flush |
#[test]
fn rf13_a_failed_temporary_flush_keeps_the_previous_file() {
    check_failed_save("flush", Op::TempFlush);
}

// @RF-13 @RNF-04 Examples: | destination replacement |
#[test]
fn rf13_a_failed_replacement_keeps_the_previous_file() {
    check_failed_save("replace", Op::Replace);
}

// ================================================================================================
// Scenario Outline: Invalid persisted settings fall back without modifying the file (@RNF-04)
// ================================================================================================

fn check_invalid(case: &str, content: &[u8]) {
    let root = Root::new(&format!("invalid-{case}"));
    root.write_settings(content);
    let before = root.tree();
    // Windows says Spanish: detection does not apply to the recovery cases.
    let state = owner(&root, "es-ES");
    assert_eq!(
        state.get(),
        snap(Language::En, SettingsLoadStatus::Invalid),
        "{}",
        String::from_utf8_lossy(content)
    );
    assert_eq!(root.tree(), before, "the invalid file is byte-identical");
    assert_eq!(
        owner(&root, "es-ES").get(),
        snap(Language::En, SettingsLoadStatus::Invalid),
        "a reload is still invalid: nothing was rewritten"
    );

    // "When I explicitly Save es / Then the file contains valid schema 1 settings for es"
    assert_eq!(
        state.set(settings(Language::Es)).unwrap(),
        snap(Language::Es, SettingsLoadStatus::Loaded)
    );
    assert_schema1_file(&root, Language::Es);
    assert_eq!(
        owner(&root, "en-US").get(),
        snap(Language::Es, SettingsLoadStatus::Loaded)
    );
}

// @RF-13 @RNF-04 Examples: | malformed JSON |
#[test]
fn rf13_malformed_json_is_invalid() {
    check_invalid("malformed", br#"{"schemaVersion":1,"language":"es""#);
}

// @RF-13 @RNF-04 Examples: | schema 1 with language fr |
#[test]
fn rf13_schema1_with_an_unsupported_language_is_invalid() {
    check_invalid("fr", br#"{"schemaVersion":1,"language":"fr"}"#);
}

// @RF-13 @RNF-04 Examples: | schema 1 with language ES | (case variants are invalid)
#[test]
fn rf13_schema1_with_an_uppercase_language_is_invalid() {
    check_invalid("upper", br#"{"schemaVersion":1,"language":"ES"}"#);
}

// @RF-13 @RNF-04 Examples: | schema 1 with missing language |
#[test]
fn rf13_schema1_without_language_is_invalid() {
    check_invalid("missing", br#"{"schemaVersion":1}"#);
}

// @RF-13 @RNF-04 Examples: | schema 1 with an unknown field |
#[test]
fn rf13_schema1_with_an_unknown_field_is_invalid() {
    check_invalid(
        "unknown",
        br#"{"schemaVersion":1,"language":"es","theme":"dark"}"#,
    );
}

// @RF-13 @RNF-04 Examples: | schema 1 with a duplicate language field |
#[test]
fn rf13_schema1_with_a_duplicate_language_is_invalid() {
    check_invalid(
        "duplicate",
        br#"{"schemaVersion":1,"language":"es","language":"es"}"#,
    );
}

// @RF-13 @RNF-04 Examples: | schemaVersion 0 |
#[test]
fn rf13_schema_version_zero_is_invalid() {
    check_invalid("zero", br#"{"schemaVersion":0,"language":"es"}"#);
}

// @RF-13 @RNF-04 Examples: | schemaVersion 1.5 |
#[test]
fn rf13_a_non_integer_schema_version_is_invalid() {
    check_invalid(
        "one-and-a-half",
        br#"{"schemaVersion":1.5,"language":"es"}"#,
    );
}

// Section 3: wrong types, trailing non-whitespace, negative and non-integer newer-looking
// versions, non-objects, empty and non-UTF-8 files are invalid too.
#[test]
fn rf13_other_schema_violations_are_invalid() {
    let cases: [(&str, &[u8]); 10] = [
        ("empty", b""),
        ("trailing", br#"{"schemaVersion":1,"language":"es"} x"#),
        (
            "version-string",
            br#"{"schemaVersion":"1","language":"es"}"#,
        ),
        ("negative", br#"{"schemaVersion":-1,"language":"es"}"#),
        (
            "two-and-a-half",
            br#"{"schemaVersion":2.5,"language":"es"}"#,
        ),
        ("no-version", br#"{"language":"es"}"#),
        ("language-null", br#"{"schemaVersion":1,"language":null}"#),
        ("region", br#"{"schemaVersion":1,"language":"es-ES"}"#),
        ("array", br#"["es"]"#),
        ("not-utf8", b"{\"schemaVersion\":1,\"language\":\"\xff\"}"),
    ];
    for (case, content) in cases {
        check_invalid(case, content);
    }
}

// ================================================================================================
// Scenario Outline: An unreadable configuration does not prevent analysis (@RNF-04)
// ================================================================================================

// @RF-13 @RNF-04 Examples: | config directory lookup |
#[test]
fn rf13_a_failed_directory_lookup_is_unavailable_english() {
    let state = SettingsState::new(None, || prefs("es-ES"));
    assert_eq!(
        state.get(),
        snap(Language::En, SettingsLoadStatus::Unavailable)
    );
    let error = state
        .set(settings(Language::Es))
        .expect_err("nowhere to save");
    assert_eq!(error.code, AppErrorCode::SettingsWriteFailed);
    assert_eq!(
        state.get(),
        snap(Language::En, SettingsLoadStatus::Unavailable)
    );
}

// @RF-13 @RNF-04 Examples: | file access denied | (and the explicit Save retry)
#[test]
fn rf13_a_denied_read_is_unavailable_english_and_save_retries() {
    let root = Root::new("unavailable-denied");
    root.write_settings(ES_FILE);
    let before = root.tree();
    let fs = Arc::new(FaultyFs::default());
    fs.arm(Some(Op::Read));
    let state = owner_with(&root, "es-ES", fs.clone());
    assert_eq!(
        state.get(),
        snap(Language::En, SettingsLoadStatus::Unavailable)
    );
    assert_eq!(root.tree(), before);

    fs.arm(None);
    assert_eq!(
        state.set(settings(Language::Es)).unwrap(),
        snap(Language::Es, SettingsLoadStatus::Loaded)
    );
    assert_schema1_file(&root, Language::Es);
}

// @RF-13 @RNF-04 Examples: | settings path is a directory |
#[test]
fn rf13_a_directory_at_the_settings_path_is_unavailable_english() {
    let root = Root::new("unavailable-dir");
    std::fs::create_dir_all(root.file()).unwrap();
    std::fs::write(root.file().join("inside.txt"), b"kept").unwrap();
    let before = root.tree();
    let state = owner(&root, "es-ES");
    assert_eq!(
        state.get(),
        snap(Language::En, SettingsLoadStatus::Unavailable)
    );
    assert_eq!(root.tree(), before);

    let error = state
        .set(settings(Language::Es))
        .expect_err("a directory cannot be replaced");
    assert_eq!(error.code, AppErrorCode::SettingsWriteFailed);
    assert_eq!(
        root.tree(),
        before,
        "the directory and its content are kept"
    );
    assert_eq!(
        state.get(),
        snap(Language::En, SettingsLoadStatus::Unavailable)
    );
}

// ================================================================================================
// Scenario: A newer settings version is preserved
// ================================================================================================

// @RF-13 a valid JSON object with schemaVersion 2; Windows says "es-ES"
#[test]
fn rf13_a_newer_version_is_english_and_read_only() {
    let cases: [(&str, &[u8]); 3] = [
        ("v2", NEWER_FILE),
        ("v2-bare", br#"{"schemaVersion":2}"#),
        // classified as newer before its other fields are interpreted
        (
            "v3-odd",
            br#"{"schemaVersion":3,"language":"xx","extra":[1,2]}"#,
        ),
    ];
    for (case, content) in cases {
        let root = Root::new(&format!("newer-{case}"));
        root.write_settings(content);
        let before = root.tree();
        let state = owner(&root, "es-ES");
        assert_eq!(
            state.get(),
            snap(Language::En, SettingsLoadStatus::NewerVersion),
            "{case}"
        );
        assert_eq!(root.tree(), before, "{case}");

        // "When set_settings is called directly with language es"
        let error = state
            .set(settings(Language::Es))
            .expect_err("a newer file is read-only");
        assert_eq!(error.code, AppErrorCode::SettingsReadOnly, "{case}");
        assert_eq!(root.tree(), before, "{case}: byte-identical");
        assert_eq!(
            state.get(),
            snap(Language::En, SettingsLoadStatus::NewerVersion),
            "{case}"
        );
    }
}

// Section 4 step 1: "The initial newerVersion state also refuses saving for this process even if
// the file changes externally".
#[test]
fn rf13_an_initial_newer_version_refuses_saves_for_the_whole_process() {
    let root = Root::new("newer-then-replaced");
    root.write_settings(NEWER_FILE);
    let state = owner(&root, "en-US");
    assert_eq!(
        state.get(),
        snap(Language::En, SettingsLoadStatus::NewerVersion)
    );
    root.write_settings(EN_FILE);
    let before = root.tree();
    let error = state.set(settings(Language::Es)).unwrap_err();
    assert_eq!(error.code, AppErrorCode::SettingsReadOnly);
    assert_eq!(root.tree(), before);
}

// ================================================================================================
// Scenario: A newer file appearing after startup is not overwritten
// ================================================================================================

// @RF-13 initialized from a valid schema 1 file; another process replaced it with schema 2
#[test]
fn rf13_a_newer_file_seen_by_the_pre_save_check_is_not_overwritten() {
    let root = Root::new("newer-after-start");
    root.write_settings(EN_FILE);
    let state = owner(&root, "en-US");
    assert_eq!(state.get(), snap(Language::En, SettingsLoadStatus::Loaded));

    root.write_settings(NEWER_FILE);
    let before = root.tree();
    let error = state
        .set(settings(Language::Es))
        .expect_err("the pre-save check sees schema 2");
    assert_eq!(error.code, AppErrorCode::SettingsReadOnly);
    assert_eq!(root.tree(), before, "the newer file is unchanged");
    assert_eq!(
        state.get(),
        snap(Language::En, SettingsLoadStatus::Loaded),
        "the active language is unchanged"
    );
}

// Section 4: "Explicit Save may replace a malformed schema-1-or-older file": a file that turned
// invalid after startup is not newer, so it is replaced.
#[test]
fn rf13_a_file_that_turned_invalid_after_startup_is_replaced() {
    let root = Root::new("invalid-after-start");
    root.write_settings(EN_FILE);
    let state = owner(&root, "en-US");
    state.get();
    root.write_settings(b"not json");
    assert_eq!(
        state.set(settings(Language::Es)).unwrap(),
        snap(Language::Es, SettingsLoadStatus::Loaded)
    );
    assert_schema1_file(&root, Language::Es);
}

// ================================================================================================
// Scenario Outline: Invalid IPC input cannot write a language preference
// ================================================================================================

// @RF-13 Examples: | language fr | language null | missing language |
// Typed deserialization of the `settings` argument rejects them before the handler (section 2).
#[test]
fn rf13_invalid_ipc_input_changes_neither_memory_nor_disk() {
    let root = Root::new("ipc");
    root.write_settings(EN_FILE);
    let state = owner(&root, "es-ES");
    assert_eq!(state.get(), snap(Language::En, SettingsLoadStatus::Loaded));
    let before = root.tree();
    for input in [
        serde_json::json!({"language": "fr"}),
        serde_json::json!({"language": null}),
        serde_json::json!({}),
    ] {
        assert!(
            serde_json::from_value::<Settings>(input.clone()).is_err(),
            "rejected: {input}"
        );
        assert_eq!(state.get(), snap(Language::En, SettingsLoadStatus::Loaded));
        assert_eq!(root.tree(), before);
    }
}

// ================================================================================================
// Scenario: Settings writes are confined to the Unicode app configuration root (@RNF-05 @RNF-09)
// ================================================================================================

/// A short generated 16-bit WAV (synthetic tone, never real audio).
fn generated_wav(path: &Path) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 8_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).unwrap();
    for i in 0..800u32 {
        let s = ((f64::from(i) * 0.3).sin() * 8_000.0) as i16;
        w.write_sample(s).unwrap();
    }
    w.finalize().unwrap();
}

// @RF-13 @RNF-05 @RNF-09
#[test]
fn rf13_a_save_touches_only_the_settings_file_of_the_unicode_root() {
    let root = Root::new("confined");
    let music = root.path().join("Música – Ñandú 日本語");
    std::fs::create_dir_all(&music).unwrap();
    generated_wav(&music.join("01 – Canción.wav"));
    generated_wav(&music.join("02 tone.wav"));
    let cache = root.path().join("cache dir");
    std::fs::create_dir_all(&cache).unwrap();
    std::fs::write(
        cache.join("results-cache.sqlite3"),
        b"SQLite format 3\0recorded",
    )
    .unwrap();
    std::fs::write(root.path().join("unrelated.txt"), "sin cambios ✓").unwrap();
    std::fs::create_dir_all(root.config_dir()).unwrap();
    std::fs::write(root.config_dir().join("other app file.json"), b"{\"x\":1}").unwrap();
    let before = root.tree();

    let state = owner(&root, "en-US");
    state.get();
    state.set(settings(Language::Es)).unwrap();

    let mut after = root.tree();
    let added = after
        .remove(&Path::new(APP_DIR).join(FILE_NAME))
        .expect("settings.json inside the configuration directory");
    assert!(added.is_some(), "settings.json is a file");
    assert_eq!(
        after, before,
        "audio, cache and unrelated files are byte-identical"
    );

    // "When I ... read it from a fresh settings owner"
    assert_eq!(
        owner(&root, "en-US").get(),
        snap(Language::Es, SettingsLoadStatus::Loaded)
    );
}

// ================================================================================================
// The native Windows source (section 3: "a safe backend adapter, separate from JSON storage")
// ================================================================================================

// Host-independent: only that the Windows user's list can be read and holds language tags. The
// detection rules themselves are tested above with injected lists.
#[cfg(windows)]
#[test]
fn rf13_the_native_source_reads_the_windows_user_language_list() {
    let tags = fakeflac_lib::settings::windows_preferred_languages()
        .expect("Windows provides the user's preferred UI languages");
    assert!(!tags.is_empty());
    for tag in &tags {
        assert!(
            !tag.is_empty() && tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'),
            "a language tag: {tag:?}"
        );
    }
}
