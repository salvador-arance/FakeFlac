//! Spectrogram for display (RF-07, ADR-0009): streaming STFT of the mono mix, reduced to at most
//! `max_width` time columns and `height` frequency rows, as a u8 dB matrix (`data[t * height + f]`).
//!
//! Level: dBFS normalised by the coherent gain of the window, so a full-scale sine reads 0 dB.
//! Frames are never all kept: each frame is folded into its output column as it is computed.

use crate::decode::MonoStream;
use crate::window::coefficients;
use fakeflac_core::params::SpectrogramParams;
use fakeflac_core::{Spectrogram, TrackError};
use realfft::RealFftPlanner;
use std::path::Path;

/// Kaiser shape for the display window. `SpectrogramParams` has no beta; this is the same value
/// as the detection default so a Kaiser display window behaves like the detection one.
const DISPLAY_KAISER_BETA: f64 = 10.0;

pub fn compute(path: &Path, params: &SpectrogramParams) -> Result<Spectrogram, TrackError> {
    let mut stream = MonoStream::open(path)?;
    let mut total = stream.info.total_frames;
    if matches!(total, None | Some(0)) {
        total = Some(stream.for_each_chunk(|_, _| {})?);
        stream = MonoStream::open(path)?;
        stream.info.total_frames = total;
    }
    let total = total.unwrap_or(0);
    if total == 0 {
        return Err(TrackError::CorruptStream("no audio frames".into()));
    }
    let sample_rate = stream.info.sample_rate;
    let n = (params.fft_size as usize).max(2);
    let hop = (((1.0 - params.overlap) * n as f64).round() as usize).clamp(1, n);
    let height = (params.height as usize).max(1);
    let half = n / 2;
    let n_frames = if total < n as u64 {
        1
    } else {
        1 + (total - n as u64) / hop as u64
    };
    let width = n_frames.min(u64::from(params.max_width.max(1)));

    let window = coefficients(params.window, n, DISPLAY_KAISER_BETA);
    let gain: f64 = window.iter().sum();
    let norm = 4.0 / (gain * gain).max(1e-30);
    let mut planner = RealFftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(n);
    let input = fft.make_input_vec();
    let output = fft.make_output_vec();
    // Frequency row `f` covers bins [lo[f], hi[f]) of the 0..=half bins.
    let ranges: Vec<(usize, usize)> = (0..height)
        .map(|f| {
            let lo = (f * half / height).min(half);
            let hi = ((f + 1) * half / height).max(lo + 1).min(half + 1);
            (lo, hi)
        })
        .collect();

    let mut acc = Accumulator {
        fft,
        window,
        norm,
        ranges,
        input,
        output,
        height,
        width,
        n_frames,
        frame_index: 0,
        cur_col: 0,
        col_frames: 0,
        col_power: vec![0.0; height],
        data: vec![0u8; width as usize * height],
        params,
    };
    let mut buf: Vec<f32> = Vec::with_capacity(n * 2);
    stream.for_each_chunk(|_, samples| {
        if acc.frame_index >= n_frames {
            // Every frame is done: a file longer than it declared adds nothing.
            return;
        }
        buf.extend_from_slice(samples);
        let mut start = 0;
        while buf.len() >= start + n && acc.frame_index < n_frames {
            acc.process(&buf[start..start + n]);
            start += hop;
        }
        buf.drain(..start.min(buf.len()));
    })?;
    if acc.frame_index == 0 {
        // Shorter than one FFT frame: one zero-padded frame.
        acc.process(&buf);
    }
    acc.flush();
    let data = acc.data;

    Ok(Spectrogram {
        width: width as u32,
        height: height as u32,
        sample_rate,
        max_frequency_hz: f64::from(sample_rate) / 2.0,
        db_range: [params.db_floor, params.db_ceiling],
        data,
    })
}

fn to_byte(power: f64, params: &SpectrogramParams) -> u8 {
    let db = 10.0 * power.max(1e-30).log10();
    let span = (params.db_ceiling - params.db_floor).max(f64::EPSILON);
    let t = ((db - params.db_floor) / span).clamp(0.0, 1.0);
    (t * 255.0).round() as u8
}

struct Accumulator<'a> {
    fft: std::sync::Arc<dyn realfft::RealToComplex<f64>>,
    window: Vec<f64>,
    norm: f64,
    ranges: Vec<(usize, usize)>,
    input: Vec<f64>,
    output: Vec<realfft::num_complex::Complex<f64>>,
    height: usize,
    width: u64,
    n_frames: u64,
    frame_index: u64,
    cur_col: u64,
    col_frames: u32,
    col_power: Vec<f64>,
    data: Vec<u8>,
    params: &'a SpectrogramParams,
}

impl Accumulator<'_> {
    /// Writes the mean power of the finished column as bytes and resets the accumulator.
    fn flush(&mut self) {
        if self.col_frames == 0 {
            return;
        }
        let h = self.height;
        let row = &mut self.data[self.cur_col as usize * h..(self.cur_col as usize + 1) * h];
        for (byte, p) in row.iter_mut().zip(self.col_power.iter_mut()) {
            *byte = to_byte(*p / f64::from(self.col_frames), self.params);
            *p = 0.0;
        }
        self.col_frames = 0;
    }

    /// Folds one frame (at most `fft_size` samples, zero-padded) into its output column.
    fn process(&mut self, frame: &[f32]) {
        let col = (self.frame_index * self.width / self.n_frames).min(self.width - 1);
        if col != self.cur_col {
            self.flush();
            self.cur_col = col;
        }
        self.frame_index += 1;
        self.input.iter_mut().for_each(|x| *x = 0.0);
        for ((x, s), w) in self.input.iter_mut().zip(frame).zip(&self.window) {
            *x = f64::from(*s) * w;
        }
        // Input and output are made by the plan itself, so their lengths always match.
        let done = self.fft.process(&mut self.input, &mut self.output);
        debug_assert!(done.is_ok(), "FFT buffer lengths come from the plan");
        if done.is_err() {
            return;
        }
        for (acc, (lo, hi)) in self.col_power.iter_mut().zip(&self.ranges) {
            let sum: f64 = self.output[*lo..*hi].iter().map(|c| c.norm_sqr()).sum();
            *acc += sum / (*hi - *lo) as f64 * self.norm;
        }
        self.col_frames += 1;
    }
}
