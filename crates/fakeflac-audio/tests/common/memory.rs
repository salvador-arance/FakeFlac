//! Length-parametrised synthetic writers for RNF-02 (memory.md section 4).
//! The existing detection writers and their fixture bytes are untouched.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use super::lossy::*;
use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::io::Write;
use std::path::Path;

/// Box-Muller normal source. Separate seeds keep the two channels independent.
pub struct Gaussian {
    rng: ChaCha8Rng,
    spare: Option<f64>,
    sigma: f64,
}

impl Gaussian {
    pub fn new(seed: u64, dbfs: f64) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            spare: None,
            sigma: 10f64.powf(dbfs / 20.0),
        }
    }
    pub fn sample(&mut self) -> f64 {
        if let Some(x) = self.spare.take() {
            return x;
        }
        let r = (-2.0 * (1.0 - self.rng.random::<f64>()).ln()).sqrt() * self.sigma;
        let t = std::f64::consts::TAU * self.rng.random::<f64>();
        self.spare = Some(r * t.sin());
        r * t.cos()
    }
}

pub fn write_wav(path: &Path, frames: u64) {
    let mut noise = [Gaussian::new(11, -12.0), Gaussian::new(12, -12.0)];
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 44_100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut out = hound::WavWriter::create(path, spec).unwrap();
    for _ in 0..frames {
        for channel in &mut noise {
            out.write_sample(
                (channel.sample() * 32767.0)
                    .round()
                    .clamp(-32768.0, 32767.0) as i16,
            )
            .unwrap();
        }
    }
    out.finalize().unwrap();
}

/// Encode one 4096-frame block at a time, including the hour-long fixture. Only the parent
/// generates audio; keeping its buffers small also keeps the opt-in run practical.
pub fn write_flac(path: &Path, blocks: usize, rate: u32, bits: usize, dbfs: f64) {
    use flacenc::component::{BitRepr, Stream, StreamInfo};
    use flacenc::error::Verify;
    use flacenc::source::{Fill, FrameBuf};
    let config = flacenc::config::Encoder::default().into_verified().unwrap();
    let mut info = StreamInfo::new(rate as usize, 2, bits).unwrap();
    info.set_block_sizes(4096, 4096).unwrap();
    info.set_frame_sizes(0, 0).unwrap();
    info.set_total_samples(blocks * 4096);
    let mut header = flacenc::bitsink::ByteSink::new();
    Stream::with_stream_info(info.clone())
        .write(&mut header)
        .unwrap();
    let mut out = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    out.write_all(header.as_slice()).unwrap();
    let mut noise = [Gaussian::new(21, dbfs), Gaussian::new(22, dbfs)];
    let full = f64::from((1i32 << (bits - 1)) - 1);
    let mut samples = vec![0; 8192];
    let mut fb = FrameBuf::with_size(2, 4096).unwrap();
    for number in 0..blocks {
        for frame in samples.chunks_exact_mut(2) {
            for (ch, x) in frame.iter_mut().enumerate() {
                *x = (noise[ch].sample() * full).round().clamp(-full - 1.0, full) as i32;
            }
        }
        fb.fill_interleaved(&samples).unwrap();
        let frame = flacenc::encode_fixed_size_frame(&config, &fb, number, &info).unwrap();
        let mut encoded = flacenc::bitsink::ByteSink::new();
        frame.write(&mut encoded).unwrap();
        out.write_all(encoded.as_slice()).unwrap();
    }
    out.flush().unwrap();
}

pub fn write_mp3_length(path: &Path, frames: usize) {
    std::fs::write(
        path,
        write_mp3(&Mp3Spec {
            sample_rate: 44_100,
            edge_hz: Some(20_000.0),
            frames,
            bitrates: vec![320],
            tag: Some(TagSpec {
                kind: TagKind::Info,
                lame: Some(LameTag {
                    encoder: "LAME3.100",
                    vbr_method: 1,
                    lowpass_byte: 200,
                }),
            }),
            gate: None,
            seed: 301,
        }),
    )
    .unwrap();
}

pub fn write_aac_length(path: &Path, frames: usize) {
    std::fs::write(
        path,
        write_m4a_aac(&AacSpec {
            sample_rate: 44_100,
            edge_hz: 20_000.0,
            bitrate_kbps: 256,
            frames,
            object_type: 2,
            seed: 302,
        }),
    )
    .unwrap();
}

pub fn write_alac_length(path: &Path, frames: usize) {
    let mut noise = Gaussian::new(303, -12.0);
    let samples: Vec<i16> = (0..frames)
        .map(|_| (noise.sample().clamp(-1.0, 1.0) * 32767.0).round() as i16)
        .collect();
    std::fs::write(path, write_m4a_alac(&samples, 44_100)).unwrap();
}

/// Change only FLAC's 36-bit STREAMINFO length; frame payload and other header bits stay intact.
pub fn set_flac_total(path: &Path, total: u64) {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    file.seek(SeekFrom::Start(21)).unwrap();
    let mut field = [0; 5];
    file.read_exact(&mut field).unwrap();
    field[0] = (field[0] & 0xf0) | ((total >> 32) & 0xf) as u8;
    field[1..].copy_from_slice(&(total as u32).to_be_bytes());
    file.seek(SeekFrom::Start(21)).unwrap();
    file.write_all(&field).unwrap();
}
