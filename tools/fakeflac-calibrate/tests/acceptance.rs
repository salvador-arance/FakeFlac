//! Tests derived from `specs/features/calibration.feature` (@RP-11 @RNF-04 @RNF-05 @RNF-09):
//! acceptance on the sample corpus of `specs/calibration/sample-corpus.md` (synthetic fixtures,
//! fixture transcoder, no ffmpeg, no real audio). One test per scenario. Protected after the
//! `test(RP-11)` commit.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_calibrate::report::{Counts, SplitSel};
use std::collections::BTreeMap;

fn counts(real: u32, suspicious: u32, fake: u32, error: u32) -> Counts {
    Counts {
        real,
        suspicious,
        fake,
        error,
    }
}

// Scenario: Global confusion matrix (@RP-11 @RNF-04)
#[test]
fn rp11_global_confusion_matrix() {
    let shared = shared_v2();
    let o = &shared.outcome;
    assert_ok(o);
    let matrix = o.evaluations[0].matrix[&SplitSel::All];
    assert_eq!(matrix.real_row, counts(4, 1, 0, 2));
    assert_eq!(matrix.real_row.total(), 7);
    assert_eq!(matrix.fake_row, counts(0, 6, 19, 0));
    assert_eq!(matrix.fake_row.total(), 25);

    let labels = Table::read(&shared.sample.calibration.join("labels.csv"));
    assert_eq!(labels.rows.len(), 33, "labels.csv rows");

    // The same matrix, computed independently from tracks.csv (evaluation set: class != reference).
    let tracks = tracks(o);
    let mut by_truth: BTreeMap<(String, String), u32> = BTreeMap::new();
    for row in &tracks.rows {
        if tracks.get(row, "class") == "reference" {
            continue;
        }
        let key = (
            tracks.get(row, "truth").to_string(),
            tracks.get(row, "verdict").to_lowercase(),
        );
        *by_truth.entry(key).or_default() += 1;
    }
    let n = |truth: &str, verdict: &str| {
        by_truth
            .get(&(truth.to_string(), verdict.to_string()))
            .copied()
            .unwrap_or(0)
    };
    assert_eq!(n("real", "real"), 4);
    assert_eq!(n("real", "suspicious"), 1);
    assert_eq!(n("real", "error"), 2);
    assert_eq!(n("fake", "suspicious"), 6);
    assert_eq!(n("fake", "fake"), 19);
}

// Scenario: Matrix per encoder setting (@RP-11)
#[test]
fn rp11_matrix_per_encoder_setting() {
    let o = &shared_v2().outcome;
    let rows: Vec<(&str, Counts)> = o.evaluations[0]
        .by_setting
        .iter()
        .map(|r| (r.setting.as_str(), r.counts))
        .collect();
    assert_eq!(
        rows,
        vec![
            ("fx128", counts(0, 0, 6, 0)),
            ("fxv0", counts(0, 0, 6, 0)),
            ("fx320", counts(0, 0, 6, 0)),
            ("fxweak", counts(0, 6, 0, 0)),
            ("confirmed_fake", counts(0, 0, 1, 0)),
        ]
    );
}

