//! `trusted.txt` and `confirmed.txt` (inputs.md sections 1 and 2).
//!
//! One parser for both files: they share the grammar and differ in the second field (origin or
//! kind). Every error of a file is reported, as `<file>:<line>: <message>`, sorted by line.

use crate::error::ToolError;
use fakeflac_core::ports::{FileEntry, FileScanner};
use fakeflac_fs::WalkDirScanner;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Origins of `trusted.txt` with their class (inputs.md 1.5), in the order the error message lists them.
pub const ORIGINS: [(&str, &str); 9] = [
    ("cd-log", "primary"),
    ("accuraterip", "primary"),
    ("bandcamp", "primary"),
    ("qobuz", "primary"),
    ("tienda", "primary"),
    ("revisado", "reviewed"),
    ("usuario", "reviewed"),
    ("analisis-limpio", "clean"),
    ("analisis", "reference"),
];

/// A track of the corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    /// Canonical absolute path.
    pub abs: PathBuf,
    /// Relative to `FAKEFLAC_CORPUS`, `/` separators, names exactly as on disk.
    pub rel: String,
    pub size: u64,
    /// Unix seconds, as `FileEntry`.
    pub mtime: i64,
}

/// A track included by `trusted.txt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trusted {
    pub track: Track,
    pub origin: String,
    pub class: String,
    /// Line of the entry that decides it.
    pub line: usize,
    /// `W1`, `W2`… when `confirmed.txt` lists it as a false positive.
    pub watch_id: String,
}

/// A confirmed fake (`confirmed.txt`, kind `fake`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmedFake {
    pub track: Track,
    pub line: usize,
}

/// Both lists, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lists {
    /// Sorted by relative path.
    pub trusted: Vec<Trusted>,
    /// Sorted by relative path.
    pub fakes: Vec<ConfirmedFake>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListKind {
    Trusted,
    Confirmed,
}

/// A line of an entry or exclusion that passed path resolution.
struct Item {
    line: usize,
    exclusion: bool,
    /// Origin or kind (empty for exclusions).
    field: String,
    canon: PathBuf,
    is_dir: bool,
    /// FLAC and WAV files covered by the path.
    files: Vec<FileEntry>,
}

struct Raw {
    line: usize,
    exclusion: bool,
    path: String,
    field: Option<String>,
}

struct Parsed {
    items: Vec<Item>,
    errors: Vec<(usize, String)>,
}

/// Reads and resolves `trusted.txt` and, when it exists, `confirmed.txt`.
pub fn load(calibration: &Path, corpus: &Path) -> Result<Lists, ToolError> {
    let trusted_bytes = std::fs::read(calibration.join("trusted.txt"))
        .map_err(|_| ToolError::input("trusted.txt: not found in FAKEFLAC_CALIBRATION"))?;
    let trusted = parse_list(&trusted_bytes, corpus, ListKind::Trusted, None);
    let (trusted_tracks, errors) = finish(trusted, "trusted.txt", ListKind::Trusted, None);
    if !errors.is_empty() {
        return Err(ToolError::input_lines(errors));
    }
    let mut trusted_list: Vec<Trusted> = Vec::new();
    for (entry, class, origin, line) in trusted_tracks.values() {
        trusted_list.push(Trusted {
            track: to_track(entry, corpus),
            origin: origin.clone(),
            class: class.clone(),
            line: *line,
            watch_id: String::new(),
        });
    }
    if trusted_list.is_empty() {
        return Err(ToolError::input("trusted.txt: no tracks listed"));
    }

    let mut fakes: Vec<ConfirmedFake> = Vec::new();
    if let Ok(bytes) = std::fs::read(calibration.join("confirmed.txt")) {
        let lines_by_track: BTreeMap<PathBuf, usize> = trusted_tracks
            .iter()
            .map(|(path, (_, _, _, line))| (path.clone(), *line))
            .collect();
        let parsed = parse_list(&bytes, corpus, ListKind::Confirmed, Some(&lines_by_track));
        let (confirmed_tracks, errors) = finish(
            parsed,
            "confirmed.txt",
            ListKind::Confirmed,
            Some(&lines_by_track),
        );
        if !errors.is_empty() {
            return Err(ToolError::input_lines(errors));
        }
        // `confirmed_tracks`: fakes (class `fake`) and watch tracks (class `false-positive`).
        let mut watch_lines: Vec<(usize, PathBuf)> = Vec::new();
        for (path, (entry, kind, _, line)) in &confirmed_tracks {
            if kind == "fake" {
                fakes.push(ConfirmedFake {
                    track: to_track(entry, corpus),
                    line: *line,
                });
            } else {
                watch_lines.push((*line, path.clone()));
            }
        }
        watch_lines.sort();
        for (i, (_, path)) in watch_lines.iter().enumerate() {
            if let Some(t) = trusted_list.iter_mut().find(|t| &t.track.abs == path) {
                t.watch_id = format!("W{}", i + 1);
            }
        }
    }
    trusted_list.sort_by(|a, b| a.track.rel.cmp(&b.track.rel));
    fakes.sort_by(|a, b| a.track.rel.cmp(&b.track.rel));
    Ok(Lists {
        trusted: trusted_list,
        fakes,
    })
}

