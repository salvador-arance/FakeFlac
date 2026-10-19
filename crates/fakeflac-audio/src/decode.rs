//! Streaming decode to mono `f32` with symphonia (ADR-0030). Read-only (RNF-05).

use crate::{mp3, mp4};
use fakeflac_core::{AudioCodec, BitrateMode, TrackError};
use std::fs::File;
use std::path::Path;
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

#[derive(Debug, Clone, Copy)]
pub struct StreamInfo {
    pub sample_rate: u32,
    pub channels: u8,
    pub bit_depth: Option<u8>,
    /// Frames declared by the container, if known.
    pub total_frames: Option<u64>,
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
    /// Whether `info.total_frames` is the exact number of frames the decoder will return.
    /// MP3 lengths come from the frame count, and the decoder trims the encoder delay.
    exact_length: bool,
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
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        let format = symphonia::default::get_probe()
            .probe(
                &hint,
                mss,
                FormatOptions::default(),
                MetadataOptions::default(),
            )
            .map_err(|e| TrackError::CorruptStream(e.to_string()))?;
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
        let mut exact_length = true;
        if let Some(info) = &mp3_info {
            // symphonia estimates the length of an MP3 and trims the encoder delay: use the frame
            // count of the stream and do not hold the decoder to it.
            total_frames = Some(info.frames * u64::from(info.samples_per_frame));
            exact_length = false;
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
            exact_length,
            format,
            decoder,
            track_id,
        })
    }

    /// Decodes the whole stream, calling `sink(frame_offset, mono_samples)` per packet.
    /// Returns the number of decoded frames. A stream that ends before the declared length,
    /// or a packet that fails to decode, is a `CorruptStream` (RNF-04).
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
        if let Some(total) = self.info.total_frames.filter(|_| self.exact_length) {
            if offset < total {
                return Err(TrackError::CorruptStream(format!(
                    "stream ended after {offset} of {total} frames"
                )));
            }
        }
        Ok(offset)
    }
}
