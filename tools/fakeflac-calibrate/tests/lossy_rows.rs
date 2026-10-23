//! Tests of the pure parts of the lossy evaluation (@RP-11 @RF-15, `specs/calibration/lossy.md` sections
//! 5 and 6): the order of the guards of a lossy row against the one of `fakeflac_core::analysis::analyze_track`
//! (lossy-source.md section 4: L1a, L0, L1b), and the reading of the cache lines written before RF-15.
//! Synthetic fixtures, no ffmpeg, no real audio. Protected after the `test(RF-15)` commit.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_calibrate::cache::{
    canonical_json, error_code, fingerprint, Extracted, FeatureCache, ProbeData, StreamData,
};
use fakeflac_calibrate::lossy::evaluate_lossy_row;
use fakeflac_calibrate::model::{LabelRow, RowMeta, RowRoot, Truth};
use fakeflac_calibrate::split::Split;
use fakeflac_core::analysis::analyze_track;
use fakeflac_core::ports::{AudioProbe, FileEntry, SpectrumAnalyzer};
use fakeflac_core::{AudioCodec, AudioFormat, DetectionParams, TrackError, Verdict};
use std::path::{Path, PathBuf};
use std::sync::Arc;

fn v5() -> DetectionParams {
    toml::from_str(&read(&v5_params())).unwrap()
}

/// The analyzer port answering one fixed result, whatever the file.
struct Fixed(Result<AudioProbe, TrackError>);

impl SpectrumAnalyzer for Fixed {
    fn analyze(&self, _path: &Path, _params: &DetectionParams) -> Result<AudioProbe, TrackError> {
        self.0.clone()
    }
}

fn label_row(extracted: Extracted) -> LabelRow {
    LabelRow {
        root: RowRoot::Corpus,
        path: "a/b.mp3".to_string(),
        truth: Truth::Real,
        origin: "confirmed_real",
        trust_origin: String::new(),
        class: "reviewed".to_string(),
        encoder: String::new(),
        setting: String::new(),
        bitrate_kbps: None,
        encoder_version: String::new(),
        fragment: None,
        source_path: String::new(),
        album_id: "a".to_string(),
        split: Split::Tune,
        watch_id: String::new(),
        extracted: Arc::new(extracted),
        undetectable: false,
    }
}

/// What the tool and the core say about the same probe: (tool error code, tool verdict, core error code,
/// core verdict).
fn both(
    result: Result<AudioProbe, TrackError>,
    format: AudioFormat,
    params: &DetectionParams,
) -> (Option<&'static str>, Verdict, Option<&'static str>, Verdict) {
    let tool = evaluate_lossy_row(
        &label_row(Extracted::from_analysis(result.clone())),
        &RowMeta::default(),
        params,
    );
    assert!(tool.evaluated, "the row is evaluated");
    let entry = FileEntry {
        path: PathBuf::from("a/b.mp3"),
        folder: PathBuf::from("a"),
        format,
        size_bytes: 1,
        modified_unix: 0,
    };
    let core = analyze_track(&Fixed(result), &entry, params);
    (
        tool.error.as_ref().map(error_code),
        tool.verdict,
        core.error.as_ref().map(error_code),
        core.verdict,
    )
}

/// The probe of a lossy fixture with the codec, duration and declared bitrate replaced.
fn probe_of(
    fixture: &str,
    codec: AudioCodec,
    duration_ms: u64,
    declared: Option<u32>,
) -> AudioProbe {
    let params = v5();
    let mut p = SymphoniaAnalyzer
        .analyze(&lossy_fixture(fixture), &params)
        .unwrap();
    p.codec = Some(codec);
    p.duration_ms = duration_ms;
    p.declared_bitrate_kbps = declared;
    p
}

