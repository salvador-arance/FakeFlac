//! Checks of the three lossy fixtures added for RF-16 (`specs/detection/lossy-fixtures.md` section 2:
//! `mp3_320_lp20k`, `m4a_aac_256_48k_lp20k`, `mp3_320_48k_short`) and of the extra condition of
//! section 3, check 2, for the two 48 kHz fixtures with a 20000 Hz edge ("Window margin"): their `fc`
//! is one of 19850, 19950, 20050 or 20150 Hz, so the verdicts of
//! `specs/features/detection-streaming-source.feature` (@RF-16) rest on the grid of `fc`, not on the
//! ± 200 Hz tolerance. Test support for @RF-16: these checks need no production change and pass; if
//! one fails, the spec returns to G1 (lossy-fixtures.md section 3).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::lossy::{self, aac_coded_edge_hz, mp3_coded_edge_hz};
use common::{lossy_bytes, lossy_fixture};
use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_core::detect::{measure, Measurements};
use fakeflac_core::ports::SpectrumAnalyzer;
use fakeflac_core::DetectionParams;
use std::collections::HashMap;
use std::sync::OnceLock;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

/// Decodes a file with symphonia, independently of the production decoder: rate and mono samples.
fn decode(path: &std::path::Path) -> Result<(u32, Vec<f32>), String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    hint.with_extension(path.extension().unwrap().to_str().unwrap());
    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| format!("probe: {e}"))?;
    let track = format.default_track(TrackType::Audio).ok_or("no track")?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|c| c.audio())
        .ok_or("no audio params")?
        .clone();
    let rate = params.sample_rate.ok_or("no rate")?;
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .map_err(|e| format!("decoder: {e}"))?;
    let mut out = Vec::new();
    loop {
        let packet = match format.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            Err(e) => return Err(format!("packet: {e}")),
        };
        if packet.track_id != track_id {
            continue;
        }
        let buf = decoder
            .decode(&packet)
            .map_err(|e| format!("decode: {e}"))?;
        assert_eq!(buf.spec().channels().count(), 1, "the fixtures are mono");
        let mut tmp = vec![0f32; buf.samples_interleaved()];
        buf.copy_to_slice_interleaved(&mut tmp);
        out.extend_from_slice(&tmp);
    }
    Ok((rate, out))
}

fn rms_dbfs(x: &[f32]) -> f64 {
    let ms = x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len() as f64;
    10.0 * ms.max(1e-30).log10()
}

/// A fixture with a 20000 Hz edge: id, coded edge, sample rate, audio frames written and samples
/// per frame.
struct Edge {
    name: &'static str,
    coded_hz: f64,
    rate: u32,
    frames: usize,
    frame_len: usize,
}

const NOMINAL_EDGE_HZ: f64 = 20_000.0;

/// The fixtures with a 20000 Hz edge (lossy-fixtures.md section 2, "Window margin"), the existing
/// `mp3_320_48k_lp20k` included.
fn edge_fixtures() -> Vec<Edge> {
    let mp3 = |name, rate: u32| Edge {
        name,
        coded_hz: mp3_coded_edge_hz(NOMINAL_EDGE_HZ, rate),
        rate,
        frames: lossy::mp3_frames(30.0, rate),
        frame_len: 1152,
    };
    vec![
        mp3("mp3_320_48k_lp20k", 48_000),
        mp3("mp3_320_lp20k", 44_100),
        Edge {
            name: "m4a_aac_256_48k_lp20k",
            coded_hz: aac_coded_edge_hz(NOMINAL_EDGE_HZ, 48_000),
            rate: 48_000,
            frames: lossy::aac_frames(30.0, 48_000),
            frame_len: 1024,
        },
    ]
}

struct Measured {
    decoded_rate: u32,
    decoded_samples: usize,
    rms_dbfs: f64,
    min_band_db: f64,
    m: Measurements,
    /// The reported cutoff of rules.md section 2: `fs` when `D >= step_suspicious_db`, else `fe`.
    fc: f64,
}

/// Decoded and measured once per fixture, with the default parameters (lossy-fixtures.md
/// section 3; the measurements do not read `[lossy]`).
fn measured(name: &str) -> &'static Measured {
    static CACHE: OnceLock<std::sync::Mutex<HashMap<String, &'static Measured>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(found) = cache.lock().unwrap().get(name) {
        return found;
    }
    let params = DetectionParams::default();
    let path = lossy_fixture(name);
    let (decoded_rate, samples) = decode(&path).unwrap_or_else(|e| panic!("{name}: {e}"));
    let probe = SymphoniaAnalyzer
        .analyze(&path, &params)
        .unwrap_or_else(|e| panic!("{name}: {e:?}"));
    let spectra = probe.spectrum.expect("spectra");
    let m = measure(&spectra, &params);
    let fc = match m.step_hz {
        Some(fs) if m.step_db >= params.verdict.step_suspicious_db => fs,
        _ => m.extent_hz,
    };
    let result: &'static Measured = Box::leak(Box::new(Measured {
        decoded_rate,
        decoded_samples: samples.len(),
        rms_dbfs: rms_dbfs(&samples),
        min_band_db: spectra
            .mean_db
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min),
        m,
        fc,
    }));
    cache.lock().unwrap().insert(name.to_string(), result);
    result
}

// ---- Writers ------------------------------------------------------------------------------------

#[test]
fn rf16_the_new_fixtures_are_deterministic_and_listed() {
    for name in [
        "mp3_320_lp20k",
        "m4a_aac_256_48k_lp20k",
        "mp3_320_48k_short",
    ] {
        assert!(lossy::BYTE_FIXTURES.contains(&name), "{name}");
        assert_eq!(
            lossy::fixture_bytes(name),
            lossy::fixture_bytes(name),
            "{name}"
        );
    }
}

