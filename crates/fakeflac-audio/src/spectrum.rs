//! Band spectra from sampled blocks (`specs/detection/rules.md`, section 1).
//!
//! `segments` blocks are spread over the track (skipping `edge_skip_percent` at each end).
//! Each block holds `frames_per_segment` FFT frames with hop `N * (1 - overlap)` (block length
//! `N + (frames_per_segment - 1) * hop`). Only the blocks are kept in memory, never the whole
//! track (RNF-02).

use crate::window::coefficients;
use fakeflac_core::{BandSpectra, DetectionParams};
use realfft::RealFftPlanner;

pub struct BlockCollector {
    n: usize,
    hop: usize,
    frames: usize,
    blocks: Vec<Block>,
}

struct Block {
    start: u64,
    samples: Vec<f32>,
}

impl BlockCollector {
    pub fn new(total_frames: Option<u64>, n: usize, params: &DetectionParams) -> Self {
        let hop = (((1.0 - params.fft.overlap) * n as f64).round() as usize).clamp(1, n);
        let frames = (params.input.frames_per_segment as usize).max(1);
        let len = (n + (frames - 1) * hop) as u64;
        let starts = match total_frames {
            Some(total) => plan(
                total,
                len,
                params.input.segments,
                params.input.edge_skip_percent,
            ),
            None => Vec::new(),
        };
        BlockCollector {
            n,
            hop,
            frames,
            blocks: starts
                .into_iter()
                .map(|start| Block {
                    start,
                    samples: Vec::with_capacity(len as usize),
                })
                .collect(),
        }
    }

    fn block_len(&self) -> usize {
        self.n + (self.frames - 1) * self.hop
    }

    /// Feeds decoded mono samples starting at frame `offset`.
    pub fn push(&mut self, offset: u64, samples: &[f32]) {
        let len = self.block_len() as u64;
        let end = offset + samples.len() as u64;
        for b in &mut self.blocks {
            let want_from = b.start + b.samples.len() as u64;
            let want_to = b.start + len;
            if want_from >= want_to || want_from >= end || want_to <= offset {
                continue;
            }
            let from = want_from.max(offset);
            let to = want_to.min(end);
            b.samples
                .extend_from_slice(&samples[(from - offset) as usize..(to - offset) as usize]);
        }
    }

    /// Mean and median band spectra over the non-silent frames (rules.md section 1, items 5 to 9);
    /// `None` if every frame was silent.
    pub fn finish(self, sample_rate: u32, params: &DetectionParams) -> Option<BandSpectra> {
        let n = self.n;
        let band_hz = params.fft.band_hz;
        let nyquist = f64::from(sample_rate) / 2.0;
        let band_count = (nyquist / band_hz + 1e-9).floor() as usize;
        if band_count == 0 {
            return None;
        }
        // Band of every FFT bin; bins at or above the lower edge of the last full band join it.
        let bin_hz = f64::from(sample_rate) / n as f64;
        let band_of: Vec<usize> = (0..=n / 2)
            .map(|k| (((k as f64 * bin_hz) / band_hz + 1e-9).floor() as usize).min(band_count - 1))
            .collect();
        let mut bins_in_band = vec![0usize; band_count];
        for b in &band_of {
            bins_in_band[*b] += 1;
        }

        let window = coefficients(params.fft.window, n, params.fft.kaiser_beta);
        let window_power: f64 = window.iter().map(|w| w * w).sum();
        let mut planner = RealFftPlanner::<f64>::new();
        let fft = planner.plan_fft_forward(n);
        let mut input = fft.make_input_vec();
        let mut output = fft.make_output_vec();
        let mut sum_power = vec![0.0f64; band_count];
        let mut frame_db: Vec<Vec<f64>> = vec![Vec::new(); band_count];
        let mut frame_power = vec![0.0f64; band_count];
        let mut kept = 0u32;

        for block in self
            .blocks
            .iter()
            .filter(|b| b.samples.len() == self.block_len())
        {
            for f in 0..self.frames {
                let frame = &block.samples[f * self.hop..f * self.hop + n];
                let rms =
                    (frame.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / n as f64).sqrt();
                if 20.0 * rms.max(1e-20).log10() < params.input.silence_dbfs {
                    continue;
                }
                for ((x, s), w) in input.iter_mut().zip(frame).zip(&window) {
                    *x = f64::from(*s) * w;
                }
                if fft.process(&mut input, &mut output).is_err() {
                    continue;
                }
                frame_power.iter_mut().for_each(|p| *p = 0.0);
                for (c, b) in output.iter().zip(&band_of) {
                    frame_power[*b] += c.norm_sqr() / window_power;
                }
                for b in 0..band_count {
                    let p = frame_power[b] / bins_in_band[b].max(1) as f64;
                    sum_power[b] += p;
                    frame_db[b].push(to_db(p));
                }
                kept += 1;
            }
        }
        if kept == 0 {
            return None;
        }
        Some(BandSpectra {
            sample_rate,
            band_hz,
            mean_db: sum_power
                .into_iter()
                .map(|p| to_db(p / f64::from(kept)))
                .collect(),
            median_db: frame_db.into_iter().map(median).collect(),
        })
    }
}

fn to_db(power: f64) -> f64 {
    10.0 * power.max(1e-30).log10()
}

/// Nearest-rank median.
fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    let rank = ((values.len() - 1) as f64 / 2.0).round() as usize;
    values[rank.min(values.len() - 1)]
}

/// Start frames of `segments` blocks of length `len`, evenly spread over the usable part.
fn plan(total: u64, len: u64, segments: u32, edge_skip_percent: f64) -> Vec<u64> {
    if total < len || segments == 0 {
        return Vec::new();
    }
    let skip = (total as f64 * edge_skip_percent / 100.0) as u64;
    let (from, to) = (skip, total.saturating_sub(skip));
    if to < from + len {
        return vec![(total - len) / 2];
    }
    let span = to - len - from;
    if segments == 1 {
        return vec![from + span / 2];
    }
    (0..u64::from(segments))
        .map(|i| from + span * i / u64::from(segments - 1))
        .collect()
}
