//! Extraction of spectra for the labelled tracks and generation of the transcodes, per extraction
//! fingerprint (calibration.md section 4, generation.md). Decoding and transcoding run on
//! `cores - 1` std threads; results are put back in order, so the thread count never changes an output.

use crate::cache::{canonical_json, codec_name, error_code, Extracted, FeatureCache, Fingerprint};
use crate::config::{Config, LossyEntry, Setting};
use crate::env::Roots;
use crate::error::ToolError;
use crate::hash::id16;
use crate::lists::{Attrs, Track};
use crate::model::{chain_key, FragmentInfo, LabelRow, RowMeta, RowRoot, Truth};
use crate::report::CacheStats;
use crate::split::Split;
use crate::transcode::{Fragment, LossyRequest, TranscodeError, TranscodeRequest, Transcoder};
use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_core::detect::measure;
use fakeflac_core::ports::SpectrumAnalyzer;
use fakeflac_core::{DetectionParams, TrackError};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

/// The `not_evaluated` text of an Opus track (lossy.md section 6.1, ADR-0030).
pub const OPUS_PENDING: &str = "opus decoder pending";

/// A labelled corpus track (trusted, confirmed fake or confirmed real) before extraction.
#[derive(Debug, Clone)]
pub struct CorpusItem {
    pub track: Track,
    pub truth: Truth,
    /// `trusted`, `confirmed_fake` or `confirmed_real`.
    pub origin: &'static str,
    pub trust_origin: String,
    pub class: String,
    pub album_id: String,
    pub split: Split,
    pub watch_id: String,
    /// The split unit (lossy.md section 3).
    pub unit: String,
    /// The attributes of the line that gives the track its label.
    pub attrs: Attrs,
    /// The line that gives the track its label.
    pub line: usize,
}

fn has_extension(path: &str, ext: &str) -> bool {
    path.rsplit_once('.')
        .is_some_and(|(_, e)| e.eq_ignore_ascii_case(ext))
}

/// Opus files are labelled and never decoded while the decoder is pending.
pub fn is_opus(path: &str) -> bool {
    has_extension(path, "opus")
}

/// Whether the file is of the lossy domain: by codec when it was probed, else by extension.
pub fn is_lossy_domain(path: &str, extracted: &Extracted) -> bool {
    if let Extracted::Probe(p) = extracted {
        if let Some(codec) = p.stream.codec {
            return codec.is_lossy();
        }
    }
    ["mp3", "m4a", "opus"]
        .iter()
        .any(|e| has_extension(path, e))
}

/// The vocabulary of `upstream=` for the encoder of a generated stage.
fn upstream_of_encoder(encoder: &str) -> &'static str {
    match encoder {
        "lame" => "mp3",
        "aac" => "aac",
        "opus" => "opus",
        _ => "unknown",
    }
}

/// What the extraction needs besides the tracks.
pub struct Context<'a> {
    pub config: &'a Config,
    pub roots: &'a Roots,
    pub transcoder: &'a dyn Transcoder,
    pub no_generate: bool,
    pub threads: usize,
}

/// A generation that failed (generation.md section 2): no row, not cached, retried next run.
#[derive(Debug, Clone)]
pub struct GenFailure {
    pub setting: String,
    pub source: String,
    pub message: String,
}

/// Result of the extraction of one fingerprint.
pub struct GroupResult {
    /// The rows of the lossless domain: every lossless table reads these.
    pub rows: Vec<LabelRow>,
    /// The rows of the lossy domain (lossy.md section 1.1).
    pub lossy_rows: Vec<LabelRow>,
    /// Lossy columns of every row, by [`LabelRow::identity`].
    pub meta: BTreeMap<String, RowMeta>,
    /// `confirmed.txt` lines of `real` labels that turned out to hold a lossless codec.
    pub real_excluded: Vec<(usize, String)>,
    pub stats: CacheStats,
    /// (source path, reason) of trusted tracks that cannot be a source.
    pub skipped_sources: Vec<(String, String)>,
    pub failures: Vec<GenFailure>,
    /// Generated entries missing under `--no-generate`.
    pub not_generated: u32,
    /// `ffmpeg -version` first line, when ffmpeg was found.
    pub ffmpeg_version: Option<String>,
    /// ffmpeg was not found but every needed entry was cached.
    pub ffmpeg_missing: bool,
    /// Generated audio kept for `keep_misclassified`: (row identity, file).
    pub kept: Vec<(String, PathBuf)>,
}

