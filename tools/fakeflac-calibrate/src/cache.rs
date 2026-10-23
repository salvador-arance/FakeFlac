//! The features cache (generation.md section 4): JSON Lines, one file per extraction fingerprint.
//!
//! The tool mirrors the core's `BandSpectra` here (ADR-0036 decision 2): the core gains no serde
//! derive. Floats are written by `serde_json` (shortest round-trip form) and read with its
//! `float_roundtrip` feature, so a report computed from the cache equals one from a fresh extraction.

use crate::hash::id16;
use fakeflac_audio::EXTRACTION_VERSION;
use fakeflac_core::detect::StreamInfo;
use fakeflac_core::ports::AudioProbe;
use fakeflac_core::{AudioCodec, BandSpectra, BitrateMode, DetectionParams, TrackError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// What a decode produced: the probe of the audio adapter, or the per-track error (data, RNF-04).
#[derive(Debug, Clone, PartialEq)]
pub enum Extracted {
    Probe(ProbeData),
    Error(TrackError),
}

/// `AudioProbe` of the core, with the spectrum rebuilt from the cache.
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeData {
    pub sample_rate: u32,
    pub bit_depth: Option<u8>,
    pub channels: u8,
    pub duration_ms: u64,
    /// `None`: every analysed frame was silent.
    pub spectrum: Option<BandSpectra>,
    /// What the stream declares (lossy.md section 5, `stream`).
    pub stream: StreamData,
}

/// The `stream` information of a cache entry (lossy.md section 5).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamData {
    pub codec: Option<AudioCodec>,
    pub declared_bitrate_kbps: Option<u32>,
    pub bitrate_mode: Option<BitrateMode>,
    pub encoder: Option<String>,
    pub encoder_lowpass_hz: Option<u32>,
}

pub fn codec_name(codec: AudioCodec) -> &'static str {
    match codec {
        AudioCodec::Pcm => "pcm",
        AudioCodec::Flac => "flac",
        AudioCodec::Alac => "alac",
        AudioCodec::Mp3 => "mp3",
        AudioCodec::Aac => "aac",
        AudioCodec::Opus => "opus",
    }
}

fn codec_from_name(name: &str) -> Option<AudioCodec> {
    Some(match name {
        "pcm" => AudioCodec::Pcm,
        "flac" => AudioCodec::Flac,
        "alac" => AudioCodec::Alac,
        "mp3" => AudioCodec::Mp3,
        "aac" => AudioCodec::Aac,
        "opus" => AudioCodec::Opus,
        _ => return None,
    })
}

pub fn bitrate_mode_name(mode: BitrateMode) -> &'static str {
    match mode {
        BitrateMode::Cbr => "cbr",
        BitrateMode::Vbr => "vbr",
        BitrateMode::Abr => "abr",
    }
}

fn bitrate_mode_from_name(name: &str) -> Option<BitrateMode> {
    Some(match name {
        "cbr" => BitrateMode::Cbr,
        "vbr" => BitrateMode::Vbr,
        "abr" => BitrateMode::Abr,
        _ => return None,
    })
}

impl ProbeData {
    /// What the core's lossy functions need (`None` for a probe without a codec).
    pub fn stream_info(&self) -> Option<StreamInfo> {
        Some(StreamInfo {
            codec: self.stream.codec?,
            sample_rate: self.sample_rate,
            declared_bitrate_kbps: self.stream.declared_bitrate_kbps,
            encoder_lowpass_hz: self.stream.encoder_lowpass_hz,
        })
    }
}

