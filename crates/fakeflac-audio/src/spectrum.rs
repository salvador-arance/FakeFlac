//! Band spectra from sampled blocks (`specs/detection/rules.md`, section 1).
//!
//! `segments` blocks are spread over the track (skipping `edge_skip_percent` at each end).
//! Each block holds two FFT frames overlapping by `overlap` (block length `N + hop`), which is
//! the "block long enough for the FFT and its overlap" of the spec. Only the blocks are kept in
//! memory, never the whole track (RNF-02).

use fakeflac_core::{BandSpectra, DetectionParams};

pub struct BlockCollector {
    n: usize,
    hop: usize,
    blocks: Vec<Block>,
}

struct Block {
    start: u64,
    samples: Vec<f32>,
}

impl BlockCollector {
    pub fn new(total_frames: Option<u64>, n: usize, params: &DetectionParams) -> Self {
        let hop = (((1.0 - params.fft.overlap) * n as f64).round() as usize).clamp(1, n);
        let len = (n + hop) as u64;
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
            blocks: starts
                .into_iter()
                .map(|start| Block {
                    start,
                    samples: Vec::with_capacity(len as usize),
                })
                .collect(),
        }
    }

    /// Feeds decoded mono samples starting at frame `offset`.
    pub fn push(&mut self, offset: u64, samples: &[f32]) {
        let len = (self.n + self.hop) as u64;
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

    /// Mean and median band spectra over the non-silent frames; `None` if every frame was silent.
    pub fn finish(self, sample_rate: u32, params: &DetectionParams) -> Option<BandSpectra> {
        let _ = (self.n, self.hop, sample_rate, params);
        todo!("RF-03: band spectra of rules.md section 1")
    }
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
