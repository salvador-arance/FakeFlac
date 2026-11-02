---
id: SPEC-WINDOWS-PLATFORM
requirements: [RNF-07, RNF-04, RNF-09]
status: proposed
---

# Windows platform acceptance (RNF-07)

RNF-07 supports Windows 10 and Windows 11 on x64. This proposal makes the
existing platform commitment verifiable. It adds no command, event, DTO, installer
format or dependency. `commands.md` remains version 8; release packaging and
WebView2 provisioning remain those of `release.md` (D1–D4, M3, M5, M6).
Scenarios: `../features/windows-platform.feature`.

## Acceptance

- **P1, automated evidence:** the candidate passes `just check`, `just check-app`
  and `just build` on Windows x64. Both NSIS and MSI artifacts are available and
  pass the existing `just release-assets` checks. Existing release tests check
  packaging configuration; a successful build alone does not establish runtime
  compatibility on either client OS.
- **P2, manual compatibility matrix:** test the same candidate's NSIS and MSI
  separately on Windows 10 x64 and Windows 11 x64 (four rows). Start each row
  without an installed FakeFlac, install, launch from Start, analyse the small
  fixture folder below, close the app and uninstall. Every step must succeed.
  This reuses release M3/M5/M6; it does not require publishing a release or a tag.
- **P3, runtime result:** use a folder containing an ordinary valid WAV or FLAC,
  a deliberately corrupt supported audio file, and a valid file with a Unicode
  name. Each valid file completes with a non-error verdict (`real`, `suspicious`
  or `fake`) and no track error. The corrupt file produces an Error row and the
  run finishes without aborting. Next, attempt a folder with no supported audio:
  the status bar shows the localized `NO_AUDIO_FOUND` message and keeps the
  previous rows, selection and albums; Export CSV stays enabled for those rows.
  This is the existing failed-start behavior in `../ui/export-csv.md` section 3,
  `../ui/main-screen.md` (Error state) and `../ui/album-summary.md` (Error state).
  Also try the empty folder before any successful analysis in a fresh app
  session: the same message is shown, the table has no rows and Export CSV is
  disabled. Record each outcome; detection accuracy remains covered by its own
  specs.
- **P4, evidence:** record date, OS edition/build/x64, app version and source
  commit, artifact name and SHA-256, WebView2 version, installer type and each
  result. Record failures explicitly. Never commit personal audio names or
  paths; use synthetic fixtures and aggregate results. Existing release checks
  of audio immutability remain applicable.

The Windows machines may be VMs or Sandbox instances. WebView2 must be available;
the existing bootstrapper may download it during installation. Record that fact
when applicable. This acceptance does not promise an offline first installation
without WebView2, ARM64, 32-bit Windows or other operating systems. No additional
minimum Windows build is introduced by this proposal; the recorded builds state
exactly what was exercised.

## Evidence at proposal time (2026-11-02)

`ci.yml` builds and tests on `windows-latest`, and `tauri.conf.json` bundles NSIS
and MSI. `scripts/release.test.mjs` covers the release wiring. These facts are
supporting evidence, not a Windows 10/11 runtime matrix.

Local candidate built from production source commit `216c945` (version 0.1.0,
Windows 11 Pro x64): `just check`, `just check-app`, `just build` and
`just release-assets` passed on 2026-11-02. NSIS is 2,877,203 bytes; MSI is
4,661,248 bytes. This local build is distinct from the published v0.1.0 assets.
Its installer SHA-256 values are:

- NSIS: `d4c6d641c53af618a6fadbffb8f743e7495e5edb5ba2127b3c0bf06707f72bd3`.
- MSI: `fc465952ec11b51423623dd2ed49bc197b9b3ebfe4873f466fa8a9696d23a351`.

The local test pack under ignored `target/rnf-07-test-pack.zip` contains these
installers, checksums, two valid synthetic WAVs (one Unicode filename), a corrupt
FLAC and an empty folder. P2–P4 results must identify these hashes if this
candidate is used. The pack is a local convenience, not a published release.

The 2026-10-31 progress block records v0.1.0 release M3–M6 passing in Windows
Sandbox; `docs/planning/comprobaciones-manuales.md` records those steps too.
Neither identifies both client OS builds, so it cannot fill both OS rows.
P2–P4 for the candidate selected for RNF-07 remain pending until recorded:

On 2026-11-02 the owner reported completing the checks, but observed that the
empty-folder attempt after an analysis retained the previous rows and did not
show the no-supported-audio message. Retaining rows matches the approved
failed-start behavior; the original P3 and test-pack instruction saying "no
rows" in that situation were incorrect. A subsequent native UIA inspection and
screenshot of the same installed candidate confirmed that the status bar does
show "No hay archivos de audio compatibles en esta carpeta" while retaining
the rows. Thus the reported missing message has not been reproduced; this
observation supports the corrected empty-folder acceptance on that installation.
The first data row overlapping Spanish column headers was reproduced in the
native app (RNF-08 U3).
The report does not identify a result and OS build for each installer/OS pair,
so it does not establish a passed four-row matrix. No complete row is marked
passed from this report alone.

| OS | Installer | Install / Start launch / P3 / uninstall | Candidate and OS evidence |
|---|---|---|---|
| Windows 10 x64 | NSIS | Pending | Pending |
| Windows 10 x64 | MSI | Pending | Pending |
| Windows 11 x64 | NSIS | Pending | Pending |
| Windows 11 x64 | MSI | Pending | Pending |

RNF-07 can be recorded as verified only after G1 approval, P1 passing and all
four matrix rows passing with P4 evidence. A failed row leaves it pending.

## G1 decision

**P-Q1:** approve this four-row matrix and the small P3 smoke check as RNF-07's
closure criterion? Recommended: yes, reusing the released installer formats and
the existing analysis/error behavior. Adding a CI launch smoke test is deferred;
it would not replace either client OS run.
