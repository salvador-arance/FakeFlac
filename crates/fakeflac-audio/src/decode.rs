//! Streaming decode to mono `f32` with symphonia (ADR-0030). Read-only (RNF-05).

use crate::{mp3, mp4};
use fakeflac_core::{AudioCodec, BitrateMode, TrackError};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::{MediaSource, MediaSourceStream};
use symphonia::core::meta::MetadataOptions;

#[derive(Debug, Clone, Copy)]
pub struct StreamInfo {
    pub sample_rate: u32,
    pub channels: u8,
    pub bit_depth: Option<u8>,
    /// Frames declared by the container, if known.
    pub total_frames: Option<u64>,
}

/// A file seen from `start` on, so that a format reader can begin at the first audio frame.
struct FromOffset {
    file: File,
    start: u64,
}

impl Read for FromOffset {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file.read(buf)
    }
}

impl Seek for FromOffset {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let absolute = match pos {
            SeekFrom::Start(p) => SeekFrom::Start(self.start.saturating_add(p)),
            other => other,
        };
        let at = self.file.seek(absolute)?;
        Ok(at.saturating_sub(self.start))
    }
}

impl MediaSource for FromOffset {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        let len = self.file.metadata().ok()?.len();
        Some(len.saturating_sub(self.start))
    }
}

/// What the stream declares besides its samples (`specs/detection/lossy-source.md` section 2).
#[derive(Debug, Clone, Default)]
pub struct Declared {
    pub codec: Option<AudioCodec>,
    pub declared_bitrate_kbps: Option<u32>,
    pub bitrate_mode: Option<BitrateMode>,
    pub encoder: Option<String>,
    pub encoder_lowpass_hz: Option<u32>,
    /// Duration from the stream's own header (MP3: frame count of the tag frame or counted).
    pub duration_ms: Option<u64>,
}

pub struct MonoStream {
    pub info: StreamInfo,
    pub declared: Declared,
    /// Frames fewer than `info.total_frames` the decoder may legitimately return. MP3 lengths
    /// come from the frame count of the tag and the decoder trims the encoder delay and padding
    /// (`Mp3Info::length_tolerance`); every other format is exact.
    length_tolerance: u64,
    format: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track_id: u32,
}

