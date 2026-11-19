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

/// Fake analyzer port: returns a fixed probe (the oracle's measurement of a fixture).
struct Probe(fakeflac_core::ports::AudioProbe);
impl SpectrumAnalyzer for Probe {
    fn analyze(
        &self,
        _: &std::path::Path,
        _: &DetectionParams,
    ) -> Result<fakeflac_core::ports::AudioProbe, fakeflac_core::TrackError> {
        Ok(self.0.clone())
    }
}

// @RF-14 Scenario: A start offset is read from the first audio page and is not audio
// (fixture precondition). lossy-fixtures.md §3 check 1 for `opus_start_offset`: 1501 packets and
// durationMs 30017 from the written pages (RFC 7845 §4.5, read here independently of the adapter),
// plus its §2.2 truth (Real, declared bitrate within 1 kbps of 128) measured on the decoder
// oracle's samples. The adapter's own reading is checked by the scenario test in tests/opus.rs.
#[test]
fn rf14_opus_start_offset_fixture_preconditions() {
    let file = opus::bytes("opus_start_offset");
    let e = opus::encoded("opus_128_fb");
    let (mut pos, mut packets, mut page_index) = (0, 0usize, 0);
    let (mut pre_skip, mut first, mut last) = (None, None, None);
    while pos < file.len() {
        assert_eq!(&file[pos..pos + 4], b"OggS");
        let flags = file[pos + 5];
        let granule = i64::from_le_bytes(file[pos + 6..pos + 14].try_into().unwrap());
        let segments = file[pos + 26] as usize;
        let lace = &file[pos + 27..pos + 27 + segments];
        let completed = lace.iter().filter(|x| **x < 255).count();
        if page_index == 0 {
            let body = pos + 27 + segments;
            pre_skip = Some(i64::from(u16::from_le_bytes([
                file[body + 10],
                file[body + 11],
            ])));
        } else if page_index >= 2 {
            packets += completed;
            if first.is_none() {
                first = Some((granule, completed as i64 * 960));
            }
            last = Some((granule, flags));
        }
        pos += 27 + segments + lace.iter().map(|x| *x as usize).sum::<usize>();
        page_index += 1;
    }
    assert_eq!(packets, 1501);
    let (f, s1) = first.unwrap();
    assert_eq!((f, s1), (48_360, 48_000));
    let offset = (f - s1).max(0);
    assert_eq!(offset, 360);
    let (g, flags) = last.unwrap();
    assert_eq!(flags & 4, 4, "end-of-stream flag on the last page");
    assert_eq!(g, 1_441_320);
    let pre_skip = pre_skip.unwrap();
    assert_eq!(pre_skip, 120);
    let samples_of_all_packets = packets as i64 * 960;
    assert!(g - offset >= pre_skip && g - offset <= samples_of_all_packets);
    let l = g - offset - pre_skip;
    assert_eq!(l, 1_440_840);
    let duration_ms = (1000 * l / 48000) as u64;
    assert_eq!(duration_ms, 30017);
    let bytes: usize = e.packets.iter().map(Vec::len).sum();
    let kbps = (8.0 * bytes as f64 / (l as f64 / 48000.0) / 1000.0).round() as u32;
    assert!(kbps.abs_diff(128) <= 1, "declared bitrate {kbps}");

    // Truth (v8): the oracle's samples after the pre-skip, with no end trim.
    let samples = opus::reference_until(&e, 120, e.packets.len() * 960);
    assert_eq!(samples[0].len(), l as usize);
    let wav = opus::fixture("opus_start_offset").with_extension("reference.wav");
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
    for x in &samples[0] {
        w.write_sample(*x).unwrap();
    }
    w.finalize().unwrap();
    let params = DetectionParams::default();
    let mut probe = SymphoniaAnalyzer.analyze(&wav, &params).unwrap();
    probe.bit_depth = None;
    probe.duration_ms = duration_ms;
    probe.codec = Some(fakeflac_core::AudioCodec::Opus);
    probe.declared_bitrate_kbps = Some(kbps);
    let path = opus::fixture("opus_start_offset");
    let entry = fakeflac_core::ports::FileEntry {
        folder: path.parent().unwrap().into(),
        path,
        format: fakeflac_core::AudioFormat::Opus,
        size_bytes: file.len() as u64,
        modified_unix: 0,
    };
    let a = fakeflac_core::analysis::analyze_track(&Probe(probe), &entry, &params);
    eprintln!("opus_start_offset (oracle): {a:?}");
    assert_eq!(a.verdict, fakeflac_core::Verdict::Real, "{a:?}");
    assert_eq!(a.estimated_source_bitrate_kbps, Some(kbps));
    let fc = f64::from(a.cutoff.as_ref().unwrap().frequency_hz);
    assert!((fc - 20000.0).abs() <= 200.0, "fc={fc}");
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
