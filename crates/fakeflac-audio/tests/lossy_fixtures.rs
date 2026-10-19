//! Checks of the synthetic lossy fixtures themselves (`specs/detection/lossy-fixtures.md`, section 3,
//! ADR-0040): the bitstream writers of `tests/common/lossy.rs` produce files that symphonia decodes,
//! whose measured cutoff, step, treble and band levels are those the specs derive their verdicts from.
//! Test support for @RF-14 and @RF-15: these tests need no production change and pass.
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

/// A fixture with an edge: its id, nominal edge, coded edge, sample rate, audio frames written
/// and samples per frame.
struct Edge {
    name: &'static str,
    nominal_hz: f64,
    coded_hz: f64,
    rate: u32,
    frames: usize,
    frame_len: usize,
}

fn fixtures_with_an_edge() -> Vec<Edge> {
    let mp3 = |name, nominal_hz, rate: u32| Edge {
        name,
        nominal_hz,
        coded_hz: mp3_coded_edge_hz(nominal_hz, rate),
        rate,
        frames: lossy::mp3_frames(30.0, rate),
        frame_len: 1152,
    };
    let aac = |name, nominal_hz| Edge {
        name,
        nominal_hz,
        coded_hz: aac_coded_edge_hz(nominal_hz, 44_100),
        rate: 44_100,
        frames: lossy::aac_frames(30.0, 44_100),
        frame_len: 1024,
    };
    vec![
        mp3("mp3_320_lp20k5", 20_500.0, 44_100),
        mp3("mp3_320_lp17k", 17_000.0, 44_100),
        mp3("mp3_128_lp17k", 17_000.0, 44_100),
        mp3("mp3_192_lp17k_notag", 17_000.0, 44_100),
        mp3("mp3_320_lp18k5", 18_500.0, 44_100),
        mp3("mp3_320_tag16k", 16_000.0, 44_100),
        mp3("mp3_vbr_lp19k5", 19_500.0, 44_100),
        mp3("mp3_abr_lp18k6", 18_600.0, 44_100),
        mp3("mp3_320_48k_lp20k", 20_000.0, 48_000),
        mp3("mp3_320_low_hf", 6_000.0, 44_100),
        aac("m4a_aac_128_lp16k", 16_000.0),
        aac("m4a_aac_256_lp15k3", 15_300.0),
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

/// Decoded and measured once per fixture (the analysis of a 30 s file takes seconds).
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
fn writers_are_deterministic() {
    for name in lossy::BYTE_FIXTURES {
        assert_eq!(
            lossy::fixture_bytes(name),
            lossy::fixture_bytes(name),
            "{name}"
        );
    }
}

/// (offset, kbps, size) of every MPEG frame of an MP3 stream written without ID3 tags.
fn mp3_frames_of(bytes: &[u8]) -> Vec<(usize, u32, usize)> {
    const BITRATES: [u32; 14] = [
        32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
    ];
    let mut frames = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        assert_eq!(bytes[at], 0xFF, "frame sync at {at}");
        assert_eq!(bytes[at + 1] & 0xFE, 0xFA, "MPEG-1 Layer III at {at}");
        let kbps = BITRATES[usize::from(bytes[at + 2] >> 4) - 1];
        let rate = [44_100u32, 48_000][usize::from((bytes[at + 2] >> 2) & 3)];
        let pad = usize::from((bytes[at + 2] >> 1) & 1);
        let size = (144_000 * kbps / rate) as usize + pad;
        frames.push((at, kbps, size));
        at += size;
    }
    assert_eq!(at, bytes.len());
    frames
}

#[test]
fn mp3_tag_frame_holds_the_audio_frame_count_and_the_stream_size() {
    // lossy-fixtures.md 1.1: frames field = audio frames (tag frame excluded), bytes field = every
    // frame including the tag frame.
    let bytes = lossy_bytes("mp3_320_lp17k");
    let frames = mp3_frames_of(&bytes);
    assert_eq!(frames.len(), 1148 + 1);
    let tag = &bytes[frames[0].0..frames[0].0 + frames[0].2];
    assert_eq!(&tag[21..25], b"Info");
    assert_eq!(u32::from_be_bytes(tag[25..29].try_into().unwrap()), 0x0F);
    assert_eq!(u32::from_be_bytes(tag[29..33].try_into().unwrap()), 1148);
    assert_eq!(
        u32::from_be_bytes(tag[33..37].try_into().unwrap()) as usize,
        bytes.len()
    );
    // LAME extension after the 100-byte TOC and the 4-byte quality field.
    assert_eq!(&tag[141..150], b"LAME3.100");
    assert_eq!(tag[150] & 0x0F, 1, "VBR method");
    assert_eq!(tag[151], 205, "low-pass byte");
    // Audio frames come after the tag frame: a CBR 320 stream has no other bitrate.
    assert!(frames.iter().all(|(_, kbps, _)| *kbps == 320));
}

#[test]
fn mp3_vbr_and_abr_fixtures_alternate_their_bitrate_indexes() {
    let vbr = mp3_frames_of(&lossy_bytes("mp3_vbr_lp19k5"));
    assert_eq!(
        vbr[1..5].iter().map(|f| f.1).collect::<Vec<_>>(),
        [224, 256, 224, 256]
    );
    let bytes = lossy_bytes("mp3_abr_lp18k6");
    let abr = mp3_frames_of(&bytes);
    assert_eq!(
        abr[1..5].iter().map(|f| f.1).collect::<Vec<_>>(),
        [160, 224, 160, 224]
    );
    let tag = &bytes[..abr[0].2];
    assert_eq!(&tag[21..25], b"Xing");
    assert_eq!(tag[150] & 0x0F, 2, "VBR method of the ABR fixture");
    assert_eq!(tag[151], 0, "no low-pass declared");
    assert!(
        mp3_frames_of(&lossy_bytes("mp3_192_lp17k_notag"))
            .iter()
            .all(|f| f.1 == 192),
        "the stream without a tag frame is all audio"
    );
}

#[test]
fn aac_sample_sizes_add_up_to_the_exact_declared_bitrate() {
    // lossy-fixtures.md 1.2: sum of sample sizes = floor(1292 · B), B = bitrate · 1024 / (8 · rate).
    for (name, kbps) in [("m4a_aac_128_lp16k", 128u64), ("m4a_aac_256_lp15k3", 256)] {
        let bytes = lossy_bytes(name);
        let (ftyp, moov) = lossy::mp4_header_len(&bytes);
        let mdat = bytes.len() - ftyp - moov - 8;
        let expected = 1292 * kbps * 1000 * 1024 / (8 * 44_100);
        assert_eq!(mdat as u64, expected, "{name}");
        let seconds = 1292.0 * 1024.0 / 44_100.0;
        let declared = 8.0 * mdat as f64 / seconds / 1000.0;
        assert_eq!(declared.round() as u64, kbps, "{name}: {declared}");
    }
}

#[test]
fn corrupt_mp3_has_no_frame_sync_and_no_id3_header() {
    let bytes = lossy_bytes("mp3_corrupt");
    assert_eq!(bytes.len(), 4096);
    assert!(bytes.iter().all(u8::is_ascii_alphabetic));
    assert!(!bytes.windows(3).any(|w| w == b"ID3"));
}

// ---- Section 3, check 1: decoding ------------------------------------------------------------------

#[test]
fn every_fixture_decodes_with_the_writers_rate_channels_and_duration() {
    for e in fixtures_with_an_edge() {
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
}

#[test]
fn the_alac_fixtures_decode_to_the_lossless_signal() {
    for name in ["m4a_alac_full_band", "m4a_alac_lp16k"] {
        let (rate, samples) = decode(&lossy_fixture(name)).unwrap();
        assert_eq!(rate, 44_100);
        assert_eq!(samples.len(), 322 * 4096, "{name}");
    }
}

#[test]
fn the_error_fixtures_do_not_decode() {
    for name in ["mp3_corrupt", "m4a_corrupt", "m4a_he_aac"] {
        assert!(decode(&lossy_fixture(name)).is_err(), "{name}");
    }
    // The short and silent fixtures do decode: their errors and verdicts come from the rules.
    let (_, short) = decode(&lossy_fixture("mp3_short")).unwrap();
    assert!(short.len().abs_diff(77 * 1152) <= 1152);
    let (_, silent) = decode(&lossy_fixture("mp3_silence")).unwrap();
    assert!(rms_dbfs(&silent) < -200.0);
}

// ---- Section 3, check 2: cutoff and step -----------------------------------------------------------

#[test]
fn measured_cutoff_is_within_200_hz_of_the_edge() {
    for e in fixtures_with_an_edge()
        .iter()
        .filter(|e| e.name != "mp3_320_low_hf")
    {
        let m = measured(e.name);
        assert!(
            (m.fc - e.nominal_hz).abs() <= 200.0,
            "{}: fc {} Hz, nominal edge {} Hz (coded {} Hz)",
            e.name,
            m.fc,
            e.nominal_hz,
            e.coded_hz
        );
        assert!(
            (m.fc - e.coded_hz).abs() <= 200.0,
            "{}: fc {} Hz, coded edge {} Hz",
            e.name,
            m.fc,
            e.coded_hz
        );
    }
}

#[test]
fn step_is_at_least_fake_min_step_for_edges_between_11_khz_and_nyquist_minus_400_hz() {
    let params = DetectionParams::default();
    for e in fixtures_with_an_edge() {
        if e.nominal_hz < 11_000.0 || e.nominal_hz > f64::from(e.rate) / 2.0 - 400.0 {
            continue;
        }
        let d = measured(e.name).m.step_db;
        assert!(
            d >= params.verdict.step_fake_db,
            "{}: D {d} dB, fake_min_step_db {}",
            e.name,
            params.verdict.step_fake_db
        );
    }
}

// ---- Section 3, check 3: band levels ---------------------------------------------------------------

#[test]
fn every_band_level_is_finite_and_above_the_power_floor() {
    // The spectrum producer floors a band power at 1e-30 (-300 dB): a band at that level is
    // "exactly zero" and rules.md defines no floor for it.
    for e in fixtures_with_an_edge() {
        let min = measured(e.name).min_band_db;
        assert!(
            min.is_finite() && min > -299.0,
            "{}: min M(b) {min}",
            e.name
        );
    }
}

// ---- Section 3, check 4: treble ----------------------------------------------------------------------

#[test]
fn treble_is_above_the_threshold_except_for_the_low_hf_fixture() {
    // Check 4: H >= low_hf_threshold_db for every fixture with an edge of 16 kHz or more (the one at
    // 15.3 kHz is expected `Fake`, not `Suspicious`, so it is checked too), below it for the
    // fixture with little treble.
    let params = DetectionParams::default();
    for e in fixtures_with_an_edge() {
        let h = measured(e.name).m.high_frequency_db;
        if e.name == "mp3_320_low_hf" {
            assert!(h < params.verdict.low_hf_threshold_db, "{}: H {h}", e.name);
        } else {
            assert!(h >= params.verdict.low_hf_threshold_db, "{}: H {h}", e.name);
        }
    }
}

// ---- Level -------------------------------------------------------------------------------------------

#[test]
fn levels_are_between_minus_30_and_minus_12_dbfs() {
    // lossy-fixtures.md 1.1: one global_gain, chosen so that the decoded RMS of a fixture lies
    // between -30 and -12 dBFS (the test stage records the value in the writer module).
    for e in fixtures_with_an_edge() {
        let level = measured(e.name).rms_dbfs;
        assert!((-30.0..=-12.0).contains(&level), "{}: {level} dBFS", e.name);
    }
}