fn to_track(entry: &FileEntry, corpus: &Path) -> Track {
    let rel = entry
        .path
        .strip_prefix(corpus)
        .map(|p| {
            p.components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        })
        .unwrap_or_default();
    Track {
        abs: entry.path.clone(),
        rel,
        size: entry.size_bytes,
        mtime: entry.modified_unix,
    }
}

/// Track path -> (file entry, class or kind, origin, deciding line).
type Resolved = BTreeMap<PathBuf, (FileEntry, String, String, usize)>;

/// Turns the parsed items into the included tracks and the final sorted error lines.
fn finish(
    parsed: Parsed,
    name: &str,
    kind: ListKind,
    trusted: Option<&BTreeMap<PathBuf, usize>>,
) -> (Resolved, Vec<String>) {
    let Parsed { items, mut errors } = parsed;
    // Exclusions: strictly inside a listed folder, and covering at least one file.
    let folders: Vec<&Item> = items
        .iter()
        .filter(|i| !i.exclusion && i.is_dir && contests(kind, i))
        .collect();
    for item in items.iter().filter(|i| i.exclusion) {
        let inside = folders
            .iter()
            .any(|f| item.canon != f.canon && item.canon.starts_with(&f.canon));
        if !inside {
            errors.push((
                item.line,
                "exclusion is not inside any listed folder".to_string(),
            ));
        } else if item.files.is_empty() {
            errors.push((
                item.line,
                "exclusion matches no FLAC or WAV file".to_string(),
            ));
        }
    }

    // The most specific line wins.
    let mut resolved: Resolved = BTreeMap::new();
    let contest: Vec<&Item> = items
        .iter()
        .filter(|i| i.exclusion || contests(kind, i))
        .collect();
    for item in items.iter().filter(|i| !i.exclusion) {
        if kind == ListKind::Confirmed && item.field == "false-positive" {
            if let Some(file) = item.files.first() {
                resolved.insert(
                    item.canon.clone(),
                    (file.clone(), item.field.clone(), String::new(), item.line),
                );
            }
            continue;
        }
        for file in &item.files {
            let Some(winner) = contest
                .iter()
                .filter(|c| file.path.starts_with(&c.canon))
                .max_by_key(|c| c.canon.components().count())
            else {
                continue;
            };
            if winner.exclusion {
                continue;
            }
            let (class, origin) = match kind {
                ListKind::Trusted => (class_of(&winner.field).to_string(), winner.field.clone()),
                ListKind::Confirmed => ("fake".to_string(), String::new()),
            };
            resolved.insert(
                file.path.clone(),
                (file.clone(), class, origin, winner.line),
            );
        }
    }

    // A confirmed fake that is also trusted.
    if kind == ListKind::Confirmed {
        if let Some(trusted) = trusted {
            let mut reported: Vec<usize> = Vec::new();
            for (path, (_, class, _, line)) in &resolved {
                if class == "fake" && !reported.contains(line) {
                    if let Some(tline) = trusted.get(path) {
                        reported.push(*line);
                        errors.push((*line, format!("also trusted (trusted.txt line {tline})")));
                    }
                }
            }
        }
    }

    let mut indexed: Vec<(usize, usize, String)> = errors
        .into_iter()
        .enumerate()
        .map(|(seq, (line, msg))| (line, seq, msg))
        .collect();
    indexed.sort();
    let lines = indexed
        .into_iter()
        .map(|(line, _, msg)| {
            if line == 0 {
                format!("{name}: {msg}")
            } else {
                format!("{name}:{line}: {msg}")
            }
        })
        .collect();
    (resolved, lines)
}

/// Whether an entry takes part in the "most specific line wins" contest of its file.
fn contests(kind: ListKind, item: &Item) -> bool {
    kind == ListKind::Trusted || item.field == "fake"
}

fn class_of(origin: &str) -> &'static str {
    ORIGINS
        .iter()
        .find(|(o, _)| *o == origin)
        .map_or("primary", |(_, c)| c)
}

