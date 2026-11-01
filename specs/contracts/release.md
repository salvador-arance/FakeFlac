---
id: SPEC-RELEASE
requirements: [RNF-11, RNF-06, RNF-07, RNF-09]
status: approved
---

# Release: versions, installers, size gate, checksums and the draft release (RNF-11, RNF-06)

> RNF-11 asks that every version be published in GitHub Releases with a Windows installer and checksums, built by CI and unsigned (ADR-0032). RNF-06 asks that the installer be under 20 MB. Both were bootstrapped in H0 (`.github/workflows/release.yml`, the size step of `ci.yml`), but never run: there is no tag and no release, and the version is 0.0.1 everywhere. This spec moves the release logic out of inline PowerShell into one tested Node script, `scripts/release.mjs`, behind `just` recipes (ADR-0019), and fixes the gaps found in the H0 workflows. It is not a backend ↔ frontend contract: the app's commands, events and types do not change (`commands.md` stays at version 7). Scenarios: `specs/features/release.feature` (tags `@RNF-11`, `@RNF-06`, and `@RNF-09` for paths with spaces and Unicode characters). Decision record: ADR-0045 (proposed). Planning sources: `docs/planning/fakeflac-phases-1-3.es.md` sections 2.4, 3.11.11 c) and e), and 3.14.3.

## 0. Open questions for G1

Each question has a default, and the rest of this spec is written with it. On 2026-10-30 the owner delegated the G1 answers to the agent, which approved the defaults of Q1–Q6 and Q8–Q14 and changed Q7 to decimal 20 MB (20 000 000 bytes). Changing a default changes only the sections and scenarios cited in its row (scenarios are in `features/release.feature`, named by their title).

| # | Question | Default (this spec) | Where |
|---|---|---|---|
| Q1 | **Where the release logic lives.** Today the version check, the size gate and the checksums are inline PowerShell in `release.yml` and `ci.yml`, with no tests. That breaks "CI only runs `just` recipes" (ADR-0019, `workflow.md`), and the size gate is missing from the release job. Move all of it into `scripts/release.mjs` (Node built-ins only, like `scripts/trace.mjs` and `scripts/prune-branches.mjs`), tested with `node --test` in `scripts/release.test.mjs`, and called through `just` recipes from both workflows? | Yes | 3, 4, 5; every scenario |
| Q2 | **Pre-release tags.** The trigger `v*.*.*` also matches `v1.0.0-beta.1`. The MSI (WiX) puts constraints on the version: it must map to a numeric `ProductVersion` with MAJOR and MINOR at most 255 and PATCH at most 65535. Tauri may accept a purely numeric pre-release part (for example `1.0.0-1`), so a pre-release is not technically impossible, but supporting it is a second shape of release (skip or rename the MSI, mark the GitHub release as a pre-release). **Policy for v1: no pre-releases.** Reject a pre-release or build part early, with a clear message, in every check (`version`, `release-check`, `set-version`)? | Reject in v1, as a policy (ADR-0045 decision 2) | 2.2; scenarios "Only MAJOR.MINOR.PATCH within the MSI range is a release version", "The release tag must name the manifests' version" |
| Q3 | **Changelog.** ADR-0029 asks for an "automatic changelog". The draft release uses GitHub's generated notes (`--generate-notes`), which list the titles of the PRs merged since the previous tag (for the first release, which has no previous tag: every merged PR). Every PR is squash-merged with a Conventional Commits title, so that list is the changelog. Is that enough, with no `CHANGELOG.md` in the repository? | Yes: generated notes, no `CHANGELOG.md` (ADR-0045 decision 4 clarifies ADR-0029) | 3.5, 3.6, 5.2, M7 |
| Q4 | **Bumping the version.** The version is in five files: `package.json`, `package-lock.json` (twice), `src-tauri/tauri.conf.json`, the root `Cargo.toml` (`[workspace.package]`) and `Cargo.lock` (seven workspace packages). Add `just set-version X.Y.Z`, which rewrites only those version strings and never commits, tags or pushes? The alternative is a documented manual edit, checked by `just version-check`. | Yes, a recipe | 2.1, 3.7, 6; scenarios "set-version rewrites only the version strings", "set-version refuses a version that cannot be released" |
| Q5 | **A rehearsal before the first tag.** Add `workflow_dispatch` to `release.yml` as a dry run: it runs `verify` and `package` (build, size gate, checksums, notes, artifact upload) but never `publish` (no attestation, no release)? The owner runs it once on `main` before the first tag (M1). It can also be started on another branch (D8). | Yes | 5.2, M1 |
| Q6 | **Assets and provenance.** Ship both the NSIS installer (recommended) and the MSI, as the planning document decided (3.14.3), plus `SHA256SUMS.txt`, and attest the provenance of both installers (`actions/attest-build-provenance`, already in `release.yml`)? Each installer counts toward RNF-06 on its own. | Yes: NSIS + MSI + `SHA256SUMS.txt`; provenance for both installers | 3.3, 5.2; scenario "The assets of a release" |
| Q7 | **RNF-06's unit and comparison.** "Under 20 MB" is read as: each installer file is **strictly smaller than 20 MB = 20 000 000 bytes** (decimal SI units). The two files are checked separately, not added up. (The H0 gate failed only above 20 MiB = 20 971 520 bytes, so it was looser.) The decimal reading is the stricter one, so it meets "20 MB" whichever unit a reader assumes; 20 MiB would let an installer between 20 000 000 and 20 971 519 bytes through. The installers are a few MB, so the margin costs nothing. | 20 MB = 20 000 000 bytes, strictly under **Decided on the owner's delegation (2026-10-30); the proposed default was 20 MiB.** | 3.3, 7 (R4); scenarios under "Installer size" |
| Q8 | **Only commits of `main` are released.** `release-check` refuses a tag whose commit is not contained in `origin/main`. Today a tag pushed on any branch would build and draft a release of unreviewed code (planning 3.11.11 e) step 2: "the human creates the tag on `main`"). | Yes | 3.2, 5.2; scenario "Only a commit of main can be released" |
| Q9 | **The version check on every PR.** Add `just version-check` (manifests agree, the version is releasable) to `just ci` and to the `specs` job of `ci.yml`, so that a PR that bumps one manifest only fails at once instead of at tag time. | Yes | 4, 5.1 |
| Q10 | **The release notes text** (section 3.5): what the files are, the SmartScreen steps ("More info", then "Run anyway"), how to check a download with `Get-FileHash`, the checksums, the provenance command, and the system requirements. English only. | As written in 3.5 | 3.5; scenario "The release notes" |
| Q11 | **Agents never release.** Creating and pushing a `v…` tag is the human's act, equivalent to G2. Extend `guard-bash` (`.claude/hooks/`, owner-reviewed) to block a `git push` of tags and `gh release create|edit|upload`, with the exact patterns and cases of section 8.1? `just release-draft` also refuses to run outside GitHub Actions (3.6). | Yes | 3.6, 6; 8.1 |
| Q12 | **Static checks of the wiring.** `scripts/release.test.mjs` also reads `justfile`, `.github/workflows/ci.yml`, `.github/workflows/release.yml` and `src-tauri/tauri.conf.json` as text and checks the points of section 5.3 (the recipes are called, no inline size or checksum logic is left, the 20 MB figure, the job permissions, the exact `if:` of `publish`, the bundle targets and the WebView2 mode). A later change to these files then needs a spec change. Agree? | Yes | 5.3; scenarios under "Wiring" |
| Q13 | **The start-up smoke test of the planning document**, section 3.11.11 c), job `build-windows`: start the `.exe` and check that it stays alive for a few seconds. It is not RNF-11 or RNF-06, and it runs the app on the CI machine. Leave it out of this spec? Manual check M3 starts the installed app for every release. | Out of scope (a later spec) | 1 |
| Q14 | **ADR.** Record the decisions of Q1, Q2, Q3, Q5, Q6, Q8 and Q11 in ADR-0045 (proposed), which details ADR-0028 and clarifies ADR-0029? | Yes | ADR-0045 |

