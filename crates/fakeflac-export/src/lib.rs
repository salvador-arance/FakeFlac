//! See docs/architecture/backend.md. Adapter crate: implements ports defined in fakeflac-core.
//! CSV export (RF-09, `specs/contracts/csv-export.md`).

use fakeflac_core::ports::CsvExporter;
use fakeflac_core::{
    AppError, AppErrorCode, AudioCodec, AudioFormat, BitrateMode, SuspicionReason, TrackAnalysis,
    TrackError, Verdict,
};
use std::io::Write;
use std::path::Path;

/// Column names, in order (csv-export.md section 3).
const HEADER: [&str; 21] = [
    "verdict",
    "file",
    "folder",
    "format",
    "sample_rate_hz",
    "bit_depth",
    "duration_ms",
    "cutoff_hz",
    "declared_bitrate_kbps",
    "bitrate_mode",
    "estimated_source_bitrate_kbps",
    "reasons",
    "error",
    "error_detail",
    "codec",
    "channels",
    "size_bytes",
    "cutoff_steepness_db_per_khz",
    "noise_floor_db",
    "algorithm_version",
    "path",
];

/// Temporary names tried before giving up (csv-export.md section 6).
const TEMP_ATTEMPTS: u32 = 16;

fn push_field(out: &mut String, value: &str) {
    if value.contains([',', '"', '\r', '\n']) {
        out.push('"');
        out.push_str(&value.replace('"', "\"\""));
        out.push('"');
    } else {
        out.push_str(value);
    }
}

fn push_record(out: &mut String, fields: &[String]) {
    for (i, f) in fields.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_field(out, f);
    }
    out.push_str("\r\n");
}

fn verdict_id(v: Verdict) -> &'static str {
    match v {
        Verdict::Real => "real",
        Verdict::Suspicious => "suspicious",
        Verdict::Fake => "fake",
        Verdict::Error => "error",
    }
}

fn format_id(f: AudioFormat) -> &'static str {
    match f {
        AudioFormat::Wav => "wav",
        AudioFormat::Flac => "flac",
        AudioFormat::Mp3 => "mp3",
        AudioFormat::M4a => "m4a",
        AudioFormat::Opus => "opus",
    }
}

fn codec_id(c: AudioCodec) -> &'static str {
    match c {
        AudioCodec::Pcm => "pcm",
        AudioCodec::Flac => "flac",
        AudioCodec::Alac => "alac",
        AudioCodec::Mp3 => "mp3",
        AudioCodec::Aac => "aac",
        AudioCodec::Opus => "opus",
    }
}

fn mode_id(m: BitrateMode) -> &'static str {
    match m {
        BitrateMode::Cbr => "cbr",
        BitrateMode::Vbr => "vbr",
        BitrateMode::Abr => "abr",
    }
}

fn reason_id(r: SuspicionReason) -> &'static str {
    match r {
        SuspicionReason::LowHighFrequencyContent => "lowHighFrequencyContent",
        SuspicionReason::GradualRolloff => "gradualRolloff",
        SuspicionReason::AmbiguousCutoff => "ambiguousCutoff",
        SuspicionReason::HighBandGaps => "highBandGaps",
        SuspicionReason::CutoffBelowDeclared => "cutoffBelowDeclared",
    }
}

/// An integer that is unknown when `0` (Q4).
fn nonzero(v: u64) -> String {
    if v == 0 {
        String::new()
    } else {
        v.to_string()
    }
}

/// One decimal, `.` separator, never `-0.0` (section 4). `v` is the stored `f32` widened.
fn one_decimal(v: f64) -> String {
    let s = format!("{v:.1}");
    if s == "-0.0" {
        "0.0".to_string()
    } else {
        s
    }
}

fn opt<T: ToString>(v: Option<T>) -> String {
    v.map_or_else(String::new, |v| v.to_string())
}

