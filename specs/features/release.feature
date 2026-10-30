@RNF-11
Feature: Release packaging: versions, installers, size gate, checksums and the draft release
  As the owner of FakeFlac
  I want every version built by CI into a draft GitHub release with its installers and checksums
  So that users can download an installer under 20 MB and check that it is the one CI built

  Spec: specs/contracts/release.md (versions and how they are located, scripts/release.mjs,
  arguments and messages, recipes, workflow wiring, procedure, acceptance R1 to R10 and
  manual checks M1 to M7).
  Decisions: ADR-0028, ADR-0029, ADR-0032, ADR-0045 (proposed).

  How the scenarios run (release.md 8.1):
    The scenarios that release.md 8.1 lists as CLI scenarios spawn
    "node scripts/release.mjs" (process.execPath and an argument array). Every
    other scenario calls the exported run(argv, env) in-process, or a pure
    function of release.md 3.8, and asserts the same exit code and lines.
    Everything happens on temporary folders made by the test, never on the
    repository (ADR-0022). The scenarios under "Wiring" only read repository
    files as text. The scenario outline under "Agents never release" is tested
    in .claude/hooks/hooks.test.mjs.
  The environment (every spawn and every run call):
    the test copies process.env, deletes every key that starts with GITHUB_,
    GH_ or GIT_ (case-insensitively), then sets GIT_CONFIG_NOSYSTEM to "1" and
    GIT_CONFIG_GLOBAL to an empty file in the test's temporary folder, and then
    sets only the variables that the scenario names. So GITHUB_ACTIONS,
    GITHUB_REF_TYPE, GITHUB_REF_NAME, GH_TOKEN, GIT_DIR and GIT_WORK_TREE of a
    CI runner (for example the real tag of a release run) never reach the
    script, git or gh, and neither does the developer's or runner's system or
    global git configuration.
  "The test root" is a temporary folder holding, written by the test, with the
  members and lines in the order given here (each file need only be valid JSON
  or have the shape that release.md 2.1 reads):
    package.json: "name"; then "config": {"version": "0.1.0"}; then a
      "description" whose text is decoy "version": "0.1.0" (the quotes escaped
      inside the JSON string, so the file holds \"version\": \"0.1.0\"); then
      the top-level "version";
    package-lock.json, in npm's own order: "name", the top-level "version",
      "lockfileVersion", then "packages" holding first packages[""] (with
      "name" then "version") and then the dependency
      packages["node_modules/decoy"] with "version": "0.1.0";
    src-tauri/tauri.conf.json: productName "FakeFlac"; then "bundle": {"version":
      "0.1.0", "targets": ["nsis", "msi"]} with the targets written inline;
      then the top-level "version";
    Cargo.toml: a [workspace.metadata.decoy] table holding the line
      version = "0.1.0"; then a [workspace.dependencies] table holding
      foo = { version = "0.1.0" }; then a [workspace.package] table holding the
      comment line # version = "0.1.0", then its version line, then
      repository = "https://github.com/salvador-arance/FakeFlac";
    Cargo.lock: Cargo's header comment and the line version = 4 (the file
      format), then the entries in Cargo's order, sorted by name: registry
      entry "decoy" version "0.1.0" (with a source line), source-less entries
      "fakeflac" and "fakeflac-core", registry entry "serde" version "1.0.228"
      (with a source line).
  "The decoys" are the eight "0.1.0" values above that are not the app's
  version: package.json config.version and the description; the package-lock.json
  dependency "decoy"; tauri.conf.json bundle.version; in Cargo.toml the
  [workspace.metadata.decoy] line, the [workspace.dependencies] entry foo and the
  comment line; the Cargo.lock registry entry "decoy". They always hold "0.1.0",
  whatever version the manifests hold.
  Where the decoys are placed: in every file except package-lock.json (below),
  a decoy comes before the real version (so an implementation that takes the first match of a version
  pattern reads "0.1.0" and fails when the manifests are at another version),
  and every file holds a decoy at all (so one that replaces every match fails
  set-version). package.json: config.version and the escaped description come
  first (the description also catches a scanner that ends a string at \").
  tauri.conf.json: bundle.version comes first. Cargo.toml: the
  [workspace.metadata.decoy] line comes first for a line-anchored pattern, the
  [workspace.dependencies] entry for an unanchored one, and the comment line
  for one that looks only inside [workspace.package]. Cargo.lock: "decoy"
  sorts before "fakeflac", so it is the first [[package]] entry.
  package-lock.json is the exception: npm always writes "name" then "version"
  at the top level and puts packages[""] first in "packages", so in a real file
  no decoy can come before the two real versions. Its dependency decoy catches
  replace-all; the scenario "The version is found by key path, whatever the key
  order" reorders the file to catch first-match.
  "Manifests at X" means that every source of release.md 2.1 holds X: package.json,
  package-lock.json, package-lock.json packages[""], src-tauri/tauri.conf.json,
  Cargo.toml [workspace.package], Cargo.lock fakeflac and Cargo.lock fakeflac-core
  (7 sources).
  "The bundle" is a temporary folder with sub-folders nsis and msi.
  "<nsis>" is "FakeFlac_0.1.0_x64-setup.exe" and "<msi>" is
  "FakeFlac_0.1.0_x64_en-US.msi" unless a scenario says otherwise.
  "Run assets" means: assets --root <test root> --bundle <bundle>
  --out <tmp>/out --notes <tmp>/notes.md --max-bytes 1000, unless a
  scenario gives another --max-bytes.
  In expected lines, <bundle>, <tmp> and <test root> stand for the path as
  passed on the command line, with every "\" replaced by "/" (release.md 3.0).
  Assertions on output:
    'stdout is "X"' and 'stderr is "X"': exactly that one line, nothing else.
    'stderr is exactly:' followed by lines: exactly those lines, in that order.
    'stdout has the line "X"' and 'stderr has the line "X"': one whole line equals X.
    'stderr is empty': no line at all.
  Exit codes: 0 success, 1 a failed check, 2 a usage error. Every printed
  line starts with "release: " (except the JSON line of draft --print).

  # --- Versions (release.md 2 and 3.1) -------------------------------------

  Scenario: Manifests that agree on a releasable version pass
    Given the test root with manifests at "0.1.0"
    When I run "version"
    Then the exit code is 0
    And stdout is "release: version 0.1.0 (7 sources agree)"
    And stderr is empty

  Scenario Outline: A manifest that disagrees is named
    Given the test root with manifests at "0.1.0"
    And the version of <source> is "0.2.0"
    When I run "version"
    Then the exit code is 1
    And stderr is exactly the line "release: the manifests do not agree on the version" followed by one line per source, in the order of release.md 2.1, each "release:   <source name>: <version>", with "0.2.0" for <source> and "0.1.0" for every other source

    Examples:
      | source                          |
      | package.json                    |
      | package-lock.json               |
      | package-lock.json packages[""]  |
      | src-tauri/tauri.conf.json       |
      | Cargo.toml [workspace.package]  |
      | Cargo.lock fakeflac             |
      | Cargo.lock fakeflac-core        |

  Scenario: The decoys are never read as the app's version
    The decoys hold "0.1.0" and the manifests "0.2.0": a decoy read as a
    version, for example by taking the first match of a version pattern in a
    file, would make the manifests disagree.
    Given the test root with manifests at "0.2.0"
    And its Cargo.toml also has a table "[other]" after [workspace.package] holding the line version = "9.9.9"
    When I run "version"
    Then the exit code is 0
    And stdout is "release: version 0.2.0 (7 sources agree)"
    And stderr is empty

  Scenario: The version is found by key path, whatever the key order
    A file npm did not write may order its members differently. Here the
    dependency decoy comes before both real versions of package-lock.json.
    Given the test root with manifests at "0.2.0"
    And its package-lock.json holds the same members in this order: "packages" first, holding packages["node_modules/decoy"] (version "0.1.0") before packages[""], then "lockfileVersion", then the top-level "version", then "name"
    When I run "version"
    Then the exit code is 0
    And stdout is "release: version 0.2.0 (7 sources agree)"
    When I run "set-version 0.3.0"
    Then the exit code is 0
    And package-lock.json equals its text before with exactly its two located version spans replaced by "0.3.0" and no other byte changed
    And the dependency "node_modules/decoy" still holds "0.1.0"

  Scenario: A Cargo.lock entry whose name line comes after its version line
    Cargo writes name before version, but the line rule does not depend on it.
    Given the test root with manifests at "0.1.0"
    And in the Cargo.lock entry "fakeflac-core" the line version = "0.2.0" comes before the line name = "fakeflac-core"
    When I run "version"
    Then the exit code is 1
    And stderr has the line "release:   Cargo.lock fakeflac: 0.1.0"
    And stderr has the line "release:   Cargo.lock fakeflac-core: 0.2.0"
    When I run "set-version 0.2.0"
    Then the exit code is 0
    And stdout is "release: set version 0.2.0 in package.json, package-lock.json, src-tauri/tauri.conf.json, Cargo.toml, Cargo.lock"
    And the Cargo.lock entry "fakeflac-core" still has its version line before its name line
    And running "version" then gives "release: version 0.2.0 (7 sources agree)"

  Scenario: A JSON version written with escapes is compared decoded and replaced whole
    "The escaped value" is the ten characters: a backslash, "u", "0030" (the
    JSON escape for the digit 0), then ".1.0". It decodes to 0.1.0.
    Given the test root with manifests at "0.1.0"
    And the top-level version of package.json is written in the file as the escaped value between its quotes
    When I run "version"
    Then the exit code is 0
    And stdout is "release: version 0.1.0 (7 sources agree)"
    When I run "set-version 0.1.0"
    Then the exit code is 0
    And stdout is "release: nothing to change, already 0.1.0"
    And package.json is byte-identical to before and still holds the escaped value
    When I run "set-version 0.2.0"
    Then the exit code is 0
    And package.json equals its text before with the escaped value (ten characters) replaced by the five characters 0.2.0 and no other byte changed

  Scenario: CRLF line endings and a UTF-8 BOM are read
    Given the test root with manifests at "0.1.0"
    And Cargo.toml and Cargo.lock use CRLF line endings
    And package.json starts with a UTF-8 BOM
    When I run "version"
    Then the exit code is 0
    And stdout is "release: version 0.1.0 (7 sources agree)"

  Scenario Outline: Only MAJOR.MINOR.PATCH within the MSI range is a release version
    Given the test root with manifests at "<version>"
    When I run "version"
    Then the exit code is <code>
    And <stream> is "<message>"

    Examples:
      | version       | code | stream | message                                                                                                     |
      | 0.0.1         | 0    | stdout | release: version 0.0.1 (7 sources agree)                                                                    |
      | 1.0.0         | 0    | stdout | release: version 1.0.0 (7 sources agree)                                                                    |
      | 255.255.65535 | 0    | stdout | release: version 255.255.65535 (7 sources agree)                                                            |
      | 1.0.0-beta.1  | 1    | stderr | release: version "1.0.0-beta.1" has a pre-release or build part; v1 ships no pre-releases (MAJOR.MINOR.PATCH only) |
      | 1.0.0+5       | 1    | stderr | release: version "1.0.0+5" has a pre-release or build part; v1 ships no pre-releases (MAJOR.MINOR.PATCH only) |
      | 01.0.0        | 1    | stderr | release: version "01.0.0" is not MAJOR.MINOR.PATCH (numbers without leading zeros)                          |
      | 1.0           | 1    | stderr | release: version "1.0" is not MAJOR.MINOR.PATCH (numbers without leading zeros)                             |
      | 1.0.0.0       | 1    | stderr | release: version "1.0.0.0" is not MAJOR.MINOR.PATCH (numbers without leading zeros)                         |
      | 256.0.0       | 1    | stderr | release: version "256.0.0" is outside the MSI range (MAJOR and MINOR at most 255, PATCH at most 65535)      |
      | 0.256.0       | 1    | stderr | release: version "0.256.0" is outside the MSI range (MAJOR and MINOR at most 255, PATCH at most 65535)      |
      | 0.0.65536     | 1    | stderr | release: version "0.0.65536" is outside the MSI range (MAJOR and MINOR at most 255, PATCH at most 65535)    |

  Scenario Outline: A broken or missing manifest is a clear error, never a crash
    The release analogue of RNF-04: bad input gives one exact message naming
    the file or the source (release.md 2.1, "Read errors").
    Given the test root with manifests at "0.1.0"
    And <damage>
    When I run "version"
    Then the exit code is 1
    And stderr is "<message>"
    And no line of the output starts with "    at " (no stack trace)

    Examples:
      | damage                                                            | message                                                         |
      | package.json holds "{not json"                                    | release: package.json is not valid JSON                         |
      | src-tauri/tauri.conf.json does not exist                          | release: missing src-tauri/tauri.conf.json                      |
      | package-lock.json does not exist                                  | release: missing package-lock.json                              |
      | package-lock.json has no packages[""].version                     | release: package-lock.json packages[""] has no version          |
      | package-lock.json has no "packages" key                           | release: package-lock.json packages[""] has no version          |
      | package.json has "version": 1                                     | release: package.json version is not a string                   |
      | src-tauri/tauri.conf.json has "version": ""                       | release: src-tauri/tauri.conf.json version is empty             |
      | package.json has a second top-level "version" key holding "0.1.0" | release: package.json has a duplicate "version" key             |
      | Cargo.toml has no version line in [workspace.package]             | release: Cargo.toml [workspace.package] has no version          |
      | Cargo.toml has no [workspace.package] table                       | release: Cargo.toml [workspace.package] has no version          |
      | the version line of Cargo.toml [workspace.package] is version = 1 | release: Cargo.toml [workspace.package] version is not a string |
      | Cargo.lock holds only the registry entries                        | release: Cargo.lock lists no workspace package                  |
      | the Cargo.lock entry "fakeflac-core" has no version line          | release: Cargo.lock fakeflac-core has no version                |
      | the Cargo.lock entry "fakeflac-core" has no name line             | release: Cargo.lock has a workspace package without a name      |
      | Cargo.lock has a second source-less entry "fakeflac-core"         | release: Cargo.lock lists the workspace package fakeflac-core more than once |
      | Cargo.lock does not exist                                         | release: missing Cargo.lock                                     |

  Scenario: Every problem is reported, not only the first
    Given the test root with manifests at "0.1.0"
    And package.json holds "{not json"
    And Cargo.lock does not exist
    When I run "version"
    Then the exit code is 1
    And stderr is exactly:
      """
      release: package.json is not valid JSON
      release: missing Cargo.lock
      """
    And no line of the output starts with "    at " (no stack trace)

  # --- Release tag (release.md 3.2) ----------------------------------------
  # The git fixture (release.md 8.1): the test root with manifests at "0.1.0"
  # is also a git repository with two commits, A then B, built once per test
  # file and copied for each case. HEAD and refs/remotes/origin/main are set in
  # the copy. <A12> and <B12> are the first 12 hex digits of A and B.

  Scenario: A tag of the manifests' version on main passes
    Given the git fixture with HEAD at B and origin/main pointing at B
    When I run "release-check v0.1.0"
    Then the exit code is 0
    And stdout is "release: tag v0.1.0 releases version 0.1.0 from commit <B12> on origin/main"

  Scenario Outline: The release tag must name the manifests' version
    Given the git fixture with HEAD at B and origin/main pointing at B
    When I run "release-check <tag>"
    Then the exit code is 1
    And stderr is "<message>"

    Examples:
      | tag           | message                                                                                          |
      | 0.1.0         | release: tag "0.1.0" is not v<MAJOR>.<MINOR>.<PATCH>                                             |
      | V0.1.0        | release: tag "V0.1.0" is not v<MAJOR>.<MINOR>.<PATCH>                                            |
      | v0.1          | release: tag "v0.1" is not v<MAJOR>.<MINOR>.<PATCH>                                              |
      | v00.1.0       | release: tag "v00.1.0" is not v<MAJOR>.<MINOR>.<PATCH>                                           |
      | v256.0.0      | release: tag "v256.0.0" is not v<MAJOR>.<MINOR>.<PATCH>                                          |
      | v0.1.0-beta.1 | release: tag "v0.1.0-beta.1" has a pre-release or build part; v1 ships no pre-releases (vMAJOR.MINOR.PATCH only) |
      | v0.1.0+5      | release: tag "v0.1.0+5" has a pre-release or build part; v1 ships no pre-releases (vMAJOR.MINOR.PATCH only) |
      | v0.2.0        | release: tag "v0.2.0" does not match the manifests' version 0.1.0                                |

  Scenario Outline: Only a commit of main can be released
    Given the git fixture with <refs>
    When I run "release-check v0.1.0"
    Then the exit code is <code>
    And <stream> is "<message>"

    Examples:
      | refs                                     | code | stream | message                                                                                     |
      | HEAD at A and origin/main pointing at B  | 0    | stdout | release: tag v0.1.0 releases version 0.1.0 from commit <A12> on origin/main                 |
      | HEAD at B and origin/main pointing at A  | 1    | stderr | release: commit <B12> is not on origin/main; tag a commit of main                           |
      | HEAD at B and no origin/main             | 1    | stderr | release: origin/main not found (fetch it, for example actions/checkout with fetch-depth: 0) |

  Scenario: A folder that is not a git repository is a git error
    Given the test root with manifests at "0.1.0" and no .git folder
    And the environment GIT_CEILING_DIRECTORIES set to the parent folder of the test root
    When I run "release-check v0.1.0"
    Then the exit code is 1
    And stderr has one line, starting with "release: git rev-parse failed (exit 128): "

  Scenario: In the workflow the tag comes from the environment
    Given the git fixture with HEAD at B and origin/main pointing at B
    When I run "release-check" with no tag and the environment GITHUB_REF_TYPE "tag" and GITHUB_REF_NAME "v0.1.0"
    Then the exit code is 0
    When I run "release-check" with no tag and the environment GITHUB_REF_TYPE "tag" and GITHUB_REF_NAME "v0.1.0-rc.1"
    Then the exit code is 1
    And stderr is "release: tag "v0.1.0-rc.1" has a pre-release or build part; v1 ships no pre-releases (vMAJOR.MINOR.PATCH only)"
    When I run "release-check" with no tag and the environment GITHUB_REF_TYPE "branch" and GITHUB_REF_NAME "main"
    Then the exit code is 2
    And stderr is "release: release-check needs a tag such as v1.2.3"

  Scenario: A tag argument wins over the environment
    Given the git fixture with HEAD at B and origin/main pointing at B
    When I run "release-check v0.1.0" with the environment GITHUB_REF_TYPE "tag" and GITHUB_REF_NAME "v0.2.0"
    Then the exit code is 0
    And stdout is "release: tag v0.1.0 releases version 0.1.0 from commit <B12> on origin/main"

  # --- Installers, checksums and notes (release.md 3.3 to 3.5) -------------

  Scenario: The assets of a release
    Given the test root with manifests at "0.1.0"
    And the bundle holds "nsis/<nsis>" containing "abc" and "msi/<msi>" containing "abcd"
    When I run assets
    Then the exit code is 0
    And <tmp>/out holds exactly "<nsis>", "<msi>" and "SHA256SUMS.txt"
    And each copied installer is byte-identical to its bundle file, and the bundle is unchanged
    And SHA256SUMS.txt is exactly these two lines, each ending with LF, the last one included:
      """
      ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  FakeFlac_0.1.0_x64-setup.exe
      88d4266fd4e6338d13b845fcf289579d209c897823b9217da3e161936f031589  FakeFlac_0.1.0_x64_en-US.msi
      """
    And stdout is exactly:
      """
      release: FakeFlac_0.1.0_x64-setup.exe 3 bytes (0.00 MiB), limit 1000 bytes
      release: FakeFlac_0.1.0_x64_en-US.msi 4 bytes (0.00 MiB), limit 1000 bytes
      release: wrote <tmp>/out (2 installers, SHA256SUMS.txt) and <tmp>/notes.md
      """
    And stderr is empty

  Scenario: The checksums file has the sha256sum text format
    Given the assets of a release have been written
    Then SHA256SUMS.txt has no BOM, no CR byte, and ends with exactly one LF
    And every line matches "^[0-9a-f]{64}  [^/\\]+$"
    And the lines are sorted by file name
    And the SHA-256 of each file in <tmp>/out other than SHA256SUMS.txt is on the line with its name

  Scenario: Running twice gives identical files
    Given the bundle of "The assets of a release"
    When I run assets twice
    Then the second run exits 0
    And SHA256SUMS.txt and notes.md are byte-identical after both runs

  Scenario: The release notes
    Given the assets of a release have been written
    Then notes.md equals the text of release.md 3.5 with {productName} "FakeFlac", {version} "0.1.0", {nsis} "<nsis>", {msi} "<msi>", {sums} the content of SHA256SUMS.txt and {repository} "salvador-arance/FakeFlac"
    And notes.md is UTF-8 without BOM, with LF line endings, and ends with one LF
    And its first line is "FakeFlac 0.1.0 for Windows 10 and 11 (64-bit)."
    And it contains both lines of SHA256SUMS.txt verbatim
    And it contains "The installers are not code-signed", "**More info**", "**Run anyway**" and "Get-FileHash .\<nsis> -Algorithm SHA256"
    And it contains the line "To check that GitHub Actions built the installers from this repository: `gh attestation verify <nsis> --repo salvador-arance/FakeFlac` and `gh attestation verify <msi> --repo salvador-arance/FakeFlac`."
    And it contains "If Microsoft Edge WebView2 is missing, the installer downloads it"

  Scenario: The notes take the product name from tauri.conf.json
    Given the test root with manifests at "0.1.0" and productName "TestApp"
    And the bundle holds "nsis/TestApp_0.1.0_x64-setup.exe" and "msi/TestApp_0.1.0_x64_en-US.msi"
    When I run assets
    Then the exit code is 0
    And the first line of notes.md is "TestApp 0.1.0 for Windows 10 and 11 (64-bit)."
    And notes.md contains "It installs TestApp for the current user.", "TestApp is free, open-source software" and "TestApp itself makes no network connection."
    And notes.md contains "FakeFlac" only in the "gh attestation verify" line (the repository name)

  Scenario: No provenance line without a GitHub repository URL
    Given the test root with manifests at "0.1.0" and a Cargo.toml without a repository line
    And the bundle of "The assets of a release"
    When I run assets
    Then the exit code is 0
    And notes.md does not contain "gh attestation verify" or "To check that GitHub Actions"

  Scenario Outline: A product name that cannot be part of a file name is a clear error
    The installer names are built by plain concatenation (release.md 3.3), so
    the product name must be safe in a file name. In the message, the name is
    shown as JSON.stringify gives it. The cells use Gherkin's table escapes:
    "\\" is one backslash and "\|" is one pipe.
    Given the test root with manifests at "0.1.0" and <product name>
    And the bundle of "The assets of a release"
    When I run assets
    Then the exit code is 1
    And stderr is "<message>"
    And <tmp>/out and <tmp>/notes.md do not exist

    Examples:
      | product name                                           | message                                                                                              |
      | a tauri.conf.json without "productName"                | release: src-tauri/tauri.conf.json has no productName                                                |
      | productName ""                                         | release: src-tauri/tauri.conf.json productName is empty                                              |
      | productName 1 (a number)                               | release: src-tauri/tauri.conf.json productName is not a string                                       |
      | productName "Fake/Flac"                                | release: src-tauri/tauri.conf.json productName "Fake/Flac" has a character not allowed in a file name  |
      | productName "Fake\\Flac" (one backslash)               | release: src-tauri/tauri.conf.json productName "Fake\\\\Flac" has a character not allowed in a file name |
      | productName "Fake:Flac"                                | release: src-tauri/tauri.conf.json productName "Fake:Flac" has a character not allowed in a file name  |
      | productName "Fake*Flac"                                | release: src-tauri/tauri.conf.json productName "Fake*Flac" has a character not allowed in a file name  |
      | productName "Fake?Flac"                                | release: src-tauri/tauri.conf.json productName "Fake?Flac" has a character not allowed in a file name  |
      | productName "Fake\|Flac"                               | release: src-tauri/tauri.conf.json productName "Fake\|Flac" has a character not allowed in a file name |
      | productName holding a tab (U+0009) between Fake and Flac | release: src-tauri/tauri.conf.json productName "Fake\tFlac" has a character not allowed in a file name |

  Scenario: Installers of other versions are ignored
    The ignored lines of both folders are sorted together by basename, in
    UTF-16 code-unit order ("-" sorts before "_", and "0" before "F"), and come
    before the size lines.
    Given the test root with manifests at "0.1.0"
    And the bundle of "The assets of a release"
    And the bundle also holds "msi/FakeFlac_0.0.1_x64_en-US.msi", "nsis/FakeFlac_0.0.1_x64-setup.exe" and "msi/0-old.msi"
    When I run assets
    Then the exit code is 0
    And <tmp>/out holds exactly "<nsis>", "<msi>" and "SHA256SUMS.txt"
    And SHA256SUMS.txt has exactly 2 lines
    And stdout is exactly:
      """
      release: ignored <bundle>/msi/0-old.msi (not an installer of 0.1.0)
      release: ignored <bundle>/nsis/FakeFlac_0.0.1_x64-setup.exe (not an installer of 0.1.0)
      release: ignored <bundle>/msi/FakeFlac_0.0.1_x64_en-US.msi (not an installer of 0.1.0)
      release: FakeFlac_0.1.0_x64-setup.exe 3 bytes (0.00 MiB), limit 1000 bytes
      release: FakeFlac_0.1.0_x64_en-US.msi 4 bytes (0.00 MiB), limit 1000 bytes
      release: wrote <tmp>/out (2 installers, SHA256SUMS.txt) and <tmp>/notes.md
      """

  Scenario Outline: Missing installers (the empty state)
    "The NSIS line" is "release: missing installer <bundle>/nsis/FakeFlac_0.1.0_x64-setup.exe"
    and "the MSI line" is "release: missing installer <bundle>/msi/FakeFlac_0.1.0_x64_en-US.msi".
    Given the test root with manifests at "0.1.0"
    And <bundle state>
    And <tmp>/out does not exist
    When I run assets
    Then the exit code is 1
    And stderr is exactly <lines>
    And <tmp>/out and <tmp>/notes.md do not exist

    Examples:
      | bundle state                                                           | lines                       |
      | the bundle folder does not exist                                       | the NSIS line, the MSI line |
      | the bundle has empty nsis and msi folders                              | the NSIS line, the MSI line |
      | the bundle holds only the installers of 0.0.1                          | the NSIS line, the MSI line |
      | the bundle holds a folder named "nsis/<nsis>" and a file "msi/<msi>"   | the NSIS line               |
      | the bundle holds only "nsis/<nsis>" of 10 bytes                        | the MSI line                |
      | the bundle holds only "msi/<msi>" of 10 bytes                          | the NSIS line               |

  Scenario: Every failure of steps 1 to 4 is reported, in order
    Given the test root with manifests at "0.1.0"
    And the bundle holds only "nsis/<nsis>" of 1001 bytes
    And <tmp>/out holds a file "notes.txt"
    When I run assets with --max-bytes 1000
    Then the exit code is 1
    And stdout has the line "release: FakeFlac_0.1.0_x64-setup.exe 1001 bytes (0.00 MiB), limit 1000 bytes"
    And stderr is exactly:
      """
      release: missing installer <bundle>/msi/FakeFlac_0.1.0_x64_en-US.msi
      release: FakeFlac_0.1.0_x64-setup.exe is 1001 bytes, not under the limit of 1000 bytes (RNF-06)
      release: <tmp>/out holds notes.txt, which assets did not write; remove it by hand
      """
    And <tmp>/out holds only "notes.txt", unchanged, and <tmp>/notes.md does not exist

  Scenario: An earlier output folder is replaced
    Given the test root with manifests at "0.1.0" and the bundle of "The assets of a release"
    And <tmp>/out holds "FakeFlac_0.0.1_x64-setup.exe", "FakeFlac_0.0.1_x64_en-US.msi" and an older "SHA256SUMS.txt"
    When I run assets
    Then the exit code is 0
    And <tmp>/out holds exactly "<nsis>", "<msi>" and "SHA256SUMS.txt"

  Scenario Outline: An output folder holding anything else is left alone
    Given the test root with manifests at "0.1.0" and the bundle of "The assets of a release"
    And <out state>
    When I run assets
    Then the exit code is 1
    And stderr is "<message>"
    And <tmp>/out is exactly what it was before, byte for byte, and <tmp>/notes.md does not exist

    Examples:
      | out state                            | message                                                                         |
      | <tmp>/out holds a file "notes.txt"   | release: <tmp>/out holds notes.txt, which assets did not write; remove it by hand    |
      | <tmp>/out holds a folder "old"       | release: <tmp>/out holds old/, which assets did not write; remove it by hand         |
      | <tmp>/out holds a file "FakeFlac.exe" | release: <tmp>/out holds FakeFlac.exe, which assets did not write; remove it by hand |
      | <tmp>/out is a file                  | release: <tmp>/out is not a folder                                              |

  Scenario: A write failure after the checks leaves nothing of the run behind
    Steps 1 to 4 pass; writing the notes (step 7) fails because the folder
    that should hold them is a file. The installers already copied and
    SHA256SUMS.txt are removed, and so is the output folder this run created.
    Given the test root with manifests at "0.1.0" and the bundle of "The assets of a release"
    And <tmp>/out does not exist
    And <tmp>/afile is a regular file holding "x"
    When I run assets with --notes <tmp>/afile/notes.md
    Then the exit code is 1
    And stderr has exactly one line, and it starts with "release: cannot write <tmp>/afile"
    And stdout has no line starting with "release: wrote"
    And <tmp>/out does not exist
    And <tmp>/afile is still a regular file holding "x"
    And the bundle is unchanged

  Scenario: assets with manifests that disagree writes nothing
    Given the test root with manifests at "0.1.0"
    And the version of Cargo.toml [workspace.package] is "0.2.0"
    And the bundle of "The assets of a release"
    When I run assets
    Then the exit code is 1
    And stderr is exactly:
      """
      release: the manifests do not agree on the version
      release:   package.json: 0.1.0
      release:   package-lock.json: 0.1.0
      release:   package-lock.json packages[""]: 0.1.0
      release:   src-tauri/tauri.conf.json: 0.1.0
      release:   Cargo.toml [workspace.package]: 0.2.0
      release:   Cargo.lock fakeflac: 0.1.0
      release:   Cargo.lock fakeflac-core: 0.1.0
      """
    And stdout is empty
    And <tmp>/out and <tmp>/notes.md do not exist

  @RNF-09
  Scenario: Paths with spaces and Unicode characters
    The release analogue of RNF-09: the script never builds a shell
    command line, prints paths as given, and the checksums file holds file
    names only.
    Given the test root, the bundle and the output folder are inside a temporary folder named "Rélease ″x″ 🎵 dir"
    And the test root has manifests at "0.1.0" and the bundle holds "nsis/<nsis>" containing "abc" and "msi/<msi>" containing "abcd"
    When I run assets
    Then the exit code is 0
    And SHA256SUMS.txt is byte-identical to the one of "The assets of a release"
    And stdout has the line "release: wrote <tmp>/out (2 installers, SHA256SUMS.txt) and <tmp>/notes.md", with the folder name "Rélease ″x″ 🎵 dir" in it unchanged

  # --- Arguments (release.md 3.0) -------------------------------------------

  Scenario Outline: Usage errors
    "(none)" means no argument at all, and "''" in a command stands for one
    empty argument (the empty string). No test root is needed: nothing is read.
    When I run "<command>"
    Then the exit code is 2
    And stderr is "<message>"

    Examples:
      | command                                                        | message                                                                                                          |
      | (none)                                                         | release: usage: node scripts/release.mjs <command> [arguments]; commands: version, release-check, assets, draft, set-version |
      | publish                                                        | release: unknown command "publish"; commands: version, release-check, assets, draft, set-version                 |
      | version --verbose                                              | release: unknown option --verbose for version                                                                    |
      | version --root=x                                               | release: unknown option --root=x for version                                                                     |
      | version --print                                                | release: unknown option --print for version                                                                      |
      | version extra                                                  | release: unexpected argument "extra" for version                                                                 |
      | version --root a --root b                                      | release: option --root is given more than once                                                                   |
      | version --root                                                 | release: option --root needs a value                                                                             |
      | version --root ''                                              | release: option --root has an empty value                                                                        |
      | assets --bundle b --out '' --notes n.md --max-bytes 1000       | release: option --out has an empty value                                                                         |
      | assets --bundle b --out o --notes n.md --max-bytes ''          | release: option --max-bytes has an empty value                                                                   |      | assets --bundle --out o --notes n.md --max-bytes 1000          | release: option --bundle needs a value                                                                           |
      | assets --bundle b --out o --notes n.md                         | release: assets needs --max-bytes                                                                                |
      | assets --bundle b --out o --notes o/n.md --max-bytes 1000      | release: --notes must not be inside --out                                                                        |
      | assets --bundle b --out o --notes o --max-bytes 1000           | release: --notes must not be inside --out                                                                        |
      | assets --bundle b --out o --notes n.md --max-bytes 0           | release: --max-bytes must be a whole number from 1 to 9007199254740991, not "0"                                  |
      | assets --bundle b --out o --notes n.md --max-bytes 20MB        | release: --max-bytes must be a whole number from 1 to 9007199254740991, not "20MB"                               |
      | assets --bundle b --out o --notes n.md --max-bytes 01000       | release: --max-bytes must be a whole number from 1 to 9007199254740991, not "01000"                              |
      | assets --bundle b --out o --notes n.md --max-bytes 1.5         | release: --max-bytes must be a whole number from 1 to 9007199254740991, not "1.5"                                |
      | assets --bundle b --out o --notes n.md --max-bytes -5          | release: --max-bytes must be a whole number from 1 to 9007199254740991, not "-5"                                 |
      | assets --bundle b --out o --notes n.md --max-bytes 9007199254740992 | release: --max-bytes must be a whole number from 1 to 9007199254740991, not "9007199254740992"              |
      | draft --assets a                                               | release: draft needs --notes                                                                                     |
      | draft --assets a --notes n.md --print --print                  | release: option --print is given more than once                                                                  |
      | set-version                                                    | release: set-version needs a version such as 1.2.3                                                               |
      | set-version 1.2.3 4.5.6                                        | release: unexpected argument "4.5.6" for set-version                                                             |
      | release-check v0.1.0 v0.2.0                                    | release: unexpected argument "v0.2.0" for release-check                                                          |
      | release-check                                                  | release: release-check needs a tag such as v1.2.3                                                                |

  Scenario Outline: Accepted --max-bytes values
    When the arguments "assets --bundle b --out o --notes n.md --max-bytes <value>" are parsed
    Then there is no usage error and --max-bytes is <value>

    Examples:
      | value            |
      | 1                |
      | 1000             |
      | 20971520         |
      | 9007199254740991 |

  Scenario Outline: An unknown option takes no value
    The argument after an unknown option is read on its own. --print is a
    flag of draft only; for every other command it is an unknown option.
    When I run "<command>"
    Then the exit code is 2
    And stderr is exactly the two lines "<first>" and "<second>"

    Examples:
      | command                                                         | first                                         | second                                          |
      | version --verbose x                                             | release: unknown option --verbose for version | release: unexpected argument "x" for version    |
      | assets --bundle b --out o --notes n.md --max-bytes 1000 --print x | release: unknown option --print for assets  | release: unexpected argument "x" for assets     |
      | set-version 1.2.3 --root=r x                                    | release: unknown option --root=r for set-version | release: unexpected argument "x" for set-version |

  Scenario: --print of draft is a flag
    When the arguments "draft --print --assets a --notes n.md" are parsed
    Then there is no usage error, --print is set, --assets is "a" and --notes is "n.md"

  Scenario: Every usage error is reported
    When I run "assets --out o --print"
    Then the exit code is 2
    And stderr is exactly:
      """
      release: unknown option --print for assets
      release: assets needs --bundle
      release: assets needs --notes
      release: assets needs --max-bytes
      """

  Scenario: Usage errors decided before anything is read
    A usage error (exit 2) wins over every problem that reading would find (exit 1).
    Given an empty temporary folder <empty> (no manifests at all)
    When I run "assets --root <empty> --bundle <empty>/missing --out <empty>/out --notes <empty>/n.md --max-bytes 0"
    Then the exit code is 2
    And stderr is "release: --max-bytes must be a whole number from 1 to 9007199254740991, not "0""
    And <empty> is still empty

  # --- Installer size (RNF-06, release.md 3.3 step 3) ----------------------

  @RNF-06
  Scenario Outline: Each installer must be strictly under the limit
    "The NSIS size line" is "release: FakeFlac_0.1.0_x64-setup.exe is <nsis bytes> bytes, not under the limit of 1000 bytes (RNF-06)"
    and "the MSI size line" is "release: FakeFlac_0.1.0_x64_en-US.msi is <msi bytes> bytes, not under the limit of 1000 bytes (RNF-06)".
    Given the test root with manifests at "0.1.0"
    And the bundle holds "nsis/<nsis>" of <nsis bytes> bytes and "msi/<msi>" of <msi bytes> bytes
    When I run assets with --max-bytes 1000
    Then the exit code is <code>
    And stdout has the line "release: FakeFlac_0.1.0_x64-setup.exe <nsis bytes> bytes (0.00 MiB), limit 1000 bytes"
    And stdout has the line "release: FakeFlac_0.1.0_x64_en-US.msi <msi bytes> bytes (0.00 MiB), limit 1000 bytes"
    And stderr is exactly <failure>

    Examples:
      | nsis bytes | msi bytes | code | failure                                     |
      | 999        | 999       | 0    | nothing (stderr is empty)                   |
      | 1000       | 999       | 1    | the NSIS size line                          |
      | 999        | 1001      | 1    | the MSI size line                           |
      | 1000       | 1000      | 1    | the NSIS size line, then the MSI size line  |

  @RNF-06
  Scenario Outline: A lone MSI is still measured, and the NSIS installer is reported missing
    Given the test root with manifests at "0.1.0"
    And the bundle holds only "msi/<msi>" of <bytes> bytes
    When I run assets with --max-bytes 1000
    Then the exit code is 1
    And stdout is exactly:
      """
      release: FakeFlac_0.1.0_x64_en-US.msi <bytes> bytes (0.00 MiB), limit 1000 bytes
      """
    And stderr is "release: missing installer <bundle>/nsis/FakeFlac_0.1.0_x64-setup.exe"

    Examples:
      | bytes |
      | 0     |
      | 999   |

  @RNF-06
  Scenario: An installer over the limit writes nothing
    Given the test root with manifests at "0.1.0" and <tmp>/out and <tmp>/notes.md from an earlier successful run
    And the bundle now holds an MSI of 1001 bytes
    When I run assets with --max-bytes 1000
    Then the exit code is 1
    And stderr is "release: FakeFlac_0.1.0_x64_en-US.msi is 1001 bytes, not under the limit of 1000 bytes (RNF-06)"
    And <tmp>/out and <tmp>/notes.md are byte-identical to before

  @RNF-06
  Scenario Outline: The 20 MiB limit of the recipe
    The figure is the one the release-assets recipe passes (see "Wiring").
    Given the test root with manifests at "0.1.0"
    And the bundle holds an NSIS installer of <bytes> bytes and an MSI of 1000 bytes
    When I run assets with --max-bytes 20971520
    Then the exit code is <code>
    And stdout has the line "release: FakeFlac_0.1.0_x64-setup.exe <bytes> bytes (<mib> MiB), limit 20971520 bytes"

    Examples:
      | bytes    | code | mib   |
      | 20971519 | 0    | 20.00 |
      | 20971520 | 1    | 20.00 |

  # --- Draft release (release.md 3.6) ---------------------------------------

  Scenario: The draft command
    Given the assets of a release have been written to <tmp>/out with notes <tmp>/notes.md
    When I run "draft --assets <tmp>/out --notes <tmp>/notes.md --print"
    Then the exit code is 0
    And stdout is one line, the JSON.stringify of the array below, where the item after --notes-file is the --notes argument verbatim, exactly the string passed on the command line (no "\" to "/" replacement, unlike <tmp> in messages), and the last three items are path.join(<assets argument>, name) with the platform's separator:
      | release | create | v0.1.0 | --draft | --verify-tag | --title | FakeFlac 0.1.0 | --generate-notes | --notes-file | <tmp>/notes.md | <tmp>/out/<nsis> | <tmp>/out/<msi> | <tmp>/out/SHA256SUMS.txt |

  Scenario: The --notes item of the draft command is never rewritten
    Here <tmp> is the temporary folder exactly as the test passes it, with no
    "\" to "/" replacement.
    Given the assets of a release have been written
    When I run draft --print with --assets <tmp>/out and --notes "<tmp>/./notes.md"
    Then the exit code is 0
    And the item after "--notes-file" in the printed array is exactly "<tmp>/./notes.md" (not normalised)
    When, on Windows only, I run draft --print with --assets <tmp>/out and --notes "<tmp>\.\notes.md"
    Then the exit code is 0
    And the item after "--notes-file" in the printed array is exactly "<tmp>\.\notes.md", backslashes included

  Scenario: The draft never runs outside GitHub Actions
    The isolated environment has no GITHUB_ACTIONS. PATH is also replaced by an
    empty temporary folder, so that even a faulty script could not start a real gh.
    Given the assets of a release have been written
    And the environment variable PATH (whatever its case) set to an empty temporary folder
    When I run "draft --assets <tmp>/out --notes <tmp>/notes.md" without --print
    Then the exit code is 1
    And stderr is "release: draft runs only in GitHub Actions (use --print to see the command)"
    And stdout is empty

  Scenario Outline: The draft checks what it uploads
    Given the assets of a release have been written
    And <change>
    When I run "draft --assets <tmp>/out --notes <tmp>/notes.md --print"
    Then the exit code is 1
    And stderr is "<message>"
    And stdout is empty

    Examples:
      | change                                                                 | message                                                                          |
      | one byte of <tmp>/out/<msi> is changed                                 | release: SHA256SUMS.txt does not match FakeFlac_0.1.0_x64_en-US.msi             |
      | an LF is appended to <tmp>/out/SHA256SUMS.txt                          | release: SHA256SUMS.txt is not exactly the checksums of the two installers       |
      | <tmp>/out/<nsis> is deleted                                            | release: <tmp>/out has no FakeFlac_0.1.0_x64-setup.exe                          |
      | <tmp>/out also holds "extra.txt"                                       | release: <tmp>/out holds an unexpected extra.txt                                 |
      | <tmp>/out also holds an empty folder "sub"                             | release: <tmp>/out holds an unexpected sub/                                      |
      | <tmp>/notes.md is deleted                                              | release: missing <tmp>/notes.md                                                  |
      | notes.md starts with "FakeFlac 0.0.9 "                                 | release: <tmp>/notes.md is not the release notes of 0.1.0                        |
      | the environment has GITHUB_REF_TYPE "tag" and GITHUB_REF_NAME "v0.1.1" | release: the workflow's tag "v0.1.1" does not match the manifests' version 0.1.0 |

  Scenario: draft with manifests that disagree runs nothing
    Given the assets of a release have been written
    And the version of package.json is now "0.2.0"
    When I run "draft --assets <tmp>/out --notes <tmp>/notes.md --print"
    Then the exit code is 1
    And stderr is exactly:
      """
      release: the manifests do not agree on the version
      release:   package.json: 0.2.0
      release:   package-lock.json: 0.1.0
      release:   package-lock.json packages[""]: 0.1.0
      release:   src-tauri/tauri.conf.json: 0.1.0
      release:   Cargo.toml [workspace.package]: 0.1.0
      release:   Cargo.lock fakeflac: 0.1.0
      release:   Cargo.lock fakeflac-core: 0.1.0
      """
    And stdout is empty (no argument array: nothing would be run)
    And <tmp>/out and <tmp>/notes.md are byte-identical to before

  Scenario: A draft for another version lists every problem
    Given the assets of a release have been written
    And the manifests are now at "0.2.0" (set-version)
    When I run "draft --assets <tmp>/out --notes <tmp>/notes.md --print"
    Then the exit code is 1
    And stderr is exactly:
      """
      release: <tmp>/out has no FakeFlac_0.2.0_x64-setup.exe
      release: <tmp>/out has no FakeFlac_0.2.0_x64_en-US.msi
      release: <tmp>/out holds an unexpected FakeFlac_0.1.0_x64-setup.exe
      release: <tmp>/out holds an unexpected FakeFlac_0.1.0_x64_en-US.msi
      release: <tmp>/notes.md is not the release notes of 0.2.0
      """

  # --- Version bump (release.md 3.7) ----------------------------------------

  Scenario: set-version rewrites only the version strings
    Given the test root with manifests at "0.1.0", Cargo.toml with CRLF line endings, tauri.conf.json starting with a UTF-8 BOM, and package.json without a final newline
    When I run "set-version 0.2.0"
    Then the exit code is 0
    And stdout is "release: set version 0.2.0 in package.json, package-lock.json, src-tauri/tauri.conf.json, Cargo.toml, Cargo.lock"
    And running "version" then gives "release: version 0.2.0 (7 sources agree)"
    And each manifest file equals its text before with exactly its located version spans replaced by "0.2.0" (package.json 1, package-lock.json 2, tauri.conf.json 1, Cargo.toml 1, Cargo.lock 2) and no other byte changed
    And tauri.conf.json still starts with its BOM and still holds the text "\"targets\": [\"nsis\", \"msi\"]" written inline
    And all eight decoys still hold "0.1.0": package.json config.version and the description, the package-lock.json dependency "decoy", tauri.conf.json bundle.version, the Cargo.toml [workspace.metadata.decoy] line, the [workspace.dependencies] entry foo and the comment line, and the Cargo.lock registry entry "decoy"
    And the Cargo.lock line version = 4 is unchanged
    And the Cargo.lock entry "serde" still has version "1.0.228"

  Scenario: set-version also aligns manifests that disagreed, and lists only the files it changed
    Given the test root with manifests at "0.2.0" except Cargo.toml [workspace.package] at "0.0.9"
    When I run "set-version 0.2.0"
    Then the exit code is 0
    And stdout is "release: set version 0.2.0 in Cargo.toml"
    And every manifest file other than Cargo.toml is byte-identical to before
    And running "version" exits 0 with version 0.2.0

  Scenario: set-version with nothing to change writes nothing
    Given the test root with manifests at "0.2.0"
    When I run "set-version 0.2.0"
    Then the exit code is 0
    And stdout is "release: nothing to change, already 0.2.0"
    And every manifest file is byte-identical to before, with the same modification time

  Scenario Outline: set-version refuses a version that cannot be released
    Given the test root with manifests at "0.1.0"
    When I run "set-version <version>"
    Then the exit code is 1
    And stderr is "<message>"
    And every manifest file is byte-identical to before

    Examples:
      | version      | message                                                                                                     |
      | 1.0.0-beta.1 | release: version "1.0.0-beta.1" has a pre-release or build part; v1 ships no pre-releases (MAJOR.MINOR.PATCH only) |
      | 1.0          | release: version "1.0" is not MAJOR.MINOR.PATCH (numbers without leading zeros)                             |
      | 256.0.0      | release: version "256.0.0" is outside the MSI range (MAJOR and MINOR at most 255, PATCH at most 65535)      |

  Scenario Outline: set-version writes nothing when a manifest is broken
    Given the test root with manifests at "0.1.0"
    And <damage>
    When I run "set-version 0.2.0"
    Then the exit code is 1
    And stderr is "<message>"
    And every manifest file is byte-identical to before

    Examples:
      | damage                                                            | message                                             |
      | package.json holds "{not json"                                    | release: package.json is not valid JSON             |
      | package.json has a second top-level "version" key holding "0.1.0" | release: package.json has a duplicate "version" key |

  # --- Wiring (static checks of repository files, release.md 5.3) ------------

  @RNF-06
  Scenario: The size gate runs on every PR and every release with 20 MiB
    When the justfile is read
    Then the recipe "release-assets" runs exactly "node scripts/release.mjs assets --bundle target/release/bundle --out target/release-staging/assets --notes target/release-staging/release-notes.md --max-bytes 20971520"
    And "20971520" appears exactly once in the justfile
    And the "build-windows" job of .github/workflows/ci.yml runs "just build" and "just release-assets" and uploads "target/release-staging/"
    And the "package" job of .github/workflows/release.yml runs "just build" and "just release-assets"
    And neither workflow contains "Get-FileHash", "20MB" or "npm run tauri build"

  @RNF-06
  Scenario: The bundle keeps the small WebView2 bootstrapper
    When src-tauri/tauri.conf.json is read as JSON
    Then bundle.targets is exactly ["nsis", "msi"]
    And bundle.windows.webviewInstallMode.type is "downloadBootstrapper"

  Scenario: The recipes and the CI version check
    When the justfile is read
    Then it has the recipes "version-check", "release-check", "release-assets", "release-draft" and "set-version" with exactly the commands of release.md section 4
    And the "ci" line is exactly "ci: check-fast test trace guard-tests version-check check-app"
    And the "specs" job of .github/workflows/ci.yml runs "just version-check"

  Scenario: The release workflow verifies, packages and only publishes from a tag
    When .github/workflows/release.yml is read as text, split into its workflow level and its jobs as release.md 5.3 says
    Then it is triggered by tags "v*.*.*" and by "workflow_dispatch"
    And the workflow level contains "contents: read" and no ": write"
    And the job "verify" contains "fetch-depth: 0", "components: \"rustfmt, clippy\"", "just release-check", "just version-check" and "just ci"
    And the job "package" contains "needs: verify", "just build", "just release-assets" and "target/release-staging/", and no ": write"
    And the job "publish" contains "needs: package", "contents: write", "id-token: write", "attestations: write", "actions/attest-build-provenance", "target/release-staging" and "just release-draft", and neither "tauri build" nor "just build"
    And "contents: write", "id-token: write" and "attestations: write" appear in no other job
    And every line containing "run:" matches "^\s*(- )?run: (just [a-z-]+|npm ci)\s*$"
    And it contains no "${{ github.ref_name }}", no "Get-FileHash", no "ConvertFrom-Json", no "20MB", no "npm run tauri build" and no "gh release"

  Scenario: The dry run never publishes
    A workflow_dispatch run has github.event_name "workflow_dispatch", so this
    expression is false and "publish" is skipped: no attestation, no release.
    When .github/workflows/release.yml is read as text
    Then the job "publish" has the line "    if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v')" exactly
    And no other job has an "if:" line at job level

  # --- Agents never release (Q11, release.md 8.1; tested in .claude/hooks/hooks.test.mjs) ---

  Scenario Outline: Agents cannot push a release tag or create a release
    When guard-bash receives the command "<command>"
    Then it exits <code>

    Examples:
      | command                                     | code |
      | git push origin v0.1.0                      | 2    |
      | git push --delete origin v0.1.0             | 2    |
      | git push origin :v0.1.0                     | 2    |
      | git push origin +v0.1.0                     | 2    |
      | git push origin refs/tags/v0.1.0            | 2    |
      | git push --tags                             | 2    |
      | git push --follow-tags origin feat/RNF-11   | 2    |
      | gh release create v0.1.0                    | 2    |
      | gh release edit v0.1.0 --draft=false        | 2    |
      | gh release upload v0.1.0 SHA256SUMS.txt     | 2    |
      | gh release delete v0.1.0                    | 2    |
      | git push -u origin feat/v2-ui               | 0    |
      | git push origin chore/release-0.1.0         | 0    |
      | git push origin feat/RNF-11                 | 0    |
      | git tag --list                              | 0    |
      | gh release view v0.1.0                      | 0    |
      | gh release list                             | 0    |
