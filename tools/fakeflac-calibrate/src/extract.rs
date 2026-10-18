//! Extraction of spectra for the labelled tracks and generation of the transcodes, per extraction
//! fingerprint (calibration.md section 4, generation.md). Decoding and transcoding run on
//! `cores - 1` std threads; results are put back in order, so the thread count never changes an output.

use crate::cache::{canonical_json, error_code, Extracted, FeatureCache, Fingerprint};
use crate::config::{Config, Setting};
use crate::env::Roots;
use crate::error::ToolError;
use crate::hash::id16;
use crate::lists::Track;
use crate::model::{FragmentInfo, LabelRow, RowRoot, Truth};
use crate::report::CacheStats;
use crate::split::Split;
use crate::transcode::{Fragment, TranscodeError, TranscodeRequest, Transcoder};
use fakeflac_audio::SymphoniaAnalyzer;
use fakeflac_core::ports::SpectrumAnalyzer;
use fakeflac_core::{DetectionParams, TrackError};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

/// A labelled corpus track (trusted or confirmed fake) before extraction.
#[derive(Debug, Clone)]
pub struct CorpusItem {
    pub track: Track,
    pub truth: Truth,
    /// `trusted` or `confirmed_fake`.
    pub origin: &'static str,
    pub trust_origin: String,
    pub class: String,
    pub album_id: String,
    pub split: Split,
    pub watch_id: String,
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
    pub rows: Vec<LabelRow>,
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

fn analyze(path: &std::path::Path, params: &DetectionParams) -> Extracted {
    Extracted::from_analysis(SymphoniaAnalyzer.analyze(path, params))
}

/// One (source, setting) pair to generate.
struct Gen<'a> {
    source: usize,
    setting: &'a Setting,
    key: Value,
    key_string: String,
    id: String,
    start_s: f64,
    duration_s: f64,
    fragment: Option<Fragment>,
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
    let corpus_results: Vec<Arc<Extracted>> = results.into_iter().flatten().collect();

    // 2. Sources and recipes.
    let min_ms = params.input.min_duration_s * 1000.0;
    let mut skipped_sources = Vec::new();
    let mut gens: Vec<Gen<'_>> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        if item.truth != Truth::Real {
            continue;
        }
        let duration_ms = match corpus_results[i].as_ref() {
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
        for setting in &config.generate.settings {
            let recipe = json!({
                "source": {"path": item.track.rel, "size": item.track.size, "mtime": item.track.mtime},
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
    let mut not_generated = 0;
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
        !generated_dir
            .join(format!("{}.flac", gens[*g].id))
            .is_file()
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
        let existing = generated_dir.join(format!("{}.flac", gen.id));
        let (flac, encoder_version, transcoded) = if existing.is_file() {
            (
                existing,
                "unknown (audio kept by an earlier run)".to_string(),
                false,
            )
        } else {
            let request = TranscodeRequest {
                source: &items[gen.source].track.abs,
                setting: gen.setting,
                fragment: gen.fragment,
                decode_args: &config.generate.decode_args,
                generated_dir: &generated_dir,
                id: &gen.id,
            };
            match ctx.transcoder.transcode(&request) {
                Ok(out) => (out.flac, out.encoder_version, true),
                Err(TranscodeError::NotFound) => {
                    abort.store(true, Ordering::SeqCst);
                    return GenResult::NotFound;
                }
                Err(TranscodeError::Failed(message)) => return GenResult::Failed(message),
            }
        };
        let extracted = analyze(&flac, params);
        let keep = keep_audio || keep_misclassified;
        if !keep {
            let _ = std::fs::remove_file(&flac);
        }
        if let Extracted::Error(TrackError::Unreadable(m)) = &extracted {
            return GenResult::Failed(format!("decoded audio is unreadable: {m}"));
        }
        GenResult::Done {
            extracted,
            ffmpeg_version: ffmpeg_for_entries.clone(),
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
                    setting: gen.setting.id.clone(),
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
    for (item, extracted) in items.iter().zip(corpus_results.iter()) {
        rows.push(LabelRow {
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
        });
    }
    let mut kept_rows: Vec<(String, PathBuf)> = Vec::new();
    for (g, gen) in gens.iter().enumerate() {
        let Some((extracted, _, encoder_version)) = gen_results[g].clone() else {
            continue;
        };
        let source = &items[gen.source];
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
            fragment: Some(FragmentInfo {
                start_s: gen.start_s,
                duration_s: gen.duration_s,
            }),
            source_path: source.track.rel.clone(),
            album_id: source.album_id.clone(),
            split: source.split,
            watch_id: String::new(),
            extracted,
        };
        if let Some((_, path)) = kept.iter().find(|(kg, _)| *kg == g) {
            kept_rows.push((row.identity(), path.clone()));
        }
        rows.push(row);
    }

    Ok(GroupResult {
        rows,
        stats,
        skipped_sources,
        failures,
        not_generated,
        ffmpeg_version: current_version,
        ffmpeg_missing,
        kept: kept_rows,
    })
}