// Scenario: False positives by origin and the primary figure (@RP-11)
#[test]
fn rp11_false_positives_by_origin_and_primary_figure() {
    let o = &shared_v2().outcome;
    let eval = &o.evaluations[0];
    let rows = &eval.fp_rows[&SplitSel::All];
    assert_eq!(rows.len(), 4, "{rows:#?}");
    let expect = [
        ("tienda", "primary", 2, 1, 0, 0, 0),
        ("revisado", "reviewed", 1, 1, 0, 0, 2),
        ("analisis-limpio", "clean", 2, 1, 0, 1, 0),
        ("analisis", "reference", 1, 1, 0, 0, 0),
    ];
    for (origin, class, tracks, albums, fake, suspicious, error) in expect {
        let row = rows
            .iter()
            .find(|r| r.origin == origin)
            .unwrap_or_else(|| panic!("no row for {origin}"));
        assert_eq!(
            (
                row.class.as_str(),
                row.tracks,
                row.albums,
                row.fake,
                row.suspicious,
                row.error
            ),
            (class, tracks, albums, fake, suspicious, error),
            "{origin}"
        );
    }

    let figure = "0/2 tracks (0.0 %, 95 % CI 0.0–65.8 %), 0/1 albums";
    assert_eq!(eval.primary_figure, figure);
    let summary = report_text(o, "summary.md");
    assert!(
        summary.contains(figure),
        "summary.md lacks the primary figure"
    );
    assert!(report_text(o, "report.md").contains(figure));
    // Small-sample warning (evaluation.md section 3): 2 tracks from 1 album.
    assert!(
        summary.contains("small sample: 2 tracks from 1 albums"),
        "summary.md lacks the small-sample warning"
    );

    // Targets T1 and T2 pass, for all and for holdout.
    for id in ["T1", "T2"] {
        let results: Vec<_> = eval.targets.iter().filter(|t| t.id == id).collect();
        assert!(!results.is_empty(), "no result for {id}");
        assert!(results.iter().all(|t| t.pass), "{id}: {results:?}");
        let lines: Vec<&str> = summary.lines().filter(|l| l.contains(id)).collect();
        assert!(!lines.is_empty(), "summary.md does not mention {id}");
        assert!(lines.iter().all(|l| l.contains("PASS")), "{id}: {lines:?}");
    }
}

// Scenario: Watch list (@RP-11)
#[test]
fn rp11_watch_list() {
    let o = &shared_v2().outcome;
    let watch = &o.evaluations[0].watch;
    assert_eq!(watch.len(), 1, "{watch:?}");
    let w = &watch[0];
    assert_eq!(w.watch_id, "W1");
    assert_eq!(w.verdict, fakeflac_core::Verdict::Real);
    assert_eq!(w.path, TRACK_2);
    assert!(w.gap_db.is_some(), "G");
    assert!(w.step_db.is_some(), "D");
    assert!(w.extent_hz.is_some(), "fe");
    assert!(w.high_frequency_db.is_some(), "H");
    assert!(w.nearest_fake_margin_db.is_some(), "nearest_fake_margin_db");
    // `fs` exists only when there is a step (D > 0): the field is reported, possibly empty.
    let _ = w.step_hz;

    let report = report_text(o, "report.md");
    assert!(report.contains(TRACK_2), "report.md shows the path of W1");
    let summary = report_text(o, "summary.md");
    assert!(summary.contains("W1"), "summary.md shows W1");
    assert!(
        !summary.contains(TRACK_2) && !summary.contains("Canción"),
        "summary.md must not show the path of W1"
    );
}

// Scenario: Margins against every threshold (@RP-11)
#[test]
fn rp11_margins_against_every_threshold() {
    let o = &shared_v2().outcome;
    let t = tracks(o);
    assert_eq!(t.rows.len(), 33, "one row per labelled track");
    for column in [
        "m_low_hf_db",
        "m_gap_fake_db",
        "m_gap_suspicious_db",
        "m_step_fake_db",
        "m_step_suspicious_db",
        "m_step_low_db",
        "m_step_high_db",
        "m_gradual_hz",
        "fs_fake_range_hz",
        "nearest_fake_margin_db",
    ] {
        assert!(t.has_column(column), "tracks.csv lacks {column}");
    }

    let gap_fake_db = 6.0; // detection-params.v2.toml
    let mut with_spectra = 0;
    let mut errors = 0;
    for row in &t.rows {
        let g = t.get(row, "G");
        if t.get(row, "verdict").eq_ignore_ascii_case("error") {
            errors += 1;
            for column in [
                "H",
                "F",
                "fe",
                "D",
                "fs",
                "G",
                "m_low_hf_db",
                "m_gap_fake_db",
                "m_gap_suspicious_db",
                "m_step_fake_db",
                "m_step_suspicious_db",
                "m_step_low_db",
                "m_step_high_db",
                "m_gradual_hz",
                "fs_fake_range_hz",
                "nearest_fake_margin_db",
            ] {
                assert_eq!(t.get(row, column), "", "Error row, column {column}");
            }
            assert!(!t.get(row, "error").is_empty(), "Error row names its error");
        } else if !g.is_empty() {
            with_spectra += 1;
            let g: f64 = g.parse().unwrap();
            let m: f64 = t.get(row, "m_gap_fake_db").parse().unwrap();
            // Written values may be rounded to 2 decimals.
            assert!(
                (m - (gap_fake_db - g)).abs() <= 0.011,
                "m_gap_fake_db {m} != {gap_fake_db} - {g}"
            );
        }
    }
    assert_eq!(errors, 2, "the corrupt and the too-short tracks");
    assert_eq!(with_spectra, 31);
}

