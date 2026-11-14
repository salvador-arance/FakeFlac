//! Ogg Opus (ADR-0047; lossy-source.md §2.4). Validate the complete container before
//! decoding, retaining only counters and headers. A second pass decodes completed packets
//! in order without an index that grows with the track (RNF-02, memory.md §8).
use fakeflac_core::TrackError;
use rusty_opus::OpusDecoder;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const RATE: u32 = 48_000;
// RFC 6716: at most 120 ms in one packet, at the fixed Opus output rate.
const MAX_FRAMES: usize = 5760;

fn corrupt(text: impl Into<String>) -> TrackError {
    TrackError::CorruptStream(text.into())
}
fn read_exact(file: &mut File, bytes: &mut [u8]) -> Result<(), TrackError> {
    file.read_exact(bytes).map_err(|e| {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            corrupt("truncated Ogg page")
        } else {
            TrackError::Unreadable(e.to_string())
        }
    })
}
fn u32le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}
fn crc(bytes: impl Iterator<Item = u8>) -> u32 {
    bytes.fold(0u32, |mut value, byte| {
        value ^= u32::from(byte) << 24;
        for _ in 0..8 {
            value = if value & 0x8000_0000 != 0 {
                (value << 1) ^ 0x04c1_1db7
            } else {
                value << 1
            };
        }
        value
    })
}

pub struct OpusStream {
    file: File,
    audio_start: u64,
    packet_count: usize,
    pub channels: u8,
    pre_skip: u64,
    pub length: u64,
    gain: f32,
    pub bitrate: Option<u32>,
}

fn head(bytes: &[u8]) -> Result<(u8, u64, f32), TrackError> {
    if !bytes.starts_with(b"OpusHead") {
        return Err(corrupt("no OpusHead"));
    }
    if bytes.len() < 19 || bytes[8] & 0xf0 != 0 || bytes[9] == 0 {
        return Err(corrupt("invalid OpusHead"));
    }
    let family = bytes[18];
    if family == 0 && bytes[9] > 2 {
        return Err(corrupt("invalid OpusHead"));
    }
    if family != 0 {
        return Err(TrackError::UnsupportedFormat(format!(
            "channel mapping family {family} is not supported"
        )));
    }
    let skip = u16::from_le_bytes([bytes[10], bytes[11]]);
    let gain = i16::from_le_bytes([bytes[16], bytes[17]]);
    Ok((
        bytes[9],
        u64::from(skip),
        10f64.powf(f64::from(gain) / 5120.0) as f32,
    ))
}

fn tags(bytes: &[u8]) -> Result<(), TrackError> {
    if !bytes.starts_with(b"OpusTags") {
        return Err(corrupt("no OpusTags"));
    }
    let mut pos = 8usize;
    let mut number = || {
        let end = pos
            .checked_add(4)
            .ok_or_else(|| corrupt("invalid OpusTags"))?;
        let value = bytes
            .get(pos..end)
            .ok_or_else(|| corrupt("invalid OpusTags"))?;
        pos = end;
        Ok::<_, TrackError>(u32le(value) as usize)
    };
    let vendor = number()?;
    // Lengths are checked before advancing; no allocation comes from a tag length.
    pos = pos
        .checked_add(vendor)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| corrupt("invalid OpusTags"))?;
    let count_end = pos
        .checked_add(4)
        .ok_or_else(|| corrupt("invalid OpusTags"))?;
    let count = u32le(
        bytes
            .get(pos..count_end)
            .ok_or_else(|| corrupt("invalid OpusTags"))?,
    ) as usize;
    pos = count_end;
    for _ in 0..count {
        let end = pos
            .checked_add(4)
            .ok_or_else(|| corrupt("invalid OpusTags"))?;
        let len = u32le(
            bytes
                .get(pos..end)
                .ok_or_else(|| corrupt("invalid OpusTags"))?,
        ) as usize;
        pos = end
            .checked_add(len)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| corrupt("invalid OpusTags"))?;
    }
    Ok(())
}

/// RFC 6716 §§3.1–3.2: duration from TOC, before the stream granule checks.
fn packet_frames(prefix: &[u8]) -> Result<u64, &'static str> {
    let toc = *prefix.first().ok_or("empty packet")?;
    let frame = if toc & 0x80 != 0 {
        120u64 << ((toc >> 3) & 3)
    } else if toc & 0x60 == 0x60 {
        480u64 << ((toc >> 3) & 1)
    } else {
        let size = (toc >> 3) & 3;
        if size == 3 {
            2880
        } else {
            480u64 << size
        }
    };
    let count = match toc & 3 {
        0 => 1,
        1 | 2 => 2,
        _ => u64::from(*prefix.get(1).ok_or("missing frame count")? & 0x3f),
    };
    let total = frame * count;
    if count == 0 || total > MAX_FRAMES as u64 {
        return Err("invalid frame count or packet duration");
    }
    Ok(total)
}

