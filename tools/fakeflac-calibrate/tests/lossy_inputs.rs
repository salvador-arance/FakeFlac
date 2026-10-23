//! Tests derived from `specs/features/calibration-lossy.feature` (@RP-11 @RF-15): the grammar of
//! `trusted.txt` and `confirmed.txt` for lossy files, split units, the lossy configuration, the grid
//! and the command lines (`specs/calibration/lossy.md` sections 1, 3, 4, 7 and 8) on the lossy variant
//! of the sample corpus (section 9), with the fixture transcoder: no ffmpeg, no real audio, no corpus.
//! Protected after the `test(RF-15)` commit.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::*;
use fakeflac_calibrate::ffmpeg::lossy_args;
use fakeflac_calibrate::run::parse_args;
use fakeflac_calibrate::{Command, Config};
use std::path::{Path, PathBuf};

fn stderr_lines(o: &fakeflac_calibrate::Outcome) -> Vec<&str> {
    o.stderr.lines().collect()
}

/// The labels.csv row of `path` (relative to the corpus, `/` separators), root `corpus`.
fn corpus_row<'a>(labels: &'a Table, path: &str) -> Option<&'a Vec<String>> {
    let (p, r) = (labels.col("path"), labels.col("root"));
    labels
        .rows
        .iter()
        .find(|row| row[p] == path && row[r] == "corpus")
}

/// Rows of labels.csv whose own path or `source_path` lies under one of the folders.
fn rows_under<'a>(labels: &'a Table, folders: &[&str]) -> Vec<&'a Vec<String>> {
    let (p, s) = (labels.col("path"), labels.col("source_path"));
    labels
        .rows
        .iter()
        .filter(|row| {
            folders
                .iter()
                .any(|f| row[p].starts_with(f) || row[s].starts_with(f))
        })
        .collect()
}

// ---- Errors in the attributes of confirmed.txt ---------------------------------------------------

/// Scenario Outline: Errors in the attributes of confirmed.txt (@RP-11 @RF-15). Line 4 of the sample
/// lossy confirmed.txt is replaced.
fn confirmed_attribute_error(test: &str, line: &str, message: &str) {
    let sample = Sample::new_lossy(test);
    let mut lines = sample.lossy_confirmed_lines();
    lines[3] = line.to_string();
    sample.write_confirmed(&lines);
    let before = names(&sample.calibration);
    let o = sample.accept_lossy();
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert_eq!(
        stderr_lines(&o),
        vec![format!("confirmed.txt:4: {message}")],
        "error output"
    );
    assert_eq!(names(&sample.calibration), before, "nothing is written");
}

const FALSO: &str = "Pérdidas\\Vídeo\\07 – Falso 320.mp3";

// Scenario Outline: Errors in the attributes of confirmed.txt (@RP-11 @RF-15), row "unknown attribute"
#[test]
fn rf15_confirmed_error_unknown_attribute() {
    confirmed_attribute_error(
        "lossy-attr-unknown",
        &format!("{FALSO} | fake source=aac"),
        "unknown attribute 'source'",
    );
}

// Scenario Outline: Errors in the attributes of confirmed.txt (@RP-11 @RF-15), row "given twice"
#[test]
fn rf15_confirmed_error_attribute_given_twice() {
    confirmed_attribute_error(
        "lossy-attr-twice",
        &format!("{FALSO} | fake weak weak"),
        "attribute 'weak' given twice",
    );
}

// Scenario Outline: Errors in the attributes of confirmed.txt (@RP-11 @RF-15), row "invalid upstream"
#[test]
fn rf15_confirmed_error_invalid_upstream() {
    confirmed_attribute_error(
        "lossy-attr-upstream",
        &format!("{FALSO} | fake upstream=vorbis"),
        "invalid value 'vorbis' for upstream (expected aac, opus, mp3, unknown or by-rate)",
    );
}

// Scenario Outline: Errors in the attributes of confirmed.txt (@RP-11 @RF-15), row "source_max_kbps"
#[test]
fn rf15_confirmed_error_source_max_kbps_not_an_integer() {
    confirmed_attribute_error(
        "lossy-attr-kbps",
        &format!("{FALSO} | fake source_max_kbps=12.5"),
        "source_max_kbps must be an integer from 8 to 640",
    );
}

// Scenario Outline: Errors in the attributes of confirmed.txt (@RP-11 @RF-15), row "formats= on a file"
#[test]
fn rf15_confirmed_error_formats_on_a_file() {
    confirmed_attribute_error(
        "lossy-attr-formats",
        &format!("{FALSO} | fake formats=lossy"),
        "formats= applies to folders only",
    );
}

// Scenario Outline: Errors in the attributes of confirmed.txt (@RP-11 @RF-15), row "invalid chain id"
#[test]
fn rf15_confirmed_error_invalid_chain_id() {
    confirmed_attribute_error(
        "lossy-attr-chain",
        &format!("{FALSO} | fake chain=Álbum"),
        "invalid id 'Álbum' for chain (1 to 16 letters, digits, '_' or '-')",
    );
}

// Scenario Outline: Errors in the attributes of confirmed.txt (@RP-11 @RF-15), row "upstream on a real line"
#[test]
fn rf15_confirmed_error_upstream_on_a_real_line() {
    confirmed_attribute_error(
        "lossy-attr-real-upstream",
        &format!("{FALSO} | real upstream=aac"),
        "upstream=, source_max_kbps= and weak apply to fake lines only",
    );
}

// Scenario Outline: Errors in the attributes of confirmed.txt (@RP-11 @RF-15), row "upstream on a lossless file"
#[test]
fn rf15_confirmed_error_upstream_on_a_lossless_file() {
    confirmed_attribute_error(
        "lossy-attr-lossless-upstream",
        "Compras\\Ñandú – Ácido, Vol. 1 (2024)\\03 – Bonus.flac | fake upstream=aac",
        "upstream=, source_max_kbps= and weak apply to lossy files only",
    );
}

