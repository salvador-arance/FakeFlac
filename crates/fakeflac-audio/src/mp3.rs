//! MP3 stream information read by the adapter itself (ADR-0040, `specs/detection/lossy-source.md`
//! section 2.1): the frame headers, the Xing/Info/VBRI tag frame and its LAME extension. No tag
//! library. Read-only (RNF-05).

use fakeflac_core::{BitrateMode, TrackError};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// Bytes scanned for the first frame sync before giving up.
const SYNC_SEARCH_LIMIT: u64 = 1 << 20;

/// Bitrates (kbps) by bitrate index 1..=14. Layer III: `BITRATES_V1` (MPEG-1) and `BITRATES_V2`
/// (MPEG-2 and 2.5); Layer II MPEG-2 shares the latter. Layers I and II are only recognised, to
/// report them as unsupported.
const BITRATES_V1: [u32; 14] = [
    32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
];
const BITRATES_V2: [u32; 14] = [8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160];
const BITRATES_L1_V1: [u32; 14] = [
    32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448,
];
const BITRATES_L1_V2: [u32; 14] = [
    32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256,
];
const BITRATES_L2_V1: [u32; 14] = [
    32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384,
];

#[derive(Debug, Clone, PartialEq)]
pub struct Mp3Info {
    /// Audio frames of the stream (the tag frame is not one).
    pub frames: u64,
    pub samples_per_frame: u32,
    pub sample_rate: u32,
    pub bitrate_mode: BitrateMode,
    pub declared_bitrate_kbps: Option<u32>,
    pub encoder: Option<String>,
    pub encoder_lowpass_hz: Option<u32>,
    /// Byte offset of the first frame header (the tag frame, when there is one).
    pub first_frame_pos: u64,
    /// Encoder delay and padding from the LAME extension, in samples.
    pub delay_padding: Option<(u32, u32)>,
}

impl Mp3Info {
    /// Samples fewer than the declared length the decoder may legitimately return: it trims the
    /// encoder delay and padding the LAME tag declares (plus two frames of slack), or up to three
    /// frames when the stream has no such fields.
    pub fn length_tolerance(&self) -> u64 {
        let frame = u64::from(self.samples_per_frame);
        match self.delay_padding {
            Some((delay, padding)) => u64::from(delay) + u64::from(padding) + 2 * frame,
            None => 3 * frame,
        }
    }

    pub fn duration_ms(&self) -> u64 {
        self.frames * u64::from(self.samples_per_frame) * 1000 / u64::from(self.sample_rate.max(1))
    }
}

#[derive(Debug, Clone, Copy)]
struct Header {
    /// 1 for MPEG-1, 2 for MPEG-2 and 25 for MPEG-2.5.
    version: u8,
    /// 1 to 3 for Layer I to III.
    layer: u8,
    /// 0 for free format.
    bitrate_kbps: u32,
    bitrate_index: u8,
    sample_rate: u32,
    padding: u32,
    mono: bool,
}

impl Header {
    fn parse(b: [u8; 4]) -> Option<Header> {
        if b[0] != 0xFF || b[1] & 0xE0 != 0xE0 {
            return None;
        }
        let version = match (b[1] >> 3) & 3 {
            0 => 25,
            2 => 2,
            3 => 1,
            _ => return None,
        };
        let layer = match (b[1] >> 1) & 3 {
            1 => 3,
            2 => 2,
            3 => 1,
            _ => return None,
        };
        let bitrate_index = b[2] >> 4;
        if bitrate_index == 15 {
            return None;
        }
        let rates: [u32; 3] = match version {
            1 => [44_100, 48_000, 32_000],
            2 => [22_050, 24_000, 16_000],
            _ => [11_025, 12_000, 8_000],
        };
        let sample_rate = *rates.get(usize::from((b[2] >> 2) & 3))?;
        let table = match (layer, version) {
            (1, 1) => &BITRATES_L1_V1,
            (1, _) => &BITRATES_L1_V2,
            (2, 1) => &BITRATES_L2_V1,
            (3, 1) => &BITRATES_V1,
            _ => &BITRATES_V2,
        };
        let bitrate_kbps = match bitrate_index {
            0 => 0,
            i => *table.get(usize::from(i) - 1)?,
        };
        Some(Header {
            version,
            layer,
            bitrate_kbps,
            bitrate_index,
            sample_rate,
            padding: u32::from((b[2] >> 1) & 1),
            mono: (b[3] >> 6) == 3,
        })
    }

