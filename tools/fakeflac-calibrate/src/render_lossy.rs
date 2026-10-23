//! The lossy parts of `summary.md` and `report.md` (lossy.md section 6). `summary.md` shows
//! aggregate figures, chain ids and counts only: never a path, a folder or file name, or a note
//! (lossy.md section 6.6).

use crate::cache::{codec_name, error_code, Extracted};
use crate::evaluate::{reason_name, verdict_name};
use crate::fmt;
use crate::lossy::is_near;
use crate::render::{Md, ReportData};
use crate::report::{ChainReport, LossyCounts, LossyWatchEntry, SplitSel};
use fakeflac_core::Verdict;
use std::collections::{BTreeMap, BTreeSet};

const SPLITS: [SplitSel; 3] = [SplitSel::All, SplitSel::Tune, SplitSel::Holdout];

/// Units per stratum, the lossy rows each params file leaves out, and the `real` labels that turned
/// out to hold a lossless codec.
pub(crate) fn header_lossy(md: &mut Md, data: &ReportData<'_>) {
    md.line("## Split units");
    md.blank();
    let rows: Vec<Vec<String>> = data
        .unit_strata
        .iter()
        .map(|(stratum, (tune, holdout))| {
            vec![
                stratum.clone(),
                (tune + holdout).to_string(),
                tune.to_string(),
                holdout.to_string(),
            ]
        })
        .collect();
    md.table(
        &["stratum", "units", "units in tune", "units in holdout"],
        &rows,
    );

    md.line("## Lossy rows");
    md.blank();
    let mut any = false;
    let base = &data.groups[data.runs[0].group];
    let labelled = base.lossy_rows.len();
    for (run, eval) in data.runs.iter().zip(data.evaluations.iter()) {
        let n = eval.lossy_not_evaluated;
        if run.params.lossy.is_none() {
            if labelled > 0 {
                any = true;
                md.line(format!(
                    "- {} evaluates no lossy row ({n} not evaluated)",
                    run.name
                ));
            }
        } else if n > 0 {
            any = true;
            md.line(format!(
                "- {} leaves {n} lossy {} not evaluated",
                run.name,
                if n == 1 { "row" } else { "rows" }
            ));
        }
    }
    // The same labels are excluded in every extraction group: print each once.
    let excluded: BTreeSet<&(usize, String)> =
        data.groups.iter().flat_map(|g| &g.real_excluded).collect();
    for (line, codec) in excluded {
        any = true;
        md.line(format!(
            "- confirmed.txt line {line}: real applies to lossy files; the track holds {codec} (excluded)"
        ));
    }
    let entries: Vec<String> = data
        .config
        .generate
        .lossy
        .iter()
        .map(|e| {
            let n = base
                .lossy_rows
                .iter()
                .filter(|r| r.origin == "generated" && r.setting == e.id)
                .count();
            format!("{} {n}", e.id)
        })
        .collect();
    if !entries.is_empty() {
        any = true;
        md.line(format!("- generated lossy entries: {}", entries.join(", ")));
    }
    if !any {
        md.line("- none");
    }
    md.blank();
}

fn counts_cells(c: &LossyCounts, undetectable: bool, source_limited: bool) -> Vec<String> {
    vec![
        c.real.to_string(),
        c.suspicious.to_string(),
        c.fake.to_string(),
        if undetectable {
            c.undetectable.to_string()
        } else {
            "—".to_string()
        },
        if source_limited {
            c.source_limited.to_string()
        } else {
            "—".to_string()
        },
        c.error.to_string(),
        c.total().to_string(),
    ]
}