**Decided defaults (not questions).** They follow from approved requirements, ADRs or the planning document, or are technical choices with no owner trade-off, and are listed so that G1 sees them:
- **D1. x64 only** (RNF-07: Windows 10/11, 64-bit). The installer names carry `x64`.
- **D2. MSI language `en-US`**, Tauri's default. The MSI name carries it.
- **D3. The WebView2 mode stays `downloadBootstrapper`** (`tauri.conf.json`). An offline WebView2 installer would add well over 100 MB and break RNF-06; `embedBootstrapper` would add about 1.8 MB for no gain on Windows 10/11. Installing on a machine without WebView2 then needs an internet connection once. This is the installer, not the app: the app itself still makes no network connection (RNF-03).
- **D4. No in-app updates and no portable build in v1** (planning 3.14.3).
- **D5. 0.x versions until H2 is closed** (planning 3.11.11 e) step 1). The first version number is the owner's choice when the first release is prepared (section 6).
- **D6. The release is a draft that the human publishes** (planning 3.11.11 e) step 8; already so in H0).
- **D7. A duplicate key at a version path is an error**, not "the last one wins" (which is what `JSON.parse` does). A manifest whose version is ambiguous is refused (2.1). The same holds for a `Cargo.lock` that Cargo never writes: a workspace package without a `name` line, or two workspace packages with the same name.
- **D8. The dry run may be started on any branch.** GitHub lets `workflow_dispatch` run on any branch once the workflow is on `main`. The dry run runs `version-check`, not `release-check`, so it does **not** require the commit to be on `main`: it publishes nothing, and rehearsing a release-wiring change on its PR branch is useful. `publish` can never run in a dry run because its `if:` requires a tag push (5.2, 5.3).
- **D9. Paths in messages** are printed as given on the command line, with `\` turned into `/` (3.0).

## 1. Scope

- **In:**
  - the version: which files hold it, how it is located in them, what a releasable version is, how it is checked and bumped;
  - the release tag and the commit it names;
  - collecting the installers, the size gate (RNF-06), `SHA256SUMS.txt` and the release notes;
  - creating the draft release;
  - the `just` recipes and how `ci.yml` and `release.yml` call them;
  - the `guard-bash` rules that keep agents from releasing (Q11);
  - the manual checks that CI cannot do (install, SmartScreen, checksum by hand, uninstall, publish).
- **Out:**
  - code signing (ADR-0032);
  - pre-releases (Q2);
  - the start-up smoke test (Q13);
  - a PR-title check (ADR-0029's consequence "a check validates the title" has never been implemented; a later task);
  - the other `ci.yml` jobs that call `cargo` or `npm` directly (`test-rust`, `test-web`, `supply-chain`): not release logic, unchanged;
  - shrinking the installer: there is no known size problem (section 7, M2);
  - in-app updates, a portable build, other architectures (D1, D4).

## 2. The release version

### 2.1 Where it is, and how it is located

The version of the app is one string, held in these places (the **manifests**). Each is identified in messages by the **source name** in the first column. Sources are always listed in this order (the `Cargo.lock` entries by `name`, in UTF-16 code-unit order):

| Source name | File | Location of the version |
|---|---|---|
| `package.json` | `package.json` | JSON key path `["version"]` |
| `package-lock.json` | `package-lock.json` | JSON key path `["version"]` |
| `package-lock.json packages[""]` | `package-lock.json` | JSON key path `["packages", "", "version"]` |
| `src-tauri/tauri.conf.json` | `src-tauri/tauri.conf.json` | JSON key path `["version"]` |
| `Cargo.toml [workspace.package]` | root `Cargo.toml` | the version line of the `[workspace.package]` table (below) |
| `Cargo.lock <name>` | `Cargo.lock` | the `version` line of each `[[package]]` entry that has no `source` line (below). Today these are the seven workspace members `fakeflac`, `fakeflac-audio`, `fakeflac-calibrate`, `fakeflac-core`, `fakeflac-export`, `fakeflac-fs`, `fakeflac-store`. `<name>` is the entry's `name` |

`assets` and `draft` also read the **product name**: JSON key path `["productName"]` of `src-tauri/tauri.conf.json`. It becomes part of file names (3.3), so it must be a non-empty string that holds none of `/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, `|` and no control character (Unicode category Cc: U+0000 to U+001F and U+007F to U+009F); otherwise it is a read error (below). The release notes read the **repository**: the first line `repository = "…"` of the `[workspace.package]` table (same line rule as the version).

**Common rules.**
- A file is read as UTF-8. A UTF-8 BOM at the start of a JSON file is stripped before the file is scanned (and written back by `set-version`). The repository's real manifests have no BOM; the BOM case exists only in the test fixtures, so that a file saved by an editor that adds one is still read.
- Lines end with LF or CRLF; both are read. For the line rules below, a line is the text between two LFs with one trailing CR removed. `set-version` keeps every line ending as it was.
- A file is read once, even when it holds two sources. A file-level error (missing, unreadable, not valid JSON) is **one** line, not one per source.

**JSON files: a key-path scanner.** No JSON library beyond `JSON.parse` is used (Q1).
1. The text (without its BOM) must parse with `JSON.parse`; otherwise the file-level error `is not valid JSON`.
2. Then a small token scanner walks the text from the start, tracking the path of object keys (decoded, so `"version"` is `version`) and array positions from the root. It records every object member whose key path is a **target path** of that file (the table above, plus `["productName"]` for `tauri.conf.json`), with the span of its value.
3. A target path recorded more than once (two `version` keys in one object, or two `packages` objects that both hold `""`) is the error `has a duplicate "<last key>" key` (D7).
4. The value at the target path must be a JSON string; otherwise `version is not a string`. The version is the **decoded** string (so a value whose first digit is written as the six-character JSON escape for U+0030, a backslash, `u`, then `0030`, followed by `.1.0`, is the version `0.1.0`), and versions are compared decoded. An empty string is `version is empty`.
5. Every other member is never read as a version, whatever its key, value or position (before or after the real one): `version` keys at other paths (the `version` of each dependency in `package-lock.json`, a nested `version` such as `config.version` in `package.json` or `bundle.version` in `tauri.conf.json`), and string values whose text looks like a version line (a `description` holding `"version": "0.1.0"`, which the file holds escaped as `\"version\": \"0.1.0\"`). The scanner does not assume any key order.
6. The **span** used by `set-version` is the raw characters between the two quotes of that value as they appear in the file, escapes included (the escaped `0.1.0` of point 4 is a span of ten characters, replaced whole by the five of the new version). Span offsets count UTF-16 code units in the text **without its BOM**, the text that was scanned; `set-version` puts the BOM back in front when it writes the file.

**`Cargo.toml`: a line rule.** No TOML parser (Q1).
- The table starts at the first line whose text, without leading and trailing spaces or tabs and without a trailing `# comment`, is exactly `[workspace.package]`. It ends before the next line whose first non-blank character is `[`, or at the end of the file.
- In that table, the version line is the **first** line matching `^[ \t]*version[ \t]*=[ \t]*"([^"\\]*)"[ \t]*(#.*)?$`. The group is the version, and also the span used by `set-version`.
- A line in the table matching `^[ \t]*version[ \t]*=` whose value is not a `"…"` basic string (for example `version = 1` or `version = '0.1.0'`) is `version is not a string`. `version = ""` is `version is empty`.
- Never read: comment lines (first non-blank character `#`, so `# version = "0.1.0"` before the real line is ignored), other keys of the table, dotted keys such as `version.workspace = true`, and every line of other tables (`[workspace]`, `[workspace.dependencies]` with `foo = { version = "0.1.0" }`, `[profile.*]` …). A duplicate `version` key in the table is not looked for: Cargo itself refuses such a file.