impl MonoStream {
    pub fn open(path: &Path) -> Result<Self, TrackError> {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        // The adapter reads what the stream declares first: an unsupported codec is reported as
        // such, not as whatever the decoder says (ADR-0040).
        let mut declared = Declared::default();
        let mut mp3_info = None;
        let mut alac_depth = None;
        match extension.as_deref() {
            Some("wav") => declared.codec = Some(AudioCodec::Pcm),
            Some("flac") => declared.codec = Some(AudioCodec::Flac),
            Some("mp3") => {
                let info = mp3::read(path)?;
                declared.codec = Some(AudioCodec::Mp3);
                declared.declared_bitrate_kbps = info.declared_bitrate_kbps;
                declared.bitrate_mode = Some(info.bitrate_mode);
                declared.encoder = info.encoder.clone();
                declared.encoder_lowpass_hz = info.encoder_lowpass_hz;
                declared.duration_ms = Some(info.duration_ms());
                mp3_info = Some(info);
            }
            Some("m4a") => {
                let info = mp4::read(path)?;
                declared.codec = Some(info.codec);
                declared.declared_bitrate_kbps = info.declared_bitrate_kbps;
                alac_depth = info.bit_depth;
            }
            _ => {}
        }
        let file = File::open(path).map_err(|e| TrackError::Unreadable(e.to_string()))?;
        let format: Box<dyn FormatReader> = if let Some(info) = &mp3_info {
            // The probe scans from byte 0 and trips over large ID3v2 tags (artwork with false
            // syncs): the MP3 reader opens at the first frame the adapter found instead.
            let source = FromOffset {
                file,
                start: info.first_frame_pos,
            };
            let mss = MediaSourceStream::new(Box::new(source), Default::default());
            let reader =
                symphonia::default::formats::MpaReader::try_new(mss, FormatOptions::default())
                    .map_err(|e| TrackError::CorruptStream(e.to_string()))?;
            Box::new(reader)
        } else {
            let mss = MediaSourceStream::new(Box::new(file), Default::default());
            let mut hint = Hint::new();
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                hint.with_extension(ext);
            }
            symphonia::default::get_probe()
                .probe(
                    &hint,
                    mss,
                    FormatOptions::default(),
                    MetadataOptions::default(),
                )
                .map_err(|e| TrackError::CorruptStream(e.to_string()))?
        };
        let track = format
            .default_track(TrackType::Audio)
            .ok_or_else(|| TrackError::UnsupportedFormat("no audio track".into()))?;
        let track_id = track.id;
        let mut total_frames = track.num_frames;
        let codec = track
            .codec_params
            .as_ref()
            .and_then(|c| c.audio())
            .ok_or_else(|| TrackError::UnsupportedFormat("no audio codec parameters".into()))?
            .clone();
        let sample_rate = codec
            .sample_rate
            .ok_or_else(|| TrackError::CorruptStream("unknown sample rate".into()))?;
        let channels = codec.channels.as_ref().map_or(0, |c| c.count()) as u8;
        let bit_depth = match declared.codec {
            Some(AudioCodec::Mp3 | AudioCodec::Aac) => None,
            Some(AudioCodec::Alac) => alac_depth.or(codec.bits_per_sample.map(|b| b as u8)),
            _ => codec.bits_per_sample.map(|b| b as u8),
        };
        let mut length_tolerance = 0;
        if let Some(info) = &mp3_info {
            // symphonia estimates the length of an MP3 and trims the encoder delay: use the frame
            // count of the stream and allow the decoder the delay and padding it trims.
            total_frames = Some(info.frames * u64::from(info.samples_per_frame));
            length_tolerance = info.length_tolerance();
        }
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(&codec, &AudioDecoderOptions::default())
            .map_err(|e| TrackError::UnsupportedFormat(e.to_string()))?;
        Ok(MonoStream {
            info: StreamInfo {
                sample_rate,
                channels,
                bit_depth,
                total_frames,
            },
            declared,
            length_tolerance,
            format,
            decoder,
            track_id,
        })
    }

    /// Decodes the whole stream, calling `sink(frame_offset, mono_samples)` per packet.
    /// Returns the number of decoded frames. A stream that ends before the declared length
    /// (MP3: by more than the tolerance), or a packet that fails to decode, is a `CorruptStream` (RNF-04).
    pub fn for_each_chunk(&mut self, mut sink: impl FnMut(u64, &[f32])) -> Result<u64, TrackError> {
        let mut interleaved: Vec<f32> = Vec::new();
        let mut mono: Vec<f32> = Vec::new();
        let mut offset: u64 = 0;
        loop {
            let packet = match self.format.next_packet() {
                Ok(Some(p)) => p,
                Ok(None) => break,
                Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(TrackError::CorruptStream(e.to_string())),
            };
            if packet.track_id != self.track_id {
                continue;
            }
            let buf = self
                .decoder
                .decode(&packet)
                .map_err(|e| TrackError::CorruptStream(e.to_string()))?;
            let frames = buf.frames();
            let channels = buf.spec().channels().count().max(1);
            interleaved.resize(buf.samples_interleaved(), 0.0);
            buf.copy_to_slice_interleaved(&mut interleaved);
            mono.clear();
            mono.extend(
                interleaved
                    .chunks_exact(channels)
                    .map(|frame| frame.iter().sum::<f32>() / channels as f32),
            );
            sink(offset, &mono[..frames.min(mono.len())]);
            offset += frames as u64;
        }
        if let Some(total) = self.info.total_frames {
            if ended_short(offset, total, self.length_tolerance) {
                return Err(TrackError::CorruptStream(format!(
                    "stream ended after {offset} of {total} frames"
                )));
            }
        }
        Ok(offset)
    }
}

