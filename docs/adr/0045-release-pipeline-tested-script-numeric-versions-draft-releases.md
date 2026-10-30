# ADR-0045: Releases: one tested Node script behind `just` recipes, numeric versions only, tags on `main`, draft releases with NSIS, MSI, checksums and provenance, and GitHub-generated notes as the changelog

- Status: proposed
- Date: 2026-10-30

## Context and rationale

RNF-11 asks that every version be published in GitHub Releases with a Windows installer and checksums built by CI, unsigned (ADR-0032). RNF-06 asks for an installer under 20 MB. ADR-0028 chose GitHub Actions and releases generated from tags. ADR-0029 chose SemVer, Conventional Commits and an automatic changelog. In H0, `release.yml` and the size step of `ci.yml` were written but never run: there is no tag and no release. Writing the spec (`specs/contracts/release.md`) found these gaps:

1. **Untested inline logic.** The version check, the size gate and the checksums are inline PowerShell in the workflows, with no tests. This breaks ADR-0019 ("CI only runs `just` recipes"). The release job does not run the size gate at all, so a release could ship over budget. The checksum step writes CRLF lines, which `sha256sum -c` rejects, and it hashes every `.exe` and `.msi` under the bundle folder, including installers of older versions left in a cached `target` folder.
2. **Pre-release tags are undefined.** The trigger `v*.*.*` matches `v1.0.0-beta.1`. The MSI puts numeric constraints on the version (MAJOR and MINOR at most 255, PATCH at most 65535). A purely numeric pre-release part might technically pass the MSI build, but nothing decides what a pre-release release looks like (MSI or not, GitHub pre-release or not), and a tag Tauri cannot map would fail late, inside `tauri build`, after the full verification run.
3. **Any commit can be released.** A tag pushed on a branch would build and draft a release of code that never went through G2.
4. **Expression injection.** The tag name is interpolated into `run:` lines with `${{ github.ref_name }}`.
5. **Write permissions on the build job.** The job that builds also holds `contents: write` and `id-token: write`.
6. **No rehearsal.** The first real tag would be the first run of the pipeline.
7. **What "automatic changelog" means** was never stated.

## Decision

1. **The release logic is one Node script, `scripts/release.mjs`** (Node built-ins only), tested with `node --test` in `scripts/release.test.mjs` on temporary folders. It is called only through `just` recipes: `version-check`, `release-check`, `release-assets`, `release-draft` and `set-version`. The workflows contain no inline version, size or checksum logic.
2. **A releasable version is `MAJOR.MINOR.PATCH`**, without a pre-release or build part, within the MSI limits (MAJOR and MINOR at most 255, PATCH at most 65535). **v1 ships no pre-releases: this is a policy**, not a claim that the MSI cannot carry one. The version is held by `package.json`, `package-lock.json`, `src-tauri/tauri.conf.json`, the root `Cargo.toml` and `Cargo.lock`, and is located in them by exact rules (a key-path scanner for JSON, line rules for the `[workspace.package]` table and the source-less `Cargo.lock` entries; a duplicate key is an error). `just version-check` checks that they agree, on every PR. `just set-version` rewrites only the located version strings, and changes no other byte. A pre-release tag is refused before packaging (in `verify`, before the `package` job builds the installers).
3. **Only commits of `main` are released, and only by the human.** `release-check` refuses a tag that does not name the manifests' version, or whose commit is not contained in `origin/main`. Agents never push tags or create releases (`guard-bash` blocks a `git push` of tags and `gh release create|edit|upload`; a guard rail, not a security boundary). The tag reaches the scripts through `GITHUB_REF_NAME`, never through a `${{ }}` expression in a command line: every `run:` line of `release.yml` is a single `just` recipe or `npm ci`.
4. **The changelog is GitHub's generated release notes.** It is the list of PRs merged since the previous tag (for the first release, which has no previous tag, every merged PR), whose squash titles are Conventional Commits. It comes after a fixed text written by the script (`gh release create --notes-file … --generate-notes`; that `gh` puts the file's text first is documented for `--notes` and confirmed for `--notes-file` at the first real draft): the files, the SmartScreen steps, how to check a download, the requirements. There is no `CHANGELOG.md`. This states what ADR-0029's "automatic changelog" means.
5. **Assets:** the NSIS installer (recommended), the MSI, and `SHA256SUMS.txt` in `sha256sum` text format (lowercase hex, two spaces, file name, LF). Only the installers of the manifests' version are collected. Both installers get a build-provenance attestation. **Each installer must be strictly smaller than 20 MB (20 000 000 bytes, decimal).** This is checked on every PR (`build-windows`, behind the one required check of ADR-0028) and in every release, before publishing.
6. **`release.yml` has three jobs.** The workflow level is `contents: read`. `verify` (tag checks and `just ci`; it installs `rustfmt` and `clippy`), `package` (build, size gate, checksums, notes; read-only; uploads the one folder `target/release-staging/` as an artifact) and `publish` (`needs: package`; downloads that folder to the same path; attestation and draft release; the only job holding `contents: write`, `id-token: write` and `attestations: write`, and it builds nothing). `publish` runs only when `github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v')`, so a `workflow_dispatch` run is a dry run of `verify` and `package`, on any branch. The release is a draft that the human publishes.