// Scenario: Deterministic split by album (@RP-11)
#[test]
fn rp11_deterministic_split_by_album() {
    let sample = Sample::new("deterministic-split");
    let a = sample.fresh_calibration("cal-a");
    let b = sample.fresh_calibration("cal-b");
    assert_ok(&run_in(&sample, &a));
    assert_ok(&run_in(&sample, &b));
    let first = std::fs::read(a.calibration.join("labels.csv")).unwrap();
    let second = std::fs::read(b.calibration.join("labels.csv")).unwrap();
    assert_eq!(first, second, "labels.csv byte-identical");

    let labels = Table::read(&a.calibration.join("labels.csv"));
    for track in [TRACK_4, TRACK_5] {
        let row = labels.find("path", track).unwrap();
        assert_eq!(labels.get(row, "album_id"), "Colección/Álbum Ü", "{track}");
    }
    let generated: Vec<_> = labels
        .rows
        .iter()
        .filter(|r| labels.get(r, "root") == "generated")
        .collect();
    assert_eq!(generated.len(), 24);
    for row in generated {
        let source = labels.find("path", labels.get(row, "source_path")).unwrap();
        assert_eq!(labels.get(row, "album_id"), labels.get(source, "album_id"));
        assert_eq!(labels.get(row, "split"), labels.get(source, "split"));
    }
}

fn run_in(sample: &Sample, with: &Sample) -> fakeflac_calibrate::Outcome {
    let mut env = sample.env();
    env.calibration = Some(with.calibration.clone());
    fakeflac_calibrate::run(
        &fakeflac_calibrate::Command::Accept(sample.accept_args(&[v2_params()])),
        &env,
        &FixtureTranscoder,
    )
}

// Scenario: A cold run decodes every labelled track once (@RP-11)
#[test]
fn rp11_cold_run_decodes_every_labelled_track_once() {
    let o = &shared_v2().outcome;
    assert_ok(o);
    assert_eq!(decoded(o), 33);
    assert_eq!(transcoded(o), 24);
}

// Scenario: A second run decodes nothing (@RP-11)
#[test]
fn rp11_second_run_decodes_nothing() {
    let sample = Sample::new("second-run");
    let first = sample.accept_v2();
    assert_ok(&first);
    let second = sample.accept_v2();
    assert_ok(&second);
    assert_eq!(decoded(&second), 0);
    assert_eq!(transcoded(&second), 0);
    assert_ne!(first.report_dir, second.report_dir, "one folder per run");

    assert_eq!(
        std::fs::read(first.report_dir.as_ref().unwrap().join("tracks.csv")).unwrap(),
        std::fs::read(second.report_dir.as_ref().unwrap().join("tracks.csv")).unwrap(),
        "tracks.csv byte-identical"
    );
    for file in ["report.md", "summary.md"] {
        assert_eq!(
            without_volatile(&report_text(&first, file)),
            without_volatile(&report_text(&second, file)),
            "{file} equal except run id, date and cache statistics"
        );
    }
}

// Scenario: A second run decodes nothing (@RP-11), labels.csv part
#[test]
fn rp11_second_run_keeps_labels_csv() {
    let sample = Sample::new("second-run-labels");
    assert_ok(&sample.accept_v2());
    let first = std::fs::read(sample.calibration.join("labels.csv")).unwrap();
    assert_ok(&sample.accept_v2());
    let second = std::fs::read(sample.calibration.join("labels.csv")).unwrap();
    assert_eq!(first, second);
}

