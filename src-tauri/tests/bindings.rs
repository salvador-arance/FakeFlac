//! S3: the committed TS bindings are the generated ones (RF-01, RF-05).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::Path;

#[test]
fn committed_bindings_match_generated() {
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("bindings.generated.ts");
    fakeflac_lib::export_bindings(&out).expect("export bindings");
    let generated = std::fs::read_to_string(&out).expect("read generated");
    let committed_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib/api/bindings.ts");
    let committed = std::fs::read_to_string(committed_path).expect("read committed");
    assert_eq!(
        committed.replace("\r\n", "\n"),
        generated.replace("\r\n", "\n"),
        "src/lib/api/bindings.ts is out of date: run `just gen`"
    );
}
