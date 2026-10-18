//! Tests derived from `specs/features/calibration.feature` (@RP-11): environment, containment, and
//! the scenarios that run the real binary (exit codes 2 and 3). Protected after the `test(RP-11)`
//! commit.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_calibrate::{run, AcceptArgs, Command, Env};
use std::path::{Path, PathBuf};
use std::process::Output;

/// Runs the binary `accept` from the repository root with only the given variables set.
fn binary(
    corpus: Option<&Path>,
    calibration: Option<&Path>,
    ffmpeg: Option<&Path>,
    args: &[&str],
) -> Output {
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_fakeflac-calibrate"));
    cmd.arg("accept")
        .args(args)
        .current_dir(repo_root())
        .stdin(std::process::Stdio::null())
        .env_remove("FAKEFLAC_CORPUS")
        .env_remove("FAKEFLAC_CALIBRATION")
        .env_remove("FAKEFLAC_FFMPEG");
    if let Some(p) = corpus {
        cmd.env("FAKEFLAC_CORPUS", p);
    }
    if let Some(p) = calibration {
        cmd.env("FAKEFLAC_CALIBRATION", p);
    }
    if let Some(p) = ffmpeg {
        cmd.env("FAKEFLAC_FFMPEG", p);
    }
    cmd.output().unwrap()
}

fn missing_message(var: &str) -> String {
    format!("error: {var} is not set (define it in .claude/settings.local.json, key \"env\")")
}

/// Scenario Outline: Missing environment variables (@RP-11)
fn missing_variables(test: &str, corpus_set: bool, calibration_set: bool) {
    let base = scratch_dir(test);
    let corpus = base.join("corpus");
    let calibration = base.join("calibration");
    std::fs::create_dir_all(&corpus).unwrap();
    std::fs::create_dir_all(&calibration).unwrap();
    let status_before = git_status();
    let out = binary(
        corpus_set.then_some(corpus.as_path()),
        calibration_set.then_some(calibration.as_path()),
        None,
        &[],
    );
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(2), "stderr: {stderr}");
    for (var, set) in [
        ("FAKEFLAC_CORPUS", corpus_set),
        ("FAKEFLAC_CALIBRATION", calibration_set),
    ] {
        if set {
            assert!(
                !stderr.contains(&format!("{var} is not set")),
                "{var} is set but named: {stderr}"
            );
        } else {
            assert!(stderr.contains(&missing_message(var)), "{stderr}");
        }
    }
    // Nothing is written anywhere.
    assert!(names(&corpus).is_empty(), "corpus: {:?}", names(&corpus));
    assert!(
        names(&calibration).is_empty(),
        "calibration: {:?}",
        names(&calibration)
    );
    assert_eq!(git_status(), status_before, "repository untouched");
    let _ = std::fs::remove_dir_all(&base);
}

// Scenario Outline: Missing environment variables (@RP-11), row "unset / unset"
#[test]
fn rp11_missing_environment_both_unset() {
    missing_variables("env-both-unset", false, false);
}

// Scenario Outline: Missing environment variables (@RP-11), row "unset / set"
#[test]
fn rp11_missing_environment_corpus_unset() {
    missing_variables("env-corpus-unset", false, true);
}

// Scenario Outline: Missing environment variables (@RP-11), row "set / unset"
#[test]
fn rp11_missing_environment_calibration_unset() {
    missing_variables("env-calibration-unset", true, false);
}

/// Scenario Outline: Invalid location of the working folder (@RP-11). `accept` through the library with
/// the repository root known, as the binary would find it.
fn invalid_location(env: Env) -> fakeflac_calibrate::Outcome {
    let args = AcceptArgs {
        params: vec![v2_params()],
        config: spec_config(),
        no_generate: false,
    };
    run(&Command::Accept(args), &env, &FixtureTranscoder)
}

// Scenario Outline: Invalid location of the working folder (@RP-11), row "a relative path"
#[test]
fn rp11_invalid_location_relative_path() {
    let base = scratch_dir("loc-relative");
    let corpus = base.join("corpus");
    std::fs::create_dir_all(&corpus).unwrap();
    let relative = PathBuf::from("fakeflac-calibrate-relative-working-folder");
    let o = invalid_location(Env {
        corpus: Some(corpus.clone()),
        calibration: Some(relative.clone()),
        ffmpeg: None,
        repo_root: Some(repo_root()),
    });
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert!(!relative.exists(), "nothing is created");
    assert!(names(&corpus).is_empty());
    let _ = std::fs::remove_dir_all(&base);
}

