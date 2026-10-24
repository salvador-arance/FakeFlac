//! Parameter-file scenarios of `specs/features/detection-edge-step.feature` (@RF-04), "Parameter
//! files": the shape of the `[edge]` section is checked when a file is deserialised (serde), its
//! values by `DetectionParams::validate` (rules.md section 4, ADR-0042). Lives in this crate
//! because `toml` is a dev-dependency here, not in `fakeflac-core`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use fakeflac_core::DetectionParams;
use std::path::Path;

fn spec_text(name: &str) -> String {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../specs/detection")
        .join(name);
    std::fs::read_to_string(file).unwrap().replace("\r\n", "\n")
}

const V6: &str = "detection-params.v6.toml";

/// The text of v6 with the (section, key) line replaced (`Some(new line)`) or removed (`None`).
fn edited(section: &str, key: &str, new_line: Option<&str>) -> String {
    let mut current = String::new();
    let mut found = false;
    let mut out = Vec::new();
    for line in spec_text(V6).lines() {
        let t = line.trim();
        if let Some(name) = t.strip_prefix('[').and_then(|r| r.split(']').next()) {
            current = name.to_string();
        }
        if current == section && t.starts_with(&format!("{key} ")) {
            found = true;
            if let Some(n) = new_line {
                out.push(n.to_string());
            }
            continue;
        }
        out.push(line.to_string());
    }
    assert!(found, "{section}.{key} not found in {V6}");
    out.join("\n")
}

fn with_extra_line(section: &str, extra: &str) -> String {
    let mut out = Vec::new();
    for line in spec_text(V6).lines() {
        out.push(line.to_string());
        if line.trim_start().starts_with(&format!("[{section}]")) {
            out.push(extra.to_string());
        }
    }
    out.join("\n")
}

fn accepted(text: &str) -> DetectionParams {
    let p: DetectionParams = toml::from_str(text).expect("the shape is valid");
    p.validate().expect("the values are valid");
    p
}

// ---- Scenario Outline: Valid parameter files deserialise and validate ---------------------------

#[test]
fn rf04_v6_file_is_accepted_and_has_an_edge_section() {
    let p = accepted(&spec_text(V6));
    assert_eq!(p.algorithm_version, 6);
    let e = p.edge.expect("[edge] present");
    assert_eq!(
        (
            e.floor_low_hz,
            e.floor_high_hz,
            e.floor_min_db,
            e.max_median_step_db,
            e.min_residual_db,
            e.max_step_db
        ),
        (20_900.0, 22_000.0, -104.0, 3.0, 15.0, 16.0)
    );
}

#[test]
fn rf04_v5_and_v2_files_are_accepted_and_have_no_edge_section() {
    for file in ["detection-params.v5.toml", "detection-params.v2.toml"] {
        let p = accepted(&spec_text(file));
        assert!(p.edge.is_none(), "{file}");
    }
}

#[test]
fn rf04_the_default_params_are_accepted_and_have_an_edge_section() {
    let p = DetectionParams::default();
    p.validate().expect("the defaults are valid");
    assert!(p.edge.is_some());
}

// v6 is v5 plus [edge]: every other section and value is unchanged (rules.md section 4).
#[test]
fn rf04_v6_is_v5_plus_the_edge_section() {
    let v5: DetectionParams = toml::from_str(&spec_text("detection-params.v5.toml")).unwrap();
    let v6: DetectionParams = toml::from_str(&spec_text(V6)).unwrap();
    let expected = DetectionParams {
        algorithm_version: 6,
        edge: v6.edge.clone(),
        ..v5
    };
    assert_eq!(v6, expected);
}

// ---- Scenario Outline: Values of [edge] at the edge of validity are accepted ---------------------

#[test]
fn rf04_edge_floor_low_hz_at_its_lower_bound_is_accepted() {
    accepted(&edited(
        "edge",
        "floor_low_hz",
        Some("floor_low_hz = 20600.0"),
    ));
}
#[test]
fn rf04_edge_max_step_db_just_above_step_fake_db_is_accepted() {
    accepted(&edited("edge", "max_step_db", Some("max_step_db = 10.01")));
}
#[test]
fn rf04_edge_negative_max_median_step_db_is_accepted() {
    accepted(&edited(
        "edge",
        "max_median_step_db",
        Some("max_median_step_db = -5.0"),
    ));
}

