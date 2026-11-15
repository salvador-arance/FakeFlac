//! Authentic releases with a lossy master (calibration.feature, @RP-11).
//! The variant is from sample-corpus.md section 6, using the synthetic fixtures.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_calibrate::report::{Counts, SplitSel};
use fakeflac_calibrate::Outcome;
use fakeflac_core::DetectionParams;
use std::sync::OnceLock;

const ALBUM: &str = "Reediciones/Ætheria – Edición ’98";
const FAKE: &str = "Reediciones/Ætheria – Edición ’98/01 – Única.flac";
const REAL: &str = "Reediciones/Ætheria – Edición ’98/02 – Otra.flac";
const BROKEN: &str = "Reediciones/Ætheria – Edición ’98/03 – Rota.flac";
const EXTRA: &str = "Reediciones/Ætheria – Edición ’98/04 – Extra.mp3";
const SECTION: &str = "## Authentic releases with a lossy master";
const TRACK_SECTION: &str = "### Authentic releases with a lossy master: tracks";
const HEADLINE: &str =
    "- lossy_master Fake (all): 1/2 tracks (50.0 %, 95 % CI 9.5–90.5 %), 1/1 albums";

fn setup_variant(sample: &Sample) -> Vec<String> {
    for (fixture, path) in [
        ("lp16k_dither", FAKE),
        ("full_band", REAL),
        ("corrupt", BROKEN),
    ] {
        place(fixture, Container::Flac, &sample.corpus.join(path));
    }
    std::fs::write(sample.corpus.join(EXTRA), b"unlabelled").unwrap();
    let mut confirmed = sample.confirmed_lines();
    confirmed.push(r"Reediciones\Ætheria – Edición ’98 | lossy-master | AccurateRip".to_string());
    sample.write_confirmed(&confirmed);
    confirmed
}

fn shared_variant() -> &'static SharedRun {
    static RUN: OnceLock<SharedRun> = OnceLock::new();
    RUN.get_or_init(|| {
        let sample = Sample::new_shared("lossy-master-variant");
        setup_variant(&sample);
        let outcome = sample.accept_v2();
        SharedRun { sample, outcome }
    })
}

/// Content of a named section, including any deeper headings within it.
fn section<'a>(markdown: &'a str, heading: &str) -> Vec<&'a str> {
    let level = heading.bytes().take_while(|&c| c == b'#').count();
    let prefix = format!("{} ", "#".repeat(level));
    let mut lines = markdown.lines();
    lines.find(|&line| line == heading).expect(heading);
    lines
        .take_while(|line| !line.starts_with(&prefix) && !line.starts_with("## "))
        .collect()
}

fn markdown_cells(line: &str) -> Vec<&str> {
    line.trim_matches('|').split('|').map(str::trim).collect()
}

fn track_rows(markdown: &str) -> Vec<Vec<&str>> {
    section(markdown, TRACK_SECTION)
        .into_iter()
        .filter(|line| line.starts_with("| Reediciones/"))
        .map(markdown_cells)
        .collect()
}

// Scenario: A lossy-master folder line labels the lossless tracks of the release (@RP-11 @RNF-09).
#[test]
fn rp11_lossy_master_folder_labels_only_lossless_files() {
    let SharedRun { sample, outcome } = shared_variant();
    assert_ok(outcome);
    let rows = labels(outcome, sample);
    assert_eq!(rows.rows.len(), 36);
    for path in [FAKE, REAL, BROKEN] {
        let row = rows
            .find("path", path)
            .unwrap_or_else(|| panic!("no row for {path}"));
        for (column, value) in [
            ("truth", "lossy_master"),
            ("origin", "confirmed_lossy_master"),
            ("class", "lossy_master"),
            ("trust_origin", ""),
            ("encoder", "none"),
            ("setting", "none"),
            ("source_path", ""),
            ("watch_id", ""),
            ("album_id", ALBUM),
            ("domain", "lossless"),
        ] {
            assert_eq!(rows.get(row, column), value, "{path}: {column}");
        }
    }
    assert!(rows.find("path", EXTRA).is_none());
    let base = shared_v2();
    assert_eq!(rows.header, labels(&base.outcome, &base.sample).header);
    assert_eq!(tracks(outcome).header, tracks(&base.outcome).header);
}

