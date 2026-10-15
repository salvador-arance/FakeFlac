//! Tauri adapter and composition root (ADR-001). Wiring and commands only:
//! no domain logic lives here (see `src-tauri/CLAUDE.md`).

pub fn run() {
    // Failing to start the shell is unrecoverable; this is the one place where `expect` is the Tauri idiom.
    #[allow(clippy::expect_used)]
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running the FakeFlac application");
}