// Scenario Outline: Errors in the attributes of confirmed.txt (@RP-11 @RF-15), row "unknown kind"
#[test]
fn rf15_confirmed_error_unknown_kind() {
    confirmed_attribute_error(
        "lossy-attr-kind",
        &format!("{FALSO} | dudoso"),
        "unknown kind 'dudoso' (expected fake, real or false-positive)",
    );
}

// Scenario Outline: Errors in the attributes of confirmed.txt (@RP-11 @RF-15), row "not an audio file"
#[test]
fn rf15_confirmed_error_not_a_supported_audio_file() {
    confirmed_attribute_error(
        "lossy-attr-not-audio",
        "Colección\\Álbum Ü\\folder.jpg | fake",
        "not a supported audio file (FLAC, WAV, MP3, M4A or Opus)",
    );
}

// ---- Labels: grammar -----------------------------------------------------------------------------

// Scenario: A false-positive line takes no attributes (@RP-11 @RF-15)
#[test]
fn rf15_a_false_positive_line_takes_no_attributes() {
    let sample = Sample::new_lossy("lossy-fp-attributes");
    let mut lines = sample.lossy_confirmed_lines();
    lines[2] = "Compras\\Ñandú – Ácido, Vol. 1 (2024)\\02 – Canción.flac | false-positive group=G1 | detector antiguo"
        .to_string();
    sample.write_confirmed(&lines);
    let o = sample.accept_lossy();
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert_eq!(
        stderr_lines(&o),
        vec![
            "confirmed.txt:3: false-positive lines take no attributes; write them on the trusted.txt line"
        ]
    );
}

// Scenario: A trusted lossy file is an honest negative, not a transcode source (@RP-11 @RF-15)
#[test]
fn rf15_a_trusted_lossy_file_is_an_honest_negative_not_a_source() {
    let sample = Sample::new_lossy("lossy-trusted-file");
    let mut confirmed = sample.lossy_confirmed_lines();
    confirmed.remove(4); // line 5: the real label of the m4a
    sample.write_confirmed(&confirmed);
    let mut trusted = sample.lossy_trusted_lines();
    trusted.push("Pérdidas\\Vídeo\\08 – Señal ’AAC’.m4a | tienda".to_string());
    sample.write_trusted(&trusted);
    let o = sample.accept_lossy();
    assert_ok(&o);
    let labels = labels(&o, &sample);
    let row = corpus_row(&labels, TRACK_13).expect("the m4a has a row");
    assert_eq!(labels.get(row, "truth"), "real");
    assert_eq!(labels.get(row, "origin"), "trusted");
    assert_eq!(labels.get(row, "class"), "primary");
    assert_eq!(labels.get(row, "domain"), "lossy");
    assert!(
        labels
            .rows
            .iter()
            .all(|r| labels.get(r, "source_path") != TRACK_13),
        "a lossy file is never a source"
    );
}

// Scenario: A folder line with formats=lossy includes MP3 and M4A files only (@RP-11 @RF-15)
#[test]
fn rf15_a_folder_line_with_formats_lossy_includes_mp3_and_m4a_only() {
    let sample = Sample::new_lossy("lossy-folder-formats");
    let mut confirmed = sample.lossy_confirmed_lines();
    confirmed.splice(3..6, ["Pérdidas\\Vídeo | real formats=lossy".to_string()]);
    sample.write_confirmed(&confirmed);
    let o = sample.accept_lossy();
    assert_ok(&o);
    let labels = labels(&o, &sample);
    for path in [TRACK_12, TRACK_13] {
        let row = corpus_row(&labels, path).unwrap_or_else(|| panic!("no row for {path}"));
        assert_eq!(labels.get(row, "truth"), "real", "{path}");
        assert_eq!(labels.get(row, "origin"), "confirmed_real", "{path}");
    }
    assert!(
        corpus_row(&labels, TRACK_14).is_none(),
        "the Opus file is never enumerated under a folder"
    );
}

// Scenario: A real label on a lossless file is excluded after extraction (@RP-11 @RF-15)
#[test]
fn rf15_a_real_label_on_a_lossless_file_is_excluded_after_extraction() {
    let sample = Sample::new_lossy("lossy-real-on-alac");
    place_lossy("m4a_alac_full_band", &sample.corpus.join(ALAC_TRACK));
    let mut confirmed = sample.lossy_confirmed_lines();
    confirmed.push("Pérdidas\\Vídeo\\10 – Sin pérdida.m4a | real".to_string());
    sample.write_confirmed(&confirmed);
    let o = sample.accept_lossy();
    assert_ok(&o);
    let labels = labels(&o, &sample);
    assert!(
        corpus_row(&labels, ALAC_TRACK).is_none(),
        "the file has no labels.csv row"
    );
    let header = report_text(&o, "report.md");
    assert!(
        header.contains(
            "confirmed.txt line 8: real applies to lossy files; the track holds ALAC (excluded)"
        ),
        "report header: {header}"
    );
}

// Scenario: upstream=by-rate is resolved from the sample rate (@RP-11 @RF-15)
#[test]
fn rf15_upstream_by_rate_is_resolved_from_the_sample_rate() {
    let sample = Sample::new_lossy("lossy-by-rate");
    let mut confirmed = sample.lossy_confirmed_lines();
    confirmed[3] = format!("{FALSO} | fake upstream=by-rate");
    sample.write_confirmed(&confirmed);
    let o = sample.accept_lossy();
    assert_ok(&o);
    let labels = labels(&o, &sample);
    let row = corpus_row(&labels, TRACK_12).expect("row");
    assert_eq!(labels.get(row, "upstream"), "aac");
    assert_eq!(labels.get(row, "source_bitrate_kbps"), "128");
}

// ---- Split units -----------------------------------------------------------------------------------

