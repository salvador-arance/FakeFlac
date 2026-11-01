//! Independent synthetic Ogg/Opus fixture writer (ADR-0047, lossy-fixtures.md §§1.3–1.4).
//! No production Ogg/Opus reader or CRC code is imported.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
use rusty_opus::{Application, Bandwidth, OpusDecoder, OpusEncoder};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

pub const ENCODED: [&str; 6] = [
    "opus_128_fb",
    "opus_128_fb_stereo",
    "opus_64_fb",
    "opus_160_src16k",
    "opus_160_swb",
    "opus_short",
];
#[derive(Clone)]
pub struct Encoded {
    pub channels: usize,
    pub samples: usize,
    pub kbps: u32,
    pub input: Vec<f32>,
    pub packets: Vec<Vec<u8>>,
}
pub fn encode(name: &str) -> Encoded {
    let channels = if name == "opus_128_fb_stereo" { 2 } else { 1 };
    let secs = if name == "opus_short" { 2.0 } else { 30.0 };
    let kbps = if name == "opus_64_fb" {
        64
    } else if name.starts_with("opus_160") {
        160
    } else {
        128
    };
    let lowpass = name == "opus_160_src16k";
    let n = (48000.0 * secs) as usize;
    let mut input = vec![0.0f32; n * channels];
    for ch in 0..channels {
        let seed = if lowpass { 2 } else { 1 } + ch as u64;
        let mut signal = super::shaped_noise(
            48000,
            secs,
            |f| if lowpass && f >= 16000.0 { 0.0 } else { 1.0 },
            seed,
        );
        super::set_rms_dbfs(&mut signal, -12.0);
        super::limit_peak(&mut signal, -1.0);
        for (i, x) in signal.into_iter().enumerate() {
            input[i * channels + ch] = x as f32;
        }
    }
    let mut encoder = OpusEncoder::new(48000, channels, Application::Audio).unwrap();
    encoder.bitrate_bps = kbps as i32 * 1000;
    encoder.max_bandwidth = if name == "opus_160_swb" {
        Bandwidth::Superwideband
    } else {
        Bandwidth::Fullband
    };
    let mut packets = Vec::new();
    let mut buffer = vec![0u8; 4000];
    for start in (0..n + 120).step_by(960) {
        let mut block = vec![0.0; 960 * channels];
        let end = (start + 960).min(n);
        if start < n {
            block[..(end - start) * channels]
                .copy_from_slice(&input[start * channels..end * channels]);
        }
        let len = encoder.encode(&block, 960, &mut buffer).unwrap();
        packets.push(buffer[..len].to_vec());
    }
    Encoded {
        channels,
        samples: n,
        kbps,
        input,
        packets,
    }
}
pub fn encoded(name: &str) -> Arc<Encoded> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<Encoded>>>> = OnceLock::new();
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    cache
        .entry(name.to_string())
        .or_insert_with(|| Arc::new(encode(name)))
        .clone()
}
/// Independent decoder oracle: decoder output, per channel, before mono mixing.
pub fn reference(e: &Encoded, pre_skip: usize) -> Vec<Vec<f32>> {
    let mut decoder = OpusDecoder::new(48000, e.channels).unwrap();
    let mut all = vec![Vec::new(); e.channels];
    let mut buf = vec![0f32; 5760 * e.channels];
    for p in &e.packets {
        let n = decoder.decode(p, 5760, &mut buf).unwrap();
        for frame in buf[..n * e.channels].chunks_exact(e.channels) {
            for (ch, x) in frame.iter().enumerate() {
                all[ch].push(*x);
            }
        }
    }
    all.into_iter()
        .map(|s| s[pre_skip..e.samples + 120].to_vec())
        .collect()
}
#[derive(Clone, Debug)]
pub struct Page {
    pub flags: u8,
    pub granule: i64,
    pub serial: u32,
    pub seq: u32,
    pub segments: Vec<u8>,
    pub body: Vec<u8>,
}
fn page(packets: &[Vec<u8>], flags: u8, granule: i64) -> Page {
    let mut segments = Vec::new();
    let mut body = Vec::new();
    for packet in packets {
        segments.extend(std::iter::repeat_n(255, packet.len() / 255));
        segments.push((packet.len() % 255) as u8);
        body.extend(packet);
    }
    Page {
        flags,
        granule,
        serial: 0x5446_4b46,
        seq: 0,
        segments,
        body,
    }
}
pub fn pages(e: &Encoded, split: bool) -> Vec<Page> {
    let mut head = b"OpusHead".to_vec();
    head.extend([1, e.channels as u8]);
    head.extend(120u16.to_le_bytes());
    head.extend(48000u32.to_le_bytes());
    head.extend(0i16.to_le_bytes());
    head.push(0);
    let mut tags = b"OpusTags".to_vec();
    tags.extend(13u32.to_le_bytes());
    tags.extend(b"fakeflac-test");
    tags.extend(0u32.to_le_bytes());
    let mut out = vec![page(&[head], 2, 0), page(&[tags], 0, 0)];
    if split {
        let mut current = page(&[], 0, -1);
        let mut completed = 0;
        for packet in &e.packets {
            let mut offset = 0;
            loop {
                let len = (packet.len() - offset).min(255);
                if current.body.len() + len > 1000 || current.segments.len() == 255 {
                    let continued = current.segments.last() == Some(&255);
                    out.push(current);
                    current = page(&[], u8::from(continued), -1);
                }
                current.segments.push(len as u8);
                current
                    .body
                    .extend_from_slice(&packet[offset..offset + len]);
                offset += len;
                if len < 255 {
                    completed += 1;
                    current.granule = completed * 960;
                    break;
                }
            }
        }
        out.push(current);
    } else {
        for (i, chunk) in e.packets.chunks(50).enumerate() {
            out.push(page(chunk, 0, ((i * 50 + chunk.len()) * 960) as i64));
        }
    }
    let last = out.last_mut().unwrap();
    last.flags |= 4;
    last.granule = (e.samples + 120) as i64;
    for (i, p) in out.iter_mut().enumerate() {
        p.seq = i as u32;
    }
    out
}
pub fn crc(bytes: &[u8]) -> u32 {
    let mut c = 0u32;
    for b in bytes {
        c ^= u32::from(*b) << 24;
        for _ in 0..8 {
            c = if c & 0x80000000 != 0 {
                (c << 1) ^ 0x04c11db7
            } else {
                c << 1
            };
        }
    }
    c
}
pub fn serialize(p: &Page) -> Vec<u8> {
    assert!(p.segments.len() <= 255);
    let mut b = b"OggS".to_vec();
    b.extend([0, p.flags]);
    b.extend(p.granule.to_le_bytes());
    b.extend(p.serial.to_le_bytes());
    b.extend(p.seq.to_le_bytes());
    b.extend([0u8; 4]);
    b.push(p.segments.len() as u8);
    b.extend(&p.segments);
    b.extend(&p.body);
    let sum = crc(&b);
    b[22..26].copy_from_slice(&sum.to_le_bytes());
    b
}
pub fn bytes(name: &str) -> Vec<u8> {
    if name == "opus_not_ogg" {
        return vec![b'A'; 4096];
    }
    let base = if ENCODED.contains(&name) {
        name
    } else {
        "opus_128_fb"
    };
    let e = encoded(base);
    let mut packets = e.as_ref().clone();
    if name == "opus_bad_packet" {
        packets.packets[100] = vec![0xfb, 0];
    }
    let mut p = pages(
        &packets,
        name == "opus_128_fb_split"
            || name == "opus_continuation"
            || name == "opus_unfinished_packet",
    );
    match name {
        "opus_128_fb_preskip" => p[0].body[10..12].copy_from_slice(&1080u16.to_le_bytes()),
        "opus_128_fb_rate44k" => p[0].body[12..16].copy_from_slice(&44100u32.to_le_bytes()),
        "opus_128_fb_gain_m20" => p[0].body[16..18].copy_from_slice(&(-5120i16).to_le_bytes()),
        "opus_128_fb_gain_m60" => p[0].body[16..18].copy_from_slice(&(-15360i16).to_le_bytes()),
        "opus_sequence_gap" => {
            p.remove(5);
        }
        "opus_no_bos" => p[0].flags = 0,
        "opus_after_eos" => {
            let mut last = p.last().unwrap().clone();
            last.seq += 1;
            p.push(last);
        }
        "opus_continuation" => {
            let i = p.iter().position(|p| p.flags & 1 != 0).unwrap();
            p[i].flags &= !1;
        }
        "opus_unfinished_packet" => {
            let i = p
                .iter()
                .position(|p| p.segments.last() == Some(&255))
                .unwrap();
            p.truncate(i + 1);
        }
        "opus_no_head" => p[0].body[7] = b'X',
        "opus_head_version" => p[0].body[8] = 0x10,
        "opus_head_channels" => p[0].body[9] = 3,
        "opus_head_only" => p.truncate(1),
        "opus_no_tags" => p[1].body[7] = b'z',
        "opus_tags_overrun" => {
            let n = p[1].body.len() as u32;
            p[1].body[8..12].copy_from_slice(&n.to_le_bytes());
        }
        "opus_tags_shared_page" => {
            let mut v = vec![p[1].body.clone(), e.packets[0].clone()];
            p[1] = page(&v, 0, 960);
            p[1].seq = 1;
            v.clear();
        }
        "opus_no_audio" => {
            p.truncate(2);
            p[1].flags = 4;
        }
        "opus_granule_decreases" => p[4].granule = p[3].granule - 960,
        "opus_granule_below_preskip" => {
            p.truncate(3);
            p[2].flags = 4;
            p[2].granule = 119;
        }
        "opus_granule_beyond_audio" => {
            p.last_mut().unwrap().granule = (e.packets.len() * 960 + 1) as i64
        }
        "opus_serial_change" => p[3].serial += 1,
        "opus_family1_6ch" | "opus_family255_3ch" => {
            let (ch, family, streams, coupled) = if name == "opus_family1_6ch" {
                (6, 1, 4, 2)
            } else {
                (3, 255, 3, 0)
            };
            p[0].body[9] = ch;
            p[0].body[18] = family;
            p[0].body.extend([streams, coupled]);
            p[0].body.extend(0..ch);
            p[0].segments = vec![p[0].body.len() as u8];
        }
        _ => {}
    }
    let mut out = Vec::new();
    for (i, page) in p.iter().enumerate() {
        let mut b = serialize(page);
        if name == "opus_bad_crc" && i == 4 {
            let last = b.len() - 1;
            b[last] ^= 1;
        }
        if name == "opus_page_version" && i == 3 {
            b[4] = 1;
            b[22..26].fill(0);
            let c = crc(&b);
            b[22..26].copy_from_slice(&c.to_le_bytes());
        }
        if name == "opus_truncated_page" && i == 6 {
            b.truncate(27 + page.segments.len() + page.body.len() / 2);
            out.extend(b);
            break;
        }
        out.extend(b);
    }
    out
}
pub fn fixture(name: &str) -> PathBuf {
    let dir = super::fixture_dir().join("opus");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(format!("{name}.opus"));
    if !p.exists() {
        let tmp = p.with_extension(format!("{}.tmp", std::process::id()));
        std::fs::write(&tmp, bytes(name)).unwrap();
        let _ = std::fs::hard_link(&tmp, &p);
        let _ = std::fs::remove_file(tmp);
    }
    p
}
