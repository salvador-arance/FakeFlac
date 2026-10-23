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

/// Arguments of every encode call of a lossy entry (lossy.md section 4.3), one list per stage:
/// stage 1 is [`encode_args`] into `<generated>/<id>.s1.<ext>`, stage `k` > 1 encodes the output of
/// stage `k - 1` (no `-ss`/`-t`), and the last stage writes `<generated>/<id>.<ext>`.
pub fn lossy_args(
    _source: &Path,
    _fragment: Option<Fragment>,
    _stages: &[&Setting],
    _generated_dir: &Path,
    _id: &str,
) -> Vec<Vec<String>> {
    todo!("RF-15 lossy.md section 4.3")
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
        // ffmpeg must never write a report file (FFREPORT) outside the generated folder.
        cmd.stdin(Stdio::null()).env_remove("FFREPORT");
        cmd
    }

    fn run(&self, args: &[String], dir: &Path) -> Result<(), TranscodeError> {
        let output = self
            .command()
            .current_dir(dir)
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
    fn transcode_lossy(
        &self,
        _request: &crate::transcode::LossyRequest<'_>,
    ) -> Result<crate::transcode::LossyOutput, TranscodeError> {
        todo!("RF-15 lossy.md section 4.3")
    }

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
        let result = self.run(&encode, request.generated_dir).and_then(|()| {
            let encoder = encoder_version(&ffmpeg_version, &request.setting.encoder, &lossy);
            let decode = decode_args(&lossy, request.decode_args, &flac);
            self.run(&decode, request.generated_dir).map(|()| encoder)
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

fn is_valid_lame_version(s: &str) -> bool {
    let Some(rest) = s.strip_prefix("LAME") else {
        return false;
    };
    let mut chars = rest.chars().peekable();
    let mut major_digits = 0;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            major_digits += 1;
            chars.next();
        } else {
            break;
        }
    }
    if major_digits == 0 {
        return false;
    }
    if chars.next() != Some('.') {
        return false;
    }
    let mut minor_digits = 0;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            minor_digits += 1;
            chars.next();
        } else {
            break;
        }
    }
    if minor_digits == 0 {
        return false;
    }
    if let Some(&c) = chars.peek() {
        if c.is_ascii_alphabetic() {
            chars.next();
        } else if c == '.' {
            chars.next();
            let mut patch_digits = 0;
            while let Some(&c2) = chars.peek() {
                if c2.is_ascii_digit() {
                    patch_digits += 1;
                    chars.next();
                } else {
                    break;
                }
            }
            if patch_digits == 0 {
                return false;
            }
            if let Some(&c2) = chars.peek() {
                if c2.is_ascii_alphabetic() {
                    chars.next();
                }
            }
        }
    }
    chars.peek().is_none()
}

/// The encoder string of the LAME tag in the first frame of an MP3 (`LAME3.100`).
fn lame_tag(path: &Path) -> Option<String> {
    let head = read_head(path, 16 * 1024);
    // 1. Skip ID3v2 if present
    let mut offset = 0;
    if head.starts_with(b"ID3") && head.len() >= 10 {
        let flags = head[5];
        let synchsafe = ((head[6] as usize & 0x7F) << 21)
            | ((head[7] as usize & 0x7F) << 14)
            | ((head[8] as usize & 0x7F) << 7)
            | (head[9] as usize & 0x7F);
        let footer = if (flags & 0x10) != 0 { 10 } else { 0 };
        offset = 10 + synchsafe + footer;
    }
    if offset >= head.len() {
        return None;
    }

    // 2. Locate the first MPEG audio frame header
    let rest = &head[offset..];
    let frame_idx = rest.windows(4).position(|w| {
        let is_sync = w[0] == 0xff && (w[1] & 0xe0) == 0xe0;
        let mpeg_ver = (w[1] >> 3) & 0x03;
        let layer = (w[1] >> 1) & 0x03;
        let bitrate_idx = (w[2] >> 4) & 0x0f;
        let sample_rate_idx = (w[2] >> 2) & 0x03;
        let emphasis = w[3] & 0x03;
        is_sync
            && mpeg_ver != 1
            && layer == 1 // Layer III
            && bitrate_idx != 0x0f
            && sample_rate_idx != 0x03
            && emphasis != 0x02
    })?;

    let frame = &rest[frame_idx..];
    if frame.len() < 4 {
        return None;
    }
    let mpeg_ver = (frame[1] >> 3) & 0x03;
    let crc_present = (frame[1] & 0x01) == 0;
    let channel_mode = (frame[3] >> 6) & 0x03;

    let header_len = if crc_present { 6 } else { 4 };
    let side_info_len = match mpeg_ver {
        3 => {
            if channel_mode == 3 {
                17
            } else {
                32
            }
        }
        _ => {
            if channel_mode == 3 {
                9
            } else {
                17
            }
        }
    };

    let xing_offset = header_len + side_info_len;
    let xing_found_offset = if frame.get(xing_offset..xing_offset + 4) == Some(b"Xing")
        || frame.get(xing_offset..xing_offset + 4) == Some(b"Info")
    {
        Some(xing_offset)
    } else {
        let search_len = frame.len().min(128);
        find(&frame[..search_len], b"Xing").or_else(|| find(&frame[..search_len], b"Info"))
    }?;

    let tag_bytes = frame.get(xing_found_offset + 120..xing_found_offset + 120 + 9)?;
    let tag_str = String::from_utf8_lossy(tag_bytes).trim().to_string();
    if is_valid_lame_version(&tag_str) {
        Some(tag_str)
    } else {
        None
    }
}