impl OpusStream {
    pub fn open(path: &Path) -> Result<Self, TrackError> {
        let mut file = File::open(path).map_err(|e| TrackError::Unreadable(e.to_string()))?;
        let mut prefix = Vec::new();
        let mut header_packet = Vec::new();
        let mut audio_start = 0;
        let mut total = 0u64;
        // Previously TOCs were checked after every page had been validated. Defer the
        // first TOC/count error to preserve that precedence without keeping an index.
        let mut packet_error = None;
        let mut packet_number = 0usize;
        let mut page_number = 0usize;
        let mut serial = None;
        let mut sequence: Option<u32> = None;
        let mut ended = false;
        let mut unfinished = false;
        let mut granule = None;
        let mut channels = 0;
        let mut pre_skip = 0;
        let mut gain = 1.0;
        let mut payload_bytes = 0u64;
        loop {
            let mut header = [0u8; 27];
            let read = file
                .read(&mut header[..1])
                .map_err(|e| TrackError::Unreadable(e.to_string()))?;
            if read == 0 {
                if page_number == 0 {
                    return Err(corrupt("not an Ogg stream"));
                }
                break;
            }
            // R1 precedes R2: an incomplete capture pattern is not Ogg either.
            let capture_read = file.read_exact(&mut header[1..4]);
            if capture_read.is_err() || &header[..4] != b"OggS" {
                return Err(corrupt("not an Ogg stream"));
            }
            read_exact(&mut file, &mut header[4..])?;
            let mut lacing = vec![0u8; usize::from(header[26])];
            read_exact(&mut file, &mut lacing)?;
            let mut body = vec![0u8; lacing.iter().map(|n| usize::from(*n)).sum()];
            read_exact(&mut file, &mut body)?;
            if header[4] != 0 {
                return Err(corrupt("unsupported Ogg page version"));
            }
            let expected_crc = u32le(&header[22..26]);
            header[22..26].fill(0);
            if crc(header
                .into_iter()
                .chain(lacing.iter().copied())
                .chain(body.iter().copied()))
                != expected_crc
            {
                return Err(corrupt("Ogg page CRC mismatch"));
            }
            let page_serial = u32le(&header[14..18]);
            if serial.is_some_and(|s| s != page_serial) {
                return Err(TrackError::UnsupportedFormat(
                    "multiplexed or chained Ogg streams are not supported".into(),
                ));
            }
            serial = Some(page_serial);
            let flags = header[5];
            if page_number == 0 && flags & 2 == 0 {
                return Err(corrupt("first Ogg page is not a beginning of stream"));
            }
            if ended {
                return Err(corrupt("Ogg page after end of stream"));
            }
            let page_sequence = u32le(&header[18..22]);
            if sequence.is_some_and(|s| s.wrapping_add(1) != page_sequence) {
                return Err(corrupt("Ogg page out of sequence"));
            }
            sequence = Some(page_sequence);
            if (flags & 1 != 0) != unfinished {
                return Err(corrupt("Ogg packet continuation mismatch"));
            }
            if page_number == 0
                && (lacing.is_empty()
                    || lacing.last() == Some(&255)
                    || lacing.iter().filter(|n| **n < 255).count() != 1)
            {
                return Err(corrupt("no OpusHead"));
            }
            if packet_number == 1 && !unfinished && page_number != 1 {
                return Err(corrupt("no OpusTags"));
            }
            let mut offset = 0;
            let mut audio_completed = false;
            for (segment_index, segment) in lacing.iter().enumerate() {
                let len = usize::from(*segment);
                let bytes = &body[offset..offset + len];
                if packet_number < 2 {
                    header_packet.extend_from_slice(bytes);
                } else {
                    if prefix.len() < 2 {
                        prefix.extend(bytes.iter().take(2 - prefix.len()));
                    }
                    payload_bytes = payload_bytes
                        .checked_add(len as u64)
                        .ok_or_else(|| corrupt("Opus payload too large"))?;
                }
                offset += len;
                unfinished = *segment == 255;
                if unfinished {
                    continue;
                }
                match packet_number {
                    0 => {
                        (channels, pre_skip, gain) = head(&header_packet)?;
                        header_packet.clear();
                    }
                    1 => {
                        tags(&header_packet)?;
                        header_packet.clear();
                        if segment_index + 1 != lacing.len() {
                            return Err(corrupt("audio data on the OpusTags page"));
                        }
                        audio_start = file
                            .stream_position()
                            .map_err(|e| TrackError::Unreadable(e.to_string()))?;
                    }
                    _ => {
                        if packet_error.is_none() {
                            match packet_frames(&prefix) {
                                Ok(frames) => match total.checked_add(frames) {
                                    Some(count) => total = count,
                                    None => {
                                        packet_error = Some(corrupt("Opus sample count overflow"))
                                    }
                                },
                                Err(e) => {
                                    let i = packet_number - 2;
                                    packet_error = Some(corrupt(format!("Opus packet {i}: {e}")));
                                }
                            }
                        }
                        prefix.clear();
                        audio_completed = true;
                    }
                }
                packet_number += 1;
            }
            if page_number == 0 && packet_number != 1 {
                return Err(corrupt("no OpusHead"));
            }
            if audio_completed {
                let current = i64::from_le_bytes([
                    header[6], header[7], header[8], header[9], header[10], header[11], header[12],
                    header[13],
                ]);
                if granule.is_some_and(|g| current < g) {
                    return Err(corrupt("Ogg granule position decreases"));
                }
                granule = Some(current);
            }
            ended = flags & 4 != 0;
            page_number += 1;
        }
        if unfinished {
            return Err(corrupt("unfinished Ogg packet at end of stream"));
        }
        if packet_number < 2 {
            return Err(corrupt("no OpusTags"));
        }
        if packet_number == 2 {
            return Err(corrupt("no Opus audio packets"));
        }
        if let Some(error) = packet_error {
            return Err(error);
        }
        let granule = granule.unwrap_or(-1);
        if granule < 0 || (granule as u64) < pre_skip {
            return Err(corrupt("granule position below pre-skip"));
        }
        let granule = granule as u64;
        if granule > total {
            return Err(corrupt("granule position beyond the audio"));
        }
        let length = granule - pre_skip;
        let bitrate = (length != 0).then(|| {
            (8.0 * payload_bytes as f64 * f64::from(RATE) / length as f64 / 1000.0).round() as u32
        });
        Ok(Self {
            file,
            audio_start,
            packet_count: packet_number - 2,
            channels,
            pre_skip,
            length,
            gain,
            bitrate,
        })
    }

