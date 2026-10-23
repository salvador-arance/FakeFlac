//! `trusted.txt` and `confirmed.txt` (inputs.md sections 1 and 2, lossy.md section 1).
//!
//! One parser for both files: they share the grammar and differ in the second field (origin or
//! kind, then attributes). Every error of a file is reported, as `<file>:<line>: <message>`, sorted
//! by line.

use crate::error::ToolError;
use fakeflac_core::ports::{FileEntry, FileScanner};
use fakeflac_core::AudioFormat;
use fakeflac_fs::WalkDirScanner;
use std::collections::{BTreeMap, BTreeSet, HashSet};
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

const NOT_SUPPORTED: &str = "not a supported audio file (FLAC, WAV, MP3, M4A or Opus)";

/// The attributes of the line that gives a track its label (lossy.md section 1.2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attrs {
    pub group: Option<String>,
    pub chain: Option<String>,
    /// `aac`, `opus`, `mp3`, `unknown` or `by-rate` (resolved after extraction).
    pub upstream: Option<String>,
    pub source_max_kbps: Option<u32>,
    pub weak: bool,
}

/// Which files a folder entry includes (lossy.md section 1.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Formats {
    Lossless,
    Lossy,
    All,
}

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
    pub attrs: Attrs,
}

/// A confirmed fake (`confirmed.txt`, kind `fake`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmedFake {
    pub track: Track,
    pub line: usize,
    pub attrs: Attrs,
}

/// An honest lossy file of `confirmed.txt` (kind `real`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmedReal {
    pub track: Track,
    pub line: usize,
    pub attrs: Attrs,
}

/// Both lists, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lists {
    /// Sorted by relative path.
    pub trusted: Vec<Trusted>,
    /// Sorted by relative path.
    pub fakes: Vec<ConfirmedFake>,
    /// Sorted by relative path.
    pub reals: Vec<ConfirmedReal>,
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
    attrs: Attrs,
    canon: PathBuf,
    is_dir: bool,
    /// The files covered by the path (for a folder, those its `formats=` selects).
    files: Vec<FileEntry>,
    covered: HashSet<PathBuf>,
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

/// A track with the line that decides it.
#[derive(Clone)]
struct Won {
    file: FileEntry,
    /// `fake`, `real`, `false-positive` (confirmed) or empty (trusted).
    kind: String,
    class: String,
    origin: String,
    line: usize,
    attrs: Attrs,
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
    for won in &trusted_tracks {
        trusted_list.push(Trusted {
            track: to_track(&won.file, corpus),
            origin: won.origin.clone(),
            class: won.class.clone(),
            line: won.line,
            watch_id: String::new(),
            attrs: won.attrs.clone(),
        });
    }
    if trusted_list.is_empty() {
        return Err(ToolError::input("trusted.txt: no tracks listed"));
    }

