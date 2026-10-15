fn main() {
    tauri_build::build();
    embed_manifest_in_tests_and_examples();
}

/// `tauri-build` embeds the Windows app manifest (comctl32 v6) only in the app binary.
/// Integration tests and examples link Tauri too and fail to start without it
/// (`STATUS_ENTRYPOINT_NOT_FOUND`), so they get the same manifest here.
fn embed_manifest_in_tests_and_examples() {
    let windows_msvc = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if !windows_msvc {
        return;
    }
    let manifest =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("windows-app-manifest.xml");
    println!("cargo:rerun-if-changed={}", manifest.display());
    // Cargo rejects `rustc-link-arg-<target>` for a target kind the package does not have.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    // Rerun when `tests/` or `examples/` appear, so the link args follow them.
    for t in ["tests", "examples"] {
        println!("cargo:rerun-if-changed={}", dir.join(t).display());
    }
    for target in ["tests", "examples"]
        .into_iter()
        .filter(|t| dir.join(t).is_dir())
    {
        println!("cargo:rustc-link-arg-{target}=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-{target}=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
}