impl Extracted {
    /// From the result of `SpectrumAnalyzer::analyze`.
    pub fn from_analysis(result: Result<AudioProbe, TrackError>) -> Extracted {
        match result {
            Ok(p) => Extracted::Probe(ProbeData {
                sample_rate: p.sample_rate,
                bit_depth: p.bit_depth,
                channels: p.channels,
                duration_ms: p.duration_ms,
                spectrum: p.spectrum,
                stream: StreamData {
                    codec: p.codec,
                    declared_bitrate_kbps: p.declared_bitrate_kbps,
                    bitrate_mode: p.bitrate_mode,
                    encoder: p.encoder,
                    encoder_lowpass_hz: p.encoder_lowpass_hz,
                },
            }),
            Err(e) => Extracted::Error(e),
        }
    }

    /// `unreadable` may be temporary, so it is never cached.
    pub fn cacheable(&self) -> bool {
        !matches!(self, Extracted::Error(TrackError::Unreadable(_)))
    }
}

/// The `TrackError` code as the spec names it (generation.md section 4.2).
pub fn error_code(e: &TrackError) -> &'static str {
    match e {
        TrackError::Unreadable(_) => "unreadable",
        TrackError::CorruptStream(_) => "corruptStream",
        TrackError::UnsupportedFormat(_) => "unsupportedFormat",
        TrackError::TooShort => "tooShort",
    }
}

/// JSON with the keys of every object sorted, no spaces: the form used for keys and hashes.
pub fn canonical_json(v: &Value) -> String {
    match v {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .into_iter()
                .map(|k| {
                    let value = map.get(k).map(canonical_json).unwrap_or_default();
                    format!("{}:{value}", Value::String(k.clone()))
                })
                .collect();
            format!("{{{}}}", parts.join(","))
        }
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(canonical_json).collect();
            format!("[{}]", parts.join(","))
        }
        other => other.to_string(),
    }
}

/// The extraction fingerprint of a params file: `EXTRACTION_VERSION` plus `[input]` and `[fft]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint {
    pub json: String,
    pub id: String,
}

pub fn fingerprint(params: &DetectionParams) -> Fingerprint {
    let value = json!({
        "extraction_version": EXTRACTION_VERSION,
        "input": serde_json::to_value(&params.input).unwrap_or(Value::Null),
        "fft": serde_json::to_value(&params.fft).unwrap_or(Value::Null),
    });
    let json = canonical_json(&value);
    let id = id16(json.as_bytes());
    Fingerprint { json, id }
}

#[derive(Serialize, Deserialize)]
struct SpectrumJson {
    band_hz: f64,
    mean_db: Vec<f64>,
    median_db: Vec<f64>,
}

#[derive(Serialize, Deserialize)]
struct StreamJson {
    codec: Option<String>,
    declared_bitrate_kbps: Option<u32>,
    bitrate_mode: Option<String>,
    encoder: Option<String>,
    encoder_lowpass_hz: Option<u32>,
}

#[derive(Serialize, Deserialize)]
struct ProbeJson {
    sample_rate: u32,
    bit_depth: Option<u8>,
    channels: u8,
    duration_ms: u64,
    spectrum: Option<SpectrumJson>,
    /// Absent in lines written before RF-15 (EXTRACTION_VERSION stays 2, lossy.md C7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stream: Option<StreamJson>,
}

/// The codec of a line without `stream`: a FLAC or WAV entry, by the extension of its path.
fn legacy_codec(key: &Value) -> AudioCodec {
    let wav = key
        .get("path")
        .and_then(Value::as_str)
        .is_some_and(|p| p.to_ascii_lowercase().ends_with(".wav"));
    if wav {
        AudioCodec::Pcm
    } else {
        AudioCodec::Flac
    }
}

#[derive(Serialize, Deserialize)]
struct LineJson {
    key: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    probe: Option<ProbeJson>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ffmpeg_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    encoder_version: Option<String>,
}

/// One cached entry.
#[derive(Debug, Clone)]
pub struct CachedEntry {
    pub extracted: Arc<Extracted>,
    pub ffmpeg_version: Option<String>,
    pub encoder_version: Option<String>,
}