/// The lossy tables of one params file with a `[lossy]` section.
pub(crate) fn lossy_section(md: &mut Md, data: &ReportData<'_>, idx: usize, full: bool) {
    let eval = &data.evaluations[idx];
    let Some(lossy) = &eval.lossy else {
        // No `[lossy]` section: only the watch list, with "not evaluated" entries.
        watch_table(md, &eval.lossy_watch, full);
        return;
    };

    md.line("### Lossy confusion matrix");
    md.blank();
    for sel in SPLITS {
        md.line(format!("Split `{}`:", sel.as_str()));
        md.blank();
        let m = lossy.matrix.get(&sel).copied().unwrap_or_default();
        let mut rows = Vec::new();
        for (label, c, undetectable, source_limited) in [
            ("real", &m.real_row, false, true),
            ("fake", &m.fake_row, true, false),
            ("fake (weak)", &m.weak_row, false, false),
        ] {
            let mut cells = vec![label.to_string()];
            cells.extend(counts_cells(c, undetectable, source_limited));
            rows.push(cells);
        }
        md.table(
            &[
                "truth \\ verdict",
                "Real",
                "Suspicious",
                "Fake",
                "Undetectable",
                "Source-limited",
                "Error",
                "total",
            ],
            &rows,
        );
        let den = |c: &LossyCounts| c.real + c.suspicious + c.fake;
        let (r, f, w) = (&m.real_row, &m.fake_row, &m.weak_row);
        md.line(format!(
            "FP rate {} · soft-FP rate {} · detection rate {} · caught rate {} · weak: detection rate {} · caught rate {}",
            fmt::rate(r.fake, den(r)),
            fmt::rate(r.suspicious, den(r)),
            fmt::rate(f.fake, den(f)),
            fmt::rate(f.fake + f.suspicious, den(f)),
            fmt::rate(w.fake, den(w)),
            fmt::rate(w.fake + w.suspicious, den(w)),
        ));
        md.blank();
    }

    md.line("### Lossy per group (split `all`)");
    md.blank();
    let rows: Vec<Vec<String>> = lossy
        .groups
        .iter()
        .map(|g| {
            let c = &g.counts;
            let n = c.real + c.suspicious + c.fake;
            let mut cells = vec![g.group.clone()];
            cells.extend(counts_cells(c, true, true));
            cells.push(fmt::rate(c.fake, n));
            cells.push(if g.positive {
                fmt::rate(c.fake + c.suspicious, n)
            } else {
                fmt::rate(c.fake, n)
            });
            cells.push(fmt::opt_db(g.s_median_hz));
            cells.push(fmt::opt_db(g.s_p5_hz));
            cells.push(fmt::opt_db(g.median_estimate_kbps));
            cells.push(fmt::opt_db(g.fake_median_estimate_kbps));
            cells.push(
                g.within_source
                    .map(|(k, n)| format!("{k}/{n}"))
                    .unwrap_or_default(),
            );
            cells
        })
        .collect();
    md.table(
        &[
            "group",
            "Real",
            "Suspicious",
            "Fake",
            "Undetectable",
            "Source-limited",
            "Error",
            "total",
            "Fake rate",
            "caught rate (positives) or FP rate (negatives)",
            "s median (Hz)",
            "s 5th percentile (Hz)",
            "median estimate (kbps)",
            "median estimate of the Fake rows (kbps)",
            "estimates at most the source bitrate",
        ],
        &rows,
    );

    watch_table(md, &eval.lossy_watch, full);

    chains_section(md, &eval.chains, full);

    if full {
        full_lossy_lists(md, data, idx);
    }
}

/// The lossy watch list (lossy.md section 6.7), shown for every params file.
fn watch_table(md: &mut Md, watch: &[LossyWatchEntry], full: bool) {
    if watch.is_empty() {
        return;
    }
    md.line("### Lossy watch list");
    md.blank();
    let mut head = vec!["id"];
    if full {
        head.push("path");
    }
    head.extend([
        "verdict",
        "reasons",
        "codec",
        "declared kbps",
        "fc",
        "c_exp",
        "s",
        "D",
        "estimate kbps",
    ]);
    let rows: Vec<Vec<String>> = watch
        .iter()
        .map(|w| {
            let mut cells = vec![w.watch_id.clone()];
            if full {
                cells.push(w.path.clone());
            }
            cells.extend([
                if w.evaluated {
                    verdict_name(w.verdict).to_string()
                } else {
                    "not evaluated".to_string()
                },
                w.reasons
                    .iter()
                    .map(|x| reason_name(*x))
                    .collect::<Vec<_>>()
                    .join(";"),
                w.codec.clone().unwrap_or_default(),
                w.declared_bitrate_kbps
                    .map(|b| b.to_string())
                    .unwrap_or_default(),
                fmt::opt_hz2(w.fc_hz),
                fmt::opt_hz2(w.c_exp_hz),
                fmt::opt_hz2(w.shortfall_hz),
                fmt::opt_db(w.step_db),
                w.estimate_kbps.map(|k| k.to_string()).unwrap_or_default(),
            ]);
            cells
        })
        .collect();
    md.table(&head, &rows);
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    Some(values[((values.len() - 1) as f64 * 0.5).round() as usize])
}