    fn samples_per_frame(&self) -> u32 {
        match (self.layer, self.version) {
            (1, _) => 384,
            (3, v) if v != 1 => 576,
            _ => 1152,
        }
    }

    /// Frame length in bytes; `None` for a free-format stream.
    fn frame_len(&self) -> Option<u64> {
        if self.bitrate_kbps == 0 {
            return None;
        }
        // Layer I counts 4-byte slots: 12 per kbps-second; Layers II and III count bytes.
        let coefficient = if self.layer == 1 {
            12
        } else {
            u64::from(self.samples_per_frame() / 8)
        };
        let base = coefficient * u64::from(self.bitrate_kbps) * 1000 / u64::from(self.sample_rate);
        Some(if self.layer == 1 {
            (base + u64::from(self.padding)) * 4
        } else {
            base + u64::from(self.padding)
        })
    }

    /// Offset of the Xing/Info field inside the tag frame.
    fn xing_offset(&self) -> usize {
        let side = match (self.version, self.mono) {
            (1, true) => 17,
            (1, false) => 32,
            (_, true) => 9,
            (_, false) => 17,
        };
        // The CRC word, when present, is not counted: the tag sits after the side information.
        4 + side
    }

    /// Same stream parameters, so that a later header is a frame of the same stream.
    fn same_stream(&self, other: &Header) -> bool {
        self.version == other.version
            && self.layer == other.layer
            && self.sample_rate == other.sample_rate
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TagKind {
    Info,
    Xing,
    Vbri,
}

#[derive(Debug)]
struct Tag {
    kind: TagKind,
    frames: Option<u32>,
    bytes: Option<u32>,
    encoder: Option<String>,
    vbr_method: Option<u8>,
    lowpass_hz: Option<u32>,
    delay_padding: Option<(u32, u32)>,
}

fn be32(b: &[u8]) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(..4)?.try_into().ok()?))
}

impl Tag {
    fn parse(frame: &[u8], header: &Header) -> Option<Tag> {
        let off = header.xing_offset();
        if let Some(id) = frame.get(off..off + 4) {
            let kind = match id {
                b"Xing" => Some(TagKind::Xing),
                b"Info" => Some(TagKind::Info),
                _ => None,
            };
            if let Some(kind) = kind {
                return Some(Tag::parse_xing(kind, frame.get(off + 4..)?));
            }
        }
        // VBRI sits at a fixed offset of 32 bytes after the header.
        if frame.get(36..40) == Some(b"VBRI") {
            return Some(Tag {
                kind: TagKind::Vbri,
                bytes: frame.get(46..).and_then(be32),
                frames: frame.get(50..).and_then(be32),
                encoder: None,
                vbr_method: None,
                lowpass_hz: None,
                delay_padding: None,
            });
        }
        None
    }

