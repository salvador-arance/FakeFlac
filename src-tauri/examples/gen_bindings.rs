//! `just gen`: writes the TS bindings of the contract to `src/lib/api/bindings.ts` (S3).

fn main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/lib/api/bindings.ts");
    if let Err(e) = fakeflac_lib::export_bindings(&path) {
        eprintln!("gen: {e}");
        std::process::exit(1);
    }
    println!("gen: wrote {}", path.display());
}
