# Single entry point for agents, hooks and CI (ADR-019).
# Windows: PowerShell is the shell for recipes; node scripts are portable (ADR-020).
set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

default:
    @just --list

# Fast checks: formatting, lint, types, architecture rules, quick tests. Target < 60 s (RP-08).
check-fast:
    cargo fmt --all -- --check
    cargo clippy --workspace --exclude fakeflac --all-targets -- -D warnings
    npm run check
    node scripts/lint-arch.mjs
    node --test ".claude/hooks/*.test.mjs" "scripts/*.test.mjs"

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

# Delete local and remote branches whose work is already on main (merged PR at the same commit,
# or contained in main) and their clean agent worktrees. Never main, the current branch or
# unmerged commits. `just prune-branches --dry-run` only lists them.
prune-branches *args:
    node scripts/prune-branches.mjs {{args}}

# Synthetic audio fixtures (ADR-022). Implemented in the first DSP task.
fixtures:
    @echo "fixtures: not implemented yet"

# Everything CI runs (check-app runs in the Windows build job; the installer build is not included).
ci: check-fast test trace guard-tests version-check check-app

# Tauri release build (NSIS + MSI).
build:
    npm run tauri build

# Release (RNF-11, RNF-06, ADR-0045, specs/contracts/release.md). All release outputs are staged
# under target/release-staging/ (cargo owns target/release/). release-check with no argument takes
# the tag from GITHUB_REF_NAME (release workflow). release-assets gates each installer at 20 MB
# (decimal, RNF-06). set-version never commits, tags or pushes; only the human tags and publishes.

# The manifests agree on a releasable version (MAJOR.MINOR.PATCH within the MSI range).
version-check:
    node scripts/release.mjs version

# The tag names the manifests' version and HEAD is on origin/main: release-check [vX.Y.Z]
release-check *args:
    node scripts/release.mjs release-check {{args}}

# Collect the installers, gate their size (RNF-06), write SHA256SUMS.txt and the release notes.
release-assets:
    node scripts/release.mjs assets --bundle target/release/bundle --out target/release-staging/assets --notes target/release-staging/release-notes.md --max-bytes 20000000

# Create the draft release (GitHub Actions only; the human publishes it).
release-draft:
    node scripts/release.mjs draft --assets target/release-staging/assets --notes target/release-staging/release-notes.md

# Bump the version in every manifest: set-version X.Y.Z
set-version version:
    node scripts/release.mjs set-version {{version}}

# Calibration and acceptance tool (RP-11, ADR-0036, specs/calibration/). Needs FAKEFLAC_CORPUS and
# FAKEFLAC_CALIBRATION (and ffmpeg for new transcodes). The release build runs from the repository root.
# None of these recipes is part of `just ci` or `just check` (ADR-022): the CI-runnable acceptance is
# the `cargo test` of the tool on the synthetic sample corpus. The arguments reach the binary
# untouched (positional arguments through node), so paths with spaces, `$`, `&` or `'` are safe.

# Prepare the corpus and evaluate DetectionParams files: accept [--params FILE]... [--config FILE] [--no-generate]
[positional-arguments]
accept *args:
    #!/usr/bin/env node
    const r = require('node:child_process').spawnSync('cargo', ['run', '--release', '-q', '-p', 'fakeflac-calibrate', '--', 'accept', ...process.argv.slice(2)], { stdio: 'inherit' })
    process.exit(r.status ?? 1)

# Accept plus a grid search on the tune split; writes a proposal that is never committed: calibrate [--grid FILE] [--config FILE]
[positional-arguments]
calibrate *args:
    #!/usr/bin/env node
    const r = require('node:child_process').spawnSync('cargo', ['run', '--release', '-q', '-p', 'fakeflac-calibrate', '--', 'calibrate', ...process.argv.slice(2)], { stdio: 'inherit' })
    process.exit(r.status ?? 1)

# Search the branch diff for the corpus paths and names (ADR-022): privacy-check [--base REF]
[positional-arguments]
privacy-check *args:
    #!/usr/bin/env node
    const r = require('node:child_process').spawnSync('cargo', ['run', '--release', '-q', '-p', 'fakeflac-calibrate', '--', 'privacy-check', ...process.argv.slice(2)], { stdio: 'inherit' })
    process.exit(r.status ?? 1)