fn record_of(t: &TrackAnalysis) -> Vec<String> {
    let info = &t.info;
    let cutoff = t.cutoff.as_ref().filter(|c| {
        c.frequency_hz.is_finite()
            && c.steepness_db_per_khz.is_finite()
            && c.noise_floor_db.is_finite()
    });
    let (error, detail) = match &t.error {
        None => ("", String::new()),
        Some(TrackError::Unreadable(d)) => ("unreadable", d.clone()),
        Some(TrackError::CorruptStream(d)) => ("corruptStream", d.clone()),
        Some(TrackError::UnsupportedFormat(d)) => ("unsupportedFormat", d.clone()),
        Some(TrackError::TooShort) => ("tooShort", String::new()),
    };
    let reasons: Vec<&str> = t.reasons.iter().map(|r| reason_id(*r)).collect();
    vec![
        verdict_id(t.verdict).to_string(),
        info.path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        info.folder.to_string_lossy().into_owned(),
        format_id(info.format).to_string(),
        nonzero(u64::from(info.sample_rate)),
        opt(info.bit_depth),
        nonzero(info.duration_ms),
        cutoff.map_or_else(String::new, |c| {
            format!("{:.0}", f64::from(c.frequency_hz).round())
        }),
        opt(info.declared_bitrate_kbps),
        info.bitrate_mode
            .map_or_else(String::new, |m| mode_id(m).to_string()),
        opt(t.estimated_source_bitrate_kbps),
        reasons.join(";"),
        error.to_string(),
        detail,
        info.codec
            .map_or_else(String::new, |c| codec_id(c).to_string()),
        nonzero(u64::from(info.channels)),
        info.size_bytes.to_string(),
        cutoff.map_or_else(String::new, |c| {
            one_decimal(f64::from(c.steepness_db_per_khz))
        }),
        cutoff.map_or_else(String::new, |c| one_decimal(f64::from(c.noise_floor_db))),
        t.algorithm_version.to_string(),
        info.path.to_string_lossy().into_owned(),
    ]
}

/// The whole CSV document (header and one record per track, in the order given), UTF-8 without
/// BOM, CRLF line ends (csv-export.md sections 1 to 4). Pure: no locale, no I/O.
pub fn build_document(tracks: &[TrackAnalysis]) -> String {
    let mut out = String::new();
    let header: Vec<String> = HEADER.iter().map(|h| (*h).to_string()).collect();
    push_record(&mut out, &header);
    for t in tracks {
        push_record(&mut out, &record_of(t));
    }
    out
}

/// Proposed file name of the save dialog (csv-export.md section 5.1). Pure: reads no file system.
pub fn default_file_name(root: &Path) -> String {
    match root.file_name() {
        Some(name) => format!("FakeFlac - {}.csv", name.to_string_lossy()),
        None => "FakeFlac.csv".to_string(),
    }
}

/// Writes the document to the destination through a temporary file (csv-export.md section 6).
#[derive(Debug, Default, Clone, Copy)]
pub struct CsvFileExporter;

fn failed(dest: &Path, reason: impl std::fmt::Display) -> AppError {
    AppError::new(
        AppErrorCode::ExportFailed,
        format!("cannot write {}: {reason}", dest.display()),
    )
}

impl CsvExporter for CsvFileExporter {
    fn export(&self, tracks: &[TrackAnalysis], dest: &Path) -> Result<(), AppError> {
        let document = build_document(tracks);
        let file_name = dest
            .file_name()
            .ok_or_else(|| failed(dest, "no file name"))?
            .to_string_lossy()
            .into_owned();
        let pid = std::process::id();

        // Create-new: never opens an existing file.
        let mut created = None;
        let mut last_err = None;
        for n in 0..TEMP_ATTEMPTS {
            let tmp = dest.with_file_name(format!("{file_name}.{pid}-{n}.tmp"));
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp)
            {
                Ok(f) => {
                    created = Some((f, tmp));
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => last_err = Some(e),
                Err(e) => return Err(failed(dest, e)),
            }
        }
        let Some((mut file, tmp)) = created else {
            let reason = last_err.map_or_else(
                || "no temporary file name available".to_string(),
                |e| e.to_string(),
            );
            return Err(failed(
                dest,
                format!("all temporary names are taken: {reason}"),
            ));
        };

        let written = file
            .write_all(document.as_bytes())
            .and_then(|()| file.sync_all());
        drop(file);
        let result = written.and_then(|()| std::fs::rename(&tmp, dest));
        if let Err(e) = result {
            return Err(match std::fs::remove_file(&tmp) {
                Ok(()) => failed(dest, e),
                Err(rm) => failed(
                    dest,
                    format!(
                        "{e}; the temporary file {} could not be deleted: {rm}",
                        tmp.display()
                    ),
                ),
            });
        }
        Ok(())
    }
}