/// Number of worker threads: `cores - 1`, at least 1.
pub fn worker_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(1).max(1))
        .unwrap_or(1)
}

/// Runs `work` on each item with `threads` threads; `sink` receives (index, result) on the calling
/// thread as results arrive.
fn parallel<T, R, W, S>(items: &[T], threads: usize, work: W, mut sink: S)
where
    T: Sync,
    R: Send,
    W: Fn(&T) -> R + Sync,
    S: FnMut(usize, R),
{
    if items.is_empty() {
        return;
    }
    let next = AtomicUsize::new(0);
    let (tx, rx) = mpsc::channel::<(usize, R)>();
    std::thread::scope(|scope| {
        for _ in 0..threads.min(items.len()).max(1) {
            let tx = tx.clone();
            let next = &next;
            let work = &work;
            scope.spawn(move || loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                let Some(item) = items.get(i) else {
                    break;
                };
                if tx.send((i, work(item))).is_err() {
                    break;
                }
            });
        }
        drop(tx);
        for (i, result) in rx {
            sink(i, result);
        }
    });
}

/// A panic inside the decoder is a per-track error, not the end of the run (ADR-0013, RNF-04).
fn analyze(path: &std::path::Path, params: &DetectionParams) -> Extracted {
    let result = catch_unwind(AssertUnwindSafe(|| SymphoniaAnalyzer.analyze(path, params)))
        .unwrap_or_else(|_| {
            Err(TrackError::CorruptStream(
                "the decoder failed unexpectedly".into(),
            ))
        });
    Extracted::from_analysis(result)
}

/// Sidecar of a kept `<id>.flac`: the versions it was made with (so reuse keeps labels.csv exact).
fn sidecar_path(dir: &std::path::Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.json"))
}

/// The (ffmpeg_version, encoder_version) of a reusable kept file: the audio exists, its sidecar
/// is readable and, when ffmpeg is known, was made by the same ffmpeg (generation.md section 4.2).
fn reusable(
    dir: &std::path::Path,
    id: &str,
    ext: &str,
    current: Option<&str>,
) -> Option<(String, String)> {
    if !dir.join(format!("{id}.{ext}")).is_file() {
        return None;
    }
    let text = std::fs::read_to_string(sidecar_path(dir, id)).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    let ffmpeg = v.get("ffmpeg_version")?.as_str()?.to_string();
    let encoder = v.get("encoder_version")?.as_str()?.to_string();
    (current.is_none() || current == Some(ffmpeg.as_str())).then_some((ffmpeg, encoder))
}

/// A lossy entry with its settings resolved.
struct LossyGen<'a> {
    entry: &'a LossyEntry,
    stages: Vec<&'a Setting>,
}

/// One (source, setting) pair, or (source, lossy entry) pair, to generate.
struct Gen<'a> {
    source: usize,
    /// The setting of a lossless transcode, or the last stage of a lossy entry.
    setting: &'a Setting,
    lossy: Option<LossyGen<'a>>,
    key: Value,
    key_string: String,
    id: String,
    start_s: f64,
    duration_s: f64,
    fragment: Option<Fragment>,
}

impl Gen<'_> {
    /// Extension of the file that is analysed: `flac` for a transcode, the last stage's for an entry.
    fn ext(&self) -> &str {
        if self.lossy.is_some() {
            &self.setting.ext
        } else {
            "flac"
        }
    }

    /// The id the entry is reported under: the setting id or the lossy entry id.
    fn label(&self) -> &str {
        self.lossy
            .as_ref()
            .map_or(self.setting.id.as_str(), |l| l.entry.id.as_str())
    }
}

enum GenResult {
    Done {
        extracted: Extracted,
        ffmpeg_version: String,
        encoder_version: String,
        transcoded: bool,
        flac: Option<PathBuf>,
    },
    Failed(String),
    NotFound,
    Skipped,
}

fn to_io(e: std::io::Error) -> ToolError {
    ToolError::internal(format!("error: {e}"))
}