/// `features/<fingerprint id>.jsonl`, loaded in memory and appended to.
pub struct FeatureCache {
    path: PathBuf,
    entries: HashMap<String, CachedEntry>,
    /// Lines that did not parse (for example cut short by a crash).
    pub skipped_lines: u32,
    needs_newline: bool,
    file: Option<std::fs::File>,
    meta: (PathBuf, String),
}

impl FeatureCache {
    /// Loads `dir/<id>.jsonl` if it exists and writes `dir/<id>.meta.json` if it does not.
    pub fn open(dir: &Path, fingerprint: &Fingerprint) -> std::io::Result<FeatureCache> {
        // Nothing is created until the first line is appended (exit 3 must leave no trace).
        let meta = dir.join(format!("{}.meta.json", fingerprint.id));
        let path = dir.join(format!("{}.jsonl", fingerprint.id));
        let mut cache = FeatureCache {
            path,
            entries: HashMap::new(),
            skipped_lines: 0,
            needs_newline: false,
            file: None,
            meta: (meta, fingerprint.json.clone()),
        };
        if let Ok(bytes) = std::fs::read(&cache.path) {
            cache.needs_newline = bytes.last().is_some_and(|b| *b != b'\n');
            let text = String::from_utf8_lossy(&bytes);
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                match parse_line(line) {
                    Some((key, entry)) => {
                        cache.entries.insert(key, entry);
                    }
                    None => cache.skipped_lines += 1,
                }
            }
        }
        Ok(cache)
    }

    /// Entry by canonical key (the last line with that key wins).
    pub fn get(&self, key: &str) -> Option<&CachedEntry> {
        self.entries.get(key)
    }

    /// Appends one line and remembers it.
    pub fn append(
        &mut self,
        key: &Value,
        extracted: &Extracted,
        ffmpeg_version: Option<&str>,
        encoder_version: Option<&str>,
    ) -> std::io::Result<()> {
        let line = LineJson {
            key: key.clone(),
            probe: match extracted {
                Extracted::Probe(p) => Some(ProbeJson {
                    sample_rate: p.sample_rate,
                    bit_depth: p.bit_depth,
                    channels: p.channels,
                    duration_ms: p.duration_ms,
                    spectrum: p.spectrum.as_ref().map(|s| SpectrumJson {
                        band_hz: s.band_hz,
                        mean_db: s.mean_db.clone(),
                        median_db: s.median_db.clone(),
                    }),
                    stream: Some(StreamJson {
                        codec: p.stream.codec.map(|c| codec_name(c).to_string()),
                        declared_bitrate_kbps: p.stream.declared_bitrate_kbps,
                        bitrate_mode: p.stream.bitrate_mode.map(|m| bitrate_mode_name(m).into()),
                        encoder: p.stream.encoder.clone(),
                        encoder_lowpass_hz: p.stream.encoder_lowpass_hz,
                    }),
                }),
                Extracted::Error(_) => None,
            },
            error: match extracted {
                Extracted::Error(e) => Some(error_code(e).to_string()),
                Extracted::Probe(_) => None,
            },
            ffmpeg_version: ffmpeg_version.map(str::to_string),
            encoder_version: encoder_version.map(str::to_string),
        };
        let mut text = serde_json::to_string(&line).map_err(std::io::Error::other)?;
        text.push('\n');
        if self.file.is_none() {
            if let Some(dir) = self.path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            if !self.meta.0.exists() {
                crate::fsutil::write_atomic(&self.meta.0, self.meta.1.as_bytes())?;
            }
            self.file = Some(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.path)?,
            );
            if self.needs_newline {
                text.insert(0, '\n');
                self.needs_newline = false;
            }
        }
        if let Some(file) = self.file.as_mut() {
            file.write_all(text.as_bytes())?;
        }
        self.entries.insert(
            canonical_json(key),
            CachedEntry {
                extracted: Arc::new(extracted.clone()),
                ffmpeg_version: line.ffmpeg_version,
                encoder_version: line.encoder_version,
            },
        );
        Ok(())
    }
}