// Scenario: A group makes two albums one split unit (@RP-11 @RF-15)
#[test]
fn rf15_a_group_makes_two_albums_one_split_unit() {
    let sample = Sample::new_lossy("lossy-group-unit");
    let mut trusted = sample.lossy_trusted_lines();
    trusted[3] = "Compras\\Ñandú – Ácido, Vol. 1 (2024) | tienda group=G1 | comprado".to_string();
    trusted[5] = "Colección\\Álbum Ü | analisis-limpio group=G1".to_string();
    sample.write_trusted(&trusted);
    let o = sample.accept_lossy();
    assert_ok(&o);
    let labels = labels(&o, &sample);
    let rows = rows_under(
        &labels,
        &["Compras/Ñandú – Ácido, Vol. 1 (2024)", "Colección/Álbum Ü"],
    );
    assert!(rows.len() > 10, "generated rows included: {}", rows.len());
    let first_split = labels.get(rows[0], "split").to_string();
    for row in &rows {
        assert_eq!(labels.get(row, "split_unit"), "G1", "{row:?}");
        assert_eq!(labels.get(row, "split"), first_split, "{row:?}");
    }
    let w1 = labels.find("watch_id", "W1").expect("watch track W1");
    assert_eq!(labels.get(w1, "path"), TRACK_2);
    assert_eq!(
        labels.get(w1, "split_unit"),
        "G1",
        "from its trusted.txt line"
    );
    let primary = o.unit_strata.get("primary").copied().unwrap_or((0, 0));
    assert_eq!(primary.0 + primary.1, 1, "G1 is counted once, as primary");
    assert!(
        !o.unit_strata.contains_key("clean"),
        "the clean album joined G1: {:?}",
        o.unit_strata
    );
}

// Scenario: A chain is one split unit (@RP-11 @RF-15)
#[test]
fn rf15_a_chain_is_one_split_unit() {
    let shared = shared_lossy();
    assert_ok(&shared.outcome);
    let labels = labels(&shared.outcome, &shared.sample);
    let rows = rows_under(&labels, &["Revisión/Ørsted", "Cadena/MP3 – Ørsted"]);
    assert!(rows.len() >= 4, "{} rows", rows.len());
    let split = labels.get(rows[0], "split").to_string();
    for row in rows {
        assert_eq!(labels.get(row, "split_unit"), "chain:C1", "{row:?}");
        assert_eq!(labels.get(row, "split"), split, "{row:?}");
    }
}

// Scenario: An album split over two units is an error (@RP-11 @RF-15)
#[test]
fn rf15_an_album_split_over_two_units_is_an_error() {
    let sample = Sample::new_lossy("lossy-album-two-units");
    let mut trusted = sample.lossy_trusted_lines();
    trusted[7] = "Sueltas\\Mixto\\05 – Cuatro.flac | analisis group=G1".to_string();
    sample.write_trusted(&trusted);
    let o = sample.accept_lossy();
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert!(
        o.stderr.contains("album has tracks in several split units"),
        "{}",
        o.stderr
    );
}

// Scenario: A chain split over two groups is an error (@RP-11 @RF-15)
#[test]
fn rf15_a_chain_split_over_two_groups_is_an_error() {
    let sample = Sample::new_lossy("lossy-chain-two-groups");
    let mut confirmed = sample.lossy_confirmed_lines();
    confirmed[6] =
        "Cadena\\MP3 – Ørsted\\01 – Tres.mp3 | fake upstream=aac chain=C1 group=G2".to_string();
    sample.write_confirmed(&confirmed);
    let o = sample.accept_lossy();
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert!(
        o.stderr
            .contains("chain C1 spans split units G2 and chain:C1"),
        "{}",
        o.stderr
    );
}

// Scenario: Adding lossy-only labels does not move a lossless album (@RP-11 @RF-15)
#[test]
fn rf15_adding_lossy_only_labels_does_not_move_a_lossless_album() {
    let before = shared_v2();
    assert_ok(&before.outcome);
    let before_labels = labels(&before.outcome, &before.sample);
    let split_of = |t: &Table| -> std::collections::BTreeMap<String, String> {
        t.rows
            .iter()
            .map(|r| {
                (
                    t.get(r, "album_id").to_string(),
                    t.get(r, "split").to_string(),
                )
            })
            .collect()
    };
    let expected = split_of(&before_labels);

    let sample = Sample::new_lossy("lossy-no-move");
    sample.write_trusted(&sample.trusted_lines());
    let mut confirmed = sample.confirmed_lines();
    confirmed.extend(sample.lossy_confirmed_lines()[3..6].iter().cloned());
    sample.write_confirmed(&confirmed);
    sample.write_config(None);
    let o = sample.accept_v2();
    assert_ok(&o);
    let after_labels = labels(&o, &sample);
    assert!(
        corpus_row(&after_labels, TRACK_12).is_some(),
        "the lossy labels are in"
    );
    let after = split_of(&after_labels);
    for (album, split) in expected {
        assert_eq!(after.get(&album), Some(&split), "album {album}");
    }
}

// ---- Configuration, grid, command lines and defaults ---------------------------------------------

// Scenario: The default config is calibration.v2.toml (@RP-11 @RF-15), the file itself
#[test]
fn rf15_calibration_v2_loads_with_its_lossy_entries_and_v1_still_loads() {
    let v2 = Config::parse(&read(&spec_config_v2())).expect("calibration.v2.toml loads");
    assert_eq!(v2.config_version, 2);
    assert_eq!(v2.generate.lossy.len(), 14);
    assert_eq!(v2.generate.settings.len(), 9, "every v1 setting is kept");
    assert_eq!(v2.labels.upstream_by_rate.len(), 2);
    let aac = &v2.labels.upstream_by_rate["44100"];
    assert_eq!(
        (aac.upstream.as_str(), aac.source_max_kbps),
        ("aac", Some(128))
    );
    let opus = &v2.labels.upstream_by_rate["48000"];
    assert_eq!(
        (opus.upstream.as_str(), opus.source_max_kbps),
        ("opus", None)
    );
    assert!(v2.generate.settings.iter().all(|s| s.lossless_transcode));

    let v1 = Config::parse(&read(&spec_config())).expect("calibration.v1.toml loads");
    assert!(v1.generate.lossy.is_empty());
    assert!(v1.labels.upstream_by_rate.is_empty());
    assert_eq!(v1.generate.settings, v2.generate.settings);
}