/// Extracts every labelled track of the corpus, generates the transcodes of the trusted ones and
/// extracts them, all through the cache of `fingerprint`.
pub fn extract_group(
    ctx: &Context<'_>,
    items: &[CorpusItem],
    params: &DetectionParams,
    fingerprint: &Fingerprint,
) -> Result<GroupResult, ToolError> {
    let config = ctx.config;
    let mut cache =
        FeatureCache::open(&ctx.roots.calibration.join("features"), fingerprint).map_err(to_io)?;
    let mut stats = CacheStats {
        fingerprint_id: fingerprint.id.clone(),
        hits: 0,
        decoded: 0,
        transcoded: 0,
        generation_failures: 0,
        skipped_lines: cache.skipped_lines,
    };

    // 1. The corpus tracks.
    let corpus_key =
        |t: &Track| json!({"kind": "corpus", "path": t.rel, "size": t.size, "mtime": t.mtime});
    let mut results: Vec<Option<Arc<Extracted>>> = vec![None; items.len()];
    let mut misses: Vec<usize> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        match cache.get(&canonical_json(&corpus_key(&item.track))) {
            Some(entry) => {
                stats.hits += 1;
                results[i] = Some(entry.extracted.clone());
            }
            None => misses.push(i),
        }
    }
    // Opus is labelled and never decoded (lossy.md section 1.1).
    for (i, item) in items.iter().enumerate() {
        if is_opus(&item.track.rel) {
            results[i] = Some(Arc::new(Extracted::Error(TrackError::UnsupportedFormat(
                OPUS_PENDING.to_string(),
            ))));
        }
    }
    misses.retain(|i| results[*i].is_none());
    // --no-generate evaluates only what is already cached: uncached tracks are not decoded. They are
    // labelled tracks, not generated entries, so they are not counted as "not generated".
    let mut not_generated = 0;
    if ctx.no_generate {
        misses.clear();
    }
    let mut io_error: Option<std::io::Error> = None;
    parallel(
        &misses,
        ctx.threads,
        |i| analyze(&items[*i].track.abs, params),
        |n, extracted| {
            let i = misses[n];
            stats.decoded += 1;
            if extracted.cacheable() && io_error.is_none() {
                if let Err(e) = cache.append(&corpus_key(&items[i].track), &extracted, None, None) {
                    io_error = Some(e);
                }
            }
            results[i] = Some(Arc::new(extracted));
        },
    );
    if let Some(e) = io_error {
        return Err(to_io(e));
    }
    let corpus_results: Vec<Option<Arc<Extracted>>> = results;

    // 2. Sources and recipes.
    let min_ms = params.input.min_duration_s * 1000.0;
    let mut skipped_sources = Vec::new();
    let mut gens: Vec<Gen<'_>> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        // Only trusted tracks are sources, and lossy files never are (lossy.md section 4.1).
        if item.truth != Truth::Real || item.origin != "trusted" {
            continue;
        }
        let Some(extracted) = corpus_results[i].as_ref() else {
            // Uncached source under --no-generate: its generated entries are not generated either.
            not_generated +=
                (config.lossless_settings().count() + config.generate.lossy.len()) as u32;
            continue;
        };
        if is_lossy_domain(&item.track.rel, extracted) {
            continue;
        }
        let duration_ms = match extracted.as_ref() {
            Extracted::Error(e) => {
                skipped_sources.push((item.track.rel.clone(), error_code(e).to_string()));
                continue;
            }
            Extracted::Probe(p) if (p.duration_ms as f64) < min_ms => {
                skipped_sources.push((item.track.rel.clone(), "tooShort".to_string()));
                continue;
            }
            Extracted::Probe(p) if p.spectrum.is_none() => {
                skipped_sources.push((item.track.rel.clone(), "silent".to_string()));
                continue;
            }
            Extracted::Probe(p) => p.duration_ms,
        };
        let duration_s = duration_ms as f64 / 1000.0;
        let (start_s, length_s, fragment) = match config.generate.fragment_s {
            Some(f) if duration_s > f => {
                let start = (((duration_s - f) / 2.0).max(0.0) * 1000.0).floor() / 1000.0;
                (
                    start,
                    f,
                    Some(Fragment {
                        start_s: start,
                        duration_s: f,
                    }),
                )
            }
            _ => (0.0, duration_s, None),
        };
        let source_json =
            json!({"path": item.track.rel, "size": item.track.size, "mtime": item.track.mtime});
        for setting in config.lossless_settings() {
            let recipe = json!({
                "source": source_json,
                "setting": setting.id,
                "args": setting.args,
                "decode_args": config.generate.decode_args,
                "fragment_start_s": start_s,
                "fragment_s": length_s,
            });
            let id = id16(canonical_json(&recipe).as_bytes());
            let key = json!({"kind": "generated", "recipe": recipe});
            gens.push(Gen {
                source: i,
                setting,
                lossy: None,
                key_string: canonical_json(&key),
                key,
                id,
                start_s,
                duration_s: length_s,
                fragment,
            });
        }
        for entry in &config.generate.lossy {
            let stages = config.stages_of(entry);
            let Some(last) = stages.last().copied() else {
                continue;
            };
            let recipe = json!({
                "kind": "lossy",
                "source": source_json,
                "stages": stages
                    .iter()
                    .map(|s| json!({"setting": s.id, "args": s.args}))
                    .collect::<Vec<_>>(),
                "fragment_start_s": start_s,
                "fragment_s": length_s,
            });
            let id = id16(canonical_json(&recipe).as_bytes());
            let key = json!({"kind": "generated", "recipe": recipe});
            gens.push(Gen {
                source: i,
                setting: last,
                lossy: Some(LossyGen { entry, stages }),
                key_string: canonical_json(&key),
                key,
                id,
                start_s,
                duration_s: length_s,
                fragment,
            });
        }
    }

    // 3. Which entries are cached, which must be generated.
    let (current_version, version_error) = if ctx.no_generate || gens.is_empty() {
        (None, None)
    } else {
        match ctx.transcoder.version() {
            Ok(v) => (Some(v), None),
            Err(e) => (None, Some(e)),
        }
    };
    let generated_dir = ctx.roots.calibration.join("generated");
    let mut gen_results: Vec<Option<(Arc<Extracted>, String, String)>> = vec![None; gens.len()];
    let mut tasks: Vec<usize> = Vec::new();
    for (g, gen) in gens.iter().enumerate() {
        let hit = cache.get(&gen.key_string).filter(|e| {
            current_version.is_none() || e.ffmpeg_version.as_deref() == current_version.as_deref()
        });
        match hit {
            Some(entry) => {
                stats.hits += 1;
                gen_results[g] = Some((
                    entry.extracted.clone(),
                    entry.ffmpeg_version.clone().unwrap_or_default(),
                    entry.encoder_version.clone().unwrap_or_default(),
                ));
            }
            None if ctx.no_generate => not_generated += 1,
            None => tasks.push(g),
        }
    }
    let needs_transcode = tasks.iter().any(|g| {
        reusable(
            &generated_dir,
            &gens[*g].id,
            gens[*g].ext(),
            current_version.as_deref(),
        )
        .is_none()
    });
    if needs_transcode {
        if let Some(e) = version_error {
            return Err(e);
        }
    }
    let ffmpeg_missing = current_version.is_none() && !ctx.no_generate && !gens.is_empty();

    // 4. Generate and extract.
    let abort = AtomicBool::new(false);
    let keep_audio = config.generate.keep_audio;
    let keep_misclassified = config.generate.keep_misclassified;
    let ffmpeg_for_entries = current_version.clone().unwrap_or_default();
    let work = |g: &usize| -> GenResult {
        let gen = &gens[*g];
        if abort.load(Ordering::SeqCst) {
            return GenResult::Skipped;
        }
        let (flac, ffmpeg_version, encoder_version, transcoded) = if let Some((f, e)) = reusable(
            &generated_dir,
            &gen.id,
            gen.ext(),
            current_version.as_deref(),
        ) {
            (
                generated_dir.join(format!("{}.{}", gen.id, gen.ext())),
                f,
                e,
                false,
            )
        } else {
            let source = &items[gen.source].track.abs;
            let done = match &gen.lossy {
                Some(l) => ctx
                    .transcoder
                    .transcode_lossy(&LossyRequest {
                        source,
                        stages: &l.stages,
                        fragment: gen.fragment,
                        generated_dir: &generated_dir,
                        id: &gen.id,
                    })
                    .map(|o| (o.file, o.encoder_version)),
                None => ctx
                    .transcoder
                    .transcode(&TranscodeRequest {
                        source,
                        setting: gen.setting,
                        fragment: gen.fragment,
                        decode_args: &config.generate.decode_args,
                        generated_dir: &generated_dir,
                        id: &gen.id,
                    })
                    .map(|o| (o.flac, o.encoder_version)),
            };
            match done {
                Ok((file, encoder_version)) => {
                    (file, ffmpeg_for_entries.clone(), encoder_version, true)
                }
                Err(TranscodeError::NotFound) => {
                    abort.store(true, Ordering::SeqCst);
                    return GenResult::NotFound;
                }
                Err(TranscodeError::Failed(message)) => return GenResult::Failed(message),
            }
        };
        let extracted = analyze(&flac, params);
        let keep = keep_audio || keep_misclassified;
        if keep {
            if transcoded {
                let side =
                    json!({"ffmpeg_version": ffmpeg_version, "encoder_version": encoder_version});
                let _ = crate::fsutil::write_atomic(
                    &sidecar_path(&generated_dir, &gen.id),
                    side.to_string().as_bytes(),
                );
            }
        } else {
            let _ = std::fs::remove_file(&flac);
            let _ = std::fs::remove_file(sidecar_path(&generated_dir, &gen.id));
        }
        if let Extracted::Error(TrackError::Unreadable(m)) = &extracted {
            return GenResult::Failed(format!("decoded audio is unreadable: {m}"));
        }
        GenResult::Done {
            extracted,
            ffmpeg_version,
            encoder_version,
            transcoded,
            flac: keep.then_some(flac),
        }
    };
    let mut failures = Vec::new();
    let mut kept: Vec<(usize, PathBuf)> = Vec::new();
    let mut not_found = false;
    let mut io_error: Option<std::io::Error> = None;
    parallel(&tasks, ctx.threads, work, |n, result| {
        let g = tasks[n];
        let gen = &gens[g];
        match result {
            GenResult::Done {
                extracted,
                ffmpeg_version,
                encoder_version,
                transcoded,
                flac,
            } => {
                stats.decoded += 1;
                if transcoded {
                    stats.transcoded += 1;
                }
                if io_error.is_none() {
                    if let Err(e) = cache.append(
                        &gen.key,
                        &extracted,
                        Some(&ffmpeg_version),
                        Some(&encoder_version),
                    ) {
                        io_error = Some(e);
                    }
                }
                if let Some(path) = flac {
                    kept.push((g, path));
                }
                gen_results[g] = Some((Arc::new(extracted), ffmpeg_version, encoder_version));
            }
            GenResult::Failed(message) => {
                stats.generation_failures += 1;
                failures.push(GenFailure {
                    setting: gen.label().to_string(),
                    source: items[gen.source].track.rel.clone(),
                    message,
                });
            }
            GenResult::NotFound => not_found = true,
            GenResult::Skipped => {}
        }
    });
    if let Some(e) = io_error {
        return Err(to_io(e));
    }
    if not_found {
        return Err(ToolError::external(
            "error: ffmpeg not found; set FAKEFLAC_FFMPEG or add it to PATH",
        ));
    }
    failures.sort_by(|a, b| (&a.source, &a.setting).cmp(&(&b.source, &b.setting)));

    // 5. The rows.
    let mut rows: Vec<LabelRow> = Vec::new();
    let mut lossy_rows: Vec<LabelRow> = Vec::new();
    let mut meta: BTreeMap<String, RowMeta> = BTreeMap::new();
    let mut real_excluded: Vec<(usize, String)> = Vec::new();
    for (item, extracted) in items.iter().zip(corpus_results.iter()) {
        let Some(extracted) = extracted else {
            continue;
        };
        let lossy = is_lossy_domain(&item.track.rel, extracted);
        if item.origin == "confirmed_real" && !lossy {
            // A `real` label on a lossless codec (ALAC): no row (lossy.md section 1.3).
            let codec = match extracted.as_ref() {
                Extracted::Probe(p) => p.stream.codec.map(|c| codec_name(c).to_ascii_uppercase()),
                Extracted::Error(_) => None,
            };
            real_excluded.push((item.line, codec.unwrap_or_else(|| "LOSSLESS".to_string())));
            continue;
        }
        let row = LabelRow {
            root: RowRoot::Corpus,
            path: item.track.rel.clone(),
            truth: item.truth,
            origin: item.origin,
            trust_origin: item.trust_origin.clone(),
            class: item.class.clone(),
            encoder: "none".to_string(),
            setting: "none".to_string(),
            bitrate_kbps: None,
            encoder_version: String::new(),
            fragment: None,
            source_path: String::new(),
            album_id: item.album_id.clone(),
            split: item.split,
            watch_id: item.watch_id.clone(),
            extracted: extracted.clone(),
            undetectable: false,
        };
        meta.insert(row.identity(), corpus_meta(item, extracted, lossy, config));
        if lossy {
            lossy_rows.push(row);
        } else {
            rows.push(row);
        }
    }
    real_excluded.sort_unstable();
    real_excluded.dedup();
    let mut kept_rows: Vec<(String, PathBuf)> = Vec::new();
    for (g, gen) in gens.iter().enumerate() {
        let Some((extracted, _, encoder_version)) = gen_results[g].clone() else {
            continue;
        };
        let source = &items[gen.source];
        let fragment = Some(FragmentInfo {
            start_s: gen.start_s,
            duration_s: gen.duration_s,
        });
        let (row, row_meta) = match &gen.lossy {
            None => {
                let undetectable = is_undetectable(
                    &corpus_results[gen.source],
                    &extracted,
                    gen.setting.lowpass_hz,
                    config,
                    params,
                );
                let row = LabelRow {
                    root: RowRoot::Generated,
                    path: format!("{}.flac", gen.id),
                    truth: Truth::Fake,
                    origin: "generated",
                    trust_origin: source.trust_origin.clone(),
                    class: "positive".to_string(),
                    encoder: gen.setting.encoder.clone(),
                    setting: gen.setting.id.clone(),
                    bitrate_kbps: gen.setting.bitrate_kbps,
                    encoder_version,
                    fragment,
                    source_path: source.track.rel.clone(),
                    album_id: source.album_id.clone(),
                    split: source.split,
                    watch_id: String::new(),
                    extracted,
                    undetectable,
                };
                let row_meta = RowMeta {
                    split_unit: source.unit.clone(),
                    ..RowMeta::default()
                };
                (row, row_meta)
            }
            Some(l) => {
                let before = &l.stages[..l.stages.len() - 1];
                // The lowest bitrate before the last stage; the first stage that has it.
                let lowest = before
                    .iter()
                    .filter_map(|s| s.bitrate_kbps.map(|b| (b, *s)))
                    .min_by_key(|(b, _)| *b);
                let cascade_fake = match (lowest, gen.setting.bitrate_kbps) {
                    (Some((low, _)), Some(last)) => low < last,
                    _ => false,
                };
                let undetectable = cascade_fake
                    && is_undetectable(
                        &corpus_results[gen.source],
                        &extracted,
                        before.iter().filter_map(|s| s.lowpass_hz).min(),
                        config,
                        params,
                    );
                let row = LabelRow {
                    root: RowRoot::Generated,
                    path: format!("{}.{}", gen.id, gen.ext()),
                    truth: if cascade_fake {
                        Truth::Fake
                    } else {
                        Truth::Real
                    },
                    origin: "generated",
                    trust_origin: source.trust_origin.clone(),
                    class: if cascade_fake {
                        "positive"
                    } else {
                        "generated"
                    }
                    .to_string(),
                    encoder: gen.setting.encoder.clone(),
                    setting: l.entry.id.clone(),
                    bitrate_kbps: gen.setting.bitrate_kbps,
                    encoder_version,
                    fragment,
                    source_path: source.track.rel.clone(),
                    album_id: source.album_id.clone(),
                    split: source.split,
                    watch_id: String::new(),
                    extracted,
                    undetectable,
                };
                let row_meta = RowMeta {
                    stages: l.entry.stages.join(">"),
                    source_bitrate_kbps: lowest.map(|(b, _)| b),
                    upstream: lowest
                        .map(|(_, s)| upstream_of_encoder(&s.encoder).to_string())
                        .unwrap_or_default(),
                    split_unit: source.unit.clone(),
                    source: corpus_results[gen.source].clone(),
                    honest_generated: !cascade_fake,
                    ..RowMeta::default()
                };
                (row, row_meta)
            }
        };
        if let Some((_, path)) = kept.iter().find(|(kg, _)| *kg == g) {
            kept_rows.push((row.identity(), path.clone()));
        }
        meta.insert(row.identity(), row_meta);
        if gen.lossy.is_some() {
            lossy_rows.push(row);
        } else {
            rows.push(row);
        }
    }

    Ok(GroupResult {
        rows,
        lossy_rows,
        meta,
        real_excluded,
        stats,
        skipped_sources,
        failures,
        not_generated,
        ffmpeg_version: current_version,
        ffmpeg_missing,
        kept: kept_rows,
    })
}