fn parse_line(line: &str) -> Option<(String, CachedEntry)> {
    let parsed: LineJson = serde_json::from_str(line).ok()?;
    let extracted = match (parsed.probe, parsed.error) {
        (Some(p), None) => Extracted::Probe(ProbeData {
            sample_rate: p.sample_rate,
            bit_depth: p.bit_depth,
            channels: p.channels,
            duration_ms: p.duration_ms,
            stream: match p.stream {
                Some(s) => StreamData {
                    codec: s.codec.as_deref().and_then(codec_from_name),
                    declared_bitrate_kbps: s.declared_bitrate_kbps,
                    bitrate_mode: s.bitrate_mode.as_deref().and_then(bitrate_mode_from_name),
                    encoder: s.encoder,
                    encoder_lowpass_hz: s.encoder_lowpass_hz,
                },
                None => StreamData {
                    codec: Some(legacy_codec(&parsed.key)),
                    ..StreamData::default()
                },
            },
            spectrum: p.spectrum.map(|s| BandSpectra {
                sample_rate: p.sample_rate,
                band_hz: s.band_hz,
                mean_db: s.mean_db,
                median_db: s.median_db,
            }),
        }),
        (None, Some(code)) => Extracted::Error(match code.as_str() {
            "corruptStream" => TrackError::CorruptStream(String::new()),
            "unsupportedFormat" => TrackError::UnsupportedFormat(String::new()),
            "tooShort" => TrackError::TooShort,
            _ => return None,
        }),
        _ => return None,
    };
    Some((
        canonical_json(&parsed.key),
        CachedEntry {
            extracted: Arc::new(extracted),
            ffmpeg_version: parsed.ffmpeg_version,
            encoder_version: parsed.encoder_version,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_json_sorts_keys() {
        let v = json!({"b": 1, "a": {"d": [2, 1], "c": "x"}});
        assert_eq!(canonical_json(&v), r#"{"a":{"c":"x","d":[2,1]},"b":1}"#);
    }

    #[test]
    fn fingerprint_ignores_rule_parameters() {
        let a = DetectionParams::default();
        let mut b = a.clone();
        b.algorithm_version = 3;
        b.verdict.gap_fake_db = 4.0;
        assert_eq!(fingerprint(&a), fingerprint(&b));
        b.fft.band_hz = 50.0;
        assert_ne!(fingerprint(&a).id, fingerprint(&b).id);
    }

    #[test]
    fn floats_round_trip_through_a_line() {
        let spectra = BandSpectra {
            sample_rate: 44_100,
            band_hz: 100.0,
            mean_db: vec![-12.345_678_901_234_567, 0.1 + 0.2],
            median_db: vec![1e-7, -80.000_000_000_000_01],
        };
        let extracted = Extracted::Probe(ProbeData {
            sample_rate: 44_100,
            bit_depth: Some(16),
            channels: 2,
            duration_ms: 1234,
            spectrum: Some(spectra),
            stream: StreamData {
                codec: Some(AudioCodec::Mp3),
                declared_bitrate_kbps: Some(320),
                bitrate_mode: Some(BitrateMode::Cbr),
                encoder: Some("LAME3.100".to_string()),
                encoder_lowpass_hz: Some(20_500),
            },
        });
        let dir = std::env::temp_dir().join(format!("fakeflac-cache-unit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let fp = fingerprint(&DetectionParams::default());
        let key = json!({"kind": "corpus", "path": "a/b.flac", "size": 1, "mtime": 2});
        {
            let mut cache = FeatureCache::open(&dir, &fp).expect("open");
            cache.append(&key, &extracted, None, None).expect("append");
        }
        let cache = FeatureCache::open(&dir, &fp).expect("reopen");
        let hit = cache.get(&canonical_json(&key)).expect("hit");
        assert_eq!(*hit.extracted, extracted);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