/// The vendor string of the `OpusTags` header, when it names libopus.
fn opus_vendor(path: &Path) -> Option<String> {
    let head = read_head(path, 16 * 1024);
    let at = find(&head, b"OpusTags")? + 8;
    let len_bytes: [u8; 4] = head.get(at..at + 4)?.try_into().ok()?;
    let len = u32::from_le_bytes(len_bytes) as usize;
    if len > 256 {
        return None;
    }
    let vendor = String::from_utf8_lossy(head.get(at + 4..at + 4 + len)?).into_owned();
    if vendor.contains("libopus") && vendor.chars().all(|c| c.is_ascii_graphic() || c == ' ') {
        Some(vendor)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mp3_with_info(id3: bool, mono: bool, tag: &[u8], padding_after: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        if id3 {
            bytes.extend_from_slice(b"ID3\x04\x00\x00\x00\x00\x00\x0a");
            bytes.extend_from_slice(&[0u8; 10]);
        }
        let mode_byte = if mono { 0xc0 } else { 0x00 };
        bytes.extend_from_slice(&[0xff, 0xfb, 0x90, mode_byte]);
        let side_info_len = if mono { 17 } else { 32 };
        bytes.extend(std::iter::repeat_n(0u8, side_info_len));
        bytes.extend_from_slice(b"Info");
        bytes.extend(std::iter::repeat_n(0u8, 116));
        bytes.extend_from_slice(tag);
        bytes.extend_from_slice(padding_after);
        bytes
    }

    #[test]
    fn lame_tag_lavc_with_lame_padding_gives_fallback() {
        let dir =
            std::env::temp_dir().join(format!("fakeflac-ffmpeg-unit-lavc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let mp3 = dir.join("lavc.mp3");
        let bytes = make_mp3_with_info(false, true, b"Lavc62.28", b"\x55\x55LAMEUUUU\x55\x55");
        std::fs::write(&mp3, bytes).expect("write");
        assert_eq!(lame_tag(&mp3), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lame_tag_real_lame_in_info_frame_gives_version() {
        let dir =
            std::env::temp_dir().join(format!("fakeflac-ffmpeg-unit-lame-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let mp3 = dir.join("lame.mp3");
        let bytes = make_mp3_with_info(true, false, b"LAME3.100", b"\x55\x55");
        std::fs::write(&mp3, bytes).expect("write");
        assert_eq!(lame_tag(&mp3).as_deref(), Some("LAME3.100"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn opus_vendor_is_read_from_the_head() {
        let dir =
            std::env::temp_dir().join(format!("fakeflac-ffmpeg-unit-opus-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let opus = dir.join("a.opus");
        let mut bytes = b"OggS....OpusTags".to_vec();
        bytes.extend_from_slice(&13u32.to_le_bytes());
        bytes.extend_from_slice(b"libopus 1.5.2");
        std::fs::write(&opus, &bytes).expect("write");
        assert_eq!(opus_vendor(&opus).as_deref(), Some("libopus 1.5.2"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