    fn parse_xing(kind: TagKind, mut rest: &[u8]) -> Tag {
        let flags = be32(rest).unwrap_or(0);
        rest = rest.get(4..).unwrap_or(&[]);
        let mut take = |n: usize| -> Option<&[u8]> {
            let field = rest.get(..n)?;
            rest = &rest[n..];
            Some(field)
        };
        let frames = if flags & 1 != 0 {
            take(4).and_then(be32)
        } else {
            None
        };
        let bytes = if flags & 2 != 0 {
            take(4).and_then(be32)
        } else {
            None
        };
        if flags & 4 != 0 {
            let _ = take(100);
        }
        if flags & 8 != 0 {
            let _ = take(4);
        }
        // LAME extension: 9 bytes of encoder, the VBR method, the low-pass byte.
        let mut tag = Tag {
            kind,
            frames,
            bytes,
            encoder: None,
            vbr_method: None,
            lowpass_hz: None,
            delay_padding: None,
        };
        if let Some(ext) = rest.get(..11) {
            let printable = |b: &u8| (0x20..0x7F).contains(b);
            let field = &ext[..9];
            if field[..4].iter().all(printable) && field.iter().all(|b| printable(b) || *b == 0) {
                let text: String = field.iter().map(|b| char::from(*b)).collect();
                tag.encoder = Some(text.trim_end_matches([' ', '\0']).to_string());
                tag.vbr_method = Some(ext[9] & 0x0F);
                tag.lowpass_hz = (ext[10] != 0).then(|| u32::from(ext[10]) * 100);
                // Bytes 21 to 23 of the extension: two 12-bit fields, delay then padding.
                // Only for the encoders whose fields these are (the decoder trims for them alone).
                let known = ["LAME", "Lavf", "Lavc"].iter().any(|p| text.starts_with(p));
                if let (true, Some(&[b0, b1, b2])) = (known, rest.get(21..24)) {
                    tag.delay_padding = Some((
                        (u32::from(b0) << 4) | (u32::from(b1) >> 4),
                        (u32::from(b1 & 0x0F) << 8) | u32::from(b2),
                    ));
                }
            }
        }
        tag
    }
}

/// Totals of the audio frames from a position to the end of the file.
#[derive(Debug, Default)]
struct Walk {
    frames: u64,
    bytes: u64,
    first_bitrate_kbps: Option<u32>,
    first_bitrate_index: Option<u8>,
    same_bitrate: bool,
    free_format: bool,
}

fn io_error(e: std::io::Error) -> TrackError {
    if e.kind() == std::io::ErrorKind::UnexpectedEof {
        TrackError::CorruptStream(e.to_string())
    } else {
        TrackError::Unreadable(e.to_string())
    }
}

fn read_header(r: &mut BufReader<File>) -> Result<Option<[u8; 4]>, TrackError> {
    let mut b = [0u8; 4];
    match r.read_exact(&mut b) {
        Ok(()) => Ok(Some(b)),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(None),
        Err(e) => Err(io_error(e)),
    }
}

/// Position after the ID3v2 tags at the start of the file (0 when there is none); stacked tags are
/// all skipped.
fn skip_id3v2(r: &mut BufReader<File>, file_len: u64) -> Result<u64, TrackError> {
    let mut pos = 0u64;
    while pos.saturating_add(10) <= file_len {
        r.seek(SeekFrom::Start(pos)).map_err(io_error)?;
        let mut b = [0u8; 10];
        if r.read_exact(&mut b).is_err() || &b[..3] != b"ID3" {
            break;
        }
        let size = b[6..10]
            .iter()
            .fold(0u64, |acc, v| (acc << 7) | u64::from(v & 0x7F));
        let footer = if b[5] & 0x10 != 0 { 10 } else { 0 };
        pos += 10 + size + footer;
    }
    Ok(pos)
}

