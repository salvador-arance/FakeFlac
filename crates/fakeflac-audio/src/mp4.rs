//! M4A stream information read by the adapter itself (ADR-0040, `specs/detection/lossy-source.md`
//! section 2.2): the sample entry of the first audio track, the AudioSpecificConfig of AAC, the ALAC
//! magic cookie and the sample sizes. No tag library. Read-only (RNF-05).

use fakeflac_core::{AudioCodec, TrackError};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// A `moov` box larger than this is not read (a real one is a few MiB at most).
const MAX_MOOV_BYTES: u64 = 256 << 20;

#[derive(Debug, Clone, PartialEq)]
pub struct Mp4Info {
    pub codec: AudioCodec,
    /// ALAC only (from the magic cookie).
    pub bit_depth: Option<u8>,
    /// AAC only: `round(8 · Σ sample sizes / duration / 1000)`.
    pub declared_bitrate_kbps: Option<u32>,
}

fn corrupt(what: &str) -> TrackError {
    TrackError::CorruptStream(what.to_string())
}

fn io_error(e: std::io::Error) -> TrackError {
    if e.kind() == std::io::ErrorKind::UnexpectedEof {
        TrackError::CorruptStream(e.to_string())
    } else {
        TrackError::Unreadable(e.to_string())
    }
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn be64(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_be_bytes(b.get(at..at + 8)?.try_into().ok()?))
}

/// A box: `(type, payload)`.
type BoxEntry<'a> = ([u8; 4], &'a [u8]);

/// The boxes of a byte range.
fn boxes(mut data: &[u8]) -> Result<Vec<BoxEntry<'_>>, TrackError> {
    let mut out = Vec::new();
    while !data.is_empty() {
        let size = be32(data, 0).ok_or_else(|| corrupt("truncated box header"))? as usize;
        let kind: [u8; 4] = data
            .get(4..8)
            .and_then(|k| k.try_into().ok())
            .ok_or_else(|| corrupt("truncated box header"))?;
        let (header, size) = match size {
            0 => (8, data.len()),
            1 => (
                16,
                be64(data, 8).ok_or_else(|| corrupt("truncated box header"))? as usize,
            ),
            n => (8, n),
        };
        if size < header || size > data.len() {
            return Err(corrupt("box larger than its container"));
        }
        out.push((kind, &data[header..size]));
        data = &data[size..];
    }
    Ok(out)
}

fn find<'a>(list: &[BoxEntry<'a>], kind: &[u8; 4]) -> Option<&'a [u8]> {
    list.iter().find(|(k, _)| k == kind).map(|(_, p)| *p)
}

pub fn read(path: &Path) -> Result<Mp4Info, TrackError> {
    let mut file = File::open(path).map_err(|e| TrackError::Unreadable(e.to_string()))?;
    let file_len = file
        .metadata()
        .map_err(|e| TrackError::Unreadable(e.to_string()))?
        .len();
    let mut pos = 0u64;
    while pos.saturating_add(8) <= file_len {
        file.seek(SeekFrom::Start(pos)).map_err(io_error)?;
        let mut head = [0u8; 16];
        let got = file.read(&mut head).map_err(io_error)?;
        let size32 = u64::from(be32(&head, 0).ok_or_else(|| corrupt("truncated box header"))?);
        let kind = &head[4..8];
        let (header, size) = match size32 {
            0 => (8, file_len - pos),
            1 => (
                16,
                be64(&head[..got], 8).ok_or_else(|| corrupt("truncated box header"))?,
            ),
            n => (8, n),
        };
        if size < header {
            return Err(corrupt("invalid box size"));
        }
        let end = pos
            .checked_add(size)
            .filter(|e| *e <= file_len)
            .ok_or_else(|| corrupt("box larger than the file"))?;
        if kind == b"moov" {
            if size > MAX_MOOV_BYTES {
                return Err(corrupt("moov box too large"));
            }
            file.seek(SeekFrom::Start(pos + header)).map_err(io_error)?;
            let mut moov = vec![0u8; (size - header) as usize];
            file.read_exact(&mut moov).map_err(io_error)?;
            return parse_moov(&moov);
        }
        pos = end;
    }
    Err(corrupt("no moov box"))
}

fn parse_moov(moov: &[u8]) -> Result<Mp4Info, TrackError> {
    let top = boxes(moov)?;
    for (kind, trak) in &top {
        if kind != b"trak" {
            continue;
        }
        let trak = boxes(trak)?;
        let Some(mdia) = find(&trak, b"mdia") else {
            continue;
        };
        let mdia = boxes(mdia)?;
        let is_audio = find(&mdia, b"hdlr").is_some_and(|h| h.get(8..12) == Some(b"soun"));
        if is_audio {
            return parse_audio_track(&mdia);
        }
    }
    Err(TrackError::UnsupportedFormat("no audio track".into()))
}