// Scenario: Lossy-master tracks are never transcode sources (@RP-11).
#[test]
fn rp11_lossy_master_is_not_a_transcode_source() {
    let SharedRun { sample, outcome } = shared_variant();
    assert_ok(outcome);
    let rows = labels(outcome, sample);
    let generated: Vec<_> = rows
        .rows
        .iter()
        .filter(|row| rows.get(row, "root") == "generated")
        .collect();
    assert_eq!(generated.len(), 24);
    assert!(generated
        .iter()
        .all(|row| !rows.get(row, "source_path").starts_with("Reediciones/")));
    assert_eq!(rows.rows.len() - generated.len(), 12);
    assert_eq!(decoded(outcome), 36);
}

// Scenario: Lossy-master tracks stay out of every negative figure and of T1 (@RP-11 @RNF-04).
#[test]
fn rp11_lossy_master_stays_out_of_negative_figures_and_t1() {
    let outcome = &shared_variant().outcome;
    assert_ok(outcome);
    let eval = &outcome.evaluations[0];
    let base = &shared_v2().outcome.evaluations[0];
    assert_eq!(eval.matrix, base.matrix);
    assert_eq!(eval.fp_rows, base.fp_rows);
    assert_eq!(eval.by_setting_split, base.by_setting_split);
    assert_eq!(eval.watch, base.watch);
    let all = eval.matrix[&SplitSel::All];
    assert_eq!(
        all.real_row,
        Counts {
            real: 4,
            suspicious: 1,
            error: 2,
            ..Counts::default()
        }
    );
    assert_eq!(
        all.fake_row,
        Counts {
            fake: 19,
            suspicious: 6,
            ..Counts::default()
        }
    );
    assert_eq!(
        eval.primary_figure,
        "0/2 tracks (0.0 %, 95 % CI 0.0–65.8 %), 0/1 albums"
    );
    assert_eq!(eval.targets, base.targets);
    for id in ["T1", "T2"] {
        let target = eval
            .targets
            .iter()
            .find(|t| t.id == id && t.split == SplitSel::All)
            .unwrap();
        assert!(target.pass, "{target:?}");
        if id == "T1" {
            assert_eq!(target.count, 0);
        }
    }
    let full = report_text(outcome, "report.md");
    for heading in [
        "### Near a threshold",
        "### Margin summary",
        "### Errors and misclassifications",
    ] {
        for line in section(&full, heading) {
            assert!(
                !line.contains("lossy_master") && !line.contains(ALBUM),
                "{heading}: {line}"
            );
        }
    }
}