pub fn read(path: &Path) -> Result<Mp3Info, TrackError> {
    let file = File::open(path).map_err(|e| TrackError::Unreadable(e.to_string()))?;
    let file_len = file
        .metadata()
        .map_err(|e| TrackError::Unreadable(e.to_string()))?
        .len();
    let mut r = BufReader::new(file);

    // First frame: scan for a sync after the ID3v2 tag.
    let start = skip_id3v2(&mut r, file_len)?;
    let (first_pos, first) = find_first_frame(&mut r, start, file_len)?
        .ok_or_else(|| TrackError::CorruptStream("no MP3 frame found".into()))?;
    if first.layer != 3 {
        return Err(TrackError::UnsupportedFormat(format!(
            "MPEG audio layer {} is not supported",
            "I".repeat(usize::from(first.layer))
        )));
    }

    // Tag frame (not audio).
    let tag = match first.frame_len() {
        Some(len) => {
            let len = len.min(file_len.saturating_sub(first_pos)) as usize;
            r.seek(SeekFrom::Start(first_pos)).map_err(io_error)?;
            let mut frame = vec![0u8; len];
            r.read_exact(&mut frame).map_err(io_error)?;
            Tag::parse(&frame, &first)
        }
        None => None,
    };
    let audio_start = match (&tag, first.frame_len()) {
        (Some(_), Some(len)) => first_pos + len,
        _ => first_pos,
    };

    let mode = match &tag {
        Some(t) => match t.kind {
            TagKind::Info => Some(BitrateMode::Cbr),
            TagKind::Xing => Some(match t.vbr_method {
                Some(1 | 8) => BitrateMode::Cbr,
                Some(2 | 9) => BitrateMode::Abr,
                _ => BitrateMode::Vbr,
            }),
            TagKind::Vbri => Some(BitrateMode::Vbr),
        },
        None => None,
    };

    // The walk counts the audio frames; it is skipped when the tag gives everything needed.
    let needs_walk = match (&tag, mode) {
        (Some(t), Some(BitrateMode::Cbr)) => t.frames.is_none(),
        (Some(t), _) => t.frames.is_none() || t.bytes.is_none(),
        (None, _) => true,
    };
    let walk = if needs_walk {
        Some(walk_frames(&mut r, audio_start, file_len, &first)?)
    } else {
        None
    };
    // CBR with a complete tag: the first audio frame's header.
    let first_audio_bitrate = match &walk {
        Some(w) => w.first_bitrate_kbps,
        None => {
            r.seek(SeekFrom::Start(audio_start)).map_err(io_error)?;
            read_header(&mut r)?
                .and_then(Header::parse)
                .filter(|h| h.same_stream(&first) && h.bitrate_kbps != 0)
                .map(|h| h.bitrate_kbps)
        }
    };

    let mode = mode.unwrap_or(match &walk {
        Some(w) if !w.free_format && w.same_bitrate => BitrateMode::Cbr,
        _ => BitrateMode::Vbr,
    });
    let frames = tag
        .as_ref()
        .and_then(|t| t.frames)
        .map(u64::from)
        .or_else(|| walk.as_ref().map(|w| w.frames))
        .unwrap_or(0);
    let spf = first.samples_per_frame();
    let declared_bitrate_kbps = match mode {
        BitrateMode::Cbr => first_audio_bitrate,
        BitrateMode::Vbr | BitrateMode::Abr => {
            let bytes = tag
                .as_ref()
                .and_then(|t| t.bytes)
                .map(u64::from)
                .or_else(|| walk.as_ref().map(|w| w.bytes))
                .unwrap_or(0);
            let seconds = frames as f64 * f64::from(spf) / f64::from(first.sample_rate);
            (seconds > 0.0 && bytes > 0)
                .then(|| (8.0 * bytes as f64 / seconds / 1000.0).round() as u32)
        }
    };
    Ok(Mp3Info {
        frames,
        samples_per_frame: spf,
        sample_rate: first.sample_rate,
        bitrate_mode: mode,
        declared_bitrate_kbps,
        encoder: tag.as_ref().and_then(|t| t.encoder.clone()),
        encoder_lowpass_hz: tag.as_ref().and_then(|t| t.lowpass_hz),
        first_frame_pos: first_pos,
        delay_padding: tag.as_ref().and_then(|t| t.delay_padding),
    })
}