// Scenario: Cached results equal fresh results (@RP-11)
#[test]
fn rp11_cached_results_equal_fresh_results() {
    let sample = Sample::new("cached-equals-fresh");
    let first = sample.accept_v2();
    assert_ok(&first);
    std::fs::remove_dir_all(sample.features_dir()).unwrap();
    let second = sample.accept_v2();
    assert_ok(&second);
    assert_eq!(decoded(&second), 33, "features folder was empty");
    assert_eq!(
        std::fs::read(first.report_dir.as_ref().unwrap().join("tracks.csv")).unwrap(),
        std::fs::read(second.report_dir.as_ref().unwrap().join("tracks.csv")).unwrap(),
    );
}

// Scenario: Cached results equal fresh results (@RP-11), the other direction: a run served from the
// cache gives the same tracks.csv as the cold run.
#[test]
fn rp11_cache_hit_gives_the_same_tracks_csv() {
    let sample = Sample::new("cache-hit-same");
    let first = sample.accept_v2();
    let second = sample.accept_v2();
    assert_eq!(decoded(&second), 0);
    assert_eq!(
        std::fs::read(first.report_dir.as_ref().unwrap().join("tracks.csv")).unwrap(),
        std::fs::read(second.report_dir.as_ref().unwrap().join("tracks.csv")).unwrap(),
        "exact floats through the cache"
    );
}

// Scenario: A rule-only change reuses the cache (@RP-11)
#[test]
fn rp11_rule_only_change_reuses_the_cache() {
    let sample = Sample::new("rule-only");
    assert_ok(&sample.accept_v2());
    let candidate = sample.candidate_params();
    let o = sample.accept(&[candidate]);
    assert_ok(&o);
    assert_eq!(decoded(&o), 0);
    assert_eq!(transcoded(&o), 0);
}

// Scenario: An extraction change invalidates the cache (@RP-11)
#[test]
fn rp11_extraction_change_invalidates_the_cache() {
    let sample = Sample::new("extraction-change");
    assert_ok(&sample.accept_v2());
    let blackman = sample.blackman_params();
    let o = sample.accept(&[blackman]);
    assert_ok(&o);
    let features: Vec<_> = files_under(&sample.features_dir())
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
        .collect();
    assert_eq!(features.len(), 2, "{features:?}");
    let stems: std::collections::BTreeSet<_> = features.iter().map(|p| p.file_stem()).collect();
    assert_eq!(stems.len(), 2, "different fingerprint ids");
    assert_eq!(decoded(&o), 33);
    assert_eq!(transcoded(&o), 24);
}

// Scenario: A changed source is extracted and transcoded again (@RP-11)
#[test]
fn rp11_changed_source_is_extracted_and_transcoded_again() {
    let sample = Sample::new("changed-source");
    assert_ok(&sample.accept_v2());
    sample.touch_track_1();
    let o = sample.accept_v2();
    assert_ok(&o);
    assert_eq!(decoded(&o), 5, "track 1 and its 4 transcodes");
    assert_eq!(transcoded(&o), 4);
}

// Scenario: Generated audio is deleted once cached (@RP-11)
#[test]
fn rp11_generated_audio_is_deleted_once_cached() {
    let shared = shared_v2();
    assert_ok(&shared.outcome);
    let generated = shared.sample.calibration.join("generated");
    assert!(
        files_under(&generated).is_empty(),
        "generated/ holds {:?}",
        files_under(&generated)
    );
}