// Scenario: The lossy-master section of the report (@RP-11 @RNF-04 @RNF-09).
#[test]
fn rp11_lossy_master_report_counts_verdicts_and_preserves_privacy() {
    let outcome = &shared_variant().outcome;
    assert_ok(outcome);
    for file in ["summary.md", "report.md"] {
        let markdown = report_text(outcome, file);
        let lines = section(&markdown, SECTION);
        let row = |split| {
            let prefix = format!("| {split} |");
            let line = lines
                .iter()
                .find(|line| line.starts_with(&prefix))
                .expect(&prefix);
            markdown_cells(line)
        };
        let all = row("all");
        assert_eq!(all.len(), 12);
        assert_eq!(all[1..6], ["2", "1", "1", "0", "1"]);
        let by_rule: u32 = all[6..8]
            .iter()
            .map(|cell| cell.parse::<u32>().unwrap())
            .sum();
        assert_eq!(by_rule, 1, "{all:?}");
        assert_eq!(all[8..], ["1", "1", "50.0 %", "9.5–90.5 %"]);
        assert_eq!(row("tune")[1..], all[1..]);
        let holdout = row("holdout");
        assert_eq!(holdout[1], "0");
        assert_eq!(holdout[10..], ["n/a", "n/a"]);
        let (_, figures) = lines.split_at(lines.iter().position(|line| *line == HEADLINE).unwrap());
        assert!(figures[1..]
            .iter()
            .find(|line| !line.trim().is_empty())
            .is_some_and(|line| line.contains("small sample: 2 tracks from 1 albums")));
        for prefix in [
            "- G (dB) over 2 tracks: min ",
            "- D (dB) over ",
            "- Fake rows, G (dB) over 1 tracks: min ",
            "- Fake rows, D (dB) over ",
        ] {
            assert!(
                figures.iter().any(|line| line.starts_with(prefix)),
                "{file} lacks {prefix}"
            );
        }
        assert!(section(&markdown, "## Corpus").contains(&"| lossy_master | 3 | 1 | 1 | 0 |"));
        assert!(section(&markdown, "### Acceptance targets")
            .iter()
            .all(|line| !line.contains("lossy_master")));
    }
    let full = report_text(outcome, "report.md");
    let rows = track_rows(&full);
    assert_eq!(
        rows.iter().map(|row| row[0]).collect::<Vec<_>>(),
        [FAKE, REAL, BROKEN]
    );
    let broken = &rows[2];
    assert!(
        broken.iter().any(|cell| cell.eq_ignore_ascii_case("error")),
        "{broken:?}"
    );
    assert!(broken.contains(&"corruptStream"), "{broken:?}");
    let summary = report_text(outcome, "summary.md");
    for private in [
        "Reediciones",
        "Ætheria – Edición ’98",
        "01 – Única",
        "02 – Otra",
        "03 – Rota",
        "04 – Extra",
        "AccurateRip",
    ] {
        assert!(!summary.contains(private), "summary.md leaks {private}");
    }
    assert!(!summary.contains(TRACK_SECTION));
}

// Scenario: The grid search never reads lossy-master tracks (@RP-11).
#[test]
fn rp11_lossy_master_does_not_influence_grid_search() {
    let sample = Sample::new("lossy-master-grid");
    setup_variant(&sample);
    sample.write_config(Some(0.0));
    let grid = sample.write_scratch(
        "grid.toml",
        &format!(
            "grid_version = 1\nbaseline = '{}'\nmust_catch = [\"fx128\"]\nmax_points = 1000\n\
             [weights]\nfp = 10.0\nfn = 1.0\nsuspicious_negative = 0.5\nsuspicious_positive = 0.1\n\
         [values]\n\"verdict.gap_fake_db\" = [4.0, 6.0]\n",
            v2_params().display()
        ),
    );
    let outcome = sample.calibrate(&grid);
    assert_ok(&outcome);
    let result = outcome.grid.as_ref().unwrap();
    let point = result
        .points
        .iter()
        .find(|p| {
            p.values
                .iter()
                .any(|(key, value)| key == "verdict.gap_fake_db" && *value == 4.0)
        })
        .unwrap();
    assert!((result.baseline.cost - 1.1).abs() < 1e-9, "{result:?}");
    assert!((point.cost - 0.5).abs() < 1e-9, "{point:?}");
    assert!(result.baseline.feasible && point.feasible);
    let proposal = result.proposal.as_ref().unwrap();
    let proposed_version = next_free_version();
    let proposal_name = format!("detection-params.v{proposed_version}.toml");
    assert_eq!(
        proposal.file_name().unwrap().to_str().unwrap(),
        proposal_name
    );
    let mut expected: DetectionParams = toml::from_str(&read(&v2_params())).unwrap();
    expected.algorithm_version = proposed_version;
    expected.verdict.gap_fake_db = 4.0;
    let actual: DetectionParams = toml::from_str(&read(proposal)).unwrap();
    assert_eq!(actual, expected);

    // Section 9 counts the paired evaluation set, even when a verdict stays the same.
    let candidate = &shared_v2_and_candidate().outcome.evaluations[1];
    assert_eq!(outcome.evaluations[1].changes, candidate.changes);
    assert_eq!(
        outcome.evaluations[1].setting_changes,
        candidate.setting_changes
    );
    for file in ["summary.md", "report.md"] {
        let markdown = report_text(&outcome, file);
        assert_eq!(markdown.lines().filter(|line| *line == HEADLINE).count(), 2);
    }
    let full = report_text(&outcome, "report.md");
    let rows = track_rows(&full);
    let expected = [FAKE, REAL, BROKEN].into_iter().flat_map(|path| {
        [
            (path, "detection-params.v2.toml"),
            (path, proposal_name.as_str()),
        ]
    });
    assert_eq!(rows.len(), 6);
    for (row, (path, params)) in rows.iter().zip(expected) {
        assert_eq!(row[0], path);
        assert!(row.contains(&params), "{row:?}");
    }
    let changed = section(
        &full,
        "### Tracks that changed verdict against detection-params.v2.toml",
    );
    assert!(changed.iter().all(|line| !line.contains(ALBUM)));
}