#[test]
fn rf16_the_coded_edges_are_those_of_section_2() {
    // lossy-fixtures.md section 2: coded 20000 (48 kHz MP3), 20059 (44.1 kHz MP3), 19992 (48 kHz AAC).
    let coded: HashMap<_, _> = edge_fixtures()
        .into_iter()
        .map(|e| (e.name, e.coded_hz.round()))
        .collect();
    assert_eq!(coded["mp3_320_48k_lp20k"], 20_000.0);
    assert_eq!(coded["mp3_320_lp20k"], 20_059.0);
    assert_eq!(coded["m4a_aac_256_48k_lp20k"], 19_992.0);
}

#[test]
fn rf16_the_48k_aac_sample_sizes_add_up_to_exactly_256_kbps() {
    // lossy-fixtures.md 1.2 at 48 kHz: 1406 frames, B = 682.67 bytes, sum of sample sizes
    // floor(1406 · B) = 959829, declared bitrate exactly 256 kbps; the largest frame has 683 bytes.
    let bytes = lossy_bytes("m4a_aac_256_48k_lp20k");
    let (ftyp, moov) = lossy::mp4_header_len(&bytes);
    let mdat = bytes.len() - ftyp - moov - 8;
    assert_eq!(lossy::aac_frames(30.0, 48_000), 1406);
    assert_eq!(mdat, 959_829);
    let seconds = 1406.0 * 1024.0 / 48_000.0;
    let declared = 8.0 * mdat as f64 / seconds / 1000.0;
    assert_eq!(declared.round() as u64, 256, "{declared}");
}

#[test]
fn rf16_the_short_48k_fixture_is_a_2_s_cut_of_the_48k_rip() {
    // 83 audio frames at 48 kHz (1.992 s), tag frame on top, same bitrate and tag as the 30 s file.
    assert_eq!(lossy::mp3_frames(2.0, 48_000), 83);
    let short = lossy_bytes("mp3_320_48k_short");
    let full = lossy_bytes("mp3_320_48k_lp20k");
    // Same header bytes (MPEG-1 Layer III, 320 kbps, 48 kHz) as the long file.
    assert_eq!(short[..3], full[..3]);
    assert!(short.len() < full.len() / 10);
}

// ---- Section 3, check 1: decoding ------------------------------------------------------------------

#[test]
fn rf16_every_new_fixture_decodes_with_the_writers_rate_channels_and_duration() {
    for e in edge_fixtures() {
        let m = measured(e.name);
        assert_eq!(m.decoded_rate, e.rate, "{}", e.name);
        let written = e.frames * e.frame_len;
        assert!(
            m.decoded_samples.abs_diff(written) <= e.frame_len,
            "{}: decoded {} samples, written {written} (± one frame of {})",
            e.name,
            m.decoded_samples,
            e.frame_len
        );
    }
    let (rate, short) = decode(&lossy_fixture("mp3_320_48k_short")).unwrap();
    assert_eq!(rate, 48_000);
    assert!(
        short.len().abs_diff(83 * 1152) <= 1152,
        "{} samples",
        short.len()
    );
}

// ---- Section 3, check 2: cutoff, step and the window margin -----------------------------------------

#[test]
fn rf16_measured_cutoff_is_within_200_hz_of_the_edge() {
    for e in edge_fixtures() {
        let m = measured(e.name);
        assert!(
            (m.fc - NOMINAL_EDGE_HZ).abs() <= 200.0 && (m.fc - e.coded_hz).abs() <= 200.0,
            "{}: fc {} Hz, nominal edge {NOMINAL_EDGE_HZ} Hz (coded {} Hz)",
            e.name,
            m.fc,
            e.coded_hz
        );
    }
}

#[test]
fn rf16_step_is_at_least_fake_min_step() {
    // Every edge is between 11 kHz and Nyquist - 400 Hz (21650 Hz at 44.1 kHz).
    let params = DetectionParams::default();
    for e in edge_fixtures() {
        let d = measured(e.name).m.step_db;
        assert!(d >= params.verdict.step_fake_db, "{}: D {d} dB", e.name);
    }
}

#[test]
fn rf16_the_48k_fixtures_report_a_band_centre_inside_the_window() {
    // lossy-fixtures.md section 2, "Window margin", and section 3, check 2.
    for name in ["mp3_320_48k_lp20k", "m4a_aac_256_48k_lp20k"] {
        let fc = measured(name).fc;
        assert!(
            [19_850.0, 19_950.0, 20_050.0, 20_150.0].contains(&fc),
            "{name}: fc {fc} Hz"
        );
    }
}

// ---- Section 3, check 3: band levels ---------------------------------------------------------------

#[test]
fn rf16_every_band_level_is_finite_and_above_the_power_floor() {
    for e in edge_fixtures() {
        let min = measured(e.name).min_band_db;
        assert!(
            min.is_finite() && min > -299.0,
            "{}: min M(b) {min}",
            e.name
        );
    }
}

// ---- Section 3, check 4: treble, and the level -----------------------------------------------------

#[test]
fn rf16_treble_is_above_the_threshold() {
    let params = DetectionParams::default();
    for e in edge_fixtures() {
        let h = measured(e.name).m.high_frequency_db;
        assert!(h >= params.verdict.low_hf_threshold_db, "{}: H {h}", e.name);
    }
}

#[test]
fn rf16_levels_are_between_minus_30_and_minus_12_dbfs() {
    for e in edge_fixtures() {
        let level = measured(e.name).rms_dbfs;
        assert!((-30.0..=-12.0).contains(&level), "{}: {level} dBFS", e.name);
    }
}