// Scenario: v2 and a candidate side by side in one run (@RP-11)
#[test]
fn rp11_v2_and_candidate_side_by_side() {
    let shared = shared_v2_and_candidate();
    let o = &shared.outcome;
    assert_ok(o);
    assert_eq!(decoded(o), 33);
    assert_eq!(transcoded(o), 24);
    assert_eq!(o.evaluations.len(), 2);
    assert_eq!(o.evaluations[0].params_name, "detection-params.v2.toml");
    assert_eq!(o.evaluations[1].params_name, "candidate.toml");
    assert_eq!(
        o.evaluations[0].fingerprint_id, o.evaluations[1].fingerprint_id,
        "only rule values differ"
    );

    // One column per params file in the side-by-side table.
    let summary = report_text(o, "summary.md");
    assert!(
        summary.lines().any(|l| l.trim_start().starts_with('|')
            && l.contains("detection-params.v2.toml")
            && l.contains("candidate.toml")),
        "summary.md has no side-by-side table header with both files"
    );

    let (v2, candidate) = (&o.evaluations[0], &o.evaluations[1]);
    let all = SplitSel::All;
    assert_eq!(candidate.matrix[&all].fake_row, counts(0, 0, 25, 0));
    assert_eq!(candidate.matrix[&all].real_row, v2.matrix[&all].real_row);

    let changes = candidate.changes.as_ref().expect("verdict changes");
    assert_eq!(changes.against, "detection-params.v2.toml");
    let moved: Vec<_> = changes
        .counts
        .iter()
        .filter(|((from, to), n)| from != to && **n > 0)
        .collect();
    assert_eq!(moved.len(), 1, "{moved:?}");
    assert_eq!(
        changes
            .counts
            .get(&("suspicious".to_string(), "fake".to_string())),
        Some(&6)
    );
    assert!(v2.changes.is_none(), "the baseline has no changes table");
}

// Scenario: A v1 params file is rejected (@RP-11)
#[test]
fn rp11_v1_params_file_is_rejected() {
    let sample = Sample::new("v1-rejected");
    let o = sample.accept(&[v1_params()]);
    assert_eq!(o.exit_code, 2, "{}", o.stderr);
    assert!(
        o.stderr
            .contains("detection-params.v1.toml: not a DetectionParams file of the v2 rule family"),
        "{}",
        o.stderr
    );
}

// Scenario: Unicode names in labels.csv (@RP-11 @RNF-09)
#[test]
fn rp11_unicode_names_in_labels_csv() {
    let shared = shared_v2();
    let bytes = std::fs::read(shared.sample.calibration.join("labels.csv")).unwrap();
    assert!(!bytes.starts_with(&[0xEF, 0xBB, 0xBF]), "no BOM");
    let text = String::from_utf8(bytes).expect("UTF-8");
    assert!(text.ends_with("\r\n"));
    assert_eq!(
        text.matches('\n').count(),
        text.matches("\r\n").count(),
        "every line ends with CRLF"
    );
    assert!(
        text.contains(&format!("\"{TRACK_1}\"")),
        "path with a comma is written between double quotes"
    );
    let labels = Table::read(&shared.sample.calibration.join("labels.csv"));
    let row = labels
        .find("path", TRACK_1)
        .expect("row read back unchanged");
    assert_eq!(labels.get(row, "root"), "corpus");
}

// Scenario: summary.md holds no corpus names or paths (@RP-11)
#[test]
fn rp11_summary_holds_no_corpus_names_or_paths() {
    let shared = shared_v2_and_candidate();
    let o = &shared.outcome;
    assert_ok(o);
    let summary = report_text(o, "summary.md");
    for root in [&shared.sample.corpus, &shared.sample.calibration] {
        let text = root.to_string_lossy().into_owned();
        for variant in [text.replace('\\', "/"), text.replace('/', "\\")] {
            assert!(!summary.contains(&variant), "summary.md contains {variant}");
        }
    }
    for name in PRIVATE_NAMES {
        assert!(!summary.contains(name), "summary.md contains '{name}'");
    }
    // The other outputs may hold names but never an absolute path of either root.
    let outputs = [
        read(&shared.sample.calibration.join("labels.csv")),
        report_text(o, "tracks.csv"),
        report_text(o, "report.md"),
    ];
    for root in [&shared.sample.corpus, &shared.sample.calibration] {
        let text = root.to_string_lossy().into_owned();
        for variant in [text.replace('\\', "/"), text.replace('/', "\\")] {
            for (i, out) in outputs.iter().enumerate() {
                assert!(!out.contains(&variant), "output {i} contains {variant}");
            }
        }
    }
}

// Scenario: The corpus is not modified (@RP-11 @RNF-05)
#[test]
fn rp11_corpus_is_not_modified() {
    let sample = Sample::new("corpus-untouched");
    let before = snapshot(&sample.corpus);
    assert!(!before.is_empty());
    let o = sample.accept_v2();
    assert_ok(&o);
    assert_eq!(snapshot(&sample.corpus), before);
}