// Scenario: Adding a lossy-master release moves no other album (@RP-11).
#[test]
fn rp11_lossy_master_release_has_its_own_split() {
    let SharedRun { sample, outcome } = shared_variant();
    assert_ok(outcome);
    let baseline = shared_v2();
    let base_rows = labels(&baseline.outcome, &baseline.sample);
    let variant_rows = labels(outcome, sample);
    for row in &base_rows.rows {
        let path = base_rows.get(row, "path");
        let variant = variant_rows.find("path", path).unwrap();
        assert_eq!(
            variant_rows.get(variant, "split"),
            base_rows.get(row, "split"),
            "{path}"
        );
    }
    let mut units = outcome.unit_strata.clone();
    assert_eq!(units.remove("lossy_master"), Some((1, 0)));
    assert_eq!(units, baseline.outcome.unit_strata);
    for path in [FAKE, REAL, BROKEN] {
        let row = variant_rows.find("path", path).unwrap();
        assert_eq!(variant_rows.get(row, "split"), "tune");
    }
}

fn assert_input_error(outcome: &Outcome, expected: &[&str]) {
    assert_eq!(outcome.exit_code, 2, "{}", outcome.stderr);
    assert_eq!(outcome.stderr.lines().collect::<Vec<_>>(), expected);
    assert!(outcome.report_dir.is_none());
}

// Scenario Outline: Errors of lossy-master lines (@RP-11).
#[test]
fn rp11_lossy_master_input_errors_write_nothing() {
    let sample = Sample::new("lossy-master-invalid");
    let mut confirmed = setup_variant(&sample);
    let cases = [
        (
            r"Compras\Ñandú – Ácido, Vol. 1 (2024) | lossy-master",
            "also trusted (trusted.txt line 4)",
        ),
        (
            r"Sueltas\Mixto\06 – Falso.flac | lossy-master",
            "already listed at line 2",
        ),
        (
            r"Reediciones\Ætheria – Edición ’98 | lossy-master formats=all",
            "lossy-master applies to lossless files only",
        ),
        (
            r"Reediciones\Ætheria – Edición ’98 | lossy-master formats=lossy",
            "lossy-master applies to lossless files only",
        ),
        (
            r"Reediciones\Ætheria – Edición ’98\04 – Extra.mp3 | lossy-master",
            "lossy-master applies to lossless files only",
        ),
        (
            r"Reediciones\Ætheria – Edición ’98 | lossy-master upstream=aac",
            "upstream=, source_max_kbps= and weak apply to fake lines only",
        ),
    ];
    for (line, message) in cases {
        confirmed[3] = line.to_string();
        sample.write_confirmed(&confirmed);
        let before = snapshot(&sample.calibration);
        let outcome = sample.accept_v2();
        assert_input_error(&outcome, &[&format!("confirmed.txt:4: {message}")]);
        assert_eq!(before, snapshot(&sample.calibration), "{line}");
    }
}