**`Cargo.lock`: a line rule.** Cargo writes this file in a fixed format.
- An **entry** runs from a line that is exactly `[[package]]` to the line before the next line starting with `[` (another `[[package]]`, or `[metadata]`), or to the end of the file.
- An entry is a **workspace package** when none of its lines starts with `source = `. Entries with a `source` line (registry or git dependencies) are ignored, whatever their version.
- In a workspace package, the name is its first line matching `^name = "(.*)"$` and the version is its first line matching `^version = "(.*)"$` (the group is the span used by `set-version`), in whichever order the two lines come. The `dependencies = [ … ]` lists are never read or changed. Lines before the first `[[package]]` (Cargo's header comment and the file-format line `version = 4`) belong to no entry and are never read.
- **A workspace package without a `name` line** is the error `release: Cargo.lock has a workspace package without a name`, once for the file however many such entries there are. It is not ignored: Cargo always writes a `name`, so such an entry means a damaged file, and ignoring it could hide a stale version (D7).
- **Two workspace packages with the same name** are the error `release: Cargo.lock lists the workspace package <name> more than once`, once per such name. Cargo refuses two workspace members with the same name, so this too is a damaged file, and which version is "the" version of that source would be ambiguous (D7).
- Both are **file-level** errors for `Cargo.lock`: no `Cargo.lock <name>` source is reported or compared when one of them is found.

**Read errors (exact messages, exit 1).** `<file>` is the path relative to `--root` with `/` (`package.json`, `src-tauri/tauri.conf.json`, `Cargo.toml`, `Cargo.lock` …); `<source>` is the source name of the table above. Each line starts with `release: `:

| Situation | Message |
|---|---|
| the file does not exist, or is not a regular file | `release: missing <file>` |
| the file exists but cannot be read | `release: cannot read <file> (<Node error code, e.g. EACCES>)` |
| a JSON file that does not parse | `release: <file> is not valid JSON` |
| a target path that is absent (including an absent `packages` or `packages[""]`), a `Cargo.toml` without a `[workspace.package]` table or without a version line in it, a source-less `Cargo.lock` entry without a version line | `release: <source> has no version` |
| a JSON value that is not a string; a `Cargo.toml` version that is not a basic string | `release: <source> version is not a string` |
| an empty version | `release: <source> version is empty` |
| a target path recorded twice (D7) | `release: <source> has a duplicate "version" key` |
| a `Cargo.lock` with no source-less entry | `release: Cargo.lock lists no workspace package` |
| a source-less `Cargo.lock` entry without a `name` line | `release: Cargo.lock has a workspace package without a name` |
| two source-less `Cargo.lock` entries with the same name | `release: Cargo.lock lists the workspace package <name> more than once` |
| `assets`, `draft`: `productName` absent / not a string / empty / duplicate | `release: src-tauri/tauri.conf.json has no productName` / `release: src-tauri/tauri.conf.json productName is not a string` / `release: src-tauri/tauri.conf.json productName is empty` / `release: src-tauri/tauri.conf.json has a duplicate "productName" key` |
| `assets`, `draft`: `productName` holds a character that a file name cannot hold (above) | `release: src-tauri/tauri.conf.json productName <q> has a character not allowed in a file name`, where `<q>` is `JSON.stringify(<productName>)` (so a tab is shown as `\t` and a `"` as `\"`) |

Every read error found is reported, in the order of the sources above (file-level errors at the position of the file's first source; for `Cargo.lock`, the "without a name" line first, then the "more than once" lines in code-unit order of `<name>`; the product name last). It is never a crash with a stack trace (3.0).

The crates' own `Cargo.toml` files use `version.workspace = true` and are not manifests. The app's `CARGO_PKG_VERSION` is therefore the version, and so is the `app_version` of the result cache's engine (`result-cache.md` section 4): every release recomputes each folder once, as ADR-0044 intends.

### 2.2 A releasable version (Q2)

A version is **releasable** when all of these hold:
- it matches `^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$`: three numbers, no leading zeros, no pre-release (`-…`) and no build (`+…`) part;
- MAJOR ≤ 255, MINOR ≤ 255 and PATCH ≤ 65535 (compared as integers of any length), the limits of an MSI `ProductVersion`.

A version that is not releasable is classified, in this order, with the messages of 3.1 step 3:
1. **pre-release or build:** it matches `^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)[-+]`;
2. **shape:** it does not match the pattern above;
3. **range:** it matches, but a number is over its limit.

No pre-release is a **policy of v1** (Q2), not a claim that the MSI cannot carry one.

Examples: `0.0.1`, `0.1.0`, `1.0.0` and `255.255.65535` are releasable. `1.0.0-beta.1` and `1.0.0+5` (pre-release or build), `01.0.0`, `1.0`, `1.0.0.0` (shape), `256.0.0`, `0.256.0` and `0.0.65536` (range) are not.

The **release tag** of version `X.Y.Z` is exactly `vX.Y.Z` (lowercase `v`).

## 3. `scripts/release.mjs`

One ES module, run as `node scripts/release.mjs <command> [arguments]`. Common rules:
- **Node built-ins only** (`node:fs`, `node:path`, `node:crypto`, `node:child_process`, `node:os`, `node:url`). No new dependency (Q1).
- **`--root DIR`** (every command): the repository folder whose manifests are read or written. Default: the parent folder of `scripts/`. Relative paths given to other options are resolved against the current working directory, not against `--root`.
- **It writes only where a command says it does** (the `--out` folder, the `--notes` file, the manifests of `--root` for `set-version`). It never changes git state (no commit, tag, push or checkout) and never touches anything else. Tests run every command on temporary folders (`os.tmpdir()`), never on the repository (ADR-0022).
- **The environment.** The script reads only `GITHUB_ACTIONS`, `GITHUB_REF_TYPE` and `GITHUB_REF_NAME`, and only from the `env` it is given (3.8). It passes that same `env` to every child process it starts (`git`, `gh`).
- **The CLI and the exported functions of 3.8 are the contract** that the tests drive.

### 3.0 Arguments, messages and exit codes

**Arguments.**
- The first argument is the **command**: `version`, `release-check`, `assets`, `draft` or `set-version`.
- An argument that starts with `--` is an **option**. A command accepts only the options of its synopsis (3.1–3.7) and `--root`. `--print` of `draft` is a flag; every other option of a synopsis takes a value. The value is the next argument, unless there is none or it starts with `--`. The form `--name=value` is not recognised (it is an unknown option).
- **An unknown option takes no value.** The argument after it is read on its own, as a positional or as the next option. So `version --verbose x` gives two lines, `release: unknown option --verbose for version` then `release: unexpected argument "x" for version`, and `--print` given to any command other than `draft` is an unknown option that takes no value (`assets … --print x` reports `--print`, then `x` as an unexpected argument).
- **An empty value** (the empty string given as the argument after an option, for example `--out ""`) is a usage error: `release: option <option> has an empty value`. That option is then not checked further (no `--max-bytes` check, no `--notes`/`--out` check).
- Every other argument is a **positional**. Options and positionals may come in any order. `release-check` takes at most one positional (the tag), `set-version` exactly one (the version), the others none.
- `--max-bytes` must match `^[1-9][0-9]*$` and be at most `Number.MAX_SAFE_INTEGER` (9 007 199 254 740 991).
- `--notes` must not be `--out` or inside it (compared as `path.resolve` results, case-insensitively on Windows).

**Usage errors (exit 2), exact messages:**

| Situation | Message |
|---|---|
| no command | `release: usage: node scripts/release.mjs <command> [arguments]; commands: version, release-check, assets, draft, set-version` |
| unknown command | `release: unknown command "<command>"; commands: version, release-check, assets, draft, set-version` |
| an option that the command does not take | `release: unknown option <option> for <command>` |
| an option given twice | `release: option <option> is given more than once` |
| an option with no value | `release: option <option> needs a value` |
| an option whose value is the empty string | `release: option <option> has an empty value` |
| a required option missing (`assets`: `--bundle`, `--out`, `--notes`, `--max-bytes`; `draft`: `--assets`, `--notes`) | `release: <command> needs <option>` |
| a positional that the command does not take | `release: unexpected argument "<argument>" for <command>` |
| `set-version` without a version | `release: set-version needs a version such as 1.2.3` |
| `release-check` without a tag and without a tag in the environment (3.2) | `release: release-check needs a tag such as v1.2.3` |
| a bad `--max-bytes` | `release: --max-bytes must be a whole number from 1 to 9007199254740991, not "<value>"` |
| `--notes` equal to or inside `--out` | `release: --notes must not be inside --out` |

An option reported as needing a value or as having an empty value is not also reported as a missing required option.

**Precedence.** The whole command line is checked **before anything is read** (no file, folder or git call). If there is any usage error, every usage error found is printed, in the order of the arguments, then the missing required options in the order of the synopsis, and the exit code is 2, with nothing read or written. A missing or unknown command is the only line printed. So **exit 2 beats exit 1**: a command line with a usage error never reports a manifest or file problem. (A version given to `set-version` that is not releasable is exit 1, checked after the usage checks and before any file is read.)

**Messages.**
- Every line the script prints starts with `release: `. Results go to stdout; failures go to stderr, one line per problem, and **every problem found is reported**, not only the first one. Lines end with LF.
- **Paths (D9):** manifest files are named relative to `--root` with `/` (2.1). Every other path (`--bundle`, `--out`, `--notes`, `--assets`) is printed **as given on the command line**, with every `\` replaced by `/` and trailing separators removed; a path built from it appends `/` and a name (`<bundle>/nsis/<name>`). It is never resolved, shortened or quoted.
- `<name>` and `<entry>` are **basenames** (no folder). A folder entry is printed with a trailing `/` (`sub/`).
- An unexpected exception is caught and printed as the one line `release: unexpected error: <message>`, exit 1. No stack trace is ever printed.

**Exit codes:** `0` success; `1` a check failed or an input is wrong (missing file, bad version, size over the limit…); `2` a usage error. Nothing else, except that `draft` without `--print` exits with `gh`'s exit code (3.6).

### 3.1 `version`: the manifests agree on a releasable version

`node scripts/release.mjs version [--root DIR]`

1. Reads every source of 2.1. Each read error is one stderr line with the exact message of 2.1. Exit 1.
2. If the sources hold more than one distinct version: the line `release: the manifests do not agree on the version`, then one line per source, `release:   <source name>: <version>` (three spaces after `release:`), in the order of 2.1. Exit 1.
3. If the one version is not releasable (2.2), one of these lines and exit 1:
   - pre-release or build: `release: version "<v>" has a pre-release or build part; v1 ships no pre-releases (MAJOR.MINOR.PATCH only)`;
   - shape: `release: version "<v>" is not MAJOR.MINOR.PATCH (numbers without leading zeros)`;
   - range: `release: version "<v>" is outside the MSI range (MAJOR and MINOR at most 255, PATCH at most 65535)`.
4. Otherwise stdout `release: version <v> (<n> sources agree)` and exit 0.

### 3.2 `release-check`: the tag names the version, and the commit is on `main`

`node scripts/release.mjs release-check [TAG] [--root DIR]`

- **The tag:** the positional `TAG` if given; **it wins over the environment**. Otherwise, when `GITHUB_REF_TYPE` is `tag`, the value of `GITHUB_REF_NAME` (so the workflow never puts the tag name into a shell command line, 5.2). Otherwise the usage error `release: release-check needs a tag such as v1.2.3`, exit 2.
- **Checks, all of them run and reported in this order:**
  1. everything `version` checks (3.1);
  2. the tag is `v` followed by a releasable version. If the tag starts with `v` and the rest is classified "pre-release or build" by 2.2: `release: tag "<tag>" has a pre-release or build part; v1 ships no pre-releases (vMAJOR.MINOR.PATCH only)`. Any other non-releasable shape (no `v`, `V`, two numbers, leading zeros, out of range): `release: tag "<tag>" is not v<MAJOR>.<MINOR>.<PATCH>`;
  3. when the manifests agree on a releasable version and the tag is well formed: the tag equals `v<version>`, else `release: tag "<tag>" does not match the manifests' version <version>`;
  4. **on `main` (Q8)**, with git run in the `--root` folder, with an argument array (never a shell) and the given `env`:
     1. `git rev-parse --verify --quiet refs/remotes/origin/main`. Exit 1: `release: origin/main not found (fetch it, for example actions/checkout with fetch-depth: 0)`; the rest of step 4 is skipped.
     2. `git rev-parse --verify --quiet HEAD^{commit}` gives `<head>`, the commit checked; its first 12 hexadecimal digits are `<short>`.
     3. `git merge-base --is-ancestor HEAD refs/remotes/origin/main`. Exit 0: on `main`. Exit 1: `release: commit <short> is not on origin/main; tag a commit of main`.
     4. Any other non-zero exit of these git commands (for example 128, not a repository) is a git error: `release: git <subcommand> failed (exit <code>): <git's stderr, trimmed, line breaks replaced by one space>`, where `<subcommand>` is `rev-parse` or `merge-base`; the rest of step 4 is skipped. If git cannot be started: `release: git could not be started (<Node error code>)`.
  Each failed check is exit 1.
- **Success:** stdout `release: tag <tag> releases version <version> from commit <short> on origin/main`, exit 0.

### 3.3 `assets`: collect the installers, gate their size, write the checksums and the notes

`node scripts/release.mjs assets --bundle DIR --out DIR --notes FILE --max-bytes N [--root DIR]`

All four options are required (3.0). The number for `--max-bytes` lives only in the `release-assets` recipe (section 4), so RNF-06's figure is in one place.

**Inputs.** The version `<v>` comes from the manifests and the product name `<p>` from `tauri.conf.json` (2.1). If any of them has a problem, the read errors of 2.1 and 3.1 (steps 1 to 3) are reported, exit 1, and the steps below do not run (the installer names are unknown), so nothing is written. The two **expected installers**, named as Tauri v2 names them:
- NSIS: `<bundle>/nsis/<p>_<v>_x64-setup.exe`;
- MSI: `<bundle>/msi/<p>_<v>_x64_en-US.msi`.

Each name is built by **exact string concatenation** of `<p>`, the literal text and `<v>`, and compared with directory entries as exact strings (UTF-16 code units, case-sensitive). No glob or pattern library is used anywhere in the script. Where this spec writes `*` in a name (`<p>_*_x64-setup.exe`, step 4), it is a **literal prefix and suffix match**: the name starts with the text before `*`, ends with the text after it, and is at least as long as the two together; the part in between may be anything, including empty. The product name cannot hold `*`, `?` or a separator (2.1), so it can never act as a pattern.

**Steps 1 to 4 always all run**, and nothing is written until all four have passed. If any fails, every failure is printed on stderr in this fixed order: step 1 (NSIS, then MSI), step 3 (NSIS, then MSI), step 4 (entries in UTF-16 code-unit order of their names); exit 1; `--out` and `--notes` are left exactly as they were. (A failure while writing, at steps 5 to 7, is handled differently: see "Write failures" below.)
1. **Both installers exist** as regular files. A missing one, or one that is a folder or anything other than a regular file: `release: missing installer <bundle>/nsis/<name>` (or `msi`). If a Tauri upgrade changes the names, this is how CI says so.
2. **Other entries are ignored.** Every other entry in `<bundle>/nsis` and `<bundle>/msi` (for example an installer of an older version left in a local or cached `target` folder) is never copied, hashed or uploaded. Each is reported on stdout as `release: ignored <bundle>/<nsis|msi>/<entry> (not an installer of <v>)` (a folder entry with a trailing `/`). The lines of both folders are sorted together by the entry's basename, in UTF-16 code-unit order (an equal basename in both folders: `nsis` first), and printed **before** the size lines of step 3. A missing `nsis` or `msi` folder is reported by step 1 only.
3. **Size gate (RNF-06, Q7).** For each expected installer that exists, of size `s` bytes, stdout `release: <name> <s> bytes (<s / 1000000 with two decimals, as toFixed(2)> MB), limit <N> bytes`, NSIS first. It passes when `s < N` (an empty file passes this gate). Otherwise stderr `release: <name> is <s> bytes, not under the limit of <N> bytes (RNF-06)`. Both existing installers are always measured and reported, even when the other one is missing.
4. **The outputs may be replaced.** If `--out` exists, it must be a folder (otherwise `release: <out> is not a folder`) holding only earlier outputs of this command: regular files named `<p>_*_x64-setup.exe`, `<p>_*_x64_en-US.msi` (literal prefix and suffix, above) or exactly `SHA256SUMS.txt`, and no folder. Every other entry: `release: <out> holds <entry>, which assets did not write; remove it by hand`, nothing deleted. If `--notes` exists and is not a regular file: `release: <notes> is not a file`. (A path whose parent is not a folder, so that `stat` fails with `ENOTDIR`, counts as not existing here, as with `ENOENT`.)
5. Delete those earlier outputs (only those files), create `--out` if needed, and copy the two installers into it under their own names. The bundle folder is never changed.
6. Write `<out>/SHA256SUMS.txt` (3.4).
7. Write the notes file (3.5), creating its parent folder if needed and replacing an existing file.
8. stdout `release: wrote <out> (2 installers, SHA256SUMS.txt) and <notes>`, exit 0.

So after a success `--out` holds exactly three files: the two installers of `<v>` and `SHA256SUMS.txt`.

**Write failures (steps 5 to 7).** The first operation that fails stops the run:
- stderr `release: cannot remove <path> (<Node error code>)` when deleting an earlier output fails (step 5), or `release: cannot write <path> (<Node error code>)` when creating a folder, copying an installer or writing a file fails (steps 5 to 7). `<path>` is the folder or file that the failing operation was removing, creating or writing (`<out>`, `<out>/<name>`, the notes file's parent folder, or `<notes>`), printed by the rules of 3.0;
- then **cleanup**: everything this run created is removed: the installers it copied, the `SHA256SUMS.txt` it wrote, the notes file if it began writing it (a replaced notes file is removed, not restored), and every folder it created (`--out`, parent folders of the notes file), deepest first. Earlier outputs deleted at step 5 are not restored. A removal that fails during cleanup adds a `release: cannot remove <path> (<code>)` line and cleanup goes on;
- exit 1, and no success line.

So a run ends either with the full result of step 8 or, for a write failure, with nothing of its own left behind; "nothing is written until all four have passed" refers to steps 1 to 4.

### 3.4 `SHA256SUMS.txt`

- One line per installer in `--out`, in ascending order of file name compared by UTF-16 code units (JavaScript's default sort). With today's names, the NSIS installer comes first (`-` sorts before `_`).
- Each line is the SHA-256 of the file as 64 **lowercase** hexadecimal digits, **two spaces**, the **file name only** (no folder), then **LF**. The last line also ends with LF. No other line, no BOM, UTF-8.
- It does not list itself.
- This is the format of GNU `sha256sum` in text mode, so `sha256sum -c SHA256SUMS.txt` checks it on Linux or in Git Bash. (The H0 PowerShell wrote CRLF line endings, which `sha256sum -c` rejects.)
- Running `assets` twice on the same bundle gives a byte-identical file.

### 3.5 Release notes (Q10)

The notes file is UTF-8 without BOM, LF line endings, ending with one LF. It is the text below, where:
- `{productName}` is `<p>` (today `FakeFlac`); no product name is written literally in the template;
- `{version}` is `<v>`; `{nsis}` and `{msi}` are the two installer names;
- `{sums}` is the content of `SHA256SUMS.txt`, verbatim;
- `{repository}` is `owner/name` taken from `repository` in the root `Cargo.toml` `[workspace.package]` (2.1) when it matches `https://github.com/<owner>/<name>` (with an optional `.git` or trailing `/`). Otherwise the paragraph marked `[provenance]` is left out, and so is its blank line.

In the published release, GitHub's generated list of merged PRs follows this text (`gh` adds it, 3.6, Q3; unverified ordering, see 3.6).

~~~~markdown
{productName} {version} for Windows 10 and 11 (64-bit).

## Files

- `{nsis}`: the installer (recommended). It installs {productName} for the current user.
- `{msi}`: the same app as a Windows Installer package, for managed installs.
- `SHA256SUMS.txt`: the SHA-256 checksum of each installer.

## Windows SmartScreen

The installers are not code-signed. {productName} is free, open-source software and does not pay for a signing certificate. The first time you run a new download, Windows SmartScreen may show "Windows protected your PC". Check the download first (below), then click **More info** and **Run anyway**.

## Check the download

In PowerShell, in the folder that holds the download:

```powershell
(Get-FileHash .\{nsis} -Algorithm SHA256).Hash.ToLower()
```

The result must equal the hash on that file's line of `SHA256SUMS.txt`:

```text
{sums}```

[provenance] To check that GitHub Actions built the installers from this repository: `gh attestation verify {nsis} --repo {repository}` and `gh attestation verify {msi} --repo {repository}`.

## Requirements

Windows 10 or 11, 64-bit. If Microsoft Edge WebView2 is missing, the installer downloads it, which needs an internet connection once. {productName} itself makes no network connection.
~~~~

(The `[provenance]` marker is not part of the text: the paragraph is one line and starts with "To check". It names both installers, so M4 checks exactly the two commands it gives.) `{sums}` ends with its own LF, so the closing fence is on its own line.

### 3.6 `draft`: create the draft release

`node scripts/release.mjs draft --assets DIR --notes FILE [--print] [--root DIR]`

1. Reads `<v>` and `<p>` as `assets` does (2.1, 3.1 steps 1 to 3; problems are exit 1 and the steps below do not run).
2. If `GITHUB_REF_TYPE` is `tag` and `GITHUB_REF_NAME` is not `v<v>`: `release: the workflow's tag "<name>" does not match the manifests' version <v>`.
3. Checks `--assets`. If it does not exist: `release: missing <assets>`; if it is not a folder: `release: <assets> is not a folder`. Otherwise it must hold exactly the two installers of `<v>` and `SHA256SUMS.txt`, as regular files, and nothing else (no other file and **no folder**):
   - each missing one: `release: <assets> has no <name>` (NSIS, MSI, then `SHA256SUMS.txt`);
   - each other entry, sorted: `release: <assets> holds an unexpected <entry>` (a folder as `<entry>/`);
   - when the three files are present: `SHA256SUMS.txt` must be byte-identical to what 3.4 gives for the two installers now, so a damaged artifact between jobs is caught. For each installer whose expected line is not a line of the file: `release: SHA256SUMS.txt does not match <name>`. If every expected line is there but the bytes still differ: `release: SHA256SUMS.txt is not exactly the checksums of the two installers`.
4. Checks the notes file: `release: missing <notes>` if it does not exist; `release: <notes> is not the release notes of <v>` if it does not start with `<p> <v> `.
5. Steps 2 to 4 all run, and every failure is printed in the order above; exit 1, nothing run.
6. The `gh` arguments are, in this order:
   `release`, `create`, `v<v>`, `--draft`, `--verify-tag`, `--title`, `<p> <v>`, `--generate-notes`, `--notes-file`, `<notes>`, then `path.join(<assets>, <nsis>)`, `path.join(<assets>, <msi>)`, `path.join(<assets>, "SHA256SUMS.txt")`. The `--notes-file` item `<notes>` is the `--notes` option value **verbatim**: no `\` to `/` replacement, no trailing-separator removal, no `path` call (unlike paths in messages, D9). `<assets>` is the `--assets` value as given, and only `path.join` touches it (which uses the platform's separator and normalises the result). These are arguments, not messages.
   `--verify-tag` makes `gh` fail if the tag is not on GitHub. `--generate-notes` with `--notes-file` is expected to put the notes first and GitHub's generated list after them (Q3). **Unverified:** `gh`'s documentation states this for `--notes`; that `--notes-file` behaves the same is confirmed by the first real draft (M7).
7. With `--print`: stdout is that argument array as one line of JSON (`JSON.stringify`), nothing is run, exit 0. (This line is the one stdout line without the `release: ` prefix.)
8. Without `--print`, only when `GITHUB_ACTIONS` is `true` (Q11): runs `gh` with that argument array (no shell, stdio inherited, the given `env`) and exits with `gh`'s exit code (1 if it cannot be started, with `release: gh could not be started (<Node error code>)`). When `GITHUB_ACTIONS` is not `true`: `release: draft runs only in GitHub Actions (use --print to see the command)`, exit 1, nothing run. This check comes after steps 1 to 5.

A draft that already exists for the tag makes `gh` fail. The human deletes the draft on GitHub and re-runs the job.

### 3.7 `set-version`: bump the version (Q4)

`node scripts/release.mjs set-version X.Y.Z [--root DIR]`

1. A version that is not releasable (2.2) is refused with the message of 3.1 step 3, exit 1, nothing read or written. A missing version is a usage error (3.0).
2. Reads every source of 2.1. Any read error is reported with the message of 2.1, exit 1, nothing written.
3. Rewrites, in memory, **only the version spans** of 2.1, whether or not the sources agreed before: for each source whose version (decoded, 2.1) is not `X.Y.Z`, the whole raw span between the quotes of its value, escapes included, is replaced by `X.Y.Z`. A source whose decoded version already is `X.Y.Z` is left as it is, even if the file writes it with escapes. Every other byte of every file stays the same: the BOM, key order, indentation, inline arrays (`"targets": ["nsis", "msi"]`), line endings (LF or CRLF), a final newline or its absence, comments, every `version` that 2.1 does not locate (dependencies in `package-lock.json`, `Cargo.lock` entries with a `source`, `[workspace.dependencies]` in `Cargo.toml`), and strings that merely contain version-like text.
4. If no file changes (every source's decoded version is already `X.Y.Z`): stdout `release: nothing to change, already X.Y.Z`, exit 0, no file is written.
5. Otherwise writes each file that changed, then runs the check of 3.1 on the result. stdout `release: set version X.Y.Z in <files>`, where `<files>` lists the changed files by their 2.1 paths, in the order of 2.1, separated by `, ` (for example `release: set version 0.2.0 in package.json, package-lock.json, src-tauri/tauri.conf.json, Cargo.toml, Cargo.lock`). Exit 0. A file whose sources were already at `X.Y.Z` is not rewritten and not listed. A write that fails is `release: cannot write <file> (<Node error code>)`, exit 1 (files written before it stay written; the owner reverts them with git).

It never runs `cargo`, `npm` or `git`. The result is committed through a normal PR (section 6).

### 3.8 Exported functions

`scripts/release.mjs` is both the CLI and a module. When run as a script (its URL equals `process.argv[1]`'s), it calls `run(process.argv.slice(2), process.env)`, writes the lines, and exits with the code. It exports:

| Export | Does | I/O |
|---|---|---|
| `run(argv: string[], env: Record<string, string \| undefined>) → { code: 0 \| 1 \| 2 \| number, stdout: string[], stderr: string[] }` | the whole CLI of 3.0–3.7, with `env` in place of `process.env`; lines without their LF. `draft` without `--print` in Actions inherits stdio for `gh` | reads and writes as the command says; starts `git` and `gh` |
| `parseArgs(argv: string[]) → { command, positionals, options, errors: string[] }` | 3.0 (no environment: the `release-check` tag rule is applied by `run`) | none |
| `versionProblem(v: string) → string \| null` | the message of 3.1 step 3, or `null` when releasable | none |
| `tagProblem(tag: string) → string \| null` | the message of 3.2 step 2, or `null` | none |
| `locateJson(text: string, paths: string[][])` | the scanner of 2.1: for each target path, its decoded value and span (`start`, `end` offsets in `text`), or why it has none (absent, duplicate, not a string). `text` is the file's text **without its BOM** (the caller strips it and puts it back on write) | none |
| `locateCargoToml(text: string)`, `locateCargoLock(text: string)` | the line rules of 2.1, with the same kind of result (for `Cargo.lock`, one result per workspace package, with its `name`) | none |
| `replaceSpans(text: string, spans: { start, end }[], value: string) → string` | 3.7 step 3 | none |
| `sha256Sums(files: { name: string, bytes: Uint8Array }[]) → string` | 3.4 | none |
| `releaseNotes({ productName, version, nsis, msi, sums, repository }) → string` | 3.5 (`repository` `null` leaves out the provenance paragraph) | none |
| `draftArgs({ productName, version, assets, notes }) → string[]` | 3.6 step 6 | none |

The exact result shapes of the three `locate*` functions are left to the implementation; the scenarios about locating go through `run` (the messages of 2.1) and through `replaceSpans` on their spans, never through those shapes.

## 4. `just` recipes

Added to `justfile` (ADR-0019). The workflows call only these for release work. All release outputs are staged under one folder, `target/release-staging/` (cargo owns `target/release/`).

| Recipe | Runs | Used by |
|---|---|---|
| `version-check` | `node scripts/release.mjs version` | `just ci` (Q9), `ci.yml` job `specs`, `release.yml` dry run |
| `release-check *args` | `node scripts/release.mjs release-check {{args}}` | `release.yml` on a tag (no argument: the tag comes from `GITHUB_REF_NAME`); locally `just release-check v0.1.0` before tagging |
| `release-assets` | `node scripts/release.mjs assets --bundle target/release/bundle --out target/release-staging/assets --notes target/release-staging/release-notes.md --max-bytes 20000000` | `ci.yml` job `build-windows`, `release.yml` job `package` |
| `release-draft` | `node scripts/release.mjs draft --assets target/release-staging/assets --notes target/release-staging/release-notes.md` | `release.yml` job `publish` only |
| `set-version version` | `node scripts/release.mjs set-version {{version}}` | the human or an agent preparing a release PR (section 6) |

- `ci` becomes `ci: check-fast test trace guard-tests version-check check-app`.
- `build` stays `npm run tauri build` (NSIS + MSI).
- `20000000` appears exactly once in `justfile`, in `release-assets`. It is RNF-06's limit, 20 MB (Q7).
- `target/` is already ignored by git (`.gitignore`).

## 5. Workflow wiring

### 5.1 `ci.yml`

- Job `specs` gains `- run: just version-check` (Q9).
- Job `build-windows` keeps `just check-app`, then runs `just build` (instead of `npm run tauri build`) and `just release-assets` (instead of the inline PowerShell size step). It uploads the one folder `target/release-staging/` as the artifact `installers` (retention 7 days, `if-no-files-found: error`). So **every PR and every push to `main` runs the RNF-06 gate**, and `ci-ok` (the one required check, ADR-0028) fails when an installer is too big.

### 5.2 `release.yml`

```
on:
  push:
    tags: ["v*.*.*"]
  workflow_dispatch:            # dry run (Q5, D8): verify and package, never publish
permissions:
  contents: read                # workflow level: read only
concurrency:
  group: release-${{ github.ref }}
  cancel-in-progress: false

jobs:
  verify   (windows-latest; no permissions block)
             checkout with fetch-depth: 0 (release-check needs origin/main);
             setup-node; rust-toolchain with components: "rustfmt, clippy"; rust-cache; setup-just; npm ci;
             on a tag push (if: github.event_name == 'push'):           just release-check   (no argument, Q8)
             on workflow_dispatch (if: github.event_name == 'workflow_dispatch'): just version-check
             just ci
  package  (needs: verify; windows-latest; no permissions block, so read only)
             checkout; setup-node; rust-toolchain; rust-cache; setup-just; npm ci;
             just build; just release-assets;
             upload-artifact name "release", path: target/release-staging/
             (retention 7 days, if-no-files-found: error)
  publish  (needs: package; ubuntu-latest;
            if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v')
            permissions: contents: write, id-token: write, attestations: write)
             checkout; setup-node; setup-just;
             download-artifact name "release", path: target/release-staging
               (restores target/release-staging/assets/ and target/release-staging/release-notes.md)
             actions/attest-build-provenance@v4, subject-path:
               target/release-staging/assets/*.exe and target/release-staging/assets/*.msi
             just release-draft, with a step-level env in block style:
               env:
                 GH_TOKEN: ${{ github.token }}
```

- **Block style around expressions (note for the implementer).** The sketch above is not literal YAML for the jobs, but every mapping that holds a `${{ … }}` value is written in YAML **block style**, as shown for `concurrency` and the `GH_TOKEN` env. Never put an expression in a flow mapping (`{ group: release-${{ github.ref }} }`): `{` and `}` are flow indicators in YAML, so the braces of the expression end or break the mapping, and the workflow fails to load or gets a different value. Flow sequences without expressions (`tags: ["v*.*.*"]`) are fine.
- **One artifact folder.** `package` uploads the single directory `target/release-staging/`, so the artifact's root is that folder's content (`assets/` and `release-notes.md`). `publish` downloads it into the same path. Nothing relies on how `upload-artifact` picks the least common ancestor of several paths.
- **Least privilege:** the workflow level is `contents: read`. Only `publish` holds `contents: write`, `id-token: write` and `attestations: write`, and it builds nothing (it has `needs: package` and no `tauri build` or `just build`). `package` has no write permission. The dry run never reaches `publish` (its `if:` needs a tag push), so it creates no attestation and no release. **Unverified:** that `actions/attest-build-provenance@v4` needs exactly `id-token: write` and `attestations: write` (plus `contents: read`, implied by `contents: write`); confirmed by the first tag's `publish` run and by M4.
- **No expression injection:** every `run:` line is a single `just <recipe>` or `npm ci`. The tag reaches `release-check` and `draft` through `GITHUB_REF_NAME`, which the runner sets. The only `${{ … }}` values are in `concurrency`, `if:` and the `GH_TOKEN` env.
- **No inline release logic:** no step computes a hash, a size or a version itself (no `Get-FileHash`, `ConvertFrom-Json`, `Select-String`, `20MB`).
- **`verify` installs `rustfmt` and `clippy`**, because `just ci` runs `just check-fast` (`cargo fmt`, `cargo clippy`) and `check-app` (`cargo clippy`). The H0 `verify` job did not, and has never run.
- `workflow_dispatch` only appears in the GitHub UI once the workflow is on `main`. So the dry run (M1) is run after the feature PR is merged. After that it can be started on any branch (D8).

### 5.3 Static wiring checks (Q12)

`scripts/release.test.mjs` reads these files as text (it never runs them) and checks the points below. These definitions apply to **both** `release.yml` and `ci.yml`: the **workflow level** is the text before the line `jobs:`, and a **job's text** runs from its line `  <id>:` (two spaces, directly under `jobs:`) to the next such line or the end of the file. Every "the `<id>` job runs / contains / uploads" below is checked on that job's text only, so a recipe called in another job does not count.
- `justfile`: the five recipes of section 4 exist with exactly those commands; the `ci` line is exactly `ci: check-fast test trace guard-tests version-check check-app`; `20000000` appears exactly once, in `release-assets`.
- `.github/workflows/ci.yml`: the `specs` job runs `just version-check`; the `build-windows` job runs `just check-app`, `just build` and `just release-assets` and uploads `target/release-staging/`; the file contains neither `Get-FileHash`, `20MB`, nor `npm run tauri build`.
- `.github/workflows/release.yml`:
  - triggers `tags: ["v*.*.*"]` and `workflow_dispatch`;
  - the workflow level contains `contents: read` and no `: write`;
  - jobs `verify`, `package` and `publish` exist;
  - `verify` contains `fetch-depth: 0`, `components: "rustfmt, clippy"`, `just release-check`, `just version-check` and `just ci`;
  - `package` contains `needs: verify`, `just build`, `just release-assets` and `target/release-staging/`, and no `: write`;
  - `publish` contains `needs: package`, the line `    if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v')` exactly (four spaces, at job level; a `workflow_dispatch` run makes it false, so the dry run never publishes), `contents: write`, `id-token: write`, `attestations: write`, `actions/attest-build-provenance`, `target/release-staging` and `just release-draft`, and neither `tauri build` nor `just build`;
  - `contents: write`, `id-token: write` and `attestations: write` appear in no job other than `publish` and not at the workflow level;
  - no job other than `publish` has a job-level `if:` line (four spaces); step-level `if:` lines in `verify` are allowed;
  - every line containing `run:` matches `^\s*(- )?run: (just [a-z-]+|npm ci)\s*$`;
  - the file contains no `${{ github.ref_name }}`, `Get-FileHash`, `ConvertFrom-Json`, `20MB`, `npm run tauri build` or `gh release`.
- `src-tauri/tauri.conf.json` (as JSON): `bundle.targets` is exactly `["nsis", "msi"]` and `bundle.windows.webviewInstallMode.type` is `downloadBootstrapper` (D3).

## 6. Release procedure (human)

1. **Prepare.** On a branch `chore/release-X.Y.Z`: `just set-version X.Y.Z`, then `just ci` (it runs `version-check`). Commit `chore(release): X.Y.Z`, open a PR, and merge it at G2 when CI is green. `cargo` refreshes nothing else: `Cargo.lock` already carries the new version.
2. **Rehearse** (before the first release, and after any change to the release wiring): run the Release workflow by hand on `main` (M1).
3. **Tag.** Only the human tags (Q11): `just release-check vX.Y.Z` on an up-to-date `main`, then `git tag vX.Y.Z` on the merge commit and `git push origin vX.Y.Z`.
4. **CI** runs `verify`, `package` and `publish`, and leaves a draft release with three assets.
5. **Check** the draft (M2–M6), then **publish** it (M7).
6. **If anything fails after the tag:** the human deletes the draft, and also the tag while the release is unpublished, fixes the cause through a PR, and tags again. A published version is never re-tagged; a fix after publication is a new PATCH version.

## 7. Acceptance (measurable)

Automated (CI: `node --test ".claude/hooks/*.test.mjs" "scripts/*.test.mjs"` inside `just check-fast`, Linux and Windows; every test works in its own temporary folder, with the isolated environment of 8.1):
- **R1 (versions).** `version` passes exactly when the sources of 2.1 agree on a releasable version. Each source on its own, set to another version, fails with that source named. The eight decoys of the feature (in `package.json` a nested `config.version` and a `description` string; a `package-lock.json` dependency; in `tauri.conf.json` a nested `bundle.version`; in `Cargo.toml` a `[workspace.metadata.decoy]` version line, a `[workspace.dependencies]` entry and a commented line; a `Cargo.lock` registry entry; all at `0.1.0`) are never read and never changed. They are placed so that an implementation that takes the **first** match of a version pattern fails `version` (with the manifests at another version), and one that replaces **every** match fails `set-version`; for `package-lock.json`, whose real key order puts the real versions first, the first-match case is covered by the reordered file of the scenario "The version is found by key path, whatever the key order". Every example of 2.2 is accepted or refused as listed there, and every read error of 2.1 has its exact message.
- **R2 (tag).** `release-check` accepts `v<version>` only when the manifests hold `<version>` and `HEAD` is contained in `origin/main`, refuses every malformed tag of the scenario outline with its message, and prefers a tag argument over the environment.
- **R3 (assets).** After `assets`, `--out` holds exactly the two installers of the manifests' version and `SHA256SUMS.txt`. Entries of other versions in the bundle are ignored. A failed run reports every failure of steps 1–4 in the order of 3.3 and leaves `--out` and `--notes` unchanged; a write failure at steps 5–7 is one `cannot write` line, exit 1, and leaves nothing that the run created. A product name that cannot be part of a file name is refused before any installer is looked for.
- **R4 (RNF-06).** With `--max-bytes N`, an installer of `N − 1` bytes passes and one of `N` or `N + 1` bytes fails, for the NSIS and the MSI alike. With the recipe's figure, a 19 999 999-byte installer passes and a 20 000 000-byte installer fails. The recipe passes `--max-bytes 20000000`, and `build-windows` and `package` run that recipe (5.3).
- **R5 (checksums).** Each line of `SHA256SUMS.txt` is the SHA-256 of the copied file (for example `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad` for a file holding `abc`), with the format of 3.4; two runs give identical bytes.
- **R6 (notes and draft).** The notes are the text of 3.5 with `{productName}` replaced, and give the `gh attestation verify` command for both installers. `draft --print` prints the argument array of 3.6, `draft` refuses an `--assets` folder that holds anything else (including a sub-folder), and refuses to run outside GitHub Actions.
- **R7 (set-version).** After `set-version`, `version` passes with the new version, and each file differs from before only in the located version spans; the decoys are unchanged. A second run with the same version changes nothing.
- **R8 (wiring).** The checks of 5.3 hold.
- **R9 (usage).** Every usage error of 3.0 gives its exact message and exit 2, before anything is read.
- **R10 (agents never release).** The `guard-bash` cases of 8.1 give their expected exit codes.

Manual (the owner; recorded in the feature PR or in the release PR):
- **M1 (dry run).** After the feature PR is merged: run the Release workflow by hand (`workflow_dispatch`) on `main`. This is the first real run of the `verify` job (including its `rustfmt` and `clippy` components and `just ci` on the release runner). `verify` and `package` succeed, `publish` is shown as skipped, and the artifact `release` holds, at its root, the folder `assets/` (the two installers and `SHA256SUMS.txt`) and `release-notes.md`, and nothing else. No release and no attestation are created. (M1 cannot confirm the two unverified points of 3.6 and 5.2, since `publish` never runs in a dry run; M4 and M7 do.)
- **M2 (RNF-06 record).** Record the two sizes that `release-assets` prints in the `build-windows` log of the feature PR, and again in the `package` log of each release. The current size is not known: the only measurement on record is from H0 (`docs/agents/progress.md`: NSIS 1.44 MB, MSI 2.12 MB), before symphonia's lossy decoders (ADR-0040) and SQLite (ADR-0044, "roughly 1 MB").
- **M3 (install).** On a clean Windows 10 or 11 x64 machine (a VM or Windows Sandbox): download the NSIS installer from the draft and run it. SmartScreen shows its warning; **More info**, then **Run anyway**, installs the app for the current user without asking for administrator rights. The app starts from the Start menu, analyses a small folder of test files and shows verdicts.
- **M4 (checksums and provenance).** On the downloaded files, `Get-FileHash` as in the notes gives exactly the hashes in `SHA256SUMS.txt`, for both installers. The two `gh attestation verify` commands of the notes (one per installer) succeed, which confirms that the `publish` job's permission set was enough for `attest-build-provenance@v4` (5.2).
- **M5 (uninstall).** Before M3, note the size, modification time and SHA-256 of the analysed test files. Uninstall from Windows Settings. The test files are unchanged (RNF-05) and the app's install folder is gone. The uninstaller offers to delete the app's data (`%LOCALAPPDATA%\io.github.fakeflac.app`, which holds the result cache, ADR-0044).
- **M6 (MSI).** Install the MSI on the same machine after M5, start the app, and uninstall it.
- **M7 (publish).** Read the draft's notes: the section 3.5 text comes **first**, then GitHub's generated list of PRs (this confirms the `--notes-file` + `--generate-notes` ordering of 3.6). For the first release, which has no previous tag, the list holds every merged PR. Check that the draft has exactly three assets, and publish it.

## 8. Tests and protected tests

### 8.1 New tests (`test(RNF-11)` commit)

- **`scripts/release.test.mjs`** (new, protected after the test commit by `scripts/test-paths.mjs`): every scenario of `features/release.feature`.
  - **Two levels.** Most scenarios call the exported functions in-process: `run(argv, env)` for command behaviour, and the pure functions of 3.8 (`parseArgs`, `versionProblem`, `tagProblem`, `replaceSpans`, `sha256Sums`, `releaseNotes`, `draftArgs`) for the outlines over many inputs. Only these scenarios **spawn the CLI** (`process.execPath` and an argument array), because they are about the process itself (exit code, stdout versus stderr, argv passing, no stack trace): "Manifests that agree on a releasable version pass"; "Every problem is reported, not only the first"; "A tag of the manifests' version on main passes"; "In the workflow the tag comes from the environment" (its first step only); "The assets of a release"; "Paths with spaces and Unicode characters"; "Usage errors decided before anything is read"; "The draft never runs outside GitHub Actions"; "set-version rewrites only the version strings". That is nine spawns.
  - **Environment isolation (every spawn and every `run` call).** The test builds the child environment explicitly: copy `process.env`, delete every key that starts with `GITHUB_`, `GH_` or `GIT_` (compared case-insensitively, so on Windows too: `GITHUB_ACTIONS`, `GITHUB_REF_TYPE`, `GITHUB_REF_NAME`, `GH_TOKEN`, `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE` …), then set `GIT_CONFIG_NOSYSTEM=1` and `GIT_CONFIG_GLOBAL` to the path of an empty file in the test's temporary folder (the same two as the git fixture below), then set only the variables the scenario names. The two git variables are in every case's environment, not only the `release-check` cases (the only command that runs git): it costs nothing and no case can forget them. So the developer's or runner's system and global git configuration (`safe.directory`, `core.autocrlf`, `init.defaultBranch`, aliases, hooks paths …) never reaches a `git` started by the script. This matters in CI: `just check-fast` runs inside `release.yml`'s `verify` job on a tag push, where `GITHUB_ACTIONS` is `true` and `GITHUB_REF_NAME` is the real tag; without it, scenarios would fail, and "The draft never runs outside GitHub Actions" could run a real `gh release create`. The same environment is passed to `run` and, through it, to `git` and `gh`.
  - **Git fixture.** The `release-check` scenarios need a git repository. It is built **once per test file** (in `before`) in a temporary folder: `git init`, then commits A and B on a branch, with the isolated environment plus `GIT_CONFIG_NOSYSTEM=1` and `GIT_CONFIG_GLOBAL` set to an empty file in the temporary folder, and every commit made with `-c user.name=FakeFlac Test -c user.email=test@example.invalid -c commit.gpgsign=false`, so that the developer's or runner's git configuration does not matter. Each case **copies** that folder (`fs.cpSync`, recursive) and sets `HEAD` and `refs/remotes/origin/main` in its copy with `git update-ref` / `git checkout --detach` as the scenario says.
  - It runs on Linux and Windows, needs only Node and git, and writes nothing outside its temporary folders.
  - **Budget:** under 10 s on a developer machine for the whole file, so that `just check-fast` stays under RP-08's 60 s. (The first figure, 5 s, was too tight. On Windows the file takes about 4.8–5.9 s, and almost all of that is the three git spawns per `release-check` case that 3.2 step 4 requires, so the time is not waste.)
- **`.claude/hooks/guard-bash.mjs`** (production) and **`.claude/hooks/hooks.test.mjs`** (existing test): two rules appended to the `rules` array, in the existing style (a case-insensitive regular expression and a reason; a match exits 2 with `Blocked by guard-bash: <reason>.`):
  - **tag push:** within a `git push` segment, that is `\bgit\s+push\b[^\n;|&]*` as in the existing rules, a `--tags` or `--follow-tags` option, or a ref token anchored on a separator: `[\s:+](refs\/tags\/|v\d+\.\d+\.\d+)`. One expression: `/\bgit\s+push\b[^\n;|&]*(\s--tags\b|\s--follow-tags\b|[\s:+](refs\/tags\/|v\d+\.\d+\.\d+))/i`. Reason: `pushing a tag is a human act (releases, ADR-0045)`. A ref token is matched only after a space, `:` or `+`, so a branch that merely contains a version after a `/` or `-` (`feat/v2-ui`, `chore/release-0.1.0`) is allowed. A branch whose name itself starts with `v<n>.<n>.<n>` is blocked; this false positive is accepted.
  - **release:** `/\bgh\s+release\s+(create|edit|upload)\b/i`. Reason: `creating or changing a release is a human act (ADR-0045)`. (`gh release delete` is already blocked by the existing `destructive gh operation` rule.)
  - New cases, appended to the existing `[cmd, expected]` table in the test commit; no existing case changes:

    | Command | Exit |
    |---|---|
    | `git push origin v0.1.0` | 2 |
    | `git push --delete origin v0.1.0` | 2 |
    | `git push origin :v0.1.0` | 2 |
    | `git push origin +v0.1.0` | 2 |
    | `git push origin refs/tags/v0.1.0` | 2 |
    | `git push --tags` | 2 |
    | `git push --follow-tags origin feat/RNF-11` | 2 |
    | `gh release create v0.1.0` | 2 |
    | `gh release edit v0.1.0 --draft=false` | 2 |
    | `gh release upload v0.1.0 SHA256SUMS.txt` | 2 |
    | `gh release delete v0.1.0` | 2 |
    | `git push -u origin feat/v2-ui` | 0 |
    | `git push origin chore/release-0.1.0` | 0 |
    | `git push origin feat/RNF-11` | 0 |
    | `git tag --list` | 0 |
    | `gh release view v0.1.0` | 0 |
    | `gh release list` | 0 |
  - **Out of scope**, as for the existing rules: forms that put something between `git` and `push` (`git -C dir push …`, `git -c k=v push …`), git aliases, scripts that push, and `gh api` calls to the releases endpoints bypass these patterns. The hook is a guard rail for agents, not a security boundary; the human-only steps are also enforced by branch protection and by `draft` refusing to run outside GitHub Actions.

### 8.2 Existing protected tests checked

- `scripts/prune-branches.test.mjs` and `scripts/guard-tests.test.mjs`: unaffected (no shared code).
- The existing cases of `.claude/hooks/hooks.test.mjs` keep their expected exit codes: none of them pushes a tag (no `--tags`, `--follow-tags`, `refs/tags/` or separator-anchored `v<n>.<n>.<n>` in a `git push` segment) or runs `gh release create|edit|upload`.
- No Rust or frontend test changes: the app's code and contract do not change.

### 8.3 Production files the feature PR changes

`scripts/release.mjs` (new), `justfile`, `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.claude/hooks/guard-bash.mjs` (Q11). `src-tauri/tauri.conf.json`, the manifests and the app's code do not change. No dependency is added.

## 9. Edits to approved specs and docs (proposed in this PR)

- `specs/requirements/RNF-11.md` and `RNF-06.md`: measurable statements and links to this spec; status `specified`.
- `specs/traceability.md`: one row for RNF-11 and RNF-06; the RNF-04/05/09 row names the `@RNF-09` scenario of `features/release.feature`.
- `docs/adr/0045-…md` (proposed) and its row in `docs/adr/README.md`.
- To be done in the feature PR (outside `specs/`): `docs/architecture/overview.md`, "Quality attributes → design": the RNF-06 row gains "size gate `just release-assets` on every PR and release (ADR-0045)", and a new RNF-11 row "tagged releases built by `release.yml`, draft published by the human (ADR-0028, ADR-0032, ADR-0045)".