/// The lossy columns of a corpus row (lossy.md section 2); `upstream=by-rate` is resolved here from
/// the decoded sample rate.
fn corpus_meta(item: &CorpusItem, extracted: &Extracted, lossy: bool, config: &Config) -> RowMeta {
    let attrs = &item.attrs;
    let mut meta = RowMeta {
        split_unit: item.unit.clone(),
        weak: attrs.weak,
        ..RowMeta::default()
    };
    if let Some(chain) = &attrs.chain {
        meta.chain_id = chain.clone();
        meta.chain_key = chain_key(&item.track.rel);
    }
    if is_opus(&item.track.rel) {
        meta.not_evaluated = OPUS_PENDING.to_string();
    }
    if lossy && item.origin == "confirmed_fake" {
        let (upstream, bound) = match attrs.upstream.as_deref() {
            Some("by-rate") => {
                let rate = match extracted {
                    Extracted::Probe(p) => Some(p.sample_rate),
                    Extracted::Error(_) => None,
                };
                match rate.and_then(|r| config.labels.upstream_by_rate.get(&r.to_string())) {
                    Some(entry) => (entry.upstream.clone(), entry.source_max_kbps),
                    None => ("unknown".to_string(), None),
                }
            }
            Some(other) => (other.to_string(), None),
            None => ("unknown".to_string(), None),
        };
        meta.upstream = upstream;
        meta.source_bitrate_kbps = attrs.source_max_kbps.or(bound);
    }
    meta
}