// ---- Scenario Outline: An invalid [edge] section is rejected --------------------------------------

fn assert_rejected_by_deserialisation(text: &str, what: &str) {
    let e = toml::from_str::<DetectionParams>(text).expect_err("the shape is invalid");
    assert!(e.to_string().contains(what), "{what}: {e}");
}

fn assert_rejected_by_validation(text: &str, what: &str) {
    let p: DetectionParams = toml::from_str(text).expect("the shape is valid");
    let e = p.validate().expect_err("the values are invalid");
    assert!(e.to_string().contains(what), "{what}: {e}");
}

#[test]
fn rf04_edge_without_floor_low_hz_is_rejected() {
    assert_rejected_by_deserialisation(&edited("edge", "floor_low_hz", None), "floor_low_hz");
}
#[test]
fn rf04_edge_without_floor_high_hz_is_rejected() {
    assert_rejected_by_deserialisation(&edited("edge", "floor_high_hz", None), "floor_high_hz");
}
#[test]
fn rf04_edge_without_floor_min_db_is_rejected() {
    assert_rejected_by_deserialisation(&edited("edge", "floor_min_db", None), "floor_min_db");
}
#[test]
fn rf04_edge_without_max_median_step_db_is_rejected() {
    assert_rejected_by_deserialisation(
        &edited("edge", "max_median_step_db", None),
        "max_median_step_db",
    );
}
#[test]
fn rf04_edge_without_min_residual_db_is_rejected() {
    assert_rejected_by_deserialisation(&edited("edge", "min_residual_db", None), "min_residual_db");
}
#[test]
fn rf04_edge_without_max_step_db_is_rejected() {
    assert_rejected_by_deserialisation(&edited("edge", "max_step_db", None), "max_step_db");
}
#[test]
fn rf04_edge_with_an_unknown_field_is_rejected() {
    assert_rejected_by_deserialisation(
        &with_extra_line("edge", "min_step_db = 0.0"),
        "min_step_db",
    );
}

const FLOOR_LOW: &str = "edge.floor_low_hz ≥ fake_max_cutoff_hz + step_window_bands · band_hz";

#[test]
fn rf04_edge_floor_low_hz_below_the_above_window_is_rejected() {
    assert_rejected_by_validation(
        &edited("edge", "floor_low_hz", Some("floor_low_hz = 20599.0")),
        FLOOR_LOW,
    );
}
#[test]
fn rf04_edge_a_wider_step_window_is_rejected() {
    assert_rejected_by_validation(
        &edited("step", "window_bands", Some("window_bands = 7")),
        FLOOR_LOW,
    );
}
#[test]
fn rf04_edge_floor_high_hz_equal_to_floor_low_hz_is_rejected() {
    assert_rejected_by_validation(
        &edited("edge", "floor_high_hz", Some("floor_high_hz = 20900.0")),
        "edge.floor_low_hz < edge.floor_high_hz",
    );
}
#[test]
fn rf04_edge_floor_min_db_zero_is_rejected() {
    assert_rejected_by_validation(
        &edited("edge", "floor_min_db", Some("floor_min_db = 0.0")),
        "edge.floor_min_db < 0",
    );
}
#[test]
fn rf04_edge_nan_max_median_step_db_is_rejected() {
    assert_rejected_by_validation(
        &edited(
            "edge",
            "max_median_step_db",
            Some("max_median_step_db = nan"),
        ),
        "edge.max_median_step_db is finite",
    );
}
#[test]
fn rf04_edge_min_residual_db_zero_is_rejected() {
    assert_rejected_by_validation(
        &edited("edge", "min_residual_db", Some("min_residual_db = 0.0")),
        "edge.min_residual_db > 0",
    );
}
#[test]
fn rf04_edge_max_step_db_equal_to_step_fake_db_is_rejected() {
    assert_rejected_by_validation(
        &edited("edge", "max_step_db", Some("max_step_db = 10.0")),
        "edge.max_step_db > step_fake_db",
    );
}
#[test]
fn rf04_edge_nan_min_residual_db_is_rejected() {
    assert_rejected_by_validation(
        &edited("edge", "min_residual_db", Some("min_residual_db = nan")),
        "edge.min_residual_db > 0",
    );
}