// Scenario Outline: Invalid location of the working folder (@RP-11), row "a folder inside the repository"
#[test]
fn rp11_invalid_location_inside_the_repository() {
    // CARGO_TARGET_TMPDIR is under the repository's target/ folder.
    let calibration = Path::new(env!("CARGO_TARGET_TMPDIR")).join("loc-inside-repo");
    let _ = std::fs::remove_dir_all(&calibration);
    std::fs::create_dir_all(&calibration).unwrap();
    let base = scratch_dir("loc-inside-repo-corpus");
    let corpus = base.join("corpus");
    std::fs::create_dir_all(&corpus).unwrap();
    let o = invalid_location(Env {
        corpus: Some(corpus.clone()),
        calibration: Some(calibration.clone()),
        ffmpeg: None,
        repo_root: Some(repo_root()),
    });
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert!(names(&calibration).is_empty(), "nothing is written");
    assert!(names(&corpus).is_empty());
    let _ = std::fs::remove_dir_all(&calibration);
    let _ = std::fs::remove_dir_all(&base);
}

// Scenario Outline: Invalid location of the working folder (@RP-11), row "a folder inside FAKEFLAC_CORPUS"
#[test]
fn rp11_invalid_location_inside_the_corpus() {
    let base = scratch_dir("loc-inside-corpus");
    let corpus = base.join("corpus");
    let calibration = corpus.join("work");
    std::fs::create_dir_all(&calibration).unwrap();
    let before = snapshot(&corpus);
    let o = invalid_location(Env {
        corpus: Some(corpus.clone()),
        calibration: Some(calibration),
        ffmpeg: None,
        repo_root: Some(repo_root()),
    });
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert_eq!(snapshot(&corpus), before, "nothing is written");
    let _ = std::fs::remove_dir_all(&base);
}

// Scenario Outline: Invalid location of the working folder (@RP-11), row "a folder that contains FAKEFLAC_CORPUS"
#[test]
fn rp11_invalid_location_contains_the_corpus() {
    let base = scratch_dir("loc-contains-corpus");
    let corpus = base.join("corpus");
    std::fs::create_dir_all(&corpus).unwrap();
    let before = snapshot(&base);
    let o = invalid_location(Env {
        corpus: Some(corpus),
        calibration: Some(base.clone()),
        ffmpeg: None,
        repo_root: Some(repo_root()),
    });
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert_eq!(snapshot(&base), before, "nothing is written");
    let _ = std::fs::remove_dir_all(&base);
}

// Scenario: ffmpeg missing when a transcode is needed (@RP-11)
#[test]
fn rp11_ffmpeg_missing_when_a_transcode_is_needed() {
    let sample = Sample::new("ffmpeg-missing");
    let missing = sample.scratch.join("no-such-ffmpeg.exe");
    // Default config: the 9 settings of calibration.v1.toml (encoder lame, aac, opus), empty features folder.
    let out = binary(
        Some(&sample.corpus),
        Some(&sample.calibration),
        Some(&missing),
        &[],
    );
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(3), "stderr: {stderr}");
    assert!(stderr.contains("ffmpeg not found"), "{stderr}");
    assert!(
        files_under(&sample.calibration.join("generated")).is_empty(),
        "nothing generated"
    );
}

// Scenario: The binary rejects the fixture transcoder (@RP-11)
#[test]
fn rp11_the_binary_rejects_the_fixture_transcoder() {
    let sample = Sample::new("binary-rejects-fixture");
    // The sample config has settings whose encoder is "fixture".
    let config = sample.config_path();
    let out = binary(
        Some(&sample.corpus),
        Some(&sample.calibration),
        None,
        &["--config", config.to_str().unwrap()],
    );
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(2), "stderr: {stderr}");
    assert!(
        files_under(&sample.calibration.join("generated")).is_empty(),
        "nothing generated"
    );
}
