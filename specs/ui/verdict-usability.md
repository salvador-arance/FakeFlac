---
id: SPEC-VERDICT-USABILITY
requirements: [RNF-08, RF-06, RF-07, RF-13, RNF-04, RNF-09]
status: approved
---

# Verdict usability acceptance (RNF-08)

RNF-08 asks that a verdict be understood without documentation through colour
and clear text. This proposal consolidates `components.md`, `main-screen.md`,
`lossy-formats.md`, `sort-filter.md` and `translations.md`, and defines closure
evidence. It changes no component props, events, atomic levels, command or DTO.
`commands.md` remains version 8. Scenarios: `../features/verdict-usability.feature`.

## Presentation and acceptance

| State | English / Spanish label | Colour family and distinct icon | Meaning for the manual check |
|---|---|---|---|
| `real` | Genuine / Real | Green, check | The analysis finds the file consistent with its quality |
| `suspicious` | Suspicious / Sospechoso | Amber, exclamation | There is a reason for doubt; inspect the explanation |
| `fake` | Fake / Falso | Red, cross | The analysis indicates a lower-quality source |
| `error` | Error / Error | Grey, warning | Analysis failed; this is not a quality verdict |

- **U1, automated:** `VerdictCell` (molecule) presents all four states with a
  visible localized label and distinct icon; colour is never the sole cue. The
  icon is decorative for assistive technology because text carries the meaning.
  `ResultsTable` (organism) preserves those labels and Unicode filenames in both
  languages. The state label is available before selecting a row or hovering.
- **U2, automated:** selecting Suspicious shows its localized reason text in
  `DetailPanel` (organism), and selecting Error shows the friendly error text.
  Reasons also remain available in the existing tooltip. Empty results do not
  show a fictitious quality verdict. The existing RF-13 catalogue defines every
  exact translation; raw enum values are not a substitute for those labels.
- **U3, computed and manual:** apply the existing `components.md` text contrast
  criterion (at least 4.5:1) to verdict labels on normal, hovered and selected
  table rows. Compute contrast from the actual foreground/background colours
  using relative luminance, then confirm the rendered colours and that the whole
  label is readable at 900 × 500 in both languages. A selected or hovered row
  must not lose its label. Keep the existing verdict colour families and distinct
  icons; this proposal does not fix exact replacement hex values.
  Regression-check the already approved table layout (`main-screen.md`,
  `components.md`, `language-settings.md` M2) at 900 × 500 in English and Spanish: when
  rendered, header labels stay within the header area, and at the top of a
  populated table the first data row begins below that area without overlapping
  text. Check an empty table, a short populated table and a table long enough to
  scroll. While scrolling, data text must not paint over the sticky header;
  returning to the top shows the first row fully below it. This makes the
  existing separate header/data layout observable without prescribing CSS.
- **U4, owner walkthrough:** using a candidate with all four states, inspect the
  table and select each row in English and Spanish. Without consulting project
  documentation, the owner explains each label using the meaning in the table
  above (paraphrases count), distinguishes Suspicious from Fake and Error, and
  locates a suspicious reason and an error explanation through the UI. All four
  meanings and both explanations must be understood in each language. Tooltips
  and the detail panel are allowed. Record pass/fail per language and any wording
  that caused confusion. This is owner acceptance, not a study of first-time users.

Synthetic component/page data can exercise the four states and reasons
deterministically for U1/U2. U3/U4 use the rendered candidate UI, with synthetic
audio or a repository-local UI fixture harness; record which was used. Detection
thresholds and ground-truth accuracy are outside this presentation check.

## Existing evidence and identified gap (2026-11-02)

- `src/lib/components/molecules/VerdictCell.test.ts` covers labels, distinct icons,
  decorative icon semantics and reason text.
- `src/lib/components/organisms/ResultsTable.locale.test.ts` covers both languages,
  reasons/errors and Unicode names. `DetailPanel{,.lossy,.streaming,.locale}.test.ts`
  covers explanations. `src/lib/i18n/catalogue.test.ts` checks catalogue coverage.
  These are existing supporting tests; they do not prove U3 or human comprehension.
- RF-08 M2/M3 passed on 2026-10-31, and RF-13 M1/M2 on 2026-11-02 (progress log).
  Those checks support keyboard/layout/language behavior, not the entire U3/U4
  acceptance above.
- **Reproduced U3 layout failure (2026-11-02):** the owner's candidate screenshot
  shows Spanish column headers overlapping the first data row; subsequent native
  app inspection reproduced it. Its cause remains under investigation. The
  layout regression check remains unresolved and must be repeated after a fix
  to the already approved RF-06/RF-13 presentation.
- **Known U3 gap:** `VerdictCell.svelte` gives the label its verdict colour.
  `ResultsTable.svelte` uses `--color-bg` normally, `--color-surface-raised` on
  hover and `--color-row-selected` on selection. Computed from current
  `tokens.css`, these text contrast ratios fail the existing 4.5:1 criterion
  where bold (rounded only for display; the comparison uses unrounded values):

  | Label colour | Normal `#1f1f1f` | Hover `#2e2e2e` | Selected `#4a4a4a` |
  |---|---|---|---|
  | Real `#5fb36b` | 6.39 | 5.27 | **3.44** |
  | Suspicious `#e0b13a` | 8.26 | 6.80 | **4.44** |
  | Fake `#d9534f` | **4.16** | **3.43** | **2.24** |
  | Error `#8a8a8a` | 4.77 | **3.93** | **2.57** |

  Therefore RNF-08 is not verified by the existing tests alone. A focused styling
  fix and regression checks must follow G1 before U3 can pass; no production or
  executable test change is part of this proposal. Existing tests remain protected
  during implementation. New coverage should check contrast for the rendered
  label in all three row states, alongside existing language/semantic coverage.

## Recorded results (2026-11-02)

Candidate: production source `235b07d` (branch `feat/RNF-08`, desktop build via
`npm run tauri build` installed in a Windows 11 26H2 build 26300.9711 x64 virtual
machine). Data: the owner's personal music folder (names not recorded).

| Check | Result |
|---|---|
| U1/U2 automated | Existing tests pass in `just check` (83 files, 1,430 tests) |
| U3 computed | `ResultsTable.contrast.test.ts`: label contrast 4.86-9.46 (4 verdicts x normal/hover/selected, both languages); lowest is Real on selected, 4.86 |
| U3 manual (900 x 500, English and Spanish) | Passed per owner: labels readable, Fake reads as red, layout fine |
| U4 owner walkthrough, English | Passed per owner |
| U4 owner walkthrough, Spanish | Passed per owner |

No confusing wording was reported. Not itemised by the owner: the exact U3 numbers
observed on screen beyond the automated ones.

Record candidate commit, date, UI harness or desktop build, language, U1/U2 test
results, U3 measurements and U4 observations. RNF-08 is verified only after
U1–U4 passing and `just check` green. A failure stays visible as pending.

## G1 decisions

| Decision | Recommendation |
|---|---|
| U-Q1 (approved 2026-11-02): is the explicit owner walkthrough in both languages sufficient for this requirement? | Yes; do not introduce a user-study recruitment or timing requirement. |
| U-Q2 (approved 2026-11-02): must closure include readable verdict text in normal, hovered and selected rows, applying the already approved 4.5:1 threshold? | Yes; fix the measured contrast gap after G1 while preserving labels, distinct icons and colour families. Replacement colours or separating label/icon colours are implementation choices subject to U3. |

No new dependency or architecture decision is proposed.
