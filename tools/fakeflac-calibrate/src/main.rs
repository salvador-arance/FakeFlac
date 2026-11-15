//! `fakeflac-calibrate accept|calibrate|privacy-check|census` (calibration.md section 2, census.md).

fn main() {
    std::process::exit(fakeflac_calibrate::run::cli_main());
}