## Alternatives considered

- **Keep the inline PowerShell and add the size step to the release job.** Rejected: still untested, still against ADR-0019, and the checksum format and stale-installer bugs would remain.
- **A Rust tool (like `tools/fakeflac-calibrate`) or a third-party action** (`tauri-action`, `release-please`). Rejected. A Rust tool is heavier to build in CI. The others are new dependencies, and they would decide the version, notes and assets for us. Node is already required, and `scripts/` already holds tested Node scripts (ADR-0020).
- **Allow pre-release tags** (skip the MSI, or rely on a purely numeric pre-release part fitting the MSI's numeric constraints, and mark the release as a pre-release). Rejected for v1 as a policy: there is no need for public betas yet, and it adds a second shape of release. Revisit with a new ADR if betas are wanted.
- **Keep the version in one place only** (drop `version` from `tauri.conf.json` so that Tauri reads `Cargo.toml`). Possible, but `package.json` and the lock files would still hold a copy. The check and the bump recipe cover all of them, so this is not needed.
- **A `CHANGELOG.md` written by a tool from the commits** (for example `release-please` or `git-cliff`). Rejected. It would be a new dependency and a file that every release PR has to touch, and it would add nothing to what the generated notes already list.
- **Publish directly instead of a draft.** Rejected: the human checks the installers before users see them (planning 3.11.11 e)).
- **Ship only the NSIS installer.** Possible and smaller. The planning document chose both, and the MSI is useful for managed installs. Both are size-gated.

## Consequences

- RNF-06 is enforced on every PR, and no release can ship an installer of 20 MB (20 000 000 bytes) or more.
- Releasing becomes a fixed procedure (`release.md` section 6): a `chore(release): X.Y.Z` PR made with `just set-version`; then the human tags the merge commit; then CI drafts the release; then the human checks and publishes it.
- A change to `justfile`, `ci.yml`, `release.yml` or the bundle settings that touches the release wiring fails the static checks of `scripts/release.test.mjs`, so it needs a spec change first.
- Every release changes the app version, and so the result cache's engine (ADR-0044): users' folders are analysed again once after an update.
- The dry run is only available once `release.yml` with `workflow_dispatch` is on `main`.
- If a Tauri upgrade changes the installer names, `just release-assets` fails with "missing installer" on the PR that upgrades it.
- Details ADR-0028 (how releases are generated from tags) and clarifies ADR-0029 (what the automatic changelog is). Applies ADR-0019 to the release workflows.
