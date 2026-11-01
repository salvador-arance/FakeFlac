//! @RF-14 @RF-15 fixture checks before deriving opus.feature scenarios;
//! specs/detection/lossy-fixtures.md §3. Uses a separate packet decoder oracle,
//! then the existing float-WAV spectral path; no new production parser is needed.
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod common;
use common::opus;
use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_core::{detect::measure, ports::SpectrumAnalyzer, DetectionParams};

#[test]
fn rf14_opus_fixture_preconditions() {
    let params = DetectionParams::default();
    for name in opus::ENCODED {
        let e = opus::encoded(name);
        let samples = opus::reference(&e, 120);
        assert_eq!(
            e.packets.len(),
            if name == "opus_short" { 101 } else { 1501 }
        );
        assert!(samples.iter().all(|s| s.len() == e.samples));
        let packet_bytes: usize = e.packets.iter().map(Vec::len).sum();
        let kbps =
            (packet_bytes as f64 * 8.0 / (e.samples as f64 / 48000.0) / 1000.0).round() as u32;
        if name != "opus_short" {
            assert!(kbps.abs_diff(e.kbps) <= 1, "{name}: bitrate {kbps}");
        }
        if name == "opus_128_fb" || name == "opus_128_fb_stereo" {
            for (ch, decoded_channel) in samples.iter().enumerate() {
                // Independent per-channel cross-correlation over 48k samples, searching ±400.
                let start = 48000;
                let end = 96000;
                let correlation = |lag: i32| {
                    (start..end)
                        .map(|i| {
                            f64::from(decoded_channel[i])
                                * f64::from(e.input[((i as i32 + lag) as usize) * e.channels + ch])
                        })
                        .sum::<f64>()
                };
                let lag = (-400..=400)
                    .max_by(|a, b| correlation(*a).total_cmp(&correlation(*b)))
                    .unwrap();
                assert!(
                    lag.abs() <= 1,
                    "{name} channel {ch}: alignment offset {lag}"
                );
                if name == "opus_128_fb" {
                    let input_power = e.input.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>()
                        / e.input.len() as f64;
                    let power = samples[ch]
                        .iter()
                        .map(|x| f64::from(*x).powi(2))
                        .sum::<f64>()
                        / decoded_channel.len() as f64;
                    assert!(
                        (10.0 * (power / input_power).log10()).abs() <= 1.0,
                        "decoded RMS"
                    );
                }
            }
        }
        if name == "opus_short" {
            continue;
        }
        let wav = opus::fixture(name).with_extension("reference.wav");
        let mut w = hound::WavWriter::create(
            &wav,
            hound::WavSpec {
                channels: 1,
                sample_rate: 48000,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        )
        .unwrap();
        for i in 0..e.samples {
            w.write_sample(samples.iter().map(|s| s[i]).sum::<f32>() / e.channels as f32)
                .unwrap();
        }
        w.finalize().unwrap();
        let probe = SymphoniaAnalyzer.analyze(&wav, &params).unwrap();
        let spectrum = probe.spectrum.unwrap();
        assert!(
            spectrum.mean_db.iter().all(|x| x.is_finite()),
            "{name}: nonfinite mean band"
        );
        let m = measure(&spectrum, &params);
        let fc = m.step_hz.unwrap_or(m.extent_hz);
        let edge = if name == "opus_160_src16k" {
            16000.0
        } else if name == "opus_160_swb" {
            12000.0
        } else {
            20000.0
        };
        eprintln!(
            "{name}: kbps={kbps} fc={fc} D={} H={}",
            m.step_db, m.high_frequency_db
        );
        assert!((fc - edge).abs() <= 200.0, "{name}: fc={fc}, edge={edge}");
        assert!(
            m.step_db >= params.lossy.as_ref().unwrap().fake_min_step_db,
            "{name}: D={}",
            m.step_db
        );
        if name == "opus_160_swb" {
            assert!(m.high_frequency_db < params.verdict.low_hf_threshold_db);
        } else {
            assert!(m.high_frequency_db >= params.verdict.low_hf_threshold_db);
        }
    }
}

// @RF-14 Scenario: A packet split across pages is reassembled (fixture precondition).
// Check the independent writer in memory so an old generated file cannot hide a broken recipe.
#[test]
fn rf14_split_fixture_contains_real_packet_continuations() {
    let encoded = opus::encoded("opus_128_fb");
    let pages = opus::pages(&encoded, true);
    let audio = &pages[2..];
    assert!(encoded.packets.iter().any(|p| p.len() > 255));
    assert!(audio.iter().all(|p| p.body.len() <= 255));
    assert!(audio.iter().any(|p| p.flags & 1 != 0));
    assert!(audio.iter().any(|p| p.segments.last() == Some(&255)));
    let mut completed = 0;
    let mut reconstructed = Vec::new();
    let mut packet = Vec::new();
    let mut unfinished = false;
    for (index, page) in audio.iter().enumerate() {
        assert_eq!(page.flags & 1 != 0, unfinished);
        let mut offset = 0;
        let mut completed_here = 0;
        for &len in &page.segments {
            let end = offset + usize::from(len);
            packet.extend_from_slice(&page.body[offset..end]);
            offset = end;
            unfinished = len == 255;
            if !unfinished {
                reconstructed.push(std::mem::take(&mut packet));
                completed += 1;
                completed_here += 1;
            }
        }
        assert_eq!(offset, page.body.len());
        if index == audio.len() - 1 {
            assert_eq!(page.granule, (encoded.samples + 120) as i64);
        } else if completed_here == 0 {
            assert_eq!(page.granule, -1);
        } else {
            assert_eq!(page.granule, completed * 960);
        }
    }
    assert!(!unfinished);
    assert_eq!(reconstructed, encoded.packets);
    // Both malformed variants must actually be constructible from these split pages.
    assert!(!opus::bytes("opus_continuation").is_empty());
    assert!(!opus::bytes("opus_unfinished_packet").is_empty());
}

// @RF-14 Scenario: The fixture encoder is deterministic on one machine.
// Concurrent test callers must observe the same complete synthetic file, including a cold write.
#[test]
fn rf14_fixture_publication_is_safe_for_concurrent_test_callers() {
    use std::sync::{Arc, Barrier};
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("opus-publication-{}-{unique}", std::process::id()));
    let expected = opus::bytes("opus_128_fb");
    let barrier = Arc::new(Barrier::new(16));
    let paths = std::thread::scope(|scope| {
        let threads: Vec<_> = (0..16)
            .map(|_| {
                let dir = &dir;
                let expected = &expected;
                let barrier = Arc::clone(&barrier);
                scope.spawn(move || {
                    barrier.wait();
                    let path = opus::fixture_in(dir, "opus_128_fb");
                    assert_eq!(std::fs::read(&path).unwrap(), *expected);
                    path
                })
            })
            .collect();
        threads
            .into_iter()
            .map(|t| t.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert!(paths.iter().all(|path| path == &paths[0]));
    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        1,
        "no leftover temporary file"
    );
    std::fs::remove_file(&paths[0]).unwrap();
    std::fs::remove_dir(dir).unwrap();
}