    /// Kept, gain-adjusted interleaved PCM, before the common mono mix.
    pub fn for_each_interleaved(
        &mut self,
        mut sink: impl FnMut(u64, &[f32]),
    ) -> Result<u64, TrackError> {
        let channels = usize::from(self.channels);
        let mut decoder =
            OpusDecoder::new(RATE as i32, channels).map_err(|e| corrupt(e.to_string()))?;
        let mut pcm = vec![0f32; MAX_FRAMES * channels];
        let mut bytes = Vec::new();
        let mut body = Vec::new();
        let mut lacing = [0u8; 255];
        let mut decoded = 0u64;
        let mut kept = 0u64;
        self.file
            .seek(SeekFrom::Start(self.audio_start))
            .map_err(|e| TrackError::Unreadable(e.to_string()))?;
        let mut i = 0;
        while i < self.packet_count {
            // The first pass validated the entire container, including pages beyond the
            // end trim. Read its audio pages sequentially; a packet can cross many pages.
            let mut header = [0u8; 27];
            read_exact(&mut self.file, &mut header)?;
            let segments = &mut lacing[..usize::from(header[26])];
            read_exact(&mut self.file, segments)?;
            body.resize(segments.iter().map(|n| usize::from(*n)).sum(), 0);
            read_exact(&mut self.file, &mut body)?;
            let mut offset = 0;
            for segment in segments {
                let len = usize::from(*segment);
                bytes.extend_from_slice(&body[offset..offset + len]);
                offset += len;
                if *segment == 255 {
                    continue;
                }
                let frames =
                    packet_frames(&bytes).map_err(|e| corrupt(format!("Opus packet {i}: {e}")))?;
                let n = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    decoder.decode(&bytes, MAX_FRAMES, &mut pcm)
                }))
                .map_err(|_| corrupt(format!("Opus packet {i}: decoder panic")))?
                .map_err(|e| corrupt(format!("Opus packet {i}: {e}")))?;
                if n as u64 != frames {
                    return Err(corrupt(format!("Opus packet {i}: decoded length mismatch")));
                }
                let start = self.pre_skip.saturating_sub(decoded).min(n as u64) as usize;
                let end = (self.pre_skip + self.length)
                    .saturating_sub(decoded)
                    .min(n as u64) as usize;
                if end > start {
                    let samples = &mut pcm[start * channels..end * channels];
                    for x in samples.iter_mut() {
                        *x *= self.gain;
                    }
                    sink(kept, samples);
                    kept += (end - start) as u64;
                }
                decoded += n as u64;
                bytes.clear();
                i += 1;
                if i == self.packet_count {
                    break;
                }
            }
        }
        Ok(kept)
    }

    pub fn for_each_chunk(&mut self, mut sink: impl FnMut(u64, &[f32])) -> Result<u64, TrackError> {
        let channels = usize::from(self.channels);
        let mut mono = Vec::new();
        self.for_each_interleaved(|offset, samples| {
            mono.clear();
            mono.extend(
                samples
                    .chunks_exact(channels)
                    .map(|frame| frame.iter().sum::<f32>() / channels as f32),
            );
            sink(offset, &mono);
        })
    }
}
