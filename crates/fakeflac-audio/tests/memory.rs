//! @RNF-02, @RNF-04, @RNF-09: `specs/features/memory.feature` and memory.md §§3–10.
//! One serial driver owns every fixture and child, including under nextest. OS peaks are
//! Windows-only; fixtures, chunks, pure checks and wiring are portable. ADR-0050, std only.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::memory as writers;
use fakeflac_audio::{decode_chunks_for_test, ChunkStats, SymphoniaAnalyzer};
use fakeflac_core::ports::SpectrumAnalyzer;
use fakeflac_core::{DetectionParams, TrackError};
use std::fmt::Write as _;
use std::io::Read;
use std::path::{Path, PathBuf};

const MIB: u64 = 1_048_576;
const CEILING: u64 = 32 * MIB;

fn check_growth(id: &str, metric: &str, short: u64, long: u64, bound: u64) -> Result<(), String> {
    let growth = long.saturating_sub(short);
    if growth <= bound {
        Ok(())
    } else {
        Err(format!(
            "memory: {id} {metric} grew by {growth} bytes, bound {bound}"
        ))
    }
}
fn check_ceiling(
    id: &str,
    metric: &str,
    peak: u64,
    baseline: u64,
    ceiling: u64,
) -> Result<(), String> {
    let over = peak.saturating_sub(baseline);
    if over <= ceiling {
        Ok(())
    } else {
        Err(format!(
            "memory: {id} {metric} is {over} bytes over the baseline, ceiling {ceiling}"
        ))
    }
}
fn check_fixture_growth(
    case: &str,
    short_bytes: u64,
    long_bytes: u64,
    min: u64,
) -> Result<(), String> {
    let growth = long_bytes.saturating_sub(short_bytes);
    if growth >= min {
        Ok(())
    } else {
        Err(format!(
            "memory: fixture {case} grows by {growth} bytes, need at least {min}"
        ))
    }
}
fn parse_peaks(text: &str) -> Result<(u64, u64), String> {
    let parse = || {
        let (ws, private) = text.trim().split_once(' ')?;
        if ws.is_empty()
            || private.is_empty()
            || !ws.bytes().all(|b| b.is_ascii_digit())
            || !private.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        Some((ws.parse().ok()?, private.parse().ok()?))
    };
    parse().ok_or_else(|| format!("memory: cannot read the peaks from \"{text}\""))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Peaks {
    private: u64,
    ws: u64,
}
impl Peaks {
    fn metrics(self) -> [(&'static str, u64); 2] {
        [("private", self.private), ("working-set", self.ws)]
    }
}
#[derive(Clone)]
struct Row {
    case: &'static str,
    op: &'static str,
    length: &'static str,
    file: u64,
    result: String,
    max_chunk: Option<u64>,
    peaks: Peaks,
    baseline: Peaks,
    baseline_repeat: Peaks,
    ceiling: u64,
}
#[derive(Clone)]
struct Pair {
    case: &'static str,
    op: &'static str,
    short: Peaks,
    long: Peaks,
}

/// A8: build every row before deciding whether the suite fails. No measurement retry.
fn report(
    suite: &str,
    baseline: Peaks,
    repeat: Peaks,
    rows: &[Row],
    pairs: &[Pair],
) -> (String, bool) {
    let build = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let mut out = format!("RNF-02 memory report (specs/contracts/memory.md)\nbuild: {build}\nsuite: {suite}\nbaseline: private {} B, working-set {} B | repeat: private {} B, working-set {} B\n", baseline.private, baseline.ws, repeat.private, repeat.ws);
    if suite == "long" {
        let mut seen = Vec::new();
        for row in rows {
            if !seen.contains(&row.case) {
                seen.push(row.case);
                writeln!(out, "baseline {}: private {} B, working-set {} B | repeat: private {} B, working-set {} B", row.case, row.baseline.private, row.baseline.ws, row.baseline_repeat.private, row.baseline_repeat.ws).unwrap();
            }
        }
    }
    out.push_str("| case | op | length | file B | result | max chunk frames | private B | working-set B | private over baseline B | working-set over baseline B | ceiling B | ceiling |\n|---|---|---|---|---|---|---|---|---|---|---|---|\n");
    let mut failures = Vec::new();
    for row in rows {
        let id = format!("{} {}", row.case, row.op);
        let mut pass = true;
        for ((metric, peak), (_, base)) in
            row.peaks.metrics().into_iter().zip(row.baseline.metrics())
        {
            if check_ceiling(&id, metric, peak, base, row.ceiling).is_err() {
                failures.push(format!("{id} ceiling {metric}"));
                pass = false;
            }
        }
        writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            row.case,
            row.op,
            row.length,
            row.file,
            row.result,
            row.max_chunk.map_or_else(|| "-".into(), |n| n.to_string()),
            row.peaks.private,
            row.peaks.ws,
            row.peaks.private.saturating_sub(row.baseline.private),
            row.peaks.ws.saturating_sub(row.baseline.ws),
            row.ceiling,
            if pass { "pass" } else { "FAIL" }
        )
        .unwrap();
    }
    out.push_str("| case | op | metric | short B | long B | growth B | bound B | growth |\n|---|---|---|---|---|---|---|---|\n");
    for pair in pairs {
        let id = format!("{} {}", pair.case, pair.op);
        for ((metric, short), (_, long)) in
            pair.short.metrics().into_iter().zip(pair.long.metrics())
        {
            let pass = check_growth(&id, metric, short, long, MIB).is_ok();
            if !pass {
                failures.push(format!("{id} growth {metric}"));
            }
            writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} | {} | {} |",
                pair.case,
                pair.op,
                metric,
                short,
                long,
                long.saturating_sub(short),
                MIB,
                if pass { "pass" } else { "FAIL" }
            )
            .unwrap();
        }
    }
    let pass = failures.is_empty();
    if pass {
        out.push_str("result: pass\n");
    } else {
        writeln!(out, "result: FAIL ({})", failures.join(", ")).unwrap();
    }
    (out, pass)
}