// Scenario: Params without a [lossy] section do not evaluate lossy rows (@RP-11 @RF-15), and lossy-source.md
// section 4: the order of the guards is L1a (decode failed), L0 (no rows for the codec in the params), L1b
// (too short, then no declared bitrate), the order of `analyze_track` of the core. The v5 params have no
// `[lossy.cutoffs]` rows for Opus, which is how L0 is reached with a `[lossy]` section.
#[test]
fn rf15_lossy_row_guards_follow_the_order_of_the_core() {
    let params = v5();
    let long = 30_000;
    let short = 2_000;
    use AudioCodec::{Mp3, Opus};
    let cases: Vec<(&str, AudioProbe, AudioFormat, Option<&str>)> = vec![
        // A long-enough lossy stream with no declared bitrate is a guard failure, not a Real verdict.
        (
            "long mp3 without a bitrate",
            probe_of("mp3_320_lp20k5", Mp3, long, None),
            AudioFormat::Mp3,
            Some("corruptStream"),
        ),
        // L0 comes before the too-short guard.
        (
            "short opus",
            probe_of("mp3_320_lp20k5", Opus, short, Some(128)),
            AudioFormat::Opus,
            Some("unsupportedFormat"),
        ),
        (
            "short opus without a bitrate",
            probe_of("mp3_320_lp20k5", Opus, short, None),
            AudioFormat::Opus,
            Some("unsupportedFormat"),
        ),
        (
            "long opus without a bitrate",
            probe_of("mp3_320_lp20k5", Opus, long, None),
            AudioFormat::Opus,
            Some("unsupportedFormat"),
        ),
        // L1b: too short first, then no bitrate.
        (
            "short mp3",
            probe_of("mp3_320_lp20k5", Mp3, short, Some(320)),
            AudioFormat::Mp3,
            Some("tooShort"),
        ),
        (
            "short mp3 without a bitrate",
            probe_of("mp3_320_lp20k5", Mp3, short, None),
            AudioFormat::Mp3,
            Some("tooShort"),
        ),
        // A zero-duration stream is too short whatever its bitrate (lossy-source.md section 2.3).
        (
            "empty mp3 without a bitrate",
            probe_of("mp3_320_lp20k5", Mp3, 0, None),
            AudioFormat::Mp3,
            Some("tooShort"),
        ),
        // Nothing wrong: the verdict of the lossy rules.
        (
            "good mp3",
            probe_of("mp3_320_lp20k5", Mp3, long, Some(320)),
            AudioFormat::Mp3,
            None,
        ),
        (
            "fake mp3",
            probe_of("mp3_320_lp17k", Mp3, long, Some(320)),
            AudioFormat::Mp3,
            None,
        ),
    ];
    let mut mismatches: Vec<String> = Vec::new();
    for (name, probe, format, expected) in cases {
        let (tool_error, tool_verdict, core_error, core_verdict) = both(Ok(probe), format, &params);
        assert_eq!(core_error, expected, "{name}: the core");
        if (tool_error, tool_verdict) != (core_error, core_verdict) {
            mismatches.push(format!(
                "{name}: tool {tool_error:?}/{tool_verdict:?}, core {core_error:?}/{core_verdict:?}"
            ));
        }
    }
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

// L1a: a decode failure is reported as it is, before L0, whatever the codec the extension implies.
#[test]
fn rf15_a_decode_failure_of_a_lossy_row_is_reported_as_it_is() {
    let params = v5();
    for (error, code) in [
        (TrackError::Unreadable("denied".into()), "unreadable"),
        (TrackError::CorruptStream("no sync".into()), "corruptStream"),
        (
            TrackError::UnsupportedFormat("HE-AAC".into()),
            "unsupportedFormat",
        ),
        (TrackError::TooShort, "tooShort"),
    ] {
        let (tool_error, tool_verdict, core_error, core_verdict) =
            both(Err(error), AudioFormat::Mp3, &params);
        assert_eq!(core_error, Some(code));
        assert_eq!(tool_error, core_error);
        assert_eq!(
            (tool_verdict, core_verdict),
            (Verdict::Error, Verdict::Error)
        );
    }
}

/// A cache folder with `lines` as the only entries of the fingerprint of v5; returns the open cache.
fn cache_with(name: &str, lines: &[String]) -> FeatureCache {
    let dir = scratch_dir(name);
    let fp = fingerprint(&v5());
    std::fs::write(
        dir.join(format!("{}.jsonl", fp.id)),
        lines.join("\n") + "\n",
    )
    .unwrap();
    FeatureCache::open(&dir, &fp).unwrap()
}

fn legacy_line(path: &str) -> (String, serde_json::Value) {
    let key = serde_json::json!({"kind": "corpus", "path": path, "size": 10, "mtime": 20});
    // Written before RF-15: no `stream` in the probe.
    let line = serde_json::json!({
        "key": key,
        "probe": {
            "sample_rate": 44100,
            "bit_depth": 16,
            "channels": 2,
            "duration_ms": 31000,
            "spectrum": null
        }
    });
    (line.to_string(), key)
}

// Scenario: A cold run decodes every lossy file once, except Opus (@RP-11 @RF-15), lossy.md section 5 (C7):
// a cache line without `stream` is a FLAC or WAV entry, its codec from the extension (any letter case)
// and no other stream value.
#[test]
fn rf15_a_cache_line_without_stream_is_read_as_a_flac_or_wav_entry() {
    let paths = [
        ("a/one.flac", AudioCodec::Flac),
        ("a/two.wav", AudioCodec::Pcm),
        ("a/THREE.WAV", AudioCodec::Pcm),
        ("a/Four.Wav", AudioCodec::Pcm),
        ("a/five.FLAC", AudioCodec::Flac),
        ("a/six.wav.flac", AudioCodec::Flac),
    ];
    let (lines, keys): (Vec<_>, Vec<_>) = paths.iter().map(|(p, _)| legacy_line(p)).unzip();
    let cache = cache_with("legacy-cache-lines", &lines);
    assert_eq!(cache.skipped_lines, 0);
    for ((path, codec), key) in paths.iter().zip(&keys) {
        let hit = cache
            .get(&canonical_json(key))
            .unwrap_or_else(|| panic!("no entry for {path}"));
        assert_eq!(
            *hit.extracted,
            Extracted::Probe(ProbeData {
                sample_rate: 44100,
                bit_depth: Some(16),
                channels: 2,
                duration_ms: 31000,
                spectrum: None,
                stream: StreamData {
                    codec: Some(*codec),
                    ..StreamData::default()
                },
            }),
            "{path}"
        );
    }
    let _ = std::fs::remove_dir_all(scratch_dir("legacy-cache-lines"));
}