fn parse_audio_track(mdia: &[BoxEntry<'_>]) -> Result<Mp4Info, TrackError> {
    let mdhd = find(mdia, b"mdhd").ok_or_else(|| corrupt("no mdhd box"))?;
    let (timescale, duration) = match mdhd.first() {
        Some(0) => (be32(mdhd, 12), be32(mdhd, 16).map(u64::from)),
        Some(1) => (be32(mdhd, 20), be64(mdhd, 24)),
        _ => (None, None),
    };
    let minf = boxes(find(mdia, b"minf").ok_or_else(|| corrupt("no minf box"))?)?;
    let stbl = boxes(find(&minf, b"stbl").ok_or_else(|| corrupt("no stbl box"))?)?;
    let stsd = find(&stbl, b"stsd").ok_or_else(|| corrupt("no stsd box"))?;
    // version and flags, entry count, then the sample entries.
    let entries = boxes(stsd.get(8..).ok_or_else(|| corrupt("truncated stsd box"))?)?;
    let (kind, entry) = entries
        .first()
        .ok_or_else(|| corrupt("stsd box without sample entry"))?;
    let children = sample_entry_children(entry)?;
    match kind {
        b"mp4a" => {
            let esds = find(&children, b"esds").or_else(|| {
                // QuickTime files wrap it in a `wave` box.
                let wave = boxes(find(&children, b"wave")?).ok()?;
                find(&wave, b"esds")
            });
            let esds = esds.ok_or_else(|| corrupt("mp4a sample entry without esds"))?;
            check_aac_lc(esds)?;
            let sizes = find(&stbl, b"stsz").and_then(sample_size_total);
            let declared_bitrate_kbps = match (sizes, timescale, duration) {
                (Some(bytes), Some(ts), Some(d)) if ts > 0 && d > 0 => {
                    let seconds = d as f64 / f64::from(ts);
                    Some((8.0 * bytes as f64 / seconds / 1000.0).round() as u32)
                }
                _ => None,
            };
            Ok(Mp4Info {
                codec: AudioCodec::Aac,
                bit_depth: None,
                declared_bitrate_kbps,
            })
        }
        b"alac" => {
            // The cookie is the payload of the inner `alac` full box: 4 bytes of version and flags,
            // then frame length (4), compatible version (1), bit depth (1), ...
            // QuickTime files wrap it in a `wave` box. Without a cookie the depth is the sample
            // entry's sample size, or unknown (the decoder's own value is used then).
            let cookie = find(&children, b"alac").or_else(|| {
                let wave = boxes(find(&children, b"wave")?).ok()?;
                find(&wave, b"alac")
            });
            let bit_depth = match cookie {
                Some(cookie) => Some(*cookie.get(9).ok_or_else(|| corrupt("short ALAC cookie"))?),
                None => entry
                    .get(18..20)
                    .map(|v| u16::from_be_bytes([v[0], v[1]]))
                    .filter(|d| (1..=32).contains(d))
                    .map(|d| d as u8),
            };
            Ok(Mp4Info {
                codec: AudioCodec::Alac,
                bit_depth,
                declared_bitrate_kbps: None,
            })
        }
        other => Err(TrackError::UnsupportedFormat(format!(
            "sample entry {}",
            String::from_utf8_lossy(other)
        ))),
    }
}

/// Child boxes of an audio sample entry (ISO/IEC 14496-12 12.2.3; QuickTime versions 1 and 2 add
/// fields before them).
fn sample_entry_children(entry: &[u8]) -> Result<Vec<BoxEntry<'_>>, TrackError> {
    let version = entry
        .get(8..10)
        .map(|v| u16::from_be_bytes([v[0], v[1]]))
        .ok_or_else(|| corrupt("short sample entry"))?;
    let skip = 28
        + match version {
            1 => 16,
            2 => 36,
            _ => 0,
        };
    boxes(
        entry
            .get(skip..)
            .ok_or_else(|| corrupt("short sample entry"))?,
    )
}

/// Total of the sample sizes of an `stsz` box.
fn sample_size_total(stsz: &[u8]) -> Option<u64> {
    let fixed = u64::from(be32(stsz, 4)?);
    let count = be32(stsz, 8)? as usize;
    if fixed != 0 {
        return Some(fixed * count as u64);
    }
    let table = stsz.get(12..12 + count.checked_mul(4)?)?;
    Some(
        table
            .chunks_exact(4)
            .map(|c| u64::from(u32::from_be_bytes([c[0], c[1], c[2], c[3]])))
            .sum(),
    )
}

/// One MPEG-4 descriptor: tag, payload and the rest of the data.
fn descriptor(data: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let tag = *data.first()?;
    let mut len = 0usize;
    let mut at = 1;
    loop {
        let b = *data.get(at)?;
        at += 1;
        len = (len << 7) | usize::from(b & 0x7F);
        if b & 0x80 == 0 || at > 4 {
            break;
        }
    }
    let payload = data.get(at..at + len)?;
    Some((tag, payload, &data[at + len..]))
}