// Scenario: The default config is calibration.v2.toml (@RP-11 @RF-15)
#[test]
fn rf15_the_default_config_is_calibration_v2() {
    let args = |v: &[&str]| v.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
    let expected = PathBuf::from("specs/calibration/calibration.v2.toml");
    let Ok(Command::Accept(a)) = parse_args(&args(&["accept"])) else {
        panic!("accept");
    };
    assert_eq!(a.config, expected);
    let Ok(Command::Calibrate(c)) = parse_args(&args(&["calibrate"])) else {
        panic!("calibrate");
    };
    assert_eq!(c.config, expected);
    let Ok(Command::Accept(explicit)) = parse_args(&args(&[
        "accept",
        "--config",
        "specs/calibration/calibration.v1.toml",
    ])) else {
        panic!("accept with --config");
    };
    assert_eq!(
        explicit.config,
        PathBuf::from("specs/calibration/calibration.v1.toml"),
        "v1 stays loadable and selectable"
    );
}

/// Scenario Outline: Invalid lossy entries in the config (@RP-11 @RF-15). The binary runs `accept`
/// with `calibration.v2.toml` plus `change`; the config is checked before any ffmpeg call.
fn invalid_lossy_entry(test: &str, change: &str, message: &str) {
    let sample = Sample::new(test);
    let config = sample.write_scratch(
        "calibration.invalid.toml",
        &format!(
            "{}
{change}
",
            read(&spec_config_v2())
        ),
    );
    let before = names(&sample.calibration);
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_fakeflac-calibrate"))
        .arg("accept")
        .arg("--config")
        .arg(&config)
        .current_dir(repo_root())
        .stdin(std::process::Stdio::null())
        .env("FAKEFLAC_CORPUS", &sample.corpus)
        .env("FAKEFLAC_CALIBRATION", &sample.calibration)
        .env_remove("FAKEFLAC_FFMPEG")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(2), "stderr: {stderr}");
    assert!(stderr.contains(message), "{stderr} lacks '{message}'");
    assert_eq!(names(&sample.calibration), before, "nothing is written");
}

// Scenario Outline: Invalid lossy entries in the config (@RP-11 @RF-15), row "stage without bitrate"
#[test]
fn rf15_invalid_lossy_entry_stage_without_bitrate() {
    invalid_lossy_entry(
        "lossy-cfg-no-bitrate",
        "[[generate.lossy]]\nid = \"x\"\nstages = [\"v0\", \"cbr320\"]",
        "lossy entry x: stage v0 has no bitrate_kbps",
    );
}

// Scenario Outline: Invalid lossy entries in the config (@RP-11 @RF-15), row "unknown stage"
#[test]
fn rf15_invalid_lossy_entry_unknown_stage() {
    invalid_lossy_entry(
        "lossy-cfg-unknown-stage",
        "[[generate.lossy]]\nid = \"y\"\nstages = [\"nope\"]",
        "lossy entry y: unknown stage nope",
    );
}

// Scenario Outline: Invalid lossy entries in the config (@RP-11 @RF-15), row "no stages"
#[test]
fn rf15_invalid_lossy_entry_no_stages() {
    invalid_lossy_entry(
        "lossy-cfg-no-stages",
        "[[generate.lossy]]\nid = \"w\"\nstages = []",
        "lossy entry w: no stages",
    );
}

// Scenario Outline: Invalid lossy entries in the config (@RP-11 @RF-15), row "Opus output"
#[test]
fn rf15_invalid_lossy_entry_opus_output() {
    invalid_lossy_entry(
        "lossy-cfg-opus",
        "[[generate.lossy]]\nid = \"z\"\nstages = [\"opus128\"]",
        "lossy entry z: Opus output is not supported yet",
    );
}

// Scenario Outline: Invalid lossy entries in the config (@RP-11 @RF-15), row "defined twice"
#[test]
fn rf15_invalid_lossy_entry_defined_twice() {
    invalid_lossy_entry(
        "lossy-cfg-twice",
        "[[generate.lossy]]\nid = \"cbr128\"\nstages = [\"cbr128\"]",
        "lossy entry cbr128 is defined twice",
    );
}