// Scenario: A lossy-master track cannot be a watch track (@RP-11).
#[test]
fn rp11_lossy_master_cannot_be_a_watch_track() {
    let sample = Sample::new("lossy-master-watch");
    let mut confirmed = setup_variant(&sample);
    confirmed.push(format!("{FAKE} | false-positive"));
    sample.write_confirmed(&confirmed);
    assert_input_error(
        &sample.accept_v2(),
        &["confirmed.txt:5: not a trusted track"],
    );
}

// Scenario: An album with lossy-master and trusted tracks is an error (@RP-11).
#[test]
fn rp11_lossy_master_album_cannot_contain_other_labelled_tracks() {
    let sample = Sample::new("lossy-master-mixed");
    let mut confirmed = setup_variant(&sample);
    confirmed[3] = format!("{FAKE} | lossy-master");
    sample.write_confirmed(&confirmed);
    let mut trusted = sample.trusted_lines();
    trusted.push(format!("{REAL} | accuraterip"));
    sample.write_trusted(&trusted);
    let before = snapshot(&sample.calibration);
    assert_input_error(
        &sample.accept_v2(),
        &["album has lossy-master and other labelled tracks (trusted.txt:9, confirmed.txt:4)"],
    );
    assert_eq!(before, snapshot(&sample.calibration));
}

// Scenario: An album with lossy-master and trusted tracks is an error (@RP-11).
// Inputs.md section 2.1 also orders this diagnostic with split-unit errors by first reference.
#[test]
fn rp11_lossy_master_mixed_album_error_sorted_with_split_unit_errors() {
    let sample = Sample::new("lossy-master-error-order");
    let mut confirmed = setup_variant(&sample);
    confirmed[3] = format!("{FAKE} | lossy-master");
    sample.write_confirmed(&confirmed);
    let mut trusted = sample.trusted_lines();
    trusted[7] = format!("{REAL} | accuraterip");
    trusted.push(r"Sueltas\Mixto\05 – Cuatro.flac | analisis group=G1".to_string());
    sample.write_trusted(&trusted);
    assert_input_error(
        &sample.accept_v2(),
        &[
            "album has lossy-master and other labelled tracks (trusted.txt:8, confirmed.txt:4)",
            "album has tracks in several split units (trusted.txt:9, confirmed.txt:2)",
        ],
    );
}

// Scenario: Adding a lossy-master release moves no other album (@RP-11).
// Inputs.md section 4: when releases share a split unit, reference > lossy_master > positive.
#[test]
fn rp11_lossy_master_group_ranks_between_reference_and_positive() {
    let sample = Sample::new("lossy-master-group");
    let mut confirmed = setup_variant(&sample);
    confirmed[3] = format!("{ALBUM} | lossy-master formats=lossless group=G1");
    let other_fake = "Otras/Positivos/01.flac";
    place(
        "lp16k_dither",
        Container::Flac,
        &sample.corpus.join(other_fake),
    );
    confirmed.push(format!("{other_fake} | fake group=G1"));
    sample.write_confirmed(&confirmed);
    let outcome = sample.accept_v2();
    assert_ok(&outcome);
    assert_eq!(outcome.unit_strata.get("lossy_master"), Some(&(1, 0)));
    assert!(!outcome.unit_strata.contains_key("positive"));
    let rows = labels(&outcome, &sample);
    for path in [FAKE, other_fake] {
        assert_eq!(
            rows.get(rows.find("path", path).unwrap(), "split_unit"),
            "G1"
        );
    }

    let mut trusted = sample.trusted_lines();
    trusted[7].push_str(" group=G1");
    confirmed[1] = r"Sueltas\Mixto\06 – Falso.flac | fake group=G1".to_string();
    sample.write_confirmed(&confirmed);
    sample.write_trusted(&trusted);
    let outcome = sample.accept_v2();
    assert_ok(&outcome);
    assert!(!outcome.unit_strata.contains_key("lossy_master"));
    assert_eq!(outcome.unit_strata.get("reference"), Some(&(1, 0)));
}

