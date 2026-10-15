# Single entry point for agents, hooks and CI (ADR-019).
# Windows: PowerShell is the shell for recipes; node scripts are portable (ADR-020).
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

default:
    @just --list

# Fast checks: formatting, lint, types, architecture rules, quick tests. Target < 60 s (RP-08).
check-fast:
    cargo fmt --all -- --check
    cargo clippy --workspace --exclude fakeflac -- -D warnings
    npm run check
    node scripts/lint-arch.mjs
    node --test ".claude/hooks/*.test.mjs"

# Full check: fast checks plus all tests.
check: check-fast test

test:
    cargo test --workspace --exclude fakeflac
    npm run test --if-present

# Regenerate TS bindings from Rust (tauri-specta, S3): src/lib/api/bindings.ts.
gen:
    cargo run -q -p fakeflac --example gen_bindings

# Fail if the committed bindings differ from what the Rust contract generates.
gen-check: gen
    git diff --exit-code -- src/lib/api/bindings.ts

# Tauri adapter checks (src-tauri). Windows only: on Linux, Tauri needs webkit system packages,
# so CI runs this in the Windows build job.
check-app: gen-check
    cargo clippy -p fakeflac --all-targets -- -D warnings
    cargo test -p fakeflac

# Traceability matrix: requirements <-> scenarios (S1, S2).
trace:
    node scripts/trace.mjs

# Crate dependency rules and atomic design import rules (S8).
lint-arch:
    node scripts/lint-arch.mjs

# Protected tests must not change after the `test(...)` commit (S7).
guard-tests:
    node scripts/guard-tests.mjs

# Synthetic audio fixtures (ADR-022). Implemented in the first DSP task.
fixtures:
    @echo "fixtures: not implemented yet"

# Everything CI runs (check-app runs in the Windows build job; the installer build is not included).
ci: check-fast test trace guard-tests check-app

# Tauri release build (NSIS + MSI).
build:
    npm run tauri build

# Acceptance against the local corpus (FAKEFLAC_CORPUS). Never runs in CI (ADR-022).
accept:
    @echo "accept: not implemented yet"

