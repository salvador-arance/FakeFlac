//! ffmpeg as an external process (`std::process::Command`, argument list, no shell). Generation.md section 2.

use crate::config::Setting;
use crate::error::ToolError;
use crate::fsutil::plain_path;
use crate::transcode::{Fragment, TranscodeError, TranscodeOutput, TranscodeRequest, Transcoder};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, OnceLock};

const NOT_FOUND: &str = "error: ffmpeg not found; set FAKEFLAC_FFMPEG or add it to PATH";

fn text(path: &Path) -> String {
    plain_path(path).to_string_lossy().into_owned()
}

fn base_args() -> Vec<String> {
    ["-nostdin", "-hide_banner", "-loglevel", "error"]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

/// Arguments of the encode call, one `String` per argument, `source` passed unchanged:
/// `-nostdin -hide_banner -loglevel error [-ss <start> -t <fragment_s>] -i <source>
///  -map 0:a:0 -map_metadata -1 <setting args> -y <out>`. Times have 3 decimals; with no fragment
/// `-ss` and `-t` are omitted. The program name is not part of the list.
pub fn encode_args(
    source: &Path,
    fragment: Option<Fragment>,
    setting: &Setting,
    out: &Path,
) -> Vec<String> {
    let mut args = base_args();
    if let Some(f) = fragment {
        args.push("-ss".to_string());
        args.push(format!("{:.3}", f.start_s));
        args.push("-t".to_string());
        args.push(format!("{:.3}", f.duration_s));
    }
    args.push("-i".to_string());
    args.push(text(source));
    args.extend(["-map", "0:a:0", "-map_metadata", "-1"].map(str::to_string));
    args.extend(setting.args.iter().cloned());
    args.push("-y".to_string());
    args.push(text(out));
    args
}

/// Arguments of the decode call:
/// `-nostdin -hide_banner -loglevel error -i <input> -map 0:a:0 -map_metadata -1 <decode_args> -y <out>`.
pub fn decode_args(input: &Path, decode_args: &[String], out: &Path) -> Vec<String> {
    let mut args = base_args();
    args.push("-i".to_string());
    args.push(text(input));
    args.extend(["-map", "0:a:0", "-map_metadata", "-1"].map(str::to_string));
    args.extend(decode_args.iter().cloned());
    args.push("-y".to_string());
    args.push(text(out));
    args
}

/// `FAKEFLAC_FFMPEG`, or `ffmpeg` on `PATH` when `None`.
#[derive(Debug, Clone)]
pub struct FfmpegTranscoder {
    pub program: Option<PathBuf>,
    version_cache: Arc<OnceLock<String>>,
}

impl FfmpegTranscoder {
    pub fn new(program: Option<PathBuf>) -> FfmpegTranscoder {
        FfmpegTranscoder {
            program,
            version_cache: Arc::new(OnceLock::new()),
        }
    }

    fn command(&self) -> Command {
        let mut cmd = match &self.program {
            Some(p) => Command::new(p),
            None => Command::new("ffmpeg"),
        };
        cmd.stdin(Stdio::null());
        cmd
    }

    fn run(&self, args: &[String]) -> Result<(), TranscodeError> {
        let output = self
            .command()
            .args(args)
            .output()
            .map_err(|_| TranscodeError::NotFound)?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        let first = stderr
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("ffmpeg failed")
            .trim()
            .to_string();
        Err(TranscodeError::Failed(first))
    }
}

impl Transcoder for FfmpegTranscoder {
    fn supports(&self, encoder: &str) -> bool {
        matches!(encoder, "lame" | "aac" | "opus")
    }

    fn version(&self) -> Result<String, ToolError> {
        if let Some(v) = self.version_cache.get() {
            return Ok(v.clone());
        }
        let output = self
            .command()
            .arg("-version")
            .output()
            .map_err(|_| ToolError::external(NOT_FOUND))?;
        if !output.status.success() {
            return Err(ToolError::external(
                "error: ffmpeg not found; `ffmpeg -version` failed",
            ));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let first = stdout.lines().next().unwrap_or("").trim();
        let version = first
            .split(" Copyright")
            .next()
            .unwrap_or(first)
            .trim()
            .to_string();
        let _ = self.version_cache.set(version.clone());
        Ok(version)
    }

    fn transcode(&self, request: &TranscodeRequest<'_>) -> Result<TranscodeOutput, TranscodeError> {
        let ffmpeg_version = self.version().map_err(|_| TranscodeError::NotFound)?;
        std::fs::create_dir_all(request.generated_dir)
            .map_err(|e| TranscodeError::Failed(e.to_string()))?;
        let lossy = request
            .generated_dir
            .join(format!("{}.{}", request.id, request.setting.ext));
        let flac = request.generated_dir.join(format!("{}.flac", request.id));

        let encode = encode_args(request.source, request.fragment, request.setting, &lossy);
        let result = self.run(&encode).and_then(|()| {
            let encoder = encoder_version(&ffmpeg_version, &request.setting.encoder, &lossy);
            let decode = decode_args(&lossy, request.decode_args, &flac);
            self.run(&decode).map(|()| encoder)
        });
        // The lossy intermediate is always deleted.
        let _ = std::fs::remove_file(&lossy);
        match result {
            Ok(encoder_version) => Ok(TranscodeOutput {
                flac,
                encoder_version,
            }),
            Err(e) => {
                let _ = std::fs::remove_file(&flac);
                Err(e)
            }
        }
    }
}

/// `<ffmpeg_version>; <encoder>` (generation.md section 2, OQ-17).
fn encoder_version(ffmpeg_version: &str, encoder: &str, lossy: &Path) -> String {
    let detail = match encoder {
        "aac" => "aac (native)".to_string(),
        "opus" => {
            opus_vendor(lossy).unwrap_or_else(|| "libopus (version not readable)".to_string())
        }
        _ => lame_tag(lossy).unwrap_or_else(|| "libmp3lame (version not readable)".to_string()),
    };
    format!("{ffmpeg_version}; {detail}")
}

fn read_head(path: &Path, max: usize) -> Vec<u8> {
    use std::io::Read;
    let mut buf = Vec::new();
    if let Ok(file) = std::fs::File::open(path) {
        let _ = file.take(max as u64).read_to_end(&mut buf);
    }
    buf
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// The encoder string of the LAME tag in the first frame of an MP3 (`LAME3.100`).
fn lame_tag(path: &Path) -> Option<String> {
    let head = read_head(path, 16 * 1024);
    let at = find(&head, b"LAME")?;
    let tag: String = head[at..]
        .iter()
        .take(9)
        .take_while(|b| b.is_ascii_alphanumeric() || **b == b'.')
        .map(|b| char::from(*b))
        .collect();
    (tag.len() > 4).then_some(tag)
}

/// The vendor string of the `OpusTags` header, when it names libopus.
fn opus_vendor(path: &Path) -> Option<String> {
    let head = read_head(path, 16 * 1024);
    let at = find(&head, b"OpusTags")? + 8;
    let len_bytes: [u8; 4] = head.get(at..at + 4)?.try_into().ok()?;
    let len = u32::from_le_bytes(len_bytes) as usize;
    let vendor = String::from_utf8_lossy(head.get(at + 4..at + 4 + len)?).into_owned();
    vendor.contains("libopus").then_some(vendor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lame_tag_is_read_from_the_head() {
        let dir = std::env::temp_dir().join(format!("fakeflac-ffmpeg-unit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let mp3 = dir.join("a.mp3");
        std::fs::write(&mp3, b"\xff\xfb\x90\x00....Info....LAME3.100\x55\x55").expect("write");
        assert_eq!(lame_tag(&mp3).as_deref(), Some("LAME3.100"));
        let opus = dir.join("a.opus");
        let mut bytes = b"OggS....OpusTags".to_vec();
        bytes.extend_from_slice(&13u32.to_le_bytes());
        bytes.extend_from_slice(b"libopus 1.5.2");
        std::fs::write(&opus, &bytes).expect("write");
        assert_eq!(opus_vendor(&opus).as_deref(), Some("libopus 1.5.2"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