/// A transcode is undetectable when its source has no content above lowpass_hz or
/// the maximum drop in HF bands is below min_removed_hf_db (evaluation.md section 2).
fn is_undetectable(
    source_extracted: &Option<Arc<Extracted>>,
    transcode_extracted: &Extracted,
    lowpass_hz: Option<u32>,
    config: &Config,
    params: &DetectionParams,
) -> bool {
    let Some(lowpass_hz) = lowpass_hz else {
        return false;
    };
    let Some(src_ext) = source_extracted.as_ref() else {
        return false;
    };
    let Extracted::Probe(src_probe) = src_ext.as_ref() else {
        return false;
    };
    let Some(src_spec) = &src_probe.spectrum else {
        return false;
    };

    let nyquist = src_spec.nyquist_hz();
    if lowpass_hz as f64 >= nyquist {
        return false;
    }

    // Criterion 1: source.fe <= setting.lowpass_hz
    let m = measure(src_spec, params);
    if m.extent_hz < nyquist && m.extent_hz <= lowpass_hz as f64 {
        return true;
    }

    // Criterion 2: maximum drop in high-frequency bands (b >= lowpass_hz/100)
    // between source and transcode is below generate.min_removed_hf_db
    if let Some(min_drop) = config.generate.min_removed_hf_db {
        if m.extent_hz < nyquist {
            if let Extracted::Probe(t_probe) = transcode_extracted {
                if let Some(t_spec) = &t_probe.spectrum {
                    let start_b = if src_spec.band_hz > 0.0 {
                        (lowpass_hz as f64 / src_spec.band_hz).floor() as usize
                    } else {
                        (lowpass_hz / 100) as usize
                    };
                    let n = src_spec.mean_db.len().min(t_spec.mean_db.len());
                    if start_b < n {
                        let max_drop = (start_b..n)
                            .map(|b| src_spec.mean_db[b] - t_spec.mean_db[b])
                            .fold(f64::NEG_INFINITY, f64::max);
                        if max_drop.is_finite() && max_drop < min_drop {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}