/// The first valid frame header at or after `start`, confirmed by a sync at the next frame when
/// the file is long enough to have one. The scan runs over an in-memory window.
fn find_first_frame(
    r: &mut BufReader<File>,
    start: u64,
    file_len: u64,
) -> Result<Option<(u64, Header)>, TrackError> {
    // The window reaches past the scan limit by more than the longest frame, for the confirmation.
    r.seek(SeekFrom::Start(start)).map_err(io_error)?;
    let mut window = Vec::new();
    r.by_ref()
        .take(SYNC_SEARCH_LIMIT + 8192)
        .read_to_end(&mut window)
        .map_err(io_error)?;
    let scan_end = window.len().min(SYNC_SEARCH_LIMIT as usize);
    for i in 0..scan_end {
        let Some(h) = window
            .get(i..i + 4)
            .and_then(|b| <[u8; 4]>::try_from(b).ok())
            .and_then(Header::parse)
        else {
            continue;
        };
        let pos = start + i as u64;
        let confirmed = match h.frame_len() {
            Some(len) if pos + len + 4 <= file_len => {
                let at = i + len as usize;
                window
                    .get(at..at + 4)
                    .and_then(|b| <[u8; 4]>::try_from(b).ok())
                    .and_then(Header::parse)
                    .is_some_and(|n| n.same_stream(&h))
            }
            _ => true,
        };
        if confirmed {
            return Ok(Some((pos, h)));
        }
    }
    Ok(None)
}

