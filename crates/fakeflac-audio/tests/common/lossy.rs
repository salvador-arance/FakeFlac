//! Bitstream writers of `specs/detection/lossy-fixtures.md` (ADR-0022, ADR-0040): MP3, AAC-LC in M4A
//! and ALAC in M4A files written inside the tests, from a seed. No encoder runs, no dependency is
//! added and no audio is committed. A writer does not encode audio: it writes the **coded spectrum**
//! (quantised MDCT coefficients) directly, so the decoder's output is noise up to a chosen edge and
//! almost nothing above it.
//!
//! This module depends only on `rand` and `rand_chacha`, so other crates include it by path
//! (`#[path = "../../crates/fakeflac-audio/tests/common/lossy.rs"] mod lossy;`, ADR-0036).
//!
//! Sources of the constants that are not in the specs:
//! - The AAC spectral codebook 1 (ISO/IEC 14496-3 table 4.A.1, 81 codewords of four values in
//!   {-1, 0, 1}) and the scale-factor codebook entry for a difference of 0 (table 4.A.2: index 60,
//!   one bit, code 0) are standard data. The codebook 1 lengths and codes below were taken from the
//!   `symphonia-codec-aac` 0.6.1 source (MPL-2.0 file `codebooks.rs`); the test of
//!   `lossy_fixtures.rs` verifies them by decoding a fixture (lossy-fixtures.md section 3).
//! - The long-window scale-factor band offsets for 44.1 and 48 kHz (ISO/IEC 14496-3 table 4.109) are
//!   the standard ones, as in the same source.
//! - The decoded level of each writer (`MP3_GLOBAL_GAIN`, `AAC_GLOBAL_GAIN`) was chosen by the test
//!   stage so that the decoded RMS of a full-band fixture lies between -30 and -12 dBFS
//!   (lossy-fixtures.md, "Level").
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// `global_gain` of every MP3 granule (lossy-fixtures.md 1.1, "Level"). Decoded RMS of a full-band
/// 44.1 kHz fixture: see `lossy_fixtures.rs`, `levels_are_between_minus_30_and_minus_12_dbfs`.
pub const MP3_GLOBAL_GAIN: u8 = 183;
/// `global_gain` of every AAC frame (every scale factor equals it, 1.2).
pub const AAC_GLOBAL_GAIN: u8 = 171;

// ---- Bit writer ---------------------------------------------------------------------------------

/// MSB-first bit writer.
#[derive(Default)]
pub struct BitWriter {
    bytes: Vec<u8>,
    bits: usize,
}

impl BitWriter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Writes the low `n` bits of `value`, most significant first.
    pub fn put(&mut self, value: u32, n: u32) {
        assert!(n <= 32);
        for i in (0..n).rev() {
            let bit = (value >> i) & 1;
            if self.bits % 8 == 0 {
                self.bytes.push(0);
            }
            if bit == 1 {
                let last = self.bytes.len() - 1;
                self.bytes[last] |= 0x80 >> (self.bits % 8);
            }
            self.bits += 1;
        }
    }

    pub fn bit_len(&self) -> usize {
        self.bits
    }

    /// The bytes written so far; the last byte is padded with zero bits.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// A coefficient in {-1, 0, +1} with probabilities 1/4, 1/2, 1/4 (lossy-fixtures.md 1.1).
fn coefficient(rng: &mut ChaCha8Rng) -> i8 {
    match rng.random_range(0..4u32) {
        0 => -1,
        1 | 2 => 0,
        _ => 1,
    }
}

// ---- MP3 (MPEG-1 Layer III) -------------------------------------------------------------------

/// MPEG-1 Layer III bitrates by bitrate index 1 to 14 (kbps).
const MP3_BITRATES: [u32; 14] = [
    32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
];