/// Whether a stream that decoded `decoded` frames ended before the `declared` length.
fn ended_short(decoded: u64, declared: u64, tolerance: u64) -> bool {
    decoded.saturating_add(tolerance) < declared
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolerance_applies_to_the_shortfall() {
        assert!(!ended_short(1000, 1000, 0));
        assert!(ended_short(999, 1000, 0));
        assert!(!ended_short(1000, 1000 + 2304, 2304));
        assert!(ended_short(0, 2305, 2304));
    }

    /// MPEG-1 Layer III, 128 kbps, 44.1 kHz, stereo: 417-byte frames of silence, behind an Info
    /// tag frame that declares `declared` frames.
    fn tagged_mp3(declared: u32, audio_frames: usize) -> Vec<u8> {
        let header = [0xFF, 0xFB, 0x90, 0x00];
        let mut tag = header.to_vec();
        tag.resize(417, 0);
        tag[36..40].copy_from_slice(b"Info");
        tag[40..44].copy_from_slice(&1u32.to_be_bytes());
        tag[44..48].copy_from_slice(&declared.to_be_bytes());
        let mut out = tag;
        for _ in 0..audio_frames {
            out.extend_from_slice(&header);
            out.resize(out.len() + 413, 0);
        }
        out
    }

    /// A file under `target/unit-tmp` removed when dropped.
    struct TempFile(std::path::PathBuf);

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn decode(name: &str, bytes: &[u8]) -> Result<u64, TrackError> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/unit-tmp");
        std::fs::create_dir_all(&dir).unwrap();
        let path = TempFile(dir.join(format!("decode-{}-{name}.mp3", std::process::id())));
        std::fs::write(&path.0, bytes).unwrap();
        MonoStream::open(&path.0)?.for_each_chunk(|_, _| {})
    }

    /// Like `tagged_mp3`, with a LAME extension declaring `delay` and `padding`.
    fn lame_mp3(declared: u32, audio_frames: usize, delay: u32, padding: u32) -> Vec<u8> {
        let mut out = tagged_mp3(declared, audio_frames);
        out[44 + 4..44 + 13].copy_from_slice(b"LAME3.99r");
        out[69] = (delay >> 4) as u8;
        out[70] = (((delay & 0xF) << 4) | (padding >> 8)) as u8;
        out[71] = (padding & 0xFF) as u8;
        out
    }

    /// An ID3v2.4 tag of `payload` bytes.
    fn id3v2(payload: Vec<u8>) -> Vec<u8> {
        let n = payload.len();
        let mut out = b"ID3\x04\0\0".to_vec();
        out.extend_from_slice(&[
            ((n >> 21) & 0x7F) as u8,
            ((n >> 14) & 0x7F) as u8,
            ((n >> 7) & 0x7F) as u8,
            (n & 0x7F) as u8,
        ]);
        out.extend(payload);
        out
    }

    #[test]
    fn lame_tagged_mp3_with_maximal_padding_passes() {
        assert!(decode("lame", &lame_mp3(40, 40, 576, 2304)).is_ok());
    }

    #[test]
    fn truncated_lame_tagged_mp3_is_corrupt() {
        let bytes = lame_mp3(40, 40, 576, 2304);
        let half = &bytes[..bytes.len() / 2];
        assert!(matches!(
            decode("lamehalf", half),
            Err(TrackError::CorruptStream(_))
        ));
    }

    #[test]
    fn mp3_behind_a_huge_id3v2_tag_decodes() {
        let mut bytes = id3v2(vec![0x55; (1 << 20) + 4096]);
        bytes.extend(tagged_mp3(40, 40));
        assert!(decode("bigid3", &bytes).is_ok());
    }

    #[test]
    fn mp3_behind_an_id3v2_tag_with_fake_syncs_decodes() {
        let mut blob = Vec::new();
        for _ in 0..2000 {
            blob.extend_from_slice(&[0xFF, 0xF3, 0x90, 0x00, 0x11, 0xFF, 0xFD, 0xA0, 0x00, 0x22]);
        }
        let mut bytes = id3v2(blob);
        bytes.extend(tagged_mp3(40, 40));
        assert!(decode("fakesync", &bytes).is_ok());
    }

    #[test]
    fn intact_tagged_mp3_passes() {
        assert!(decode("intact", &tagged_mp3(40, 40)).is_ok());
    }

    #[test]
    fn truncated_tagged_mp3_is_corrupt() {
        let bytes = tagged_mp3(40, 40);
        let half = &bytes[..bytes.len() / 2];
        assert!(matches!(
            decode("half", half),
            Err(TrackError::CorruptStream(_))
        ));
    }
}