/// Splits the text in lines and classifies them (inputs.md 1.1), then resolves the paths.
fn parse_list(
    bytes: &[u8],
    corpus: &Path,
    kind: ListKind,
    trusted: Option<&BTreeMap<PathBuf, usize>>,
) -> Parsed {
    let mut errors: Vec<(usize, String)> = Vec::new();
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(bytes);
    let text = match std::str::from_utf8(bytes) {
        Ok(t) => t.to_string(),
        Err(e) => {
            let line = bytes[..e.valid_up_to()]
                .iter()
                .filter(|b| **b == b'\n')
                .count()
                + 1;
            errors.push((line, "invalid UTF-8".to_string()));
            String::from_utf8_lossy(bytes).into_owned()
        }
    };

    let mut root_line: Option<usize> = None;
    let mut root_ok = true;
    let mut raws: Vec<Raw> = Vec::new();
    for (index, raw_line) in text.split('\n').enumerate() {
        let line = index + 1;
        let trimmed = raw_line.strip_suffix('\r').unwrap_or(raw_line).trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some(value) = root_value(trimmed) {
            if let Some(first) = root_line {
                errors.push((line, format!("library_root is already set at line {first}")));
            } else {
                root_line = Some(line);
                if let Err(message) = check_root(value, corpus) {
                    root_ok = false;
                    errors.push((line, message));
                }
            }
            continue;
        }
        let (exclusion, body) = match trimmed.strip_prefix('!') {
            Some(rest) => (true, rest.trim()),
            None => (false, trimmed),
        };
        if root_line.is_none() {
            errors.push((line, "entry before library_root".to_string()));
            continue;
        }
        let mut fields = body.split('|');
        let path = fields.next().unwrap_or("").trim().to_string();
        let field = fields.next().map(|f| f.trim().to_string());
        raws.push(Raw {
            line,
            exclusion,
            path,
            field,
        });
    }
    if root_line.is_none() {
        root_ok = false;
        errors.push((0, "library_root is missing".to_string()));
    }
    if !root_ok {
        return Parsed {
            items: Vec::new(),
            errors,
        };
    }

    let scanner = WalkDirScanner;
    let mut items: Vec<Item> = Vec::new();
    // Canonical path -> (line, is exclusion), for "already listed" and "contradicts".
    let mut seen: BTreeMap<PathBuf, (usize, bool)> = BTreeMap::new();
    for raw in raws {
        let (canon, is_dir) = match resolve_path(&raw.path, corpus) {
            Ok(r) => r,
            Err(message) => {
                errors.push((raw.line, message));
                continue;
            }
        };
        let field = raw.field.clone().unwrap_or_default();
        if !raw.exclusion {
            if let Err(message) = check_field(kind, &field) {
                errors.push((raw.line, message));
                continue;
            }
        }
        if let Some((first, was_exclusion)) = seen.get(&canon) {
            let message = if *was_exclusion == raw.exclusion {
                format!("already listed at line {first}")
            } else {
                format!("contradicts line {first}")
            };
            errors.push((raw.line, message));
            continue;
        }
        seen.insert(canon.clone(), (raw.line, raw.exclusion));

        let files = if is_dir {
            scanner.scan(&canon).unwrap_or_default()
        } else if has_lossless_extension(&canon) {
            single_file_entry(&canon).into_iter().collect()
        } else {
            Vec::new()
        };
        if !raw.exclusion {
            if let Some(message) = entry_problem(kind, &field, is_dir, &files, &canon, trusted) {
                errors.push((raw.line, message));
                continue;
            }
        }
        items.push(Item {
            line: raw.line,
            exclusion: raw.exclusion,
            field: if raw.exclusion { String::new() } else { field },
            canon,
            is_dir,
            files,
        });
    }
    Parsed { items, errors }
}

/// `library_root = value` (the value, untrimmed), or `None` when the line is something else.
fn root_value(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix("library_root")?;
    let rest = rest.trim_start().strip_prefix('=')?;
    Some(rest)
}

fn check_root(value: &str, corpus: &Path) -> Result<(), String> {
    let path = Path::new(value.trim());
    if !path.is_absolute() {
        return Err("library_root is not an absolute path".to_string());
    }
    if !path.is_dir() {
        return Err("library_root does not exist".to_string());
    }
    match std::fs::canonicalize(path) {
        Ok(c) if c == corpus => Ok(()),
        _ => Err("library_root does not match FAKEFLAC_CORPUS".to_string()),
    }
}