    let mut fakes: Vec<ConfirmedFake> = Vec::new();
    let mut reals: Vec<ConfirmedReal> = Vec::new();
    if let Ok(bytes) = std::fs::read(calibration.join("confirmed.txt")) {
        let lines_by_track: BTreeMap<PathBuf, usize> = trusted_tracks
            .iter()
            .map(|w| (w.file.path.clone(), w.line))
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
        // Fakes, honest lossy files and watch tracks.
        let mut watch_lines: Vec<(usize, PathBuf)> = Vec::new();
        for won in &confirmed_tracks {
            match won.kind.as_str() {
                "fake" => fakes.push(ConfirmedFake {
                    track: to_track(&won.file, corpus),
                    line: won.line,
                    attrs: won.attrs.clone(),
                }),
                "real" => reals.push(ConfirmedReal {
                    track: to_track(&won.file, corpus),
                    line: won.line,
                    attrs: won.attrs.clone(),
                }),
                _ => watch_lines.push((won.line, won.file.path.clone())),
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
    reals.sort_by(|a, b| a.track.rel.cmp(&b.track.rel));
    Ok(Lists {
        trusted: trusted_list,
        fakes,
        reals,
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

/// The winner of every file among the entries selected by `take` (and every exclusion): the most
/// specific line wins.
fn resolve(items: &[Item], take: impl Fn(&Item) -> bool) -> BTreeMap<PathBuf, Won> {
    let contest: Vec<&Item> = items.iter().filter(|i| i.exclusion || take(i)).collect();
    let mut resolved: BTreeMap<PathBuf, Won> = BTreeMap::new();
    for item in items.iter().filter(|i| !i.exclusion && take(i)) {
        for file in &item.files {
            let Some(winner) = contest
                .iter()
                .filter(|c| {
                    if c.exclusion {
                        file.path.starts_with(&c.canon)
                    } else {
                        c.covered.contains(&file.path)
                    }
                })
                .max_by_key(|c| c.canon.components().count())
            else {
                continue;
            };
            if winner.exclusion {
                continue;
            }
            let (class, origin) = if winner.field == "fake" || winner.field == "real" {
                (winner.field.clone(), String::new())
            } else {
                (class_of(&winner.field).to_string(), winner.field.clone())
            };
            resolved.insert(
                file.path.clone(),
                Won {
                    file: file.clone(),
                    kind: String::new(),
                    class,
                    origin,
                    line: winner.line,
                    attrs: winner.attrs.clone(),
                },
            );
        }
    }
    resolved
}

/// Turns the parsed items into the included tracks and the final sorted error lines.
fn finish(
    parsed: Parsed,
    name: &str,
    kind: ListKind,
    trusted: Option<&BTreeMap<PathBuf, usize>>,
) -> (Vec<Won>, Vec<String>) {
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

    let mut result: Vec<Won> = Vec::new();
    match kind {
        ListKind::Trusted => {
            for (_, won) in resolve(&items, |_| true) {
                result.push(won);
            }
        }
        ListKind::Confirmed => {
            let fakes = resolve(&items, |i| i.field == "fake");
            let reals = resolve(&items, |i| i.field == "real");
            // A track in both: the later line is the error.
            for (path, real) in &reals {
                if let Some(fake) = fakes.get(path) {
                    let (first, later) = if fake.line <= real.line {
                        (fake.line, real.line)
                    } else {
                        (real.line, fake.line)
                    };
                    errors.push((later, format!("already listed at line {first}")));
                }
            }
            // A confirmed fake or real that is also trusted.
            if let Some(trusted) = trusted {
                let mut reported: Vec<usize> = Vec::new();
                for (path, won) in fakes.iter().chain(reals.iter()) {
                    if !reported.contains(&won.line) {
                        if let Some(tline) = trusted.get(path) {
                            reported.push(won.line);
                            errors.push((
                                won.line,
                                format!("also trusted (trusted.txt line {tline})"),
                            ));
                        }
                    }
                }
            }
            for (_, mut won) in fakes {
                won.kind = "fake".to_string();
                result.push(won);
            }
            for (path, mut won) in reals {
                if result.iter().any(|w| w.file.path == path) {
                    continue;
                }
                won.kind = "real".to_string();
                result.push(won);
            }
            for item in items.iter().filter(|i| !i.exclusion) {
                if item.field == "false-positive" {
                    if let Some(file) = item.files.first() {
                        result.push(Won {
                            file: file.clone(),
                            kind: "false-positive".to_string(),
                            class: "false-positive".to_string(),
                            origin: String::new(),
                            line: item.line,
                            attrs: Attrs::default(),
                        });
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
    (result, lines)
}

/// Whether an entry takes part in the "most specific line wins" contest of its file.
fn contests(kind: ListKind, item: &Item) -> bool {
    kind == ListKind::Trusted || item.field == "fake" || item.field == "real"
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
        let field_text = raw.field.clone().unwrap_or_default();
        let mut words = field_text.split_whitespace();
        let word = words.next().unwrap_or("").to_string();
        let tokens: Vec<&str> = words.collect();
        let (attrs, formats) = if raw.exclusion {
            (Attrs::default(), Formats::All)
        } else {
            if let Err(message) = check_field(kind, &word) {
                errors.push((raw.line, message));
                continue;
            }
            match parse_attrs(kind, &word, is_dir, &tokens) {
                Ok(parsed) => parsed,
                Err(message) => {
                    errors.push((raw.line, message));
                    continue;
                }
            }
        };
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

        let files: Vec<FileEntry> = if is_dir {
            scanner
                .scan(&canon)
                .unwrap_or_default()
                .into_iter()
                .filter(|f| selects(formats, f.format))
                .collect()
        } else {
            single_file_entry(&canon).into_iter().collect()
        };
        if !raw.exclusion {
            if let Some(message) = entry_problem(
                kind, &word, &attrs, formats, is_dir, &files, &canon, trusted,
            ) {
                errors.push((raw.line, message));
                continue;
            }
        }
        let covered = files.iter().map(|f| f.path.clone()).collect();
        items.push(Item {
            line: raw.line,
            exclusion: raw.exclusion,
            field: if raw.exclusion { String::new() } else { word },
            attrs,
            canon,
            is_dir,
            files,
            covered,
        });
    }
    Parsed { items, errors }
}

/// Whether a folder entry with these `formats=` includes a file of this format. Opus files are never
/// enumerated under folders.
fn selects(formats: Formats, format: AudioFormat) -> bool {
    match (formats, format) {
        (_, AudioFormat::Opus) => false,
        (Formats::All, _) => true,
        (Formats::Lossless, f) => matches!(f, AudioFormat::Flac | AudioFormat::Wav),
        (Formats::Lossy, f) => matches!(f, AudioFormat::Mp3 | AudioFormat::M4a),
    }
}

fn is_lossy_format(format: AudioFormat) -> bool {
    matches!(
        format,
        AudioFormat::Mp3 | AudioFormat::M4a | AudioFormat::Opus
    )
}

fn valid_id(id: &str) -> bool {
    (1..=16).contains(&id.chars().count())
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// The attributes of field 2 (lossy.md section 1.2). The error is the first problem.
fn parse_attrs(
    kind: ListKind,
    word: &str,
    is_dir: bool,
    tokens: &[&str],
) -> Result<(Attrs, Formats), String> {
    let mut attrs = Attrs::default();
    let mut formats = if word == "real" {
        Formats::Lossy
    } else {
        Formats::Lossless
    };
    if tokens.is_empty() {
        return Ok((attrs, formats));
    }
    if kind == ListKind::Confirmed && word == "false-positive" {
        return Err(
            "false-positive lines take no attributes; write them on the trusted.txt line"
                .to_string(),
        );
    }
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for token in tokens {
        let (name, value) = match token.split_once('=') {
            Some((n, v)) => (n, Some(v)),
            None => (*token, None),
        };
        let known = matches!(
            name,
            "formats" | "group" | "chain" | "upstream" | "source_max_kbps"
        ) || (name == "weak" && value.is_none());
        if !known {
            return Err(format!("unknown attribute '{name}'"));
        }
        if !seen.insert(name) {
            return Err(format!("attribute '{name}' given twice"));
        }
        let value = value.unwrap_or("");
        match name {
            "formats" => {
                formats = match value {
                    "lossless" => Formats::Lossless,
                    "lossy" => Formats::Lossy,
                    "all" => Formats::All,
                    other => {
                        return Err(format!(
                            "invalid value '{other}' for formats (expected lossless, lossy or all)"
                        ))
                    }
                };
                if !is_dir {
                    return Err("formats= applies to folders only".to_string());
                }
            }
            "group" | "chain" => {
                if !valid_id(value) {
                    return Err(format!(
                        "invalid id '{value}' for {name} (1 to 16 letters, digits, '_' or '-')"
                    ));
                }
                if name == "group" {
                    attrs.group = Some(value.to_string());
                } else {
                    attrs.chain = Some(value.to_string());
                }
            }
            _ => {
                match name {
                    "upstream" => {
                        if !["aac", "opus", "mp3", "unknown", "by-rate"].contains(&value) {
                            return Err(format!(
                                "invalid value '{value}' for upstream (expected aac, opus, mp3, unknown or by-rate)"
                            ));
                        }
                        attrs.upstream = Some(value.to_string());
                    }
                    "source_max_kbps" => match value.parse::<u32>() {
                        Ok(n) if (8..=640).contains(&n) => attrs.source_max_kbps = Some(n),
                        _ => return Err("source_max_kbps must be an integer from 8 to 640".into()),
                    },
                    _ => attrs.weak = true,
                }
                if word != "fake" {
                    return Err(
                        "upstream=, source_max_kbps= and weak apply to fake lines only".to_string(),
                    );
                }
            }
        }
    }
    Ok((attrs, formats))
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

fn check_field(kind: ListKind, word: &str) -> Result<(), String> {
    match kind {
        ListKind::Trusted => {
            if word.is_empty() {
                Err("missing origin".to_string())
            } else if ORIGINS.iter().any(|(o, _)| *o == word) {
                Ok(())
            } else {
                let expected: Vec<&str> = ORIGINS.iter().map(|(o, _)| *o).collect();
                Err(format!(
                    "unknown origin '{word}' (expected one of: {})",
                    expected.join(", ")
                ))
            }
        }
        ListKind::Confirmed => {
            if word.is_empty() {
                Err("missing kind".to_string())
            } else if word == "fake" || word == "real" || word == "false-positive" {
                Ok(())
            } else {
                Err(format!(
                    "unknown kind '{word}' (expected fake, real or false-positive)"
                ))
            }
        }
    }
}

/// The problem of an entry once its path exists and its field is valid.
#[allow(clippy::too_many_arguments)]
fn entry_problem(
    kind: ListKind,
    word: &str,
    attrs: &Attrs,
    formats: Formats,
    is_dir: bool,
    files: &[FileEntry],
    canon: &Path,
    trusted: Option<&BTreeMap<PathBuf, usize>>,
) -> Option<String> {
    if kind == ListKind::Confirmed && word == "false-positive" {
        if is_dir {
            return Some("must be a file".to_string());
        }
        if files.is_empty() {
            return Some(NOT_SUPPORTED.to_string());
        }
        let known = trusted.is_some_and(|t| t.contains_key(canon));
        if !known {
            return Some("not a trusted track".to_string());
        }
        return None;
    }
    if is_dir {
        if files.is_empty() {
            return Some(
                match formats {
                    Formats::Lossless => "no FLAC or WAV files under this folder",
                    Formats::Lossy => "no MP3 or M4A files under this folder",
                    Formats::All => "no supported audio files under this folder",
                }
                .to_string(),
            );
        }
    } else if files.is_empty() {
        return Some(NOT_SUPPORTED.to_string());
    } else if word == "real" && files.iter().any(|f| !is_lossy_format(f.format)) {
        return Some("real applies to lossy files only".to_string());
    }
    let lossy_attrs = attrs.upstream.is_some() || attrs.source_max_kbps.is_some() || attrs.weak;
    if kind == ListKind::Confirmed
        && word == "fake"
        && lossy_attrs
        && !files.iter().any(|f| is_lossy_format(f.format))
    {
        return Some("upstream=, source_max_kbps= and weak apply to lossy files only".to_string());
    }
    None
}

fn single_file_entry(path: &Path) -> Option<FileEntry> {
    let meta = std::fs::metadata(path).ok()?;
    let ext = path.extension()?.to_str()?;
    let format = AudioFormat::from_extension(ext).or_else(|| {
        ext.eq_ignore_ascii_case("opus")
            .then_some(AudioFormat::Opus)
    })?;
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

    #[test]
    fn attributes_of_a_line() {
        let ok = |kind, word, dir, tokens: &[&str]| parse_attrs(kind, word, dir, tokens);
        let (attrs, _) = ok(
            ListKind::Confirmed,
            "fake",
            false,
            &["upstream=aac", "source_max_kbps=128", "weak", "chain=C1"],
        )
        .unwrap();
        assert_eq!(attrs.upstream.as_deref(), Some("aac"));
        assert_eq!(attrs.source_max_kbps, Some(128));
        assert!(attrs.weak);
        assert_eq!(attrs.chain.as_deref(), Some("C1"));
        assert_eq!(
            ok(ListKind::Trusted, "tienda", false, &["weak"]).unwrap_err(),
            "upstream=, source_max_kbps= and weak apply to fake lines only"
        );
        assert_eq!(
            ok(ListKind::Confirmed, "fake", false, &["source_max_kbps=7"]).unwrap_err(),
            "source_max_kbps must be an integer from 8 to 640"
        );
        assert!(valid_id("C1") && !valid_id("") && !valid_id("a b") && !valid_id(&"x".repeat(17)));
    }
}