/// Checks the AudioSpecificConfig of an `esds` box: only AAC-LC (object type 2) is decoded.
fn check_aac_lc(esds: &[u8]) -> Result<(), TrackError> {
    let bad = || corrupt("malformed esds box");
    let (tag, es, _) = descriptor(esds.get(4..).ok_or_else(bad)?).ok_or_else(bad)?;
    if tag != 0x03 {
        return Err(bad());
    }
    // ES_ID (2), flags (1) and the optional fields they announce.
    let flags = *es.get(2).ok_or_else(bad)?;
    let mut at = 3;
    if flags & 0x80 != 0 {
        at += 2;
    }
    if flags & 0x40 != 0 {
        at += 1 + usize::from(*es.get(at).ok_or_else(bad)?);
    }
    if flags & 0x20 != 0 {
        at += 2;
    }
    let (tag, config, _) = descriptor(es.get(at..).ok_or_else(bad)?).ok_or_else(bad)?;
    if tag != 0x04 {
        return Err(bad());
    }
    // object type, stream type, buffer size (3), max and average bitrate, then the specific info.
    match config.first() {
        // MPEG-4 audio, and MPEG-2 AAC Main, LC and SSR: the AudioSpecificConfig decides.
        Some(0x40 | 0x66 | 0x67 | 0x68) => {}
        Some(other) => {
            return Err(TrackError::UnsupportedFormat(format!(
                "audio object type indication {other:#04x} is not AAC"
            )))
        }
        None => return Err(bad()),
    }
    let (tag, asc, _) = descriptor(config.get(13..).ok_or_else(bad)?).ok_or_else(bad)?;
    if tag != 0x05 {
        return Err(bad());
    }
    let mut object_type = u32::from(*asc.first().ok_or_else(bad)? >> 3);
    if object_type == 31 {
        let b0 = *asc.first().ok_or_else(bad)?;
        let b1 = *asc.get(1).ok_or_else(bad)?;
        object_type = 32 + u32::from(((b0 & 7) << 3) | (b1 >> 5));
    }
    match object_type {
        2 => Ok(()),
        5 | 29 => Err(TrackError::UnsupportedFormat(
            "HE-AAC is not supported".into(),
        )),
        n => Err(TrackError::UnsupportedFormat(format!(
            "AAC object type {n} is not supported"
        ))),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
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
        let path = dir.join(format!("mp4-{}-{name}.m4a", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        TempFile(path)
    }

    fn ftyp() -> Vec<u8> {
        let mut v = 16u32.to_be_bytes().to_vec();
        v.extend_from_slice(b"ftyp");
        v.extend_from_slice(b"M4A \0\0\0\0");
        v
    }

    fn with_large_box(largesize: u64) -> Vec<u8> {
        let mut v = ftyp();
        v.extend_from_slice(&1u32.to_be_bytes());
        v.extend_from_slice(b"free");
        v.extend_from_slice(&largesize.to_be_bytes());
        v
    }

    #[test]
    fn wrapping_largesize_is_corrupt_not_a_loop() {
        // pos (16) + largesize wraps to 0.
        let path = write("wrap", &with_large_box(u64::MAX - 15));
        assert!(matches!(read(&path.0), Err(TrackError::CorruptStream(_))));
    }

    #[test]
    fn maximal_largesize_is_corrupt_not_a_panic() {
        let path = write("max", &with_large_box(u64::MAX));
        assert!(matches!(read(&path.0), Err(TrackError::CorruptStream(_))));
    }

    #[test]
    fn box_larger_than_the_file_is_corrupt() {
        let path = write("big", &with_large_box(1 << 40));
        assert!(matches!(read(&path.0), Err(TrackError::CorruptStream(_))));
    }

    #[test]
    fn zero_size_box_runs_to_the_end_of_the_file() {
        let mut v = ftyp();
        v.extend_from_slice(&0u32.to_be_bytes());
        v.extend_from_slice(b"mdat");
        v.extend_from_slice(&[0u8; 32]);
        let path = write("zero", &v);
        assert!(matches!(read(&path.0), Err(TrackError::CorruptStream(_))));
    }

    /// esds: version/flags, ES descriptor, DecoderConfig (`object_type`), ASC descriptor.
    fn esds(object_type: u8, asc_first: u8) -> Vec<u8> {
        let asc = [0x05, 0x02, asc_first, 0x10];
        let mut config = vec![object_type, 0x15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        config.extend_from_slice(&asc);
        let mut es = vec![0, 1, 0];
        es.push(0x04);
        es.push(config.len() as u8);
        es.extend_from_slice(&config);
        let mut esds = vec![0, 0, 0, 0, 0x03, es.len() as u8];
        esds.extend_from_slice(&es);
        esds
    }

    #[test]
    fn esds_with_a_non_aac_object_type_is_unsupported() {
        assert!(matches!(
            check_aac_lc(&esds(0x6B, 0x12)),
            Err(TrackError::UnsupportedFormat(_))
        ));
    }

    #[test]
    fn esds_with_an_mpeg2_aac_indication_defers_to_the_audio_object_type() {
        // AAC LC (object type 2: first ASC byte 0x12) is accepted under 0x67.
        assert!(check_aac_lc(&esds(0x67, 0x12)).is_ok());
        // SBR (object type 5: 0x2A) is still HE-AAC.
        assert!(matches!(
            check_aac_lc(&esds(0x40, 0x2A)),
            Err(TrackError::UnsupportedFormat(_))
        ));
    }
}