// Scenario: Lossy parameters cannot be searched (@RP-11 @RF-15)
#[test]
fn rf15_lossy_parameters_cannot_be_searched() {
    let sample = Sample::new("lossy-grid");
    let text = format!(
        "grid_version = 1\nbaseline = '{}'\nmust_catch = [\"fx128\"]\nmax_points = 1000\n\n\
         [weights]\nfp = 10.0\nfn = 1.0\nsuspicious_negative = 0.5\nsuspicious_positive = 0.1\n\n\
         [values]\n\"lossy.fake_shortfall_hz\" = [1000.0, 1200.0]\n",
        v5_params().display()
    );
    let grid = sample.write_scratch("grid.toml", &text);
    let o = sample.calibrate(&grid);
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert!(
        o.stderr
            .contains("lossy.fake_shortfall_hz: lossy parameters cannot be searched yet"),
        "{}",
        o.stderr
    );
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

// Scenario: Cascade command lines (@RP-11 @RF-15)
#[test]
fn rf15_cascade_command_lines() {
    let config = Config::parse(&read(&spec_config_v2())).unwrap();
    let entry = config
        .generate
        .lossy
        .iter()
        .find(|e| e.id == "aac128_cbr320")
        .expect("the lossy entry aac128_cbr320");
    assert_eq!(entry.stages, vec!["aac128", "cbr320"]);
    let stages: Vec<_> = entry
        .stages
        .iter()
        .map(|id| {
            config
                .generate
                .settings
                .iter()
                .find(|s| &s.id == id)
                .unwrap()
        })
        .collect();
    let source = Path::new(TRACK_1);
    let generated = PathBuf::from("generated");
    let id = "0123456789abcdef";
    let s1 = generated.join(format!("{id}.s1.m4a"));
    let last = generated.join(format!("{id}.mp3"));

    let calls = lossy_args(source, None, &stages, &generated, id);
    assert_eq!(calls.len(), 2, "2 encode calls and no decode call");

    let mut first = strings(&["-nostdin", "-hide_banner", "-loglevel", "error", "-i"]);
    first.push(TRACK_1.to_string());
    first.extend(strings(&[
        "-map",
        "0:a:0",
        "-map_metadata",
        "-1",
        "-c:a",
        "aac",
        "-b:a",
        "128k",
        "-y",
    ]));
    first.push(s1.to_string_lossy().into_owned());
    assert_eq!(calls[0], first, "each one a list of separate arguments");

    let mut second = strings(&["-nostdin", "-hide_banner", "-loglevel", "error", "-i"]);
    second.push(s1.to_string_lossy().into_owned());
    second.extend(strings(&[
        "-map",
        "0:a:0",
        "-map_metadata",
        "-1",
        "-c:a",
        "libmp3lame",
        "-b:a",
        "320k",
        "-y",
    ]));
    second.push(last.to_string_lossy().into_owned());
    assert_eq!(calls[1], second);
}

// ---- Round 2: follow-ups of the spec PR #57 and surviving mutants -------------------------------------

// Scenario Outline: Errors in the attributes of confirmed.txt (@RP-11 @RF-15), row "real folder, formats=all"
#[test]
fn rf15_confirmed_error_real_folder_with_formats_all() {
    confirmed_attribute_error(
        "lossy-real-folder-all",
        "Pérdidas\\Vídeo | real formats=all",
        "real applies to lossy files only",
    );
}

// Scenario Outline: Errors in the attributes of confirmed.txt (@RP-11 @RF-15): a `real` folder line with
// formats=lossless (lossy.md section 1.2)
#[test]
fn rf15_confirmed_error_real_folder_with_formats_lossless() {
    confirmed_attribute_error(
        "lossy-real-folder-lossless",
        "Pérdidas\\Vídeo | real formats=lossless",
        "real applies to lossy files only",
    );
}

// Scenario Outline: Errors in the attributes of confirmed.txt (@RP-11 @RF-15), row "real on a FLAC file"
#[test]
fn rf15_confirmed_error_real_on_a_flac_file() {
    confirmed_attribute_error(
        "lossy-real-flac",
        "Compras\\Ñandú – Ácido, Vol. 1 (2024)\\03 – Bonus.flac | real",
        "real applies to lossy files only",
    );
}

// Scenario Outline: Errors in the attributes of confirmed.txt (@RP-11 @RF-15): a `real` file entry naming
// a WAV file (lossy.md section 1.2)
#[test]
fn rf15_confirmed_error_real_on_a_wav_file() {
    confirmed_attribute_error(
        "lossy-real-wav",
        "Revisión\\Ørsted\\01 – Tres.wav | real",
        "real applies to lossy files only",
    );
}

// Scenario Outline: Errors in the attributes of confirmed.txt (@RP-11 @RF-15): `weak` is a bare flag, so
// `weak=x` is an error on its line (lossy.md section 1.2 gives no message for it)
#[test]
fn rf15_confirmed_error_weak_with_a_value() {
    let sample = Sample::new_lossy("lossy-attr-weak-value");
    let mut lines = sample.lossy_confirmed_lines();
    lines[3] = format!("{FALSO} | fake weak=x");
    sample.write_confirmed(&lines);
    let o = sample.accept_lossy();
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    let errors = stderr_lines(&o);
    assert_eq!(errors.len(), 1, "one error: {errors:?}");
    assert!(
        errors[0].starts_with("confirmed.txt:4: ") && errors[0].contains("weak"),
        "{errors:?}"
    );
}

// Scenario: A folder line with formats=lossy includes MP3 and M4A files only (@RP-11 @RF-15): the default
// `formats` of a `real` folder line is `lossy` (lossy.md sections 1.1 and 1.2)
#[test]
fn rf15_a_real_folder_line_defaults_to_lossy_files() {
    let sample = Sample::new_lossy("lossy-real-folder-default");
    // A FLAC under the folder: `formats=all` would take it, the default must not.
    place(
        "full_band",
        Container::Flac,
        &sample.corpus.join("Pérdidas/Vídeo/11 – Pista.flac"),
    );
    let mut confirmed = sample.lossy_confirmed_lines();
    confirmed.splice(3..6, ["Pérdidas\\Vídeo | real".to_string()]);
    sample.write_confirmed(&confirmed);
    let o = sample.accept_lossy();
    assert_ok(&o);
    let labels = labels(&o, &sample);
    for path in [TRACK_12, TRACK_13] {
        let row = corpus_row(&labels, path).unwrap_or_else(|| panic!("no row for {path}"));
        assert_eq!(labels.get(row, "origin"), "confirmed_real", "{path}");
    }
    for path in ["Pérdidas/Vídeo/11 – Pista.flac", TRACK_14] {
        assert!(corpus_row(&labels, path).is_none(), "{path} is not taken");
    }
}

// A track in both a real and a fake line (lossy.md section 1.3): the error is on the later line, once.
// Here the fake file line comes first.
#[test]
fn rf15_a_track_in_a_fake_line_and_a_later_real_line_is_listed_twice() {
    let sample = Sample::new_lossy("lossy-fake-then-real");
    let mut confirmed = sample.lossy_confirmed_lines();
    confirmed.push("Pérdidas\\Vídeo | real".to_string()); // line 8, covers file 07 of line 4
    sample.write_confirmed(&confirmed);
    let o = sample.accept_lossy();
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert_eq!(
        stderr_lines(&o),
        vec!["confirmed.txt:8: already listed at line 4"]
    );
}

// Same, with the real folder line first: the error is on the later (fake) line.
#[test]
fn rf15_a_track_in_a_real_line_and_a_later_fake_line_is_listed_twice() {
    let sample = Sample::new_lossy("lossy-real-then-fake");
    let mut confirmed = sample.lossy_confirmed_lines();
    let fake_file = confirmed.remove(3);
    confirmed.insert(3, "Pérdidas\\Vídeo | real".to_string()); // line 4
    confirmed.insert(4, fake_file); // line 5
    sample.write_confirmed(&confirmed);
    let o = sample.accept_lossy();
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert_eq!(
        stderr_lines(&o),
        vec!["confirmed.txt:5: already listed at line 4"]
    );
}

// A track that a `real` line labels and `trusted.txt` includes too is `also trusted` (lossy.md section 1.3)
#[test]
fn rf15_a_real_track_that_is_also_trusted_is_an_error() {
    let sample = Sample::new_lossy("lossy-real-also-trusted");
    let mut trusted = sample.lossy_trusted_lines();
    trusted.push("Pérdidas\\Vídeo\\08 – Señal ’AAC’.m4a | tienda".to_string()); // line 9
    sample.write_trusted(&trusted);
    let o = sample.accept_lossy(); // confirmed.txt line 5 says `real` for the same file
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert_eq!(
        stderr_lines(&o),
        vec!["confirmed.txt:5: also trusted (trusted.txt line 9)"]
    );
}

// Scenario: An album split over two units is an error (@RP-11 @RF-15): the exact message, lossy.md section 3
#[test]
fn rf15_an_album_split_over_two_units_names_the_lines_of_its_units() {
    let sample = Sample::new_lossy("lossy-album-two-units-refs");
    let mut trusted = sample.lossy_trusted_lines();
    trusted[7] = "Sueltas\\Mixto\\05 – Cuatro.flac | analisis group=G1".to_string();
    sample.write_trusted(&trusted);
    let o = sample.accept_lossy();
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert_eq!(
        stderr_lines(&o),
        vec!["album has tracks in several split units (trusted.txt:8, confirmed.txt:2)"]
    );
}

// Scenario: An album split over two units is an error (@RP-11 @RF-15): one error per album, each ref the
// FIRST line (by file, then line number) that gives one of the album's tracks that unit, several errors
// ordered by their first ref (not by album id), no file prefix
#[test]
fn rf15_several_split_albums_are_reported_one_error_each_in_order_of_their_first_ref() {
    let sample = Sample::new_lossy("lossy-albums-two-units-order");
    let mut trusted = sample.lossy_trusted_lines();
    let revision = trusted[6].clone();
    trusted[6] = "Sueltas\\Mixto\\05 – Cuatro.flac | analisis group=G1".to_string(); // line 7
    trusted[7] = revision; // line 8: Revisión/Ørsted/, chain=C1
                           // Both tracks are unit G5; the order of their paths is the reverse of the order of their lines.
    trusted.push("Revisión\\Ørsted\\03 – Corto.flac | revisado group=G5".to_string()); // line 9
    trusted.push("Revisión\\Ørsted\\02 – Roto.flac | revisado group=G5".to_string()); // line 10
    sample.write_trusted(&trusted);
    let o = sample.accept_lossy();
    assert_eq!(o.exit_code, 2, "stderr: {}", o.stderr);
    assert_eq!(
        stderr_lines(&o),
        vec![
            "album has tracks in several split units (trusted.txt:7, confirmed.txt:2)",
            "album has tracks in several split units (trusted.txt:8, trusted.txt:9)",
        ]
    );
}

// ---- [labels.upstream_by_rate] checks (lossy.md section 1.5) -------------------------------------------

// Scenario Outline: Invalid lossy entries in the config (@RP-11 @RF-15): the checks of
// [labels.upstream_by_rate] exit 2 before any extraction
#[test]
fn rf15_upstream_by_rate_key_with_non_digits_is_invalid() {
    invalid_lossy_entry(
        "lossy-cfg-rate-key-text",
        "[labels.upstream_by_rate.\"44.1k\"]\nupstream = \"aac\"",
        "labels.upstream_by_rate: invalid sample rate '44.1k' (expected an integer in Hz, for example 44100)",
    );
}

#[test]
fn rf15_upstream_by_rate_key_with_a_leading_zero_is_invalid() {
    invalid_lossy_entry(
        "lossy-cfg-rate-key-zero",
        "[labels.upstream_by_rate.\"044100\"]\nupstream = \"aac\"",
        "labels.upstream_by_rate: invalid sample rate '044100' (expected an integer in Hz, for example 44100)",
    );
}

#[test]
fn rf15_upstream_by_rate_key_zero_is_invalid() {
    invalid_lossy_entry(
        "lossy-cfg-rate-key-0",
        "[labels.upstream_by_rate.\"0\"]\nupstream = \"aac\"",
        "labels.upstream_by_rate: invalid sample rate '0' (expected an integer in Hz, for example 44100)",
    );
}

#[test]
fn rf15_upstream_by_rate_upstream_by_rate_is_invalid() {
    invalid_lossy_entry(
        "lossy-cfg-rate-by-rate",
        "[labels.upstream_by_rate.\"32000\"]\nupstream = \"by-rate\"",
        "labels.upstream_by_rate '32000': invalid upstream 'by-rate' (expected aac, opus, mp3 or unknown)",
    );
}

#[test]
fn rf15_upstream_by_rate_unknown_codec_is_invalid() {
    invalid_lossy_entry(
        "lossy-cfg-rate-codec",
        "[labels.upstream_by_rate.\"32000\"]\nupstream = \"vorbis\"",
        "labels.upstream_by_rate '32000': invalid upstream 'vorbis' (expected aac, opus, mp3 or unknown)",
    );
}

#[test]
fn rf15_upstream_by_rate_source_bound_below_range_is_invalid() {
    invalid_lossy_entry(
        "lossy-cfg-rate-bound-low",
        "[labels.upstream_by_rate.\"32000\"]\nupstream = \"mp3\"\nsource_max_kbps = 7",
        "labels.upstream_by_rate '32000': source_max_kbps must be an integer from 8 to 640",
    );
}

#[test]
fn rf15_upstream_by_rate_source_bound_above_range_is_invalid() {
    invalid_lossy_entry(
        "lossy-cfg-rate-bound-high",
        "[labels.upstream_by_rate.\"32000\"]\nupstream = \"mp3\"\nsource_max_kbps = 641",
        "labels.upstream_by_rate '32000': source_max_kbps must be an integer from 8 to 640",
    );
}

// Pins: the ends of the range and every codec of the table are valid; a missing `upstream` or an unknown
// field is a TOML error.
#[test]
fn rf15_upstream_by_rate_valid_entries_load_and_malformed_ones_are_toml_errors() {
    let base = read(&spec_config_v2());
    let with = |extra: &str| Config::parse(&format!("{base}\n{extra}\n"));
    for (rate, codec, bound) in [
        ("22050", "mp3", "source_max_kbps = 8"),
        ("32000", "unknown", "source_max_kbps = 640"),
        ("96000", "opus", ""),
    ] {
        let table =
            format!("[labels.upstream_by_rate.\"{rate}\"]\nupstream = \"{codec}\"\n{bound}");
        assert!(with(&table).is_ok(), "{table}");
    }
    assert!(with("[labels.upstream_by_rate.\"22050\"]\nsource_max_kbps = 128").is_err());
    assert!(with("[labels.upstream_by_rate.\"22050\"]\nupstream = \"aac\"\nextra = 1").is_err());
}

// ---- Round 3: per-stratum holdout warning, real_excluded once, I5 -------------------------------------

/// The warning lines of `summary.md` (the `## Warnings` section).
fn warning_lines(o: &fakeflac_calibrate::Outcome) -> Vec<String> {
    let summary = report_text(o, "summary.md");
    let mut in_warnings = false;
    let mut out = Vec::new();
    for line in summary.lines() {
        if line.starts_with("## ") {
            in_warnings = line == "## Warnings";
        } else if in_warnings && line.starts_with("- ") {
            out.push(line.to_string());
        }
    }
    out
}

/// Scenario: The holdout warning applies per stratum (@RP-11 @RF-15), lossy.md section 3, last bullet.
/// The only primary unit is `group=G1` over two `tienda` albums, `holdout_fraction = 0.5`: the unit goes to
/// holdout (1 unit, 2 albums). The album-class warning is silent (2 albums in holdout), the per-stratum
/// one is not: 1 unit in holdout is fewer than 2. The spec does not fix the wording for a lossless
/// stratum: the test asserts the stratum key, the counts and the threshold, in the form that `lossy:`
/// strata use today (`stratum <key>: <h> units in holdout (fewer than <min>), <t> in tune`).
#[test]
fn rf15_one_primary_unit_over_two_albums_warns_per_stratum() {
    let sample = Sample::new_lossy("lossy-stratum-one-unit");
    sample.write_lossy_config(Some(0.5));
    let mut trusted = sample.lossy_trusted_lines();
    trusted[3] = "Compras\\Ñandú – Ácido, Vol. 1 (2024) | tienda group=G1 | comprado".to_string();
    trusted[5] = "Colección\\Álbum Ü | tienda group=G1".to_string();
    sample.write_trusted(&trusted);
    let o = sample.accept_lossy();
    assert_ok(&o);
    assert_eq!(
        o.unit_strata.get("primary"),
        Some(&(0, 1)),
        "{:?}",
        o.unit_strata
    );
    let warnings = warning_lines(&o);
    assert!(
        !warnings.iter().any(|w| w.contains("class primary:")),
        "2 albums in holdout: no album-class warning: {warnings:?}"
    );
    let line = warnings
        .iter()
        .find(|w| w.contains("stratum primary"))
        .unwrap_or_else(|| panic!("no per-stratum warning for primary: {warnings:?}"));
    assert!(line.contains("1 units in holdout"), "{line}");
    assert!(line.contains("fewer than 2"), "{line}");
    assert!(line.contains("0 in tune"), "{line}");
}

/// Scenario: The holdout warning applies per stratum (@RP-11 @RF-15). `min_holdout_albums = 3`, two primary
/// units (`G1`, `G2`) of two albums each: both go to holdout (4 albums, no album-class warning) but 2 units
/// are fewer than 3.
#[test]
fn rf15_two_primary_units_below_min_holdout_warn_per_stratum() {
    let sample = Sample::new_lossy("lossy-stratum-two-units");
    let path = sample.write_lossy_config(None);
    let text = read(&path);
    assert!(text.contains("min_holdout_albums = 2"));
    std::fs::write(
        &path,
        text.replace("min_holdout_albums = 2", "min_holdout_albums = 3"),
    )
    .unwrap();
    for album in ["Tienda2/Uno", "Tienda2/Dos"] {
        place(
            "full_band",
            Container::Flac,
            &sample.corpus.join(album).join("01 – a.flac"),
        );
    }
    let mut trusted = sample.lossy_trusted_lines();
    trusted[3] = "Compras\\Ñandú – Ácido, Vol. 1 (2024) | tienda group=G1 | comprado".to_string();
    trusted[5] = "Colección\\Álbum Ü | tienda group=G1".to_string();
    trusted.push("Tienda2\\Uno | tienda group=G2".to_string());
    trusted.push("Tienda2\\Dos | tienda group=G2".to_string());
    sample.write_trusted(&trusted);
    let o = sample.accept_lossy();
    assert_ok(&o);
    assert_eq!(
        o.unit_strata.get("primary"),
        Some(&(0, 2)),
        "{:?}",
        o.unit_strata
    );
    let warnings = warning_lines(&o);
    assert!(
        !warnings.iter().any(|w| w.contains("class primary:")),
        "4 albums in holdout: no album-class warning: {warnings:?}"
    );
    let line = warnings
        .iter()
        .find(|w| w.contains("stratum primary"))
        .unwrap_or_else(|| panic!("no per-stratum warning for primary: {warnings:?}"));
    assert!(line.contains("2 units in holdout"), "{line}");
    assert!(line.contains("fewer than 3"), "{line}");
    assert!(line.contains("0 in tune"), "{line}");
}

/// Pin (owner decision (a)): the existing per-album-class warning stays where it is. In the default
/// lossy sample the only primary album is in tune.
#[test]
fn rf15_the_album_class_holdout_warning_is_kept() {
    let shared = shared_lossy();
    assert_ok(&shared.outcome);
    assert_eq!(
        shared.outcome.unit_strata["primary"],
        (1, 0),
        "{:?}",
        shared.outcome.unit_strata
    );
    let warnings = warning_lines(&shared.outcome);
    assert!(
        warnings
            .iter()
            .any(|w| w == "- class primary: 0 albums in holdout (fewer than 2), 1 in tune"),
        "{warnings:?}"
    );
}

/// Scenario: A real label on a lossless file is excluded after extraction (@RP-11 @RF-15), lossy.md section 1.3
/// (mutant M7): two params files with different extraction settings are two extraction groups; the excluded
/// label is printed once in each of `summary.md` and `report.md`.
#[test]
fn rf15_a_real_excluded_line_is_printed_once_across_extraction_groups() {
    let sample = Sample::new_lossy("lossy-real-excluded-once");
    place_lossy("m4a_alac_full_band", &sample.corpus.join(ALAC_TRACK));
    let mut confirmed = sample.lossy_confirmed_lines();
    confirmed.push("Pérdidas\\Vídeo\\10 – Sin pérdida.m4a | real".to_string());
    sample.write_confirmed(&confirmed);
    let o = sample.accept(&[v2_params(), sample.blackman_params()]);
    assert_ok(&o);
    assert_eq!(o.cache.len(), 2, "two extraction groups");
    let needle =
        "confirmed.txt line 8: real applies to lossy files; the track holds ALAC (excluded)";
    for file in ["summary.md", "report.md"] {
        let text = report_text(&o, file);
        assert_eq!(text.matches(needle).count(), 1, "{file}");
    }
}

/// Scenario: Lossy watch tracks are listed (@RP-11 @RF-15), lossy.md section 6.7 (mutant M8): a params file
/// without `[lossy]` still lists the lossy `false-positive` track, as a "not evaluated" entry.
#[test]
fn rf15_the_lossy_watch_list_is_listed_for_params_without_a_lossy_section() {
    let sample = Sample::new_lossy("lossy-watch-no-section");
    let mut trusted = sample.lossy_trusted_lines();
    trusted.push(format!("{FALSO} | tienda"));
    sample.write_trusted(&trusted);
    let mut confirmed = sample.lossy_confirmed_lines();
    confirmed[3] = format!("{FALSO} | false-positive");
    sample.write_confirmed(&confirmed);
    let o = sample.accept_lossy();
    assert_ok(&o);
    let (v2, v5) = (&o.evaluations[0], &o.evaluations[1]);
    assert!(v2.lossy.is_none());
    assert_eq!(v2.lossy_watch.len(), 1, "{:?}", v2.lossy_watch);
    assert_eq!(v2.lossy_watch[0].watch_id, "W2");
    assert_eq!(v2.lossy_watch[0].path, TRACK_12);
    assert!(!v2.lossy_watch[0].evaluated, "v2 has no [lossy] section");
    assert_eq!(v5.lossy_watch.len(), 1);
    assert!(v5.lossy_watch[0].evaluated);

    for file in ["summary.md", "report.md"] {
        let text = report_text(&o, file);
        let rows: Vec<&str> = text
            .lines()
            .filter(|l| l.contains("W2") && l.contains("not evaluated"))
            .collect();
        assert_eq!(rows.len(), 1, "{file}: one not-evaluated row for W2");
    }
    assert!(
        !report_text(&o, "summary.md").contains("Falso 320"),
        "summary.md shows the watch id only"
    );
}

// ---- I5: [labels.upstream_by_rate] source_max_kbps outside the integers 8 to 640 ---------------------

fn bad_bound(test: &str, value: &str) {
    invalid_lossy_entry(
        test,
        &format!(
            "[labels.upstream_by_rate.\"32000\"]\nupstream = \"mp3\"\nsource_max_kbps = {value}"
        ),
        "labels.upstream_by_rate '32000': source_max_kbps must be an integer from 8 to 640",
    );
}

// Scenario Outline: Invalid lossy entries in the config (@RP-11 @RF-15): lossy.md section 1.5 reserves TOML
// errors for a missing `upstream` and unknown fields only
#[test]
fn rf15_upstream_by_rate_negative_source_bound_is_invalid() {
    bad_bound("lossy-cfg-rate-bound-neg", "-1");
}

#[test]
fn rf15_upstream_by_rate_fractional_source_bound_is_invalid() {
    bad_bound("lossy-cfg-rate-bound-frac", "128.5");
}

#[test]
fn rf15_upstream_by_rate_huge_source_bound_is_invalid() {
    bad_bound("lossy-cfg-rate-bound-huge", "5000000000");
}