// Scenario: A target cannot select the lossy_master class (@RP-11).
#[test]
fn rp11_lossy_master_class_cannot_be_selected_by_targets() {
    let sample = Sample::new("lossy-master-target");
    setup_variant(&sample);
    let config = read(&sample.config_path())
        .replace("classes = [\"primary\"]", "classes = [\"lossy_master\"]");
    std::fs::write(sample.config_path(), config).unwrap();
    let before = snapshot(&sample.root);
    let outcome = sample.accept_v2();
    assert_eq!(outcome.exit_code, 2, "{}", outcome.stderr);
    assert!(
        outcome
            .stderr
            .contains("targets 'T1': class 'lossy_master' is report only and cannot be selected"),
        "{}",
        outcome.stderr
    );
    assert!(outcome.report_dir.is_none());
    assert_eq!(before, snapshot(&sample.root));
}

// Scenario: Without lossy-master lines the outputs do not change (@RP-11).
#[test]
fn rp11_legacy_corpus_outputs_keep_their_schema_and_omit_lossy_master() {
    let SharedRun { sample, outcome } = shared_v2();
    assert_ok(outcome);
    let labels_path = sample.calibration.join("labels.csv");
    let headers = [
        (&labels_path, concat!(
            "path,root,truth,origin,trust_origin,class,encoder,setting,bitrate_kbps,encoder_version,",
            "sample_rate_hz,fragment_start_s,fragment_s,source_path,album_id,split,watch_id,domain,",
            "codec,declared_bitrate_kbps,bitrate_mode,stages,source_bitrate_kbps,upstream,label_strength,",
            "chain_id,chain_key,split_unit,not_evaluated"
        )),
        (&outcome.report_dir.as_ref().unwrap().join("tracks.csv"), concat!(
            "params_file,algorithm_version,path,root,truth,class,trust_origin,setting,album_id,split,",
            "watch_id,verdict,reasons,error,H,F,fe,D,fs,G,U,B,full_band,D_Q,F_top,R,edge_step,",
            "m_low_hf_db,m_gap_fake_db,m_gap_suspicious_db,m_step_fake_db,m_step_suspicious_db,",
            "m_step_low_db,m_step_high_db,m_gradual_hz,fs_fake_range_hz,m_full_band_level_db,",
            "m_full_band_burst_db,full_band_margin_db,m_full_band_step_db,m_edge_median_db,",
            "m_edge_residual_db,m_edge_step_db,nearest_fake_margin_db,codec,declared_bitrate_kbps,",
            "bitrate_mode,encoder_lowpass_hz,fc_hz,c_exp_hz,shortfall_hz,m_lossy_suspicious_hz,",
            "m_lossy_fake_hz,m_lossy_step_db,estimate_kbps,source_limited"
        )),
    ];
    for (path, header) in headers {
        let csv = read(path);
        assert_eq!(csv.lines().next(), Some(header), "{}", path.display());
        assert!(
            !csv.contains("lossy_master") && !csv.contains("lossy-master"),
            "{}",
            path.display()
        );
    }
    for file in ["summary.md", "report.md"] {
        let markdown = report_text(outcome, file);
        for unwanted in ["lossy_master", "lossy-master", SECTION, TRACK_SECTION] {
            assert!(!markdown.contains(unwanted), "{file}: {unwanted}");
        }
    }
}
