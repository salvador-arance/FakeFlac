//! `privacy-check` (calibration.md section 6, OQ-16): searches the files changed in the branch for the
//! two root paths and for the folder names and file stems of the labelled corpus tracks.

use crate::env::Env;
use crate::error::ToolError;
use crate::fsutil::plain_path;
use crate::lists;
use crate::run::{Outcome, PrivacyArgs};
use std::collections::BTreeSet;
use std::path::Path;

/// Names shorter than this are never searched (too many false alarms).
const MIN_NAME_CHARS: usize = 8;
/// Larger files are skipped: they are not text.
const MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;

fn exceptions(paths: &[&Path]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for dir in paths {
        for name in ["privacy-exceptions.txt", ".privacy-exceptions"] {
            if let Ok(text) = std::fs::read_to_string(dir.join(name)) {
                for line in text.lines() {
                    let t = line.trim();
                    if !t.is_empty() && !t.starts_with('#') {
                        out.insert(t.to_lowercase());
                    }
                }
            }
        }
    }
    out
}

/// Folder names and file stems of the labelled tracks with 8 or more characters.
fn corpus_names(corpus_tracks: &[String]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for rel in corpus_tracks {
        let parts: Vec<&str> = rel.split('/').collect();
        let Some((file, folders)) = parts.split_last() else {
            continue;
        };
        for folder in folders {
            names.insert((*folder).to_string());
        }
        let stem = Path::new(file)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        names.insert(stem);
    }
    names
        .into_iter()
        .filter(|n| n.chars().count() >= MIN_NAME_CHARS)
        .collect()
}

fn root_patterns(env: &Env) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for root in [&env.corpus, &env.calibration].into_iter().flatten() {
        let mut variants = vec![root.to_string_lossy().into_owned()];
        if let Ok(canonical) = std::fs::canonicalize(root) {
            variants.push(plain_path(&canonical).to_string_lossy().into_owned());
        }
        for v in variants {
            let v = v.trim_end_matches(['\\', '/']).to_string();
            if v.is_empty() {
                continue;
            }
            out.insert(v.replace('/', "\\").to_lowercase());
            out.insert(v.replace('\\', "/").to_lowercase());
            out.insert(v.replace('/', "\\").replace('\\', "\\\\").to_lowercase());
        }
    }
    out
}

pub fn check(args: &PrivacyArgs, env: &Env) -> Result<Outcome, ToolError> {
    let roots = env.check()?;
    let Some(repo) = &env.repo_root else {
        return Err(ToolError::input(
            "error: privacy-check needs a git checkout (git rev-parse --show-toplevel failed)",
        ));
    };
    if args.base.is_empty() || args.base.starts_with('-') {
        return Err(ToolError::input(
            "error: --base must be a git revision, not an option",
        ));
    }
    let range = format!("{}...HEAD", args.base);
    let output = std::process::Command::new("git")
        .args([
            "-c",
            "core.quotepath=off",
            "diff",
            "--name-only",
            "--diff-filter=d",
            &range,
        ])
        .current_dir(repo)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| ToolError::input(format!("error: cannot run git: {e}")))?;
    if !output.status.success() {
        let first = String::from_utf8_lossy(&output.stderr)
            .lines()
            .next()
            .unwrap_or("")
            .to_string();
        return Err(ToolError::input(format!(
            "error: git diff {range} failed: {first}"
        )));
    }
    let changed: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .filter(|l| !l.is_empty())
        .collect();

    let loaded = lists::load(&roots.calibration, &roots.corpus)?;
    let tracks: Vec<String> = loaded
        .trusted
        .iter()
        .map(|t| t.track.rel.clone())
        .chain(loaded.fakes.iter().map(|f| f.track.rel.clone()))
        .collect();
    let skip = exceptions(&[repo.as_path(), roots.calibration.as_path()]);
    let names: Vec<String> = corpus_names(&tracks)
        .into_iter()
        .map(|n| n.to_lowercase())
        .filter(|n| !skip.contains(n))
        .collect();
    let roots_found = root_patterns(env);

    let mut matches: Vec<String> = Vec::new();
    let mut unchecked: Vec<String> = Vec::new();
    for file in &changed {
        // The committed content of HEAD, not the working tree (what a PR would publish).
        let shown = std::process::Command::new("git")
            .args(["show", &format!("HEAD:{file}")])
            .current_dir(repo)
            .stdin(std::process::Stdio::null())
            .output();
        let bytes = match shown {
            Ok(o) if o.status.success() => o.stdout,
            _ => {
                unchecked.push(format!("{file}: not readable from HEAD"));
                continue;
            }
        };
        if bytes.len() as u64 > MAX_FILE_BYTES {
            unchecked.push(format!("{file}: larger than 20 MiB, not checked"));
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        for (i, line) in text.lines().enumerate() {
            let lower = line.to_lowercase();
            let kind = if roots_found.iter().any(|p| lower.contains(p.as_str())) {
                Some("root path")
            } else if names.iter().any(|n| lower.contains(n.as_str())) {
                Some("corpus name")
            } else {
                None
            };
            if let Some(kind) = kind {
                matches.push(format!("{file}:{}: {kind}", i + 1));
            }
        }
    }

    let mut stdout = String::new();
    for m in &matches {
        stdout.push_str(m);
        stdout.push('\n');
    }
    for u in &unchecked {
        stdout.push_str(&format!("unchecked: {u}\n"));
    }
    let exit_code = i32::from(!matches.is_empty());
    stdout.push_str(&format!(
        "privacy-check: {} files checked, {} matches\n",
        changed.len(),
        matches.len()
    ));
    Ok(Outcome {
        exit_code,
        stdout,
        ..Outcome::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_names_are_not_searched() {
        let names = corpus_names(&[
            "Short/Album Name Long/01 – Track.flac".to_string(),
            "Album Name Long/02 – Other song.flac".to_string(),
        ]);
        assert!(names.contains("Album Name Long"));
        assert!(names.contains("02 – Other song"));
        assert!(!names.contains("Short"));
    }
}