#[derive(Debug)]
struct Measurement {
    result: String,
    duration: u64,
    width: u32,
    height: u32,
    peaks: Peaks,
}

#[cfg(windows)]
fn command_output(
    mut command: std::process::Command,
    timeout: std::time::Duration,
) -> std::process::Output {
    use std::process::Stdio;
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .expect("memory: cannot start measurement process");
    let start = std::time::Instant::now();
    loop {
        if child
            .try_wait()
            .expect("memory: cannot wait for measurement process")
            .is_some()
        {
            return child
                .wait_with_output()
                .expect("memory: cannot read child output");
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            let output = child
                .wait_with_output()
                .expect("memory: collect timed out child");
            panic!(
                "memory: measurement timed out: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[cfg(windows)]
fn peaks_of_self() -> Peaks {
    let pid = std::process::id();
    let mut command = std::process::Command::new("powershell.exe");
    command.args(["-NoProfile", "-NonInteractive", "-Command"]).arg(format!("$p = Get-Process -Id {pid}; '{{0}} {{1}}' -f $p.PeakWorkingSet64, $p.PeakPagedMemorySize64"));
    let output = command_output(command, std::time::Duration::from_secs(30));
    assert!(
        output.status.success(),
        "memory: peaks command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = std::str::from_utf8(&output.stdout).expect("memory: peaks output is not UTF-8");
    let (ws, private) = parse_peaks(text).unwrap();
    Peaks { private, ws }
}

fn error_code(error: TrackError) -> &'static str {
    match error {
        TrackError::CorruptStream(_) => "corruptStream",
        TrackError::TooShort => "tooShort",
        TrackError::Unreadable(_) => "unreadable",
        TrackError::UnsupportedFormat(_) => "unsupportedFormat",
    }
}

fn one_call(op: &str, path: Option<&Path>) -> Measurement {
    let mut m = Measurement {
        result: "ok".into(),
        duration: 0,
        width: 0,
        height: 0,
        peaks: Peaks::default(),
    };
    let params = DetectionParams::default();
    match op {
        "baseline" => {}
        "analyze" => match SymphoniaAnalyzer.analyze(path.unwrap(), &params) {
            Ok(probe) => {
                m.duration = probe.duration_ms;
                if m.duration == 0 {
                    assert!(probe.spectrum.is_none());
                }
            }
            Err(error) => m.result = error_code(error).into(),
        },
        "spectrogram" => match SymphoniaAnalyzer.spectrogram(path.unwrap(), &params.spectrogram) {
            Ok(s) => {
                m.width = s.width;
                m.height = s.height;
            }
            Err(error) => m.result = error_code(error).into(),
        },
        _ => panic!("memory: unknown operation"),
    }
    m
}

// @RNF-02: "Each measurement runs in a process of its own", "Unreadable peaks fail the suite".
#[test]
#[ignore = "child entry point, started only by the serial drivers"]
fn memory_child() {
    let Some(op) = std::env::var_os("FAKEFLAC_MEMORY_OP") else {
        return;
    };
    let path = std::env::var_os("FAKEFLAC_MEMORY_PATH").map(PathBuf::from);
    #[allow(unused_mut, unused_variables)]
    let mut m = one_call(op.to_str().expect("operation is ASCII"), path.as_deref());
    #[cfg(windows)]
    {
        m.peaks = peaks_of_self();
    }
    #[cfg(not(windows))]
    {
        panic!("OS memory measurement is Windows-only");
    }
    #[allow(unreachable_code)]
    {
        println!(
            "\nMEMORY|{}|{}|{}|{}|{}|{}",
            m.result, m.duration, m.width, m.height, m.peaks.ws, m.peaks.private
        );
    }
}

fn parse_measurement(stdout: &[u8], stderr: &[u8], success: bool) -> Measurement {
    let diagnostic = || {
        format!(
            "memory: child failed or printed no valid result\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(stdout),
            String::from_utf8_lossy(stderr)
        )
    };
    assert!(success, "{}", diagnostic());
    let text = std::str::from_utf8(stdout).unwrap_or_else(|_| panic!("{}", diagnostic()));
    let lines: Vec<_> = text
        .lines()
        .filter(|line| line.starts_with("MEMORY|"))
        .collect();
    assert_eq!(lines.len(), 1, "{}", diagnostic());
    let fields: Vec<_> = lines[0].split('|').collect();
    assert_eq!(fields.len(), 7, "{}", diagnostic());
    assert!(
        [
            "ok",
            "corruptStream",
            "tooShort",
            "unreadable",
            "unsupportedFormat"
        ]
        .contains(&fields[1]),
        "{}",
        diagnostic()
    );
    let number = |text: &str| {
        assert!(
            !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()),
            "{}",
            diagnostic()
        );
        text.parse::<u64>()
            .unwrap_or_else(|_| panic!("{}", diagnostic()))
    };
    Measurement {
        result: fields[1].into(),
        duration: number(fields[2]),
        width: u32::try_from(number(fields[3])).unwrap_or_else(|_| panic!("{}", diagnostic())),
        height: u32::try_from(number(fields[4])).unwrap_or_else(|_| panic!("{}", diagnostic())),
        peaks: Peaks {
            ws: number(fields[5]),
            private: number(fields[6]),
        },
    }
}

#[cfg(windows)]
fn measure(op: &str, path: Option<&Path>) -> Measurement {
    let mut command = std::process::Command::new(std::env::current_exe().expect("test binary"));
    command
        .args([
            "--exact",
            "memory_child",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("FAKEFLAC_MEMORY_OP", op)
        .env_remove("FAKEFLAC_MEMORY_PATH");
    if let Some(path) = path {
        command.env("FAKEFLAC_MEMORY_PATH", path.as_os_str());
    }
    let output = command_output(command, std::time::Duration::from_secs(600));
    parse_measurement(&output.stdout, &output.stderr, output.status.success())
}

struct Root {
    root: PathBuf,
    dir: PathBuf,
}
impl Root {
    fn new(driver: &str) -> Self {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("memory")
            .join(format!("{driver}-{}", std::process::id()));
        if root.exists() {
            std::fs::remove_dir_all(&root).unwrap();
        }
        let this = Self {
            dir: root.join("Memoria ″Ñandú″"),
            root,
        };
        std::fs::create_dir_all(&this.dir).unwrap();
        this
    }
    fn empty(&self) {
        assert!(
            std::fs::read_dir(&self.dir).unwrap().next().is_none(),
            "one case group at a time"
        );
    }
    fn path(&self, n: usize, ext: &str) -> PathBuf {
        self.dir.join(format!("{n:02} – Canción ‘x’.{ext}"))
    }
    fn clear(&self) {
        for entry in std::fs::read_dir(&self.dir).unwrap() {
            std::fs::remove_file(entry.unwrap().path()).unwrap();
        }
        self.empty();
    }
}
impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Wav,
    Flac,
    Mp3,
    Aac,
    Alac,
    Opus,
}
#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    kind: Kind,
    short: u64,
    long: u64,
    rate: u32,
    bits: u8,
    ceiling: u64,
}
impl Case {
    fn ext(self) -> &'static str {
        match self.kind {
            Kind::Wav => "wav",
            Kind::Flac => "flac",
            Kind::Mp3 => "mp3",
            Kind::Aac | Kind::Alac => "m4a",
            Kind::Opus => "opus",
        }
    }
    fn channels(self) -> u8 {
        if matches!(self.kind, Kind::Mp3 | Kind::Aac | Kind::Alac) {
            1
        } else {
            2
        }
    }
    fn frame_tolerance(self) -> u64 {
        match self.kind {
            Kind::Mp3 => 1152,
            Kind::Aac => 1024,
            _ => u64::from(self.rate / 1000),
        }
    }
    fn duration_tolerance(self) -> u64 {
        match self.kind {
            Kind::Mp3 => 27,
            Kind::Aac => 24,
            _ => 1,
        }
    }
    fn write(self, path: &Path, frames: u64) {
        match self.kind {
            Kind::Wav => writers::write_wav(path, frames),
            Kind::Flac => writers::write_flac(
                path,
                frames as usize / 4096,
                self.rate,
                self.bits.into(),
                if self.bits == 24 { -45.0 } else { -42.0 },
            ),
            Kind::Mp3 => writers::write_mp3_length(path, frames as usize / 1152),
            Kind::Aac => writers::write_aac_length(path, frames as usize / 1024),
            Kind::Alac => writers::write_alac_length(path, frames as usize),
            Kind::Opus => common::opus::write_length(path, frames as usize),
        }
    }
}
const GATE: [Case; 6] = [
    Case {
        name: "wav",
        kind: Kind::Wav,
        short: 1_323_000,
        long: 13_230_000,
        rate: 44_100,
        bits: 16,
        ceiling: CEILING,
    },
    Case {
        name: "flac",
        kind: Kind::Flac,
        short: 322 * 4096,
        long: 3229 * 4096,
        rate: 44_100,
        bits: 16,
        ceiling: CEILING,
    },
    Case {
        name: "mp3",
        kind: Kind::Mp3,
        short: 1148 * 1152,
        long: 11_484 * 1152,
        rate: 44_100,
        bits: 0,
        ceiling: CEILING,
    },
    Case {
        name: "m4a_aac",
        kind: Kind::Aac,
        short: 1292 * 1024,
        long: 12_920 * 1024,
        rate: 44_100,
        bits: 0,
        ceiling: CEILING,
    },
    Case {
        name: "m4a_alac",
        kind: Kind::Alac,
        short: 323 * 4096,
        long: 3229 * 4096,
        rate: 44_100,
        bits: 16,
        ceiling: CEILING,
    },
    Case {
        name: "opus",
        kind: Kind::Opus,
        short: 1_440_000,
        long: 14_400_000,
        rate: 48_000,
        bits: 0,
        ceiling: CEILING,
    },
];

fn be32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes(bytes[..4].try_into().unwrap())
}
fn be16(bytes: &[u8]) -> u16 {
    u16::from_be_bytes(bytes[..2].try_into().unwrap())
}

/// Independent container inspection, before any measured child. These checks use file headers,
/// never the production Opus reader. Exact frame/packet counts supplement the seam's tolerance.
fn validate_fixture(case: Case, path: &Path, frames: u64, unknown: bool) {
    match case.kind {
        Kind::Wav => {
            let reader = hound::WavReader::open(path).unwrap();
            let spec = reader.spec();
            assert_eq!(
                (spec.channels, spec.sample_rate, spec.bits_per_sample),
                (2, case.rate, 16)
            );
            assert_eq!(u64::from(reader.duration()), frames);
        }
        Kind::Flac => {
            let mut header = [0; 42];
            std::fs::File::open(path)
                .unwrap()
                .read_exact(&mut header)
                .unwrap();
            assert_eq!(&header[..4], b"fLaC");
            assert_eq!(header[4] & 0x7f, 0);
            assert_eq!(&header[5..8], &[0, 0, 34]);
            assert_eq!((be16(&header[8..]), be16(&header[10..])), (4096, 4096));
            let packed = u64::from_be_bytes(header[18..26].try_into().unwrap());
            assert_eq!(packed >> 44, u64::from(case.rate));
            assert_eq!(((packed >> 41) & 7) + 1, u64::from(case.channels()));
            assert_eq!(((packed >> 36) & 31) + 1, u64::from(case.bits));
            assert_eq!(packed & ((1 << 36) - 1), if unknown { 0 } else { frames });
        }
        Kind::Mp3 => {
            let bytes = std::fs::read(path).unwrap();
            assert_eq!(&bytes[21..25], b"Info");
            assert_eq!(u64::from(be32(&bytes[29..])) * 1152, frames);
            let mut at = 0;
            let mut packets = 0;
            while at < bytes.len() {
                let h = &bytes[at..at + 4];
                assert_eq!(&h[..2], &[0xff, 0xfb]);
                assert_eq!(h[2] >> 4, 14); // MPEG-1 Layer III 320 kbps
                assert_eq!((h[2] >> 2) & 3, 0); // 44100 Hz
                assert_eq!(h[3] >> 6, 3); // mono
                at += (144_000 * 320 / 44_100) + usize::from((h[2] >> 1) & 1);
                packets += 1;
            }
            assert_eq!(at, bytes.len());
            assert_eq!(packets, frames / 1152 + 1);
        }
        Kind::Aac | Kind::Alac => {
            let bytes = std::fs::read(path).unwrap();
            assert_eq!(&bytes[4..8], b"ftyp");
            let find = |tag: &[u8]| bytes.windows(4).position(|x| x == tag).unwrap();
            let mdhd = find(b"mdhd");
            assert_eq!(be32(&bytes[mdhd + 16..]), case.rate);
            assert_eq!(u64::from(be32(&bytes[mdhd + 20..])), frames);
            let stsz = find(b"stsz");
            assert_eq!(
                u64::from(be32(&bytes[stsz + 12..])),
                frames
                    / if matches!(case.kind, Kind::Aac) {
                        1024
                    } else {
                        4096
                    }
            );
            let entry = find(if matches!(case.kind, Kind::Aac) {
                b"mp4a"
            } else {
                b"alac"
            });
            assert_eq!(be16(&bytes[entry + 20..]), 1);
            assert_eq!(be32(&bytes[entry + 28..]) >> 16, case.rate);
            if matches!(case.kind, Kind::Alac) {
                assert_eq!(be16(&bytes[entry + 22..]), 16);
            }
            assert!(find(b"moov") < find(b"mdat"));
        }
        Kind::Opus => {
            let mut file = std::fs::File::open(path).unwrap();
            let mut packets = 0;
            let mut page = 0;
            let mut last_granule = 0;
            loop {
                let mut header = [0; 27];
                if file.read(&mut header[..1]).unwrap() == 0 {
                    break;
                }
                file.read_exact(&mut header[1..]).unwrap();
                assert_eq!(&header[..4], b"OggS");
                assert_eq!(header[4], 0);
                let mut lacing = vec![0; header[26] as usize];
                file.read_exact(&mut lacing).unwrap();
                let mut body = vec![0; lacing.iter().map(|n| usize::from(*n)).sum()];
                file.read_exact(&mut body).unwrap();
                let completed = lacing.iter().filter(|n| **n < 255).count();
                match page {
                    0 => {
                        assert!(body.starts_with(b"OpusHead"));
                        assert_eq!(body[9], case.channels());
                        assert_eq!(u16::from_le_bytes(body[10..12].try_into().unwrap()), 120);
                        assert_eq!(
                            u32::from_le_bytes(body[12..16].try_into().unwrap()),
                            case.rate
                        );
                    }
                    1 => assert!(body.starts_with(b"OpusTags")),
                    _ => {
                        assert!(completed <= 50);
                        packets += completed;
                    }
                }
                last_granule = u64::from_le_bytes(header[6..14].try_into().unwrap());
                page += 1;
            }
            assert_eq!(packets as u64, (frames + 120).div_ceil(960));
            assert_eq!(last_granule, frames + 120);
        }
    }
}

fn chunks(case: Case, path: &Path, frames: u64) -> ChunkStats {
    let stats = decode_chunks_for_test(path).unwrap();
    assert!(stats.max_chunk_frames <= 65_535, "{}: {stats:?}", case.name);
    assert!(
        stats.frames.abs_diff(frames) <= case.frame_tolerance(),
        "{}: {stats:?}, expected {frames}",
        case.name
    );
    if frames == 0 {
        assert_eq!(
            stats,
            ChunkStats {
                chunks: 0,
                frames: 0,
                max_chunk_frames: 0
            }
        );
    } else {
        assert!(stats.chunks > 0);
        assert!(stats.max_chunk_frames > 0);
    }
    stats
}

struct Suite {
    baseline: Peaks,
    repeat: Peaks,
    rows: Vec<Row>,
    pairs: Vec<Pair>,
}
impl Suite {
    fn new() -> Self {
        #[cfg(windows)]
        let (baseline, repeat) = (
            measure("baseline", None).peaks,
            measure("baseline", None).peaks,
        );
        #[cfg(not(windows))]
        let (baseline, repeat) = (Peaks::default(), Peaks::default());
        Self {
            baseline,
            repeat,
            rows: Vec::new(),
            pairs: Vec::new(),
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn call(
        &mut self,
        case: Case,
        op: &'static str,
        path: &Path,
        frames: u64,
        length: &'static str,
        max_chunk: Option<u64>,
        expected: &str,
    ) -> Peaks {
        #[cfg(windows)]
        let m = measure(op, Some(path));
        #[cfg(not(windows))]
        let m = one_call(op, Some(path));
        assert_eq!(m.result, expected, "{} {op}", case.name);
        if expected == "ok" {
            if op == "analyze" {
                assert!(
                    m.duration.abs_diff(frames * 1000 / u64::from(case.rate))
                        <= case.duration_tolerance(),
                    "{} duration {}",
                    case.name,
                    m.duration
                );
            } else {
                assert_eq!((m.width, m.height), (1200, 512));
            }
        }
        self.rows.push(Row {
            case: case.name,
            op,
            length,
            file: path.metadata().unwrap().len(),
            result: m.result,
            max_chunk,
            peaks: m.peaks,
            baseline: self.baseline,
            baseline_repeat: self.repeat,
            ceiling: case.ceiling,
        });
        m.peaks
    }
    fn pair(
        &mut self,
        case: Case,
        paths: &[PathBuf; 2],
        long_label: &'static str,
        spectrogram: bool,
        unknown: bool,
    ) {
        for (path, frames) in paths.iter().zip([case.short, case.long]) {
            validate_fixture(case, path, frames, unknown);
        }
        check_fixture_growth(
            case.name,
            paths[0].metadata().unwrap().len(),
            paths[1].metadata().unwrap().len(),
            4 * MIB,
        )
        .unwrap();
        let stats = [
            chunks(case, &paths[0], case.short),
            chunks(case, &paths[1], case.long),
        ];
        assert_eq!(
            stats[0].max_chunk_frames, stats[1].max_chunk_frames,
            "{} chunk shape",
            case.name
        );
        for op in if spectrogram {
            &['a', 's'][..]
        } else {
            &['a'][..]
        } {
            let op = if *op == 'a' { "analyze" } else { "spectrogram" };
            let short = self.call(
                case,
                op,
                &paths[0],
                case.short,
                "30 s",
                Some(stats[0].max_chunk_frames),
                "ok",
            );
            let long = self.call(
                case,
                op,
                &paths[1],
                case.long,
                long_label,
                Some(stats[1].max_chunk_frames),
                "ok",
            );
            self.pairs.push(Pair {
                case: case.name,
                op,
                short,
                long,
            });
        }
    }
    fn finish(mut self, suite: &str) {
        if suite == "gate" {
            let order = |case: &str, op: &str| match (case, op) {
                ("wav", _) => 0,
                ("flac", "analyze") => 1,
                ("flac_unknown_length", _) => 2,
                ("mp3", _) => 3,
                ("m4a_aac", _) => 4,
                ("m4a_alac", _) => 5,
                ("opus", _) => 6,
                ("flac", "spectrogram") => 7,
                ("flac_truncated", _) => 8,
                ("flac_huge_length", _) => 9,
                ("wav_empty", _) => 10,
                _ => unreachable!(),
            };
            self.rows.sort_by_key(|row| order(row.case, row.op));
            self.pairs.sort_by_key(|pair| order(pair.case, pair.op));
            assert_eq!(self.rows.len(), 19);
            assert_eq!(self.pairs.len(), 9);
        }
        #[cfg(windows)]
        {
            let (text, pass) = report(suite, self.baseline, self.repeat, &self.rows, &self.pairs);
            println!("{text}");
            assert!(pass, "memory: bounds failed (full report above)");
        }
        #[cfg(not(windows))]
        {
            let _ = suite;
            println!("RNF-02 portable fixture/chunk checks passed; OS peaks are Windows-only");
        }
    }
}

// @RNF-02: all non-long criterion, real-work, serial fixture, Unicode, empty and error scenarios.
// @RNF-04: "A truncated long track fails without growing memory", "A stream that declares a huge length allocates nothing for it".
// @RNF-09: "Unicode names are measured like any other name".
#[test]
fn memory_gate() {
    let start = std::time::Instant::now();
    let root = Root::new("gate");
    let mut suite = Suite::new();
    for case in GATE {
        root.empty();
        let paths = [root.path(1, case.ext()), root.path(2, case.ext())];
        case.write(&paths[0], case.short);
        case.write(&paths[1], case.long);
        suite.pair(case, &paths, "300 s", case.name == "flac", false);
        if case.name == "flac" {
            let unknown = Case {
                name: "flac_unknown_length",
                ..case
            };
            let derived = [root.path(3, "flac"), root.path(4, "flac")];
            for (source, target) in paths.iter().zip(&derived) {
                std::fs::copy(source, target).unwrap();
                writers::set_flac_total(target, 0);
            }
            suite.pair(unknown, &derived, "300 s", false, true);
            for path in &derived {
                std::fs::remove_file(path).unwrap();
            }
            let truncated = root.path(5, "flac");
            std::fs::copy(&paths[1], &truncated).unwrap();
            let half = paths[1].metadata().unwrap().len() / 2;
            std::fs::OpenOptions::new()
                .write(true)
                .open(&truncated)
                .unwrap()
                .set_len(half)
                .unwrap();
            validate_fixture(case, &truncated, case.long, false);
            assert!(matches!(
                decode_chunks_for_test(&truncated),
                Err(TrackError::CorruptStream(_))
            ));
            let peak = suite.call(
                Case {
                    name: "flac_truncated",
                    ..case
                },
                "analyze",
                &truncated,
                case.long,
                "300 s",
                None,
                "corruptStream",
            );
            let short = suite
                .rows
                .iter()
                .find(|r| r.case == "flac" && r.op == "analyze" && r.length == "30 s")
                .unwrap()
                .peaks;
            suite.pairs.push(Pair {
                case: "flac_truncated",
                op: "analyze",
                short,
                long: peak,
            });
            std::fs::remove_file(&truncated).unwrap();
            let huge = root.path(6, "flac");
            std::fs::copy(&paths[0], &huge).unwrap();
            writers::set_flac_total(&huge, (1 << 36) - 1);
            validate_fixture(case, &huge, (1 << 36) - 1, false);
            suite.call(
                Case {
                    name: "flac_huge_length",
                    ..case
                },
                "analyze",
                &huge,
                case.short,
                "30 s",
                None,
                "corruptStream",
            );
            std::fs::remove_file(&huge).unwrap();
        }
        root.clear();
    }
    root.empty();
    let empty = root.path(1, "wav");
    writers::write_wav(&empty, 0);
    let case = Case {
        name: "wav_empty",
        ..GATE[0]
    };
    validate_fixture(case, &empty, 0, false);
    let stats = chunks(case, &empty, 0);
    suite.call(
        case,
        "analyze",
        &empty,
        0,
        "0 s",
        Some(stats.max_chunk_frames),
        "ok",
    );
    root.clear();
    suite.finish("gate");
    println!(
        "memory gate elapsed: {:.3} s (indicative budget 120 s; not asserted)",
        start.elapsed().as_secs_f64()
    );
}

// @RNF-02: "A one-hour track costs no more memory than a 30-second one (long)",
// "A 24-bit 96 kHz track stays within its own ceiling (long)". Each group has its own two baselines.
#[test]
#[ignore = "RNF-02 long suite; run just memory"]
fn memory_long() {
    let root = Root::new("long");
    let mut all: Option<Suite> = None;
    for case in [
        Case {
            name: "flac_1h",
            long: 38_759 * 4096,
            ..GATE[1]
        },
        Case {
            name: "opus_1h",
            long: 172_800_000,
            ..GATE[5]
        },
        Case {
            name: "flac_hires",
            short: 703 * 4096,
            long: 7031 * 4096,
            rate: 96_000,
            bits: 24,
            ceiling: 64 * MIB,
            ..GATE[1]
        },
    ] {
        root.empty();
        let mut suite = Suite::new();
        let paths = [root.path(1, case.ext()), root.path(2, case.ext())];
        case.write(&paths[0], case.short);
        case.write(&paths[1], case.long);
        suite.pair(
            case,
            &paths,
            if case.name == "flac_hires" {
                "300 s"
            } else {
                "3600 s"
            },
            case.name == "flac_1h",
            false,
        );
        root.clear();
        if let Some(all) = &mut all {
            all.rows.extend(suite.rows);
            all.pairs.extend(suite.pairs);
        } else {
            all = Some(suite);
        }
    }
    all.unwrap().finish("long");
}

// @RNF-02: "Reading the peaks", "Peaks that cannot be read", "Unreadable peaks fail the suite".
#[test]
fn rnf02_reading_the_peaks() {
    assert_eq!(parse_peaks("123 456\r\n"), Ok((123, 456)));
    assert_eq!(parse_peaks("  123 456 \n"), Ok((123, 456)));
    for text in [
        "123",
        "",
        "12 x",
        "12  34",
        "12\t34",
        "12 34 56",
        "-1 2",
        "+1 2",
        "18446744073709551616 2",
    ] {
        assert_eq!(
            parse_peaks(text),
            Err(format!("memory: cannot read the peaks from \"{text}\""))
        );
    }
}

// @RNF-02: growth, ceiling and fixture-growth exact messages, inclusive bounds, negative growth.
#[test]
fn rnf02_exact_checks_and_inclusive_bounds() {
    assert_eq!(
        check_growth("opus analyze", "private", 1_000_000, 3_000_000, MIB),
        Err("memory: opus analyze private grew by 2000000 bytes, bound 1048576".into())
    );
    assert_eq!(
        check_growth("opus analyze", "private", 1_000_000, 2_048_576, MIB),
        Ok(())
    );
    assert_eq!(
        check_growth("opus analyze", "private", 3_000_000, 1_000_000, MIB),
        Ok(())
    );
    assert_eq!(check_ceiling("flac analyze", "working-set", 50_000_000, 10_000_000, CEILING), Err("memory: flac analyze working-set is 40000000 bytes over the baseline, ceiling 33554432".into()));
    assert_eq!(
        check_ceiling(
            "flac analyze",
            "working-set",
            43_554_432,
            10_000_000,
            CEILING
        ),
        Ok(())
    );
    assert_eq!(
        check_ceiling("flac analyze", "working-set", 0, 10_000_000, CEILING),
        Ok(())
    );
    assert_eq!(
        check_fixture_growth("opus", 9_000_000, 12_000_000, 4 * MIB),
        Err("memory: fixture opus grows by 3000000 bytes, need at least 4194304".into())
    );
    assert_eq!(
        check_fixture_growth("opus", 1, 4 * MIB + 1, 4 * MIB),
        Ok(())
    );
    assert!(check_growth("x", "private", 0, MIB + 1, MIB).is_err());
    assert!(check_ceiling("x", "private", CEILING + 1, 0, CEILING).is_err());
}

fn given_report() -> Suite {
    let mut rows = Vec::new();
    let mut pairs = Vec::new();
    for (case, op) in [
        ("wav", "analyze"),
        ("flac", "analyze"),
        ("flac_unknown_length", "analyze"),
        ("mp3", "analyze"),
        ("m4a_aac", "analyze"),
        ("m4a_alac", "analyze"),
        ("opus", "analyze"),
        ("flac", "spectrogram"),
    ] {
        for length in ["30 s", "300 s"] {
            rows.push(Row {
                case,
                op,
                length,
                file: 10_000_000,
                result: "ok".into(),
                max_chunk: Some(4096),
                peaks: Peaks {
                    private: 1_000_000,
                    ws: 2_000_000,
                },
                baseline: Peaks::default(),
                baseline_repeat: Peaks::default(),
                ceiling: CEILING,
            });
        }
        pairs.push(Pair {
            case,
            op,
            short: Peaks {
                private: 1_000_000,
                ws: 2_000_000,
            },
            long: Peaks {
                private: 1_000_000,
                ws: 2_000_000,
            },
        });
    }
    for case in ["flac_truncated", "flac_huge_length", "wav_empty"] {
        rows.push(Row {
            case,
            op: "analyze",
            length: if case == "wav_empty" { "0 s" } else { "300 s" },
            file: 100,
            result: if case == "wav_empty" {
                "ok"
            } else {
                "corruptStream"
            }
            .into(),
            max_chunk: if case == "wav_empty" { Some(0) } else { None },
            peaks: Peaks {
                private: 1_000_000,
                ws: 2_000_000,
            },
            baseline: Peaks::default(),
            baseline_repeat: Peaks::default(),
            ceiling: CEILING,
        });
    }
    pairs.push(Pair {
        case: "flac_truncated",
        op: "analyze",
        short: Peaks {
            private: 1_000_000,
            ws: 2_000_000,
        },
        long: Peaks {
            private: 1_000_000,
            ws: 2_000_000,
        },
    });
    Suite {
        baseline: Peaks::default(),
        repeat: Peaks::default(),
        rows,
        pairs,
    }
}

// @RNF-02: "Which figures fail the suite", "The report has a fixed shape" (given figures).
#[test]
fn rnf02_reports_print_all_rows_and_all_failures() {
    let mut data = given_report();
    let (text, pass) = report("gate", data.baseline, data.repeat, &data.rows, &data.pairs);
    assert!(pass);
    assert!(text.starts_with("RNF-02 memory report (specs/contracts/memory.md)\nbuild: "));
    assert!(text.contains("\nsuite: gate\nbaseline: private "));
    assert_eq!(text.lines().last(), Some("result: pass"));
    assert_eq!(
        text.lines().filter(|line| line.starts_with("| ")).count(),
        19 + 18 + 2
    );
    let positions: Vec<_> = [
        "| wav | analyze | 30 s",
        "| flac | analyze | 30 s",
        "| flac_unknown_length | analyze | 30 s",
        "| mp3 | analyze | 30 s",
        "| m4a_aac | analyze | 30 s",
        "| m4a_alac | analyze | 30 s",
        "| opus | analyze | 30 s",
        "| flac | spectrogram | 30 s",
        "| flac_truncated | analyze |",
        "| flac_huge_length | analyze |",
        "| wav_empty | analyze |",
    ]
    .iter()
    .map(|needle| text.find(needle).unwrap())
    .collect();
    assert!(positions.windows(2).all(|p| p[0] < p[1]));
    data.pairs
        .iter_mut()
        .find(|p| p.case == "opus")
        .unwrap()
        .long
        .private = 3_000_000;
    let (text, pass) = report("gate", data.baseline, data.repeat, &data.rows, &data.pairs);
    assert!(!pass);
    assert_eq!(
        text.lines().last(),
        Some("result: FAIL (opus analyze growth private)")
    );
    data.pairs
        .iter_mut()
        .find(|p| p.case == "opus")
        .unwrap()
        .long
        .private = 1_000_000;
    data.rows
        .iter_mut()
        .find(|r| r.case == "flac_huge_length")
        .unwrap()
        .peaks
        .private = CEILING + 1;
    let (text, pass) = report("gate", data.baseline, data.repeat, &data.rows, &data.pairs);
    assert!(!pass);
    assert_eq!(
        text.lines().last(),
        Some("result: FAIL (flac_huge_length analyze ceiling private)")
    );
    data.pairs
        .iter_mut()
        .find(|p| p.case == "opus")
        .unwrap()
        .long = Peaks {
        private: 3_000_000,
        ws: 4_000_000,
    };
    let (text, pass) = report("gate", data.baseline, data.repeat, &data.rows, &data.pairs);
    assert!(!pass);
    assert_eq!(text.lines().last(), Some("result: FAIL (flac_huge_length analyze ceiling private, opus analyze growth private, opus analyze growth working-set)"));
    assert_eq!(
        text.lines().filter(|line| line.starts_with("| ")).count(),
        39
    );
    assert!(text.contains("| flac_huge_length | analyze | 300 s | 100 | corruptStream | - |"));
    assert!(text.contains("| wav_empty | analyze | 0 s | 100 | ok | 0 |"));
}

// @RNF-02: "A one-hour track costs no more memory than a 30-second one (long)";
// memory.md 5.2: each long case uses its own baseline rather than the first case's baseline.
#[test]
fn rnf02_long_report_uses_each_cases_baseline() {
    let data = given_report();
    let mut row = data.rows[0].clone();
    row.case = "flac_hires";
    row.baseline = Peaks {
        private: 10_000_000,
        ws: 20_000_000,
    };
    row.baseline_repeat = Peaks {
        private: 10_004_096,
        ws: 20_004_096,
    };
    row.peaks = Peaks {
        private: 10_000_000 + 64 * MIB,
        ws: 20_000_000 + 64 * MIB,
    };
    row.ceiling = 64 * MIB;
    let (text, pass) = report(
        "long",
        Peaks::default(),
        Peaks::default(),
        &[row.clone()],
        &[],
    );
    assert!(pass);
    assert!(text.contains("baseline flac_hires: private 10000000 B, working-set 20000000 B | repeat: private 10004096 B, working-set 20004096 B"));
    assert!(text.contains("| 67108864 | 67108864 | 67108864 | pass |"));
    row.peaks.private += 1;
    let (text, pass) = report("long", Peaks::default(), Peaks::default(), &[row], &[]);
    assert!(!pass);
    assert_eq!(
        text.lines().last(),
        Some("result: FAIL (flac_hires analyze ceiling private)")
    );
}

// @RNF-02: "Every measured call does real work": non-zero, missing, malformed or duplicate
// child output must fail, with stderr retained; no process is started by these pure tests.
#[test]
fn rnf02_bad_child_measurements_fail() {
    for (stdout, success) in [
        (b"".as_slice(), true),
        (b"MEMORY|ok|0|0|0|1|2".as_slice(), false),
        (b"MEMORY|ok|0|0|0|1|x".as_slice(), true),
        (b"MEMORY|ok|0|0|0|1|2\nMEMORY|ok|0|0|0|1|2".as_slice(), true),
    ] {
        let error =
            std::panic::catch_unwind(|| parse_measurement(stdout, b"child diagnostic", success))
                .expect_err("invalid child measurement must fail");
        let text = error
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied())
            .unwrap();
        assert!(text.contains("child diagnostic"));
    }
    let m = parse_measurement(b"running 1 test\nMEMORY|ok|299909|0|0|123|456\n", b"", true);
    assert_eq!(m.result, "ok");
    assert_eq!(m.duration, 299909);
    assert_eq!(
        m.peaks,
        Peaks {
            ws: 123,
            private: 456
        }
    );
}

fn recipe<'a>(text: &'a str, name: &str) -> Option<(&'a str, String)> {
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if line.starts_with(&format!("{name}:")) || line.starts_with(&format!("{name} ")) {
            let body = lines
                .take_while(|line| {
                    line.is_empty() || line.starts_with(' ') || line.starts_with('\t')
                })
                .collect::<Vec<_>>()
                .join("\n");
            return Some((line, body));
        }
    }
    None
}
fn word_memory(text: &str) -> bool {
    text.split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|word| word == "memory")
}

// @RNF-02: "The long suite is opt-in and never runs in CI" (A9, read-only on every platform).
#[test]
fn rnf02_long_suite_wiring() {
    let justfile = include_str!("../../../justfile");
    let (header, body) =
        recipe(justfile, "memory").expect("RNF-02 requires an opt-in memory recipe");
    assert_eq!(header.trim(), "memory:");
    for token in [
        "--release",
        "--ignored",
        "--test memory",
        "--test-threads=1",
        "--nocapture",
    ] {
        assert!(body.contains(token), "memory recipe needs {token}");
    }
    for name in ["ci", "check", "check-fast", "test", "check-app"] {
        let (header, body) = recipe(justfile, name).unwrap();
        assert!(!word_memory(header.split_once(':').unwrap().1));
        assert!(!word_memory(&body));
    }
    for workflow in [
        include_str!("../../../.github/workflows/ci.yml"),
        include_str!("../../../.github/workflows/release.yml"),
    ] {
        assert!(!workflow.contains("just memory"));
    }
}