/// Counts the frames of the same stream from `start`, resynchronising over junk (tags, padding).
fn walk_frames(
    r: &mut BufReader<File>,
    start: u64,
    file_len: u64,
    first: &Header,
) -> Result<Walk, TrackError> {
    let mut walk = Walk {
        same_bitrate: true,
        ..Walk::default()
    };
    let mut pos = start;
    r.seek(SeekFrom::Start(pos)).map_err(io_error)?;
    while let Some(bytes) = read_header(r)? {
        let header = Header::parse(bytes).filter(|h| h.same_stream(first));
        let Some(h) = header else {
            pos += 1;
            r.seek_relative(-3).map_err(io_error)?;
            continue;
        };
        let Some(len) = h.frame_len() else {
            walk.free_format = true;
            break;
        };
        if pos + len > file_len {
            break;
        }
        walk.frames += 1;
        walk.bytes += len;
        match walk.first_bitrate_index {
            None => {
                walk.first_bitrate_index = Some(h.bitrate_index);
                walk.first_bitrate_kbps = Some(h.bitrate_kbps);
            }
            Some(i) if i != h.bitrate_index => walk.same_bitrate = false,
            Some(_) => {}
        }
        pos += len;
        r.seek_relative(len as i64 - 4).map_err(io_error)?;
    }
    Ok(walk)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file under `target/unit-tmp` removed when dropped.
    struct TempFile(std::path::PathBuf);

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn write(name: &str, bytes: &[u8]) -> TempFile {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/unit-tmp");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("mp3-{}-{name}.mp3", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        TempFile(path)
    }

    fn frames(header: [u8; 4], len: usize, count: usize) -> Vec<u8> {
        let mut out = Vec::new();
        for _ in 0..count {
            out.extend_from_slice(&header);
            out.resize(out.len() + len - 4, 0);
        }
        out
    }

    #[test]
    fn layer_ii_is_unsupported_not_corrupt() {
        // MPEG-1 Layer II, 192 kbps, 44.1 kHz: 144 * 192000 / 44100 = 626 bytes.
        let path = write("l2", &frames([0xFF, 0xFD, 0xA0, 0x00], 626, 12));
        assert!(matches!(
            read(&path.0),
            Err(TrackError::UnsupportedFormat(_))
        ));
    }

    #[test]
    fn layer_i_is_unsupported_not_corrupt() {
        // MPEG-1 Layer I, 256 kbps, 44.1 kHz: (12 * 256000 / 44100) * 4 = 276 bytes.
        let path = write("l1", &frames([0xFF, 0xFF, 0x80, 0x00], 276, 12));
        assert!(matches!(
            read(&path.0),
            Err(TrackError::UnsupportedFormat(_))
        ));
    }

    #[test]
    fn stacked_id3v2_tags_are_skipped() {
        let mut bytes = Vec::new();
        for _ in 0..2 {
            bytes.extend_from_slice(b"ID3\x04\0\0\0\0\0\x04");
            bytes.extend_from_slice(&[0u8; 4]);
        }
        bytes.extend_from_slice(&frames([0xFF, 0xFB, 0x90, 0x00], 417, 6));
        let path = write("id3", &bytes);
        let info = read(&path.0).unwrap();
        assert_eq!(info.frames, 6);
        assert_eq!(info.declared_bitrate_kbps, Some(128));
    }

    #[test]
    fn long_unpadded_layer_i_is_unsupported_not_corrupt() {
        // MPEG-1 Layer I, 128 kbps, 44.1 kHz, unpadded: (12 * 128000 / 44100) * 4 = 136 bytes.
        // The file is longer than the scan window, so the confirming frame is read from it.
        let count = (SYNC_SEARCH_LIMIT as usize + 8192) / 136 + 16;
        let path = write("l1long", &frames([0xFF, 0xFF, 0x40, 0x00], 136, count));
        assert!(matches!(
            read(&path.0),
            Err(TrackError::UnsupportedFormat(_))
        ));
    }

    fn info_frame(header: [u8; 4], encoder: &[u8; 9]) -> Vec<u8> {
        let mut tag = header.to_vec();
        tag.resize(417, 0);
        tag[36..40].copy_from_slice(b"Info");
        tag[40..44].copy_from_slice(&1u32.to_be_bytes());
        tag[44..48].copy_from_slice(&5u32.to_be_bytes());
        tag[48..57].copy_from_slice(encoder);
        tag[57] = 0;
        tag[58] = 180; // 18 kHz
        tag[69..72].copy_from_slice(&[0x24, 0x09, 0x00]);
        tag.extend(frames([0xFF, 0xFB, 0x90, 0x00], 417, 5));
        tag
    }

    #[test]
    fn crc_protected_lame_tag_frame_is_recognised() {
        let path = write("crc", &info_frame([0xFF, 0xFA, 0x90, 0x00], b"LAME3.99r"));
        let info = read(&path.0).unwrap();
        assert_eq!(info.encoder.as_deref(), Some("LAME3.99r"));
        assert_eq!(info.encoder_lowpass_hz, Some(18_000));
        assert_eq!(info.bitrate_mode, BitrateMode::Cbr);
        assert_eq!(info.frames, 5);
        assert_eq!(info.delay_padding, Some((576, 2304)));
    }

    #[test]
    fn non_lame_encoder_has_no_delay_and_padding() {
        let path = write("other", &info_frame([0xFF, 0xFB, 0x90, 0x00], b"Other 1.0"));
        let info = read(&path.0).unwrap();
        assert_eq!(info.encoder.as_deref(), Some("Other 1.0"));
        assert_eq!(info.delay_padding, None);
        assert_eq!(info.length_tolerance(), 3 * 1152);
    }

    #[test]
    fn lame_delay_and_padding_are_read() {
        let mut tag = vec![0xFF, 0xFB, 0x90, 0x00];
        tag.resize(417, 0);
        tag[36..40].copy_from_slice(b"Info");
        tag[40..44].copy_from_slice(&1u32.to_be_bytes());
        tag[44..48].copy_from_slice(&5u32.to_be_bytes());
        tag[48..57].copy_from_slice(b"LAME3.99r");
        // 576 = 0x240 and 2304 = 0x900.
        tag[69..72].copy_from_slice(&[0x24, 0x09, 0x00]);
        tag.extend(frames([0xFF, 0xFB, 0x90, 0x00], 417, 5));
        let path = write("lame", &tag);
        let info = read(&path.0).unwrap();
        assert_eq!(info.delay_padding, Some((576, 2304)));
        assert_eq!(info.length_tolerance(), 576 + 2304 + 2 * 1152);
    }
}
