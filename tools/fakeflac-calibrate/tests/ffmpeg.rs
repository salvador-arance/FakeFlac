//! Tests derived from `specs/features/calibration.feature` (@RP-11): ffmpeg command lines
//! (`specs/calibration/generation.md` section 2), no ffmpeg needed. Protected after the
//! `test(RP-11)` commit.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_calibrate::ffmpeg::{decode_args, encode_args};
use fakeflac_calibrate::transcode::Fragment;
use std::path::PathBuf;

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

// Scenario: ffmpeg command lines (@RP-11)
#[test]
fn rp11_ffmpeg_command_lines() {
    let setting = spec_setting("cbr128");
    let config = fakeflac_calibrate::Config::parse(&read(&spec_config())).unwrap();
    let source = PathBuf::from(TRACK_1);
    let mp3 = PathBuf::from("generated/0123456789abcdef.mp3");
    let flac = PathBuf::from("generated/0123456789abcdef.flac");
    let fragment = Fragment {
        start_s: 90.5,
        duration_s: 60.0,
    };

    let encode = encode_args(&source, Some(fragment), &setting, &mp3);
    let mut expected = strings(&[
        "-nostdin",
        "-hide_banner",
        "-loglevel",
        "error",
        "-ss",
        "90.500",
        "-t",
        "60.000",
        "-i",
    ]);
    expected.push(TRACK_1.to_string());
    expected.extend(strings(&[
        "-map",
        "0:a:0",
        "-map_metadata",
        "-1",
        "-c:a",
        "libmp3lame",
        "-b:a",
        "128k",
        "-y",
    ]));
    expected.push(mp3.to_string_lossy().into_owned());
    assert_eq!(
        encode, expected,
        "each one a separate argument, source unchanged"
    );

    let decode = decode_args(&mp3, &config.generate.decode_args, &flac);
    let mut expected = strings(&["-nostdin", "-hide_banner", "-loglevel", "error", "-i"]);
    expected.push(mp3.to_string_lossy().into_owned());
    expected.extend(strings(&[
        "-map",
        "0:a:0",
        "-map_metadata",
        "-1",
        "-c:a",
        "flac",
        "-sample_fmt",
        "s16",
        "-y",
    ]));
    expected.push(flac.to_string_lossy().into_owned());
    assert_eq!(decode, expected);
}

// Scenario: ffmpeg encode command with full track default (@RP-11)
#[test]
fn rp11_ffmpeg_encode_command_with_full_track_default() {
    let setting = spec_setting("cbr128");
    let source = PathBuf::from(TRACK_1);
    let out = PathBuf::from("generated/0123456789abcdef.mp3");
    let encode = encode_args(&source, None, &setting, &out);
    assert!(!encode.iter().any(|a| a == "-ss"), "{encode:?}");
    assert!(!encode.iter().any(|a| a == "-t"), "{encode:?}");
    let i = encode.iter().position(|a| a == "-i").unwrap();
    assert_eq!(encode[i + 1], TRACK_1);
    assert_eq!(
        encode[..4],
        strings(&["-nostdin", "-hide_banner", "-loglevel", "error"])[..]
    );
}