fn mp3_bitrate_index(kbps: u32) -> u32 {
    MP3_BITRATES
        .iter()
        .position(|b| *b == kbps)
        .unwrap_or_else(|| panic!("no MPEG-1 Layer III bitrate index for {kbps} kbps")) as u32
        + 1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagKind {
    /// CBR files (LAME, ffmpeg).
    Info,
    /// VBR and ABR files.
    Xing,
}

/// The LAME extension of the tag frame.
#[derive(Debug, Clone)]
pub struct LameTag {
    /// The 9-byte encoder field (`LAME3.100`).
    pub encoder: &'static str,
    /// Low 4 bits of the byte after the encoder field.
    pub vbr_method: u8,
    /// Low-pass in Hz / 100; 0 declares none.
    pub lowpass_byte: u8,
}

#[derive(Debug, Clone)]
pub struct TagSpec {
    pub kind: TagKind,
    pub lame: Option<LameTag>,
}

/// Optional second edge: in `share` of the granules the coefficients from `edge_hz` upwards are zero.
#[derive(Debug, Clone, Copy)]
pub struct Gate {
    pub edge_hz: f64,
    pub share: f64,
}

#[derive(Debug, Clone)]
pub struct Mp3Spec {
    pub sample_rate: u32,
    /// Nominal edge in Hz; `None` codes every coefficient as zero (a silent file).
    pub edge_hz: Option<f64>,
    /// Number of audio frames (the tag frame is on top).
    pub frames: usize,
    /// Bitrate (kbps) of audio frame `i` is `bitrates[i % bitrates.len()]`.
    pub bitrates: Vec<u32>,
    pub tag: Option<TagSpec>,
    pub gate: Option<Gate>,
    pub seed: u64,
}

/// "30 s" / "2 s" in audio frames: `round(seconds · sample_rate / 1152)` (lossy-fixtures.md 1.1).
pub fn mp3_frames(seconds: f64, sample_rate: u32) -> usize {
    (seconds * f64::from(sample_rate) / 1152.0).round() as usize
}

/// `k`: the number of coded coefficients per granule for a nominal edge (a multiple of 4).
pub fn mp3_coded_coefficients(edge_hz: f64, sample_rate: u32) -> usize {
    let bin_hz = f64::from(sample_rate) / 2.0 / 576.0;
    (((edge_hz / bin_hz / 4.0).round() as usize) * 4).min(576)
}

/// The coded edge `k · bin_hz` in Hz.
pub fn mp3_coded_edge_hz(edge_hz: f64, sample_rate: u32) -> f64 {
    mp3_coded_coefficients(edge_hz, sample_rate) as f64 * f64::from(sample_rate) / 2.0 / 576.0
}

fn sample_rate_index(sample_rate: u32) -> u32 {
    match sample_rate {
        44_100 => 0,
        48_000 => 1,
        other => panic!("unsupported MPEG-1 sample rate {other}"),
    }
}

/// Writes the MP3 stream: an optional tag frame, then `spec.frames` audio frames.
pub fn write_mp3(spec: &Mp3Spec) -> Vec<u8> {
    let sr = spec.sample_rate;
    let mut rng = ChaCha8Rng::seed_from_u64(spec.seed);
    let k = spec
        .edge_hz
        .map_or(0, |edge| mp3_coded_coefficients(edge, sr));
    let gate_k = spec.gate.map(|g| mp3_coded_coefficients(g.edge_hz, sr));

    // Frame sizes: `144 · bitrate / sample_rate` bytes, the padding bit set by an accumulator as an
    // encoder does.
    let mut remainder = 0u32;
    let mut next_frame = |kbps: u32| -> (usize, bool) {
        let numerator = 144_000 * kbps;
        remainder += numerator % sr;
        let pad = remainder >= sr;
        if pad {
            remainder -= sr;
        }
        ((numerator / sr) as usize + usize::from(pad), pad)
    };

    let mut audio: Vec<Vec<u8>> = Vec::with_capacity(spec.frames);
    let mut tag_frame_size = None;
    if spec.tag.is_some() {
        tag_frame_size = Some(next_frame(spec.bitrates[0]));
    }
    for i in 0..spec.frames {
        let kbps = spec.bitrates[i % spec.bitrates.len()];
        let (size, pad) = next_frame(kbps);
        let mut frame = mp3_header(kbps, sr, pad);
        // Side information (17 bytes) and main data.
        let mut main = BitWriter::new();
        let mut granules = [(0u32, 0u32); 2];
        for granule in &mut granules {
            let gated = spec.gate.is_some_and(|g| rng.random::<f64>() < g.share);
            let coded = if gated { gate_k.unwrap_or(k).min(k) } else { k };
            let start = main.bit_len();
            for quad in 0..coded / 4 {
                let _ = quad;
                let values: [i8; 4] = [
                    coefficient(&mut rng),
                    coefficient(&mut rng),
                    coefficient(&mut rng),
                    coefficient(&mut rng),
                ];
                // Quadruple table B (ISO/IEC 11172-3 table B.7): four bits, the complement of the
                // pattern of non-zero values (v, w, x, y); then one sign bit per non-zero value.
                let pattern = values
                    .iter()
                    .fold(0u32, |p, v| (p << 1) | u32::from(*v != 0));
                main.put(15 - pattern, 4);
                for v in values.iter().filter(|v| **v != 0) {
                    main.put(u32::from(*v < 0), 1);
                }
            }
            *granule = ((main.bit_len() - start) as u32, 0);
        }
        let mut side = BitWriter::new();
        side.put(0, 9); // main_data_begin
        side.put(0, 5); // private bits (mono)
        side.put(0, 4); // scfsi
        for (part2_3_length, _) in granules {
            side.put(part2_3_length, 12);
            side.put(0, 9); // big_values
            side.put(u32::from(MP3_GLOBAL_GAIN), 8);
            side.put(0, 4); // scalefac_compress
            side.put(0, 1); // window_switching_flag
            side.put(0, 15); // table_select
            side.put(0, 4); // region0_count
            side.put(0, 3); // region1_count
            side.put(0, 1); // preflag
            side.put(0, 1); // scalefac_scale
            side.put(1, 1); // count1table_select: table B
        }
        let side = side.into_bytes();
        assert_eq!(side.len(), 17);
        frame.extend_from_slice(&side);
        let main_bytes = main.into_bytes();
        assert!(
            21 + main_bytes.len() <= size,
            "{} bytes of coded data do not fit a {size}-byte frame of {kbps} kbps",
            main_bytes.len()
        );
        frame.extend_from_slice(&main_bytes);
        frame.resize(size, 0); // ancillary data: zero bits
        audio.push(frame);
    }

    let mut out = Vec::new();
    if let (Some(tag), Some((size, pad))) = (&spec.tag, tag_frame_size) {
        let total_bytes = size + audio.iter().map(Vec::len).sum::<usize>();
        let mut frame = mp3_header(spec.bitrates[0], sr, pad);
        frame.resize(21, 0); // side information: zero
        frame.extend_from_slice(match tag.kind {
            TagKind::Info => b"Info",
            TagKind::Xing => b"Xing",
        });
        frame.extend_from_slice(&0x0Fu32.to_be_bytes()); // frames, bytes, TOC, quality
        frame.extend_from_slice(&(spec.frames as u32).to_be_bytes());
        frame.extend_from_slice(&(total_bytes as u32).to_be_bytes());
        frame.extend_from_slice(&[0u8; 100]); // TOC
        frame.extend_from_slice(&0u32.to_be_bytes()); // quality
        if let Some(lame) = &tag.lame {
            let mut ext = [0u8; 36];
            let encoder = lame.encoder.as_bytes();
            assert!(encoder.len() <= 9);
            ext[..encoder.len()].copy_from_slice(encoder);
            ext[9] = lame.vbr_method & 0x0F;
            ext[10] = lame.lowpass_byte;
            frame.extend_from_slice(&ext);
        }
        frame.resize(size, 0);
        out.extend_from_slice(&frame);
    }
    for frame in audio {
        out.extend_from_slice(&frame);
    }
    out
}

/// MPEG-1, Layer III, no CRC, mono, no emphasis.
fn mp3_header(kbps: u32, sample_rate: u32, pad: bool) -> Vec<u8> {
    vec![
        0xFF,
        0xFB,
        ((mp3_bitrate_index(kbps) << 4)
            | (sample_rate_index(sample_rate) << 2)
            | (u32::from(pad) << 1)) as u8,
        0xC0,
    ]
}

// ---- MP4 container -----------------------------------------------------------------------------

fn boxed(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
    out.extend_from_slice(kind);
    out.extend_from_slice(payload);
    out
}

fn full_box(kind: &[u8; 4], version: u8, flags: u32, payload: &[u8]) -> Vec<u8> {
    let mut body = vec![version];
    body.extend_from_slice(&flags.to_be_bytes()[1..]);
    body.extend_from_slice(payload);
    boxed(kind, &body)
}

fn be32(v: u32) -> [u8; 4] {
    v.to_be_bytes()
}

fn be16(v: u16) -> [u8; 2] {
    v.to_be_bytes()
}

/// An audio sample entry (ISO/IEC 14496-12 12.2.3) with one child box.
fn audio_sample_entry(kind: &[u8; 4], sample_rate: u32, bits: u16, child: &[u8]) -> Vec<u8> {
    let mut p = vec![0u8; 6]; // reserved
    p.extend_from_slice(&be16(1)); // data_reference_index
    p.extend_from_slice(&[0u8; 8]); // reserved
    p.extend_from_slice(&be16(1)); // channel count
    p.extend_from_slice(&be16(bits)); // sample size
    p.extend_from_slice(&be16(0)); // pre_defined
    p.extend_from_slice(&be16(0)); // reserved
    p.extend_from_slice(&be32(sample_rate << 16));
    p.extend_from_slice(child);
    boxed(kind, &p)
}

struct Mp4Track {
    sample_entry: Vec<u8>,
    sample_rate: u32,
    /// Samples (audio frames) per MP4 sample: 1024 for AAC, 4096 for ALAC.
    samples_per_frame: u32,
    sample_sizes: Vec<u32>,
    /// The `mdat` payload: every sample, in order.
    mdat: Vec<u8>,
}

/// `ftyp`, `moov` (with `mvhd`, one audio `trak` and its `stbl`) **before** `mdat`.
fn write_mp4(track: &Mp4Track) -> Vec<u8> {
    let frames = track.sample_sizes.len() as u32;
    let duration = frames * track.samples_per_frame;
    let ts = track.sample_rate;

    let mut ftyp = b"M4A ".to_vec();
    ftyp.extend_from_slice(&be32(0));
    ftyp.extend_from_slice(b"M4A mp42isom");
    let ftyp = boxed(b"ftyp", &ftyp);

    let build_moov = |chunk_offset: u32| -> Vec<u8> {
        let mut mvhd = Vec::new();
        mvhd.extend_from_slice(&[0u8; 8]); // creation, modification
        mvhd.extend_from_slice(&be32(ts));
        mvhd.extend_from_slice(&be32(duration));
        mvhd.extend_from_slice(&be32(0x0001_0000)); // rate
        mvhd.extend_from_slice(&be16(0x0100)); // volume
        mvhd.extend_from_slice(&[0u8; 10]);
        for m in [0x0001_0000u32, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x4000_0000] {
            mvhd.extend_from_slice(&be32(m));
        }
        mvhd.extend_from_slice(&[0u8; 24]);
        mvhd.extend_from_slice(&be32(2)); // next track id
        let mvhd = full_box(b"mvhd", 0, 0, &mvhd);

        let mut tkhd = Vec::new();
        tkhd.extend_from_slice(&[0u8; 8]);
        tkhd.extend_from_slice(&be32(1)); // track id
        tkhd.extend_from_slice(&be32(0));
        tkhd.extend_from_slice(&be32(duration));
        tkhd.extend_from_slice(&[0u8; 8]);
        tkhd.extend_from_slice(&be16(0)); // layer
        tkhd.extend_from_slice(&be16(0)); // alternate group
        tkhd.extend_from_slice(&be16(0x0100)); // volume
        tkhd.extend_from_slice(&be16(0));
        for m in [0x0001_0000u32, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x4000_0000] {
            tkhd.extend_from_slice(&be32(m));
        }
        tkhd.extend_from_slice(&[0u8; 8]); // width, height
        let tkhd = full_box(b"tkhd", 0, 3, &tkhd);

        let mut mdhd = Vec::new();
        mdhd.extend_from_slice(&[0u8; 8]);
        mdhd.extend_from_slice(&be32(ts));
        mdhd.extend_from_slice(&be32(duration));
        mdhd.extend_from_slice(&be16(0x55C4)); // "und"
        mdhd.extend_from_slice(&be16(0));
        let mdhd = full_box(b"mdhd", 0, 0, &mdhd);

        let mut hdlr = Vec::new();
        hdlr.extend_from_slice(&be32(0));
        hdlr.extend_from_slice(b"soun");
        hdlr.extend_from_slice(&[0u8; 12]);
        hdlr.push(0);
        let hdlr = full_box(b"hdlr", 0, 0, &hdlr);

        let smhd = full_box(b"smhd", 0, 0, &[0u8; 4]);
        let url = full_box(b"url ", 0, 1, &[]);
        let mut dref = be32(1).to_vec();
        dref.extend_from_slice(&url);
        let dinf = boxed(b"dinf", &full_box(b"dref", 0, 0, &dref));

        let mut stsd = be32(1).to_vec();
        stsd.extend_from_slice(&track.sample_entry);
        let stsd = full_box(b"stsd", 0, 0, &stsd);

        let mut stts = be32(1).to_vec();
        stts.extend_from_slice(&be32(frames));
        stts.extend_from_slice(&be32(track.samples_per_frame));
        let stts = full_box(b"stts", 0, 0, &stts);

        let mut stsc = be32(1).to_vec();
        stsc.extend_from_slice(&be32(1));
        stsc.extend_from_slice(&be32(frames));
        stsc.extend_from_slice(&be32(1));
        let stsc = full_box(b"stsc", 0, 0, &stsc);

        let mut stsz = be32(0).to_vec();
        stsz.extend_from_slice(&be32(frames));
        for size in &track.sample_sizes {
            stsz.extend_from_slice(&be32(*size));
        }
        let stsz = full_box(b"stsz", 0, 0, &stsz);

        let mut stco = be32(1).to_vec();
        stco.extend_from_slice(&be32(chunk_offset));
        let stco = full_box(b"stco", 0, 0, &stco);

        let stbl = boxed(b"stbl", &[stsd, stts, stsc, stsz, stco].concat());
        let minf = boxed(b"minf", &[smhd, dinf, stbl].concat());
        let mdia = boxed(b"mdia", &[mdhd, hdlr, minf].concat());
        let trak = boxed(b"trak", &[tkhd, mdia].concat());
        boxed(b"moov", &[mvhd, trak].concat())
    };

    let moov_len = build_moov(0).len();
    let moov = build_moov((ftyp.len() + moov_len + 8) as u32);
    let mut out = ftyp;
    out.extend_from_slice(&moov);
    out.extend_from_slice(&boxed(b"mdat", &track.mdat));
    out
}

/// Length in bytes of the `ftyp` and `moov` boxes at the start of an M4A written by this module.
pub fn mp4_header_len(file: &[u8]) -> (usize, usize) {
    let ftyp = u32::from_be_bytes(file[..4].try_into().unwrap()) as usize;
    let moov = u32::from_be_bytes(file[ftyp..ftyp + 4].try_into().unwrap()) as usize;
    (ftyp, moov)
}

// ---- AAC-LC in M4A -----------------------------------------------------------------------------

/// Long-window scale-factor band offsets for 44.1 and 48 kHz (ISO/IEC 14496-3 table 4.109).
const SWB_OFFSET_LONG_48K: [usize; 50] = [
    0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 48, 56, 64, 72, 80, 88, 96, 108, 120, 132, 144, 160,
    176, 196, 216, 240, 264, 292, 320, 352, 384, 416, 448, 480, 512, 544, 576, 608, 640, 672, 704,
    736, 768, 800, 832, 864, 896, 928, 1024,
];

/// AAC spectral codebook 1 (ISO/IEC 14496-3 table 4.A.1): length in bits and code of the quadruple
/// `(w, x, y, z)`, each value in {-1, 0, 1}, at index `27 (w + 1) + 9 (x + 1) + 3 (y + 1) + (z + 1)`.
const SPECTRUM_CODEBOOK1_LENS: [u8; 81] = [
    11, 9, 11, 10, 7, 10, 11, 9, 11, 10, 7, 10, 7, 5, 7, 9, 7, 10, 11, 9, 11, 9, 7, 9, 11, 9, 11,
    9, 7, 9, 7, 5, 7, 9, 7, 9, 7, 5, 7, 5, 1, 5, 7, 5, 7, 9, 7, 9, 7, 5, 7, 9, 7, 9, 11, 9, 11, 9,
    7, 9, 11, 9, 11, 10, 7, 9, 7, 5, 7, 9, 7, 10, 11, 9, 11, 10, 7, 9, 11, 9, 11,
];

const SPECTRUM_CODEBOOK1_CODES: [u32; 81] = [
    0x7f8, 0x1f1, 0x7fd, 0x3f5, 0x068, 0x3f0, 0x7f7, 0x1ec, 0x7f5, 0x3f1, 0x072, 0x3f4, 0x074,
    0x011, 0x076, 0x1eb, 0x06c, 0x3f6, 0x7fc, 0x1e1, 0x7f1, 0x1f0, 0x061, 0x1f6, 0x7f2, 0x1ea,
    0x7fb, 0x1f2, 0x069, 0x1ed, 0x077, 0x017, 0x06f, 0x1e6, 0x064, 0x1e5, 0x067, 0x015, 0x062,
    0x012, 0x000, 0x014, 0x065, 0x016, 0x06d, 0x1e9, 0x063, 0x1e4, 0x06b, 0x013, 0x071, 0x1e3,
    0x070, 0x1f3, 0x7fe, 0x1e7, 0x7f3, 0x1ef, 0x060, 0x1ee, 0x7f0, 0x1e2, 0x7fa, 0x3f3, 0x06a,
    0x1e8, 0x075, 0x010, 0x073, 0x1f4, 0x06e, 0x3f7, 0x7f6, 0x1e0, 0x7f9, 0x3f2, 0x066, 0x1f5,
    0x7ff, 0x1f7, 0x7f4,
];

#[derive(Debug, Clone)]
pub struct AacSpec {
    /// 44.1 kHz only (sampling frequency index 4).
    pub sample_rate: u32,
    pub edge_hz: f64,
    pub bitrate_kbps: u32,
    /// Number of AAC frames (raw data blocks).
    pub frames: usize,
    /// AudioSpecificConfig object type: 2 (LC) or 5 (explicit SBR signalling, HE-AAC).
    pub object_type: u8,
    pub seed: u64,
}

/// "30 s" in AAC frames: 1292 at 44.1 kHz.
pub fn aac_frames(seconds: f64, sample_rate: u32) -> usize {
    (seconds * f64::from(sample_rate) / 1024.0).round() as usize
}

/// First coefficient that is zero: `round(edge_hz / bin_hz)` with `bin_hz = sample_rate / 2 / 1024`.
pub fn aac_edge_bin(edge_hz: f64, sample_rate: u32) -> usize {
    (edge_hz / (f64::from(sample_rate) / 2.0 / 1024.0)).round() as usize
}

/// The coded edge in Hz.
pub fn aac_coded_edge_hz(edge_hz: f64, sample_rate: u32) -> f64 {
    aac_edge_bin(edge_hz, sample_rate) as f64 * f64::from(sample_rate) / 2.0 / 1024.0
}

/// Fill elements that bring `bits` bits to the end of the frame: sizes of the payloads in bytes.
/// A fill element holds up to 269 payload bytes; its size is `7 + 8 c` bits for `c <= 14` and
/// `15 + 8 c` bits for `c >= 15` (the count escape). The plan leaves 0 to 7 bits of padding after the
/// end element.
fn fill_plan(bits: usize) -> Vec<usize> {
    const MAX_SIZE: usize = 15 + 8 * 269;
    let mut plan = Vec::new();
    let mut rest = bits;
    while rest >= MAX_SIZE + 8 {
        plan.push(269);
        rest -= MAX_SIZE;
    }
    match rest {
        0..=7 => {}
        8..=126 => plan.push((rest - 7) / 8),
        127..=134 => {
            // Neither one short nor one long element fits: two short ones.
            let total = (rest - 14) / 8;
            plan.push(total / 2);
            plan.push(total - total / 2);
        }
        _ => plan.push((rest - 15) / 8),
    }
    plan
}

/// One raw data block: a `single_channel_element`, fill elements to the exact `frame_len` bytes,
/// then the end element.
fn aac_frame(rng: &mut ChaCha8Rng, edge_bin: usize, max_sfb: usize, frame_len: usize) -> Vec<u8> {
    let mut w = BitWriter::new();
    w.put(0, 3); // ID_SCE
    w.put(0, 4); // element_instance_tag
    w.put(u32::from(AAC_GLOBAL_GAIN), 8);
    w.put(0, 1); // ics_reserved_bit
    w.put(0, 2); // ONLY_LONG_SEQUENCE
    w.put(0, 1); // window_shape
    w.put(max_sfb as u32, 6);
    w.put(0, 1); // predictor_data_present
    for _ in 0..max_sfb {
        w.put(1, 4); // sect_cb: spectral codebook 1
        w.put(1, 5); // sect_len_incr: one band
    }
    for _ in 0..max_sfb {
        w.put(0, 1); // scale factor difference 0: index 60 of the scale-factor book, 1 bit
    }
    w.put(0, 1); // pulse_data_present
    w.put(0, 1); // tns_data_present
    w.put(0, 1); // gain_control_data_present
    for band in 0..max_sfb {
        for quad in (SWB_OFFSET_LONG_48K[band]..SWB_OFFSET_LONG_48K[band + 1]).step_by(4) {
            let mut index = 0usize;
            for bin in quad..quad + 4 {
                let value = if bin < edge_bin { coefficient(rng) } else { 0 };
                index = index * 3 + (value + 1) as usize;
            }
            w.put(
                SPECTRUM_CODEBOOK1_CODES[index],
                u32::from(SPECTRUM_CODEBOOK1_LENS[index]),
            );
        }
    }
    let remaining = (frame_len * 8)
        .checked_sub(w.bit_len() + 3)
        .unwrap_or_else(|| {
            panic!(
                "{} bits do not fit a {frame_len}-byte frame",
                w.bit_len() + 3
            )
        });
    for count in fill_plan(remaining) {
        w.put(6, 3); // ID_FIL
        if count <= 14 {
            w.put(count as u32, 4);
        } else {
            w.put(15, 4);
            w.put((count - 14) as u32, 8);
        }
        for _ in 0..count {
            w.put(0, 8);
        }
    }
    w.put(7, 3); // ID_END
    let mut bytes = w.into_bytes();
    assert!(bytes.len() <= frame_len);
    bytes.resize(frame_len, 0);
    bytes
}

fn descriptor(tag: u8, payload: &[u8]) -> Vec<u8> {
    assert!(payload.len() < 128);
    let mut out = vec![tag, payload.len() as u8];
    out.extend_from_slice(payload);
    out
}

/// AudioSpecificConfig of a 44.1 kHz mono stream (ISO/IEC 14496-3 1.6.2.1).
fn audio_specific_config(object_type: u8) -> Vec<u8> {
    let mut w = BitWriter::new();
    match object_type {
        2 => {
            w.put(2, 5);
            w.put(4, 4); // 44100 Hz
            w.put(1, 4); // one channel
            w.put(0, 3); // frameLengthFlag, dependsOnCoreCoder, extensionFlag
        }
        5 => {
            w.put(5, 5);
            w.put(4, 4); // core sampling frequency index
            w.put(1, 4);
            w.put(4, 4); // extensionSamplingFrequencyIndex
            w.put(2, 5); // underlying object type: AAC-LC
            w.put(0, 3);
        }
        other => panic!("object type {other} is not written"),
    }
    w.into_bytes()
}

/// M4A with AAC-LC frames of a given edge and bitrate (lossy-fixtures.md 1.2).
pub fn write_m4a_aac(spec: &AacSpec) -> Vec<u8> {
    assert_eq!(spec.sample_rate, 44_100);
    let mut rng = ChaCha8Rng::seed_from_u64(spec.seed);
    let edge_bin = aac_edge_bin(spec.edge_hz, spec.sample_rate);
    let max_sfb = (0..SWB_OFFSET_LONG_48K.len())
        .find(|b| SWB_OFFSET_LONG_48K[*b] >= edge_bin)
        .unwrap();
    // Frame `n` has exactly floor((n + 1) · B) - floor(n · B) bytes, B = bitrate · 1024 / (8 · rate).
    let numerator = u128::from(spec.bitrate_kbps) * 1000 * 1024;
    let denominator = 8 * u128::from(spec.sample_rate);
    let total_to = |n: usize| (n as u128 * numerator / denominator) as u32;
    let mut sizes = Vec::with_capacity(spec.frames);
    let mut mdat = Vec::new();
    for n in 0..spec.frames {
        let len = (total_to(n + 1) - total_to(n)) as usize;
        sizes.push(len as u32);
        mdat.extend_from_slice(&aac_frame(&mut rng, edge_bin, max_sfb, len));
    }

    // esds: ES_Descriptor > DecoderConfigDescriptor > DecoderSpecificInfo, SLConfigDescriptor.
    let asc = descriptor(0x05, &audio_specific_config(spec.object_type));
    let mut dec_config = vec![0x40, 0x15, 0, 0, 0]; // AAC, audio stream, buffer size
    dec_config.extend_from_slice(&be32(spec.bitrate_kbps * 1000)); // max bitrate
    dec_config.extend_from_slice(&be32(spec.bitrate_kbps * 1000)); // average bitrate
    dec_config.extend_from_slice(&asc);
    let mut es = be16(1).to_vec(); // ES_ID
    es.push(0); // flags
    es.extend_from_slice(&descriptor(0x04, &dec_config));
    es.extend_from_slice(&descriptor(0x06, &[0x02]));
    let esds = full_box(b"esds", 0, 0, &descriptor(0x03, &es));
    write_mp4(&Mp4Track {
        sample_entry: audio_sample_entry(b"mp4a", spec.sample_rate, 16, &esds),
        sample_rate: spec.sample_rate,
        samples_per_frame: 1024,
        sample_sizes: sizes,
        mdat,
    })
}

// ---- ALAC in M4A -------------------------------------------------------------------------------

/// M4A with ALAC frames of 4096 samples, **uncompressed** (escape flag set, samples verbatim),
/// 16-bit mono. `samples.len()` must be a multiple of 4096.
pub fn write_m4a_alac(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    const FRAME: usize = 4096;
    assert_eq!(samples.len() % FRAME, 0);
    let mut sizes = Vec::new();
    let mut mdat = Vec::new();
    for chunk in samples.chunks(FRAME) {
        let mut w = BitWriter::new();
        w.put(0, 3); // ID_SCE
        w.put(0, 4); // element instance tag
        w.put(0, 12); // unused
        w.put(0, 1); // partial frame
        w.put(0, 2); // bytes shifted
        w.put(1, 1); // escape flag: uncompressed
        for s in chunk {
            w.put(u32::from(*s as u16), 16);
        }
        w.put(7, 3); // ID_END
        let bytes = w.into_bytes();
        sizes.push(bytes.len() as u32);
        mdat.extend_from_slice(&bytes);
    }
    let mut cookie = Vec::new();
    cookie.extend_from_slice(&be32(FRAME as u32));
    cookie.extend_from_slice(&[0, 16, 40, 10, 14, 1]); // version, bit depth, pb, mb, kb, channels
    cookie.extend_from_slice(&be16(255)); // max run
    cookie.extend_from_slice(&be32(FRAME as u32 * 2 + 8)); // max frame bytes
    cookie.extend_from_slice(&be32(0)); // average bitrate
    cookie.extend_from_slice(&be32(sample_rate));
    assert_eq!(cookie.len(), 24);
    let alac = full_box(b"alac", 0, 0, &cookie);
    write_mp4(&Mp4Track {
        sample_entry: audio_sample_entry(b"alac", sample_rate, 16, &alac),
        sample_rate,
        samples_per_frame: FRAME as u32,
        sample_sizes: sizes,
        mdat,
    })
}

// ---- Named fixtures (lossy-fixtures.md section 2) --------------------------------------------------

/// Names of the fixtures of section 2 that this module can build alone (the two ALAC fixtures need
/// a lossless signal and are built by `common::lossy_fixture`).
pub const BYTE_FIXTURES: [&str; 17] = [
    "mp3_320_lp20k5",
    "mp3_320_lp17k",
    "mp3_128_lp17k",
    "mp3_192_lp17k_notag",
    "mp3_320_lp18k5",
    "mp3_320_tag16k",
    "mp3_vbr_lp19k5",
    "mp3_abr_lp18k6",
    "mp3_320_48k_lp20k",
    "mp3_320_low_hf",
    "mp3_silence",
    "mp3_short",
    "mp3_corrupt",
    "m4a_aac_128_lp16k",
    "m4a_aac_256_lp15k3",
    "m4a_he_aac",
    "m4a_corrupt",
];

fn lame(lowpass_byte: u8, vbr_method: u8) -> LameTag {
    LameTag {
        encoder: "LAME3.100",
        vbr_method,
        lowpass_byte,
    }
}

fn cbr(
    kbps: u32,
    edge_hz: f64,
    tag_lowpass: Option<u8>,
    sample_rate: u32,
    seconds: f64,
    seed: u64,
) -> Mp3Spec {
    Mp3Spec {
        sample_rate,
        edge_hz: Some(edge_hz),
        frames: mp3_frames(seconds, sample_rate),
        bitrates: vec![kbps],
        tag: tag_lowpass.map(|b| TagSpec {
            kind: TagKind::Info,
            lame: Some(lame(b, 1)),
        }),
        gate: None,
        seed,
    }
}

/// The bytes of a fixture of section 2 (`None` for the ALAC and HE-AAC-free names this module does
/// not know). Deterministic.
pub fn fixture_bytes(name: &str) -> Option<Vec<u8>> {
    let mp3 = |spec: Mp3Spec| Some(write_mp3(&spec));
    let aac = |bitrate_kbps: u32, edge_hz: f64, object_type: u8, seed: u64| {
        Some(write_m4a_aac(&AacSpec {
            sample_rate: 44_100,
            edge_hz,
            bitrate_kbps,
            frames: aac_frames(30.0, 44_100),
            object_type,
            seed,
        }))
    };
    match name {
        "mp3_320_lp20k5" => mp3(cbr(320, 20_500.0, Some(205), 44_100, 30.0, 101)),
        "mp3_320_lp17k" => mp3(cbr(320, 17_000.0, Some(205), 44_100, 30.0, 102)),
        "mp3_128_lp17k" => mp3(cbr(128, 17_000.0, Some(170), 44_100, 30.0, 103)),
        "mp3_192_lp17k_notag" => mp3(cbr(192, 17_000.0, None, 44_100, 30.0, 104)),
        "mp3_320_lp18k5" => mp3(cbr(320, 18_500.0, Some(205), 44_100, 30.0, 105)),
        "mp3_320_tag16k" => mp3(cbr(320, 16_000.0, Some(160), 44_100, 30.0, 106)),
        "mp3_vbr_lp19k5" => mp3(Mp3Spec {
            bitrates: vec![224, 256],
            tag: Some(TagSpec {
                kind: TagKind::Xing,
                lame: Some(lame(195, 4)),
            }),
            ..cbr(224, 19_500.0, None, 44_100, 30.0, 107)
        }),
        "mp3_abr_lp18k6" => mp3(Mp3Spec {
            bitrates: vec![160, 224],
            tag: Some(TagSpec {
                kind: TagKind::Xing,
                lame: Some(lame(0, 2)),
            }),
            ..cbr(160, 18_600.0, None, 44_100, 30.0, 108)
        }),
        "mp3_320_48k_lp20k" => mp3(cbr(320, 20_000.0, Some(205), 48_000, 30.0, 109)),
        "mp3_320_low_hf" => mp3(cbr(320, 6_000.0, Some(205), 44_100, 30.0, 110)),
        "mp3_silence" => mp3(Mp3Spec {
            edge_hz: None,
            ..cbr(320, 0.0, None, 44_100, 30.0, 111)
        }),
        "mp3_short" => mp3(cbr(320, 20_500.0, Some(205), 44_100, 2.0, 101)),
        "mp3_corrupt" => {
            // 4 KiB of ASCII letters: no 0xFF byte (no frame sync) and no ID3 header.
            let mut rng = ChaCha8Rng::seed_from_u64(112);
            Some(
                (0..4096)
                    .map(|_| {
                        let n = rng.random_range(0..52u8);
                        if n < 26 {
                            b'A' + n
                        } else {
                            b'a' + n - 26
                        }
                    })
                    .collect(),
            )
        }
        "m4a_aac_128_lp16k" => aac(128, 16_000.0, 2, 201),
        "m4a_aac_256_lp15k3" => aac(256, 15_300.0, 2, 202),
        "m4a_he_aac" => aac(128, 16_000.0, 5, 203),
        "m4a_corrupt" => {
            // `m4a_aac_128_lp16k` cut in the middle of its `moov` box.
            let full = aac(128, 16_000.0, 2, 201)?;
            let (ftyp, moov) = mp4_header_len(&full);
            Some(full[..ftyp + moov / 2].to_vec())
        }
        _ => None,
    }
}
