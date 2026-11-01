//! @RF-14 @RF-15 @RF-07 @RNF-04 @RNF-05 @RNF-09: opus.feature.
//! Sample seam: decode_opus_samples_for_test returns the actual pipeline's
//! per-channel samples before mono mix, after pre-skip/end trim/output gain.
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
use common::opus;
use fakeflac_audio::{decode_opus_samples_for_test, SymphoniaAnalyzer};
use fakeflac_core::{
    analysis::analyze_track, ports::FileEntry, AudioCodec, AudioFormat, DetectionParams,
    SuspicionReason, TrackAnalysis, TrackError, Verdict,
};
use std::path::Path;
fn entry(path: &Path) -> FileEntry {
    let meta = std::fs::metadata(path).unwrap();
    FileEntry {
        path: path.into(),
        folder: path.parent().unwrap().into(),
        format: AudioFormat::Opus,
        size_bytes: meta.len(),
        modified_unix: meta
            .modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    }
}
fn v(version: u32) -> DetectionParams {
    toml::from_str(
        &std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../specs/detection/detection-params.v{version}.toml"
        )))
        .unwrap(),
    )
    .unwrap()
}
fn analyze(name: &str) -> TrackAnalysis {
    let path = opus::fixture(name);
    analyze_track(&SymphoniaAnalyzer, &entry(&path), &v(8))
}
fn compare(mut a: TrackAnalysis, b: TrackAnalysis) {
    assert_eq!(a.error, None);
    assert_eq!(b.error, None);
    a.info.path = b.info.path.clone();
    a.info.folder = b.info.folder.clone();
    a.info.size_bytes = b.info.size_bytes;
    a.info.modified_unix = b.info.modified_unix;
    assert_eq!(a, b);
}
fn assert_truth(name: &str, a: &TrackAnalysis) {
    let (verdict, reasons, estimate, edge) = match name {
        "opus_160_src16k" => (Verdict::Fake, vec![], None, 16000.0),
        "opus_160_swb" => (
            Verdict::Suspicious,
            vec![SuspicionReason::LowHighFrequencyContent],
            None,
            12000.0,
        ),
        "opus_short" => {
            assert_eq!(a.error, Some(TrackError::TooShort));
            assert_eq!(a.verdict, Verdict::Error);
            return;
        }
        _ => (
            Verdict::Real,
            vec![],
            Some(if name == "opus_64_fb" { 64 } else { 128 }),
            20000.0,
        ),
    };
    assert_eq!(a.verdict, verdict, "{name}: {a:?}");
    assert_eq!(a.reasons, reasons, "{name}");
    assert_eq!(a.estimated_source_bitrate_kbps, estimate, "{name}");
    assert_eq!(a.error, None);
    assert!(
        (f64::from(a.cutoff.as_ref().unwrap().frequency_hz) - edge).abs() <= 200.0,
        "{name}"
    );
    assert_eq!(a.algorithm_version, 8);
}
/// Scenario Outline: Stream information of Opus fixtures; Verdict of Opus fixtures.
#[test]
fn rf14_rf15_opus_stream_information_and_verdicts() {
    for name in [
        "opus_128_fb",
        "opus_128_fb_stereo",
        "opus_64_fb",
        "opus_160_src16k",
        "opus_128_fb_preskip",
        "opus_160_swb",
    ] {
        let a = analyze(name);
        let e = opus::encoded(if name == "opus_128_fb_preskip" {
            "opus_128_fb"
        } else {
            name
        });
        assert_eq!(a.info.format, AudioFormat::Opus);
        assert_eq!(a.info.codec, Some(AudioCodec::Opus));
        assert_eq!(a.info.sample_rate, 48000);
        assert_eq!(a.info.channels, e.channels as u8);
        assert_eq!(a.info.bit_depth, None);
        assert_eq!(a.info.bitrate_mode, None);
        let n = if name == "opus_128_fb_preskip" {
            e.samples - 960
        } else {
            e.samples
        };
        assert_eq!(a.info.duration_ms, n as u64 * 1000 / 48000);
        let total: usize = e.packets.iter().map(Vec::len).sum();
        let expected = (8.0 * total as f64 / (n as f64 / 48000.0) / 1000.0).round() as u32;
        assert_eq!(a.info.declared_bitrate_kbps, Some(expected));
        assert!(expected.abs_diff(e.kbps) <= 1);
        assert_truth(name, &a);
    }
}
/// Scenario: The input sample rate of OpusHead is informational only.
#[test]
fn rf14_opus_header_rate_is_informational() {
    compare(analyze("opus_128_fb"), analyze("opus_128_fb_rate44k"));
}
/// Scenario: Pre-skip and end trim are applied; independent raw decoder oracle.
#[test]
fn rf14_opus_preskip_and_end_trim() {
    for name in ["opus_128_fb", "opus_128_fb_stereo"] {
        let decoded = decode_opus_samples_for_test(&opus::fixture(name)).unwrap();
        let expected = opus::reference(&opus::encoded(name), 120);
        assert_eq!(decoded, expected);
        assert!(decoded.iter().all(|s| s.len() == 1440000));
    }
    let a = decode_opus_samples_for_test(&opus::fixture("opus_128_fb")).unwrap();
    let b = decode_opus_samples_for_test(&opus::fixture("opus_128_fb_preskip")).unwrap();
    assert_eq!(b[0], a[0][960..]);
    assert_eq!(b[0].len(), 1439040);
}
/// Scenario: The output gain of OpusHead is applied to every sample.
#[test]
fn rf14_opus_output_gain() {
    let a = decode_opus_samples_for_test(&opus::fixture("opus_128_fb")).unwrap();
    let b = decode_opus_samples_for_test(&opus::fixture("opus_128_fb_gain_m20")).unwrap();
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(&b) {
        assert_eq!(a.len(), b.len());
        assert!(a.iter().zip(b).all(|(a, b)| (0.1 * a - b).abs() <= 1e-6));
    }
    let a = analyze("opus_128_fb");
    let b = analyze("opus_128_fb_gain_m20");
    assert_eq!(a.verdict, b.verdict);
    assert_eq!(a.reasons, b.reasons);
    assert_eq!(
        a.cutoff.as_ref().unwrap().frequency_hz,
        b.cutoff.as_ref().unwrap().frequency_hz
    );
}
/// Scenario: An output gain that makes the track silent.
#[test]
fn rf14_opus_silent_gain() {
    let a = analyze("opus_128_fb_gain_m60");
    assert_eq!(a.verdict, Verdict::Suspicious);
    assert_eq!(a.reasons, [SuspicionReason::LowHighFrequencyContent]);
    assert_eq!(a.cutoff, None);
    assert_eq!(a.estimated_source_bitrate_kbps, None);
}
/// Scenario: A packet split across pages is reassembled.
#[test]
fn rf14_opus_split_packets() {
    compare(analyze("opus_128_fb"), analyze("opus_128_fb_split"));
    assert_eq!(
        decode_opus_samples_for_test(&opus::fixture("opus_128_fb")).unwrap(),
        decode_opus_samples_for_test(&opus::fixture("opus_128_fb_split")).unwrap()
    );
}
/// Scenario: An Opus file analysed with DetectionParams v7 is an error.
#[test]
fn rf15_opus_v7_has_no_table() {
    let path = opus::fixture("opus_128_fb");
    let a = analyze_track(&SymphoniaAnalyzer, &entry(&path), &v(7));
    assert_eq!(a.verdict, Verdict::Error);
    assert_eq!(
        a.error,
        Some(TrackError::UnsupportedFormat(
            "no [lossy.cutoffs] rows for opus".into()
        ))
    );
    assert_eq!(a.algorithm_version, 7);
}
/// Scenario Outline: Broken Ogg or Opus structure is corruptStream.
#[test]
fn rf14_broken_opus_streams() {
    for (name, detail) in [
        ("opus_not_ogg", "not an Ogg stream"),
        ("opus_bad_crc", "Ogg page CRC mismatch"),
        ("opus_page_version", "unsupported Ogg page version"),
        ("opus_truncated_page", "truncated Ogg page"),
        ("opus_sequence_gap", "Ogg page out of sequence"),
        ("opus_no_bos", "first Ogg page is not a beginning of stream"),
        ("opus_after_eos", "Ogg page after end of stream"),
        ("opus_continuation", "Ogg packet continuation mismatch"),
        (
            "opus_unfinished_packet",
            "unfinished Ogg packet at end of stream",
        ),
        ("opus_no_head", "no OpusHead"),
        ("opus_head_version", "invalid OpusHead"),
        ("opus_head_channels", "invalid OpusHead"),
        ("opus_head_only", "no OpusTags"),
        ("opus_no_tags", "no OpusTags"),
        ("opus_tags_overrun", "invalid OpusTags"),
        ("opus_tags_shared_page", "audio data on the OpusTags page"),
        ("opus_no_audio", "no Opus audio packets"),
        ("opus_granule_decreases", "Ogg granule position decreases"),
        (
            "opus_granule_below_preskip",
            "granule position below pre-skip",
        ),
        (
            "opus_granule_beyond_audio",
            "granule position beyond the audio",
        ),
        ("opus_bad_packet", "Opus packet 100"),
    ] {
        let a = analyze(name);
        assert_eq!(a.verdict, Verdict::Error, "{name}");
        assert!(
            matches!(&a.error,Some(TrackError::CorruptStream(text))if text.contains(detail)),
            "{name}: {:?}",
            a.error
        );
        assert_eq!(a.info.format, AudioFormat::Opus);
        assert_eq!(a.info.codec, Some(AudioCodec::Opus));
        assert_eq!(a.cutoff, None);
        assert_eq!(a.estimated_source_bitrate_kbps, None);
    }
}
/// Scenario Outline: Recognised but unsupported Opus streams are unsupportedFormat.
#[test]
fn rf14_unsupported_opus_streams() {
    for (name, detail) in [
        (
            "opus_family1_6ch",
            "channel mapping family 1 is not supported",
        ),
        (
            "opus_family255_3ch",
            "channel mapping family 255 is not supported",
        ),
        (
            "opus_serial_change",
            "multiplexed or chained Ogg streams are not supported",
        ),
    ] {
        let a = analyze(name);
        assert_eq!(a.verdict, Verdict::Error);
        assert_eq!(a.error, Some(TrackError::UnsupportedFormat(detail.into())));
        assert_eq!(a.cutoff, None);
        assert_eq!(a.estimated_source_bitrate_kbps, None);
    }
}
/// Scenario: A short Opus file is too short.
#[test]
fn rf14_short_opus() {
    assert_truth("opus_short", &analyze("opus_short"));
}
/// Scenario: The defaults are the v8 file; Invalid Opus rows are rejected.
#[test]
fn rf15_opus_params() {
    let p = v(8);
    assert_eq!(p, DetectionParams::default());
    p.validate().unwrap();
    let mut without = p.clone();
    without.algorithm_version = 7;
    without.lossy.as_mut().unwrap().cutoffs.opus = None;
    assert_eq!(without, v(7));
    for (kind, detail) in [
        (0, "lossy.cutoffs.opus: bitrate_kbps increases"),
        (1, "lossy.cutoffs.opus: cutoff_hz does not decrease"),
        (2, "lossy.cutoffs.opus: cutoff_hz > 0"),
    ] {
        let mut invalid = p.clone();
        let rows = invalid
            .lossy
            .as_mut()
            .unwrap()
            .cutoffs
            .opus
            .as_mut()
            .unwrap();
        match kind {
            0 => rows.swap(0, 1),
            1 => rows[5].cutoff_hz = 19000.0,
            _ => rows[0].cutoff_hz = 0.0,
        }
        assert!(invalid.validate().unwrap_err().to_string().contains(detail));
    }
}
/// Scenario: The spectrogram of an Opus file.
#[test]
fn rf07_opus_spectrogram() {
    let s = SymphoniaAnalyzer
        .spectrogram(&opus::fixture("opus_128_fb"), &v(8).spectrogram)
        .unwrap();
    assert_eq!(s.height, 512);
    assert!(s.width <= 1200);
    assert_eq!(s.max_frequency_hz, 24000.0);
    for row in 0..s.height {
        let hz = f64::from(row) * s.max_frequency_hz / f64::from(s.height);
        if hz > 21500.0 {
            let mean = (0..s.width as usize)
                .map(|time| f64::from(s.data[time * s.height as usize + row as usize]))
                .sum::<f64>()
                / f64::from(s.width);
            assert!(mean <= 63.0, "row {row}, {hz} Hz: {mean}");
        }
    }
}
/// Scenario: Analysed Opus files are never modified; Unicode names of Opus files.
#[test]
fn rnf05_rnf09_opus_unicode_readonly() {
    let dir = opus::fixture("opus_128_fb")
        .parent()
        .unwrap()
        .join("Álbum ″Ñandú″");
    std::fs::create_dir_all(&dir).unwrap();
    for (fixture, name, verdict) in [
        ("opus_128_fb", "01 – Canción ‘x’.opus", Verdict::Real),
        ("opus_bad_crc", "02 – Ópera «Ça va» 🎵.opus", Verdict::Error),
    ] {
        let path = dir.join(name);
        let original = opus::bytes(fixture);
        std::fs::write(&path, &original).unwrap();
        let before = std::fs::metadata(&path).unwrap();
        let a = analyze_track(&SymphoniaAnalyzer, &entry(&path), &v(8));
        assert_eq!(a.info.path, path);
        assert_eq!(a.info.path.file_name().unwrap().to_str().unwrap(), name);
        assert_eq!(a.verdict, verdict);
        if verdict == Verdict::Error {
            assert!(matches!(a.error, Some(TrackError::CorruptStream(_))));
        }
        let after = std::fs::metadata(&path).unwrap();
        assert_eq!(before.len(), after.len());
        assert_eq!(before.modified().unwrap(), after.modified().unwrap());
        assert_eq!(std::fs::read(path).unwrap(), original);
    }
}

