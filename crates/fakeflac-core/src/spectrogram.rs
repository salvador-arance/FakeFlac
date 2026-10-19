//! Spectrogram of one track (RF-07, ADR-0009): a u8 dB matrix, row-major by time.

/// `data[t * height + f]` is the byte of time bin `t` and frequency bin `f` (0 Hz at `f = 0`,
/// `max_frequency_hz` at the top). Byte 0 is `db_range[0]` and 255 is `db_range[1]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Spectrogram {
    pub width: u32,
    pub height: u32,
    pub sample_rate: u32,
    pub max_frequency_hz: f64,
    pub db_range: [f64; 2],
    pub data: Vec<u8>,
}