fn chains_section(md: &mut Md, chains: &[ChainReport], full: bool) {
    if chains.is_empty() {
        return;
    }
    md.line("### Chains");
    md.blank();
    let mut rows = Vec::new();
    for chain in chains {
        let mut versions: BTreeMap<&str, BTreeMap<&'static str, u32>> = BTreeMap::new();
        for e in &chain.entries {
            *versions
                .entry(e.version.as_str())
                .or_default()
                .entry(verdict_name(e.verdict))
                .or_default() += 1;
        }
        let present = versions.keys().copied().collect::<Vec<_>>().join(", ");
        let counts = versions
            .iter()
            .map(|(v, c)| {
                let list = c
                    .iter()
                    .map(|(name, n)| format!("{name} {n}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{v}: {list}")
            })
            .collect::<Vec<_>>()
            .join("; ");
        // |fc(mp3) - fc(aac)| over the keys that have both versions.
        let mut diffs: Vec<f64> = Vec::new();
        for key in &chain.keys {
            let fc_of = |version: &str| {
                let mut found = chain
                    .entries
                    .iter()
                    .filter(|e| &e.key == key && e.version == version);
                match (found.next(), found.next()) {
                    (Some(e), None) => e.fc_hz,
                    _ => None,
                }
            };
            if let (Some(a), Some(b)) = (fc_of("mp3"), fc_of("aac")) {
                diffs.push((a - b).abs());
            }
        }
        let watch: Vec<&str> = chain
            .entries
            .iter()
            .filter(|e| !e.watch_id.is_empty())
            .map(|e| e.watch_id.as_str())
            .collect();
        rows.push(vec![
            chain.id.clone(),
            chain.keys.len().to_string(),
            chain.pair_groups.len().to_string(),
            present,
            counts,
            fmt::opt_hz2(median(&mut diffs)),
            watch.join(", "),
        ]);
    }
    md.table(
        &[
            "chain",
            "keys",
            "pair groups",
            "versions",
            "verdicts per version",
            "median |fc(mp3) - fc(aac)| (Hz)",
            "watch ids",
        ],
        &rows,
    );
    if !full {
        return;
    }
    md.line("Chain entries:");
    md.blank();
    let mut rows = Vec::new();
    for chain in chains {
        for e in &chain.entries {
            rows.push(vec![
                chain.id.clone(),
                e.key.clone(),
                if chain.pair_groups.contains(&e.key) {
                    "paired".to_string()
                } else {
                    String::new()
                },
                e.path.clone(),
                e.version.clone(),
                verdict_name(e.verdict).to_string(),
                e.reasons
                    .iter()
                    .map(|x| reason_name(*x))
                    .collect::<Vec<_>>()
                    .join(";"),
                fmt::opt_hz2(e.fc_hz),
                e.estimate_kbps.map(|k| k.to_string()).unwrap_or_default(),
                e.declared_bitrate_kbps
                    .map(|k| k.to_string())
                    .unwrap_or_default(),
                e.watch_id.clone(),
            ]);
        }
    }
    md.table(
        &[
            "chain",
            "key",
            "pair group",
            "path",
            "version",
            "verdict",
            "reasons",
            "fc (Hz)",
            "estimate kbps",
            "declared kbps",
            "watch id",
        ],
        &rows,
    );
}

/// The per-track lists of `report.md` for lossy rows.
fn full_lossy_lists(md: &mut Md, data: &ReportData<'_>, idx: usize) {
    let run = &data.runs[idx];
    let group = &data.groups[run.group];
    let rows: Vec<_> = group
        .lossy_rows
        .iter()
        .zip(run.lossy_evals.iter())
        .collect();

    md.line("### Lossy: near a threshold");
    md.blank();
    let table: Vec<Vec<String>> = rows
        .iter()
        .filter(|(_, e)| e.evaluated && is_near(e, &run.params, data.config))
        .map(|(r, e)| {
            vec![
                r.path.clone(),
                r.truth.as_str().to_string(),
                verdict_name(e.verdict).to_string(),
                fmt::opt_hz2(e.shortfall_hz),
                fmt::opt_db(e.m.as_ref().map(|m| m.step_db)),
            ]
        })
        .collect();
    md.table(&["path", "truth", "verdict", "s (Hz)", "D"], &table);

    md.line("### Lossy: errors and misclassifications");
    md.blank();
    let table: Vec<Vec<String>> = rows
        .iter()
        .filter(|(_, e)| e.evaluated)
        .filter_map(|(r, e)| {
            let kind = match (r.truth, e.verdict) {
                (_, Verdict::Error) => "error",
                (crate::model::Truth::Real, Verdict::Fake) => "FP",
                (crate::model::Truth::Real, Verdict::Suspicious) => "soft FP",
                (crate::model::Truth::Fake, Verdict::Real) if !r.undetectable => "FN",
                (crate::model::Truth::Fake, Verdict::Suspicious) => "caught, not detected",
                _ => return None,
            };
            Some(vec![
                kind.to_string(),
                r.path.clone(),
                r.truth.as_str().to_string(),
                verdict_name(e.verdict).to_string(),
                e.reasons
                    .iter()
                    .map(|x| reason_name(*x))
                    .collect::<Vec<_>>()
                    .join(";"),
                e.error
                    .as_ref()
                    .map(|x| error_code(x).to_string())
                    .unwrap_or_default(),
                codec_of(&r.extracted),
            ])
        })
        .collect();
    md.table(
        &[
            "kind", "path", "truth", "verdict", "reasons", "error", "codec",
        ],
        &table,
    );

    let not_evaluated: Vec<Vec<String>> = rows
        .iter()
        .filter(|(_, e)| !e.evaluated)
        .map(|(r, _)| {
            let note = group
                .meta
                .get(&r.identity())
                .map(|m| m.not_evaluated.clone())
                .unwrap_or_default();
            vec![
                r.path.clone(),
                if note.is_empty() {
                    "params without [lossy]".to_string()
                } else {
                    note
                },
            ]
        })
        .collect();
    if !not_evaluated.is_empty() {
        md.line("### Lossy: rows not evaluated");
        md.blank();
        md.table(&["path", "reason"], &not_evaluated);
    }

    let limited: Vec<Vec<String>> = rows
        .iter()
        .filter(|(_, e)| e.source_limited)
        .map(|(r, _)| vec![r.path.clone(), r.setting.clone(), r.source_path.clone()])
        .collect();
    if !limited.is_empty() {
        md.line("### Lossy: source-limited encodes");
        md.blank();
        md.table(&["path", "entry", "source_path"], &limited);
    }
    let undetectable: Vec<Vec<String>> = rows
        .iter()
        .filter(|(r, _)| r.undetectable)
        .map(|(r, e)| {
            vec![
                r.path.clone(),
                r.setting.clone(),
                r.source_path.clone(),
                verdict_name(e.verdict).to_string(),
            ]
        })
        .collect();
    if !undetectable.is_empty() {
        md.line("### Lossy: undetectable cascades");
        md.blank();
        md.table(&["path", "entry", "source_path", "verdict"], &undetectable);
    }
}

fn codec_of(extracted: &Extracted) -> String {
    match extracted {
        Extracted::Probe(p) => p
            .stream
            .codec
            .map(|c| codec_name(c).to_string())
            .unwrap_or_default(),
        Extracted::Error(_) => String::new(),
    }
}