/// Child entry point for opus.feature determinism and scalar-path scenarios.
#[test]
fn opus_fixture_child() {
    let Some(dir) = std::env::var_os("FAKEFLAC_OPUS_FIXTURE_CHILD") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    for name in opus::ENCODED {
        std::fs::write(dir.join(format!("{name}.opus")), opus::bytes(name)).unwrap();
    }
}
fn child(label: &str, scalar: bool) -> std::path::PathBuf {
    let dir = opus::fixture("opus_128_fb").parent().unwrap().join(label);
    std::fs::create_dir_all(&dir).unwrap();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "opus_fixture_child", "--nocapture"])
        .env("FAKEFLAC_OPUS_FIXTURE_CHILD", &dir);
    if scalar {
        command.env("RUSTY_OPUS_ISA", "scalar");
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    dir
}
/// @RF-14 Scenario: The fixture encoder is deterministic on one machine.
#[test]
fn rf14_opus_encoder_determinism() {
    let dir = child("same-isa", false);
    for name in opus::ENCODED {
        let e = opus::encode(name);
        let first: Vec<u8> = opus::pages(&e, false)
            .iter()
            .flat_map(opus::serialize)
            .collect();
        let second = opus::bytes(name);
        assert_eq!(first, second, "{name}: within process");
        assert_eq!(
            second,
            std::fs::read(dir.join(format!("{name}.opus"))).unwrap(),
            "{name}: child process"
        );
    }
}
/// @RF-14 Scenario: Opus fixtures encoded on the scalar path keep their truth.
#[test]
fn rf14_opus_scalar_truth() {
    let dir = child("scalar", true);
    for name in opus::ENCODED {
        let path = dir.join(format!("{name}.opus"));
        let a = analyze_track(&SymphoniaAnalyzer, &entry(&path), &v(8));
        assert_truth(name, &a);
        let n = if name == "opus_short" { 96000 } else { 1440000 };
        assert_eq!(a.info.duration_ms, n * 1000 / 48000);
        let decoded = decode_opus_samples_for_test(&path).unwrap();
        assert!(decoded.iter().all(|s| s.len() as u64 == n));
        let file = std::fs::read(path).unwrap();
        let mut pos = 0;
        let mut packets = 0;
        while pos < file.len() {
            let segments = file[pos + 26] as usize;
            let lace = &file[pos + 27..pos + 27 + segments];
            packets += lace.iter().filter(|x| **x < 255).count();
            pos += 27 + segments + lace.iter().map(|x| *x as usize).sum::<usize>();
        }
        assert_eq!(packets - 2, if name == "opus_short" { 101 } else { 1501 });
        if name != "opus_short" {
            assert!(
                a.info
                    .declared_bitrate_kbps
                    .unwrap()
                    .abs_diff(opus::encoded(name).kbps)
                    <= 1
            );
        }
    }
}
/// @RF-14 Scenario: The Ogg writer of the tests is not the adapter's reader.
#[test]
fn rf14_ogg_writer_is_independent() {
    let source = include_str!("common/opus.rs");
    assert!(!source.contains("fakeflac_audio::"));
    assert!(source.contains("0x04c11db7"));
}
/// @RF-15 @RF-04 Scenario: v8 gives the v7 results on every file that is not Opus.
#[test]
fn rf15_v8_preserves_all_non_opus_results() {
    let mut paths = Vec::new();
    for name in [
        "full_band",
        "lp16k",
        "lp19k",
        "lp20k",
        "lp16k_96k",
        "low_hf",
        "gradual",
        "bass_tone_lp16k",
        "silence",
        "short",
        "corrupt",
        "lp16k_unknown_length",
        "lp16k_dither",
        "lp20k_floor",
        "sfb21_gaps",
        "sfb21_gaps_weak",
        "hf_bursts",
        "sfb21_gaps_lp20k5",
        "sfb21_gaps_aa21k",
        "shelf19k5",
        "shelf19k5_bursts",
        "edge_bump19k3",
        "edge_bump19k3_lp19k5",
        "holes_lp19k5",
    ] {
        paths.push(common::fixture(name, common::Container::Flac));
    }
    for name in [
        "mp3_320_lp20k5",
        "mp3_320_lp17k",
        "mp3_128_lp17k",
        "mp3_192_lp17k_notag",
        "mp3_320_lp18k5",
        "mp3_320_tag16k",
        "mp3_vbr_lp19k5",
        "mp3_abr_lp18k6",
        "mp3_320_48k_lp20k",
        "mp3_320_lp20k",
        "mp3_320_48k_short",
        "mp3_320_low_hf",
        "mp3_silence",
        "mp3_short",
        "mp3_corrupt",
        "m4a_aac_128_lp16k",
        "m4a_aac_256_lp15k3",
        "m4a_aac_256_48k_lp20k",
        "m4a_alac_full_band",
        "m4a_alac_lp16k",
        "m4a_he_aac",
        "m4a_corrupt",
    ] {
        paths.push(common::lossy_fixture(name));
    }
    for path in paths {
        let entry = common::entry(&path);
        let old = analyze_track(&SymphoniaAnalyzer, &entry, &v(7));
        let new = analyze_track(&SymphoniaAnalyzer, &entry, &v(8));
        assert_eq!(old.verdict, new.verdict, "{}", path.display());
        assert_eq!(old.reasons, new.reasons);
        assert_eq!(old.cutoff, new.cutoff);
        assert_eq!(
            old.estimated_source_bitrate_kbps,
            new.estimated_source_bitrate_kbps
        );
        assert_eq!(old.error, new.error);
        assert_eq!(old.algorithm_version, 7);
        assert_eq!(new.algorithm_version, 8);
    }
}