fn check_field(kind: ListKind, field: &str) -> Result<(), String> {
    match kind {
        ListKind::Trusted => {
            if field.is_empty() {
                Err("missing origin".to_string())
            } else if ORIGINS.iter().any(|(o, _)| *o == field) {
                Ok(())
            } else {
                let expected: Vec<&str> = ORIGINS.iter().map(|(o, _)| *o).collect();
                Err(format!(
                    "unknown origin '{field}' (expected one of: {})",
                    expected.join(", ")
                ))
            }
        }
        ListKind::Confirmed => {
            if field.is_empty() {
                Err("missing kind".to_string())
            } else if field == "fake" || field == "false-positive" {
                Ok(())
            } else {
                Err(format!(
                    "unknown kind '{field}' (expected fake or false-positive)"
                ))
            }
        }
    }
}

/// The problem of an entry once its path exists and its field is valid.
fn entry_problem(
    kind: ListKind,
    field: &str,
    is_dir: bool,
    files: &[FileEntry],
    canon: &Path,
    trusted: Option<&BTreeMap<PathBuf, usize>>,
) -> Option<String> {
    if kind == ListKind::Confirmed && field == "false-positive" {
        if is_dir {
            return Some("must be a file".to_string());
        }
        if files.is_empty() {
            return Some("not a FLAC or WAV file".to_string());
        }
        let known = trusted.is_some_and(|t| t.contains_key(canon));
        if !known {
            return Some("not a trusted track".to_string());
        }
        return None;
    }
    if is_dir {
        if files.is_empty() {
            return Some("no FLAC or WAV files under this folder".to_string());
        }
    } else if files.is_empty() {
        return Some("not a FLAC or WAV file".to_string());
    }
    None
}

fn has_lossless_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "flac" | "wav"))
}

fn single_file_entry(path: &Path) -> Option<FileEntry> {
    let meta = std::fs::metadata(path).ok()?;
    let format = fakeflac_core::AudioFormat::from_extension(path.extension()?.to_str()?)?;
    let modified_unix =
        meta.modified()
            .ok()
            .map_or(0, |t| match t.duration_since(std::time::UNIX_EPOCH) {
                Ok(after) => after.as_secs() as i64,
                Err(before) => -(before.duration().as_secs() as i64),
            });
    Some(FileEntry {
        path: path.to_path_buf(),
        folder: path.parent()?.to_path_buf(),
        format,
        size_bytes: meta.len(),
        modified_unix,
    })
}

/// Resolves a path as written (inputs.md 1.3) to its canonical form. The error is the first problem.
fn resolve_path(text: &str, corpus: &Path) -> Result<(PathBuf, bool), String> {
    if text.is_empty() {
        return Err("missing path".to_string());
    }
    let components: Vec<&str> = text.split(['\\', '/']).filter(|c| !c.is_empty()).collect();
    if components.contains(&"..") {
        return Err("'..' is not allowed".to_string());
    }
    let absolute = Path::new(text).is_absolute();
    let path = if absolute {
        PathBuf::from(text)
    } else {
        let mut p = corpus.to_path_buf();
        for c in components.iter().filter(|c| **c != ".") {
            p.push(c);
        }
        p
    };
    let exists = std::fs::metadata(&path).is_ok();
    if absolute && !exists {
        // Compared per component, not by string prefix.
        let parts = |p: &Path| -> Vec<String> {
            let text = p.to_string_lossy().replace('\\', "/").to_lowercase();
            let text = match text.strip_prefix("//?/unc/") {
                Some(rest) => format!("//{rest}"),
                None => text.strip_prefix("//?/").unwrap_or(&text).to_string(),
            };
            text.split('/')
                .filter(|c| !c.is_empty())
                .map(str::to_string)
                .collect()
        };
        if !parts(&path).starts_with(&parts(corpus)) {
            return Err("path is outside library_root".to_string());
        }
    }
    if !exists {
        return Err(format!("path does not exist: {text}"));
    }
    let canon = std::fs::canonicalize(&path).map_err(|_| format!("path does not exist: {text}"))?;
    if !canon.starts_with(corpus) {
        return Err("path is outside library_root".to_string());
    }
    let is_dir = canon.is_dir();
    Ok((canon, is_dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_line_forms() {
        assert_eq!(root_value("library_root = C:\\x"), Some(" C:\\x"));
        assert_eq!(root_value("library_root=C:\\x"), Some("C:\\x"));
        assert_eq!(root_value("library_rootx = C:\\x"), None);
        assert_eq!(root_value("library_root"), None);
    }

    #[test]
    fn classes_of_origins() {
        assert_eq!(class_of("tienda"), "primary");
        assert_eq!(class_of("usuario"), "reviewed");
        assert_eq!(class_of("analisis-limpio"), "clean");
        assert_eq!(class_of("analisis"), "reference");
    }
}
