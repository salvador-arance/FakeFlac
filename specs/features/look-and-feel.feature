@RNF-10
Feature: Look and feel of a Windows desktop tool like the reference
  Spec: specs/ui/look-and-feel.md (element map L1 to L7, asset register, checks A1 and A2,
  acceptance K1 to K3, manual check M1, G1 questions Q1 to Q7).
  Only what can be automated and is already specified elsewhere is here: the regions of the
  main screen (main-screen.md, album-summary.md), the toolbar's icon buttons (components.md),
  the single window (main-screen.md, tauri.conf.json) and the asset register (look-and-feel.md
  section 2). How the app looks next to docs/reference/ is the manual check M1, which is the
  acceptance for the visual side of every element (last scenario).
  Not repeated here: the region order with stub content (MainLayout tests), the Albums panel
  states (album-summary-app.feature), the table, sort and widths (sort-filter.feature,
  resizable-widths.feature) and verdict readability (verdict-usability.feature).

  @RF-06 @RF-07 @RF-10 @RNF-04 @RNF-09
  Scenario: A finished run shows the regions the requirement names
    Given an analysis finished with a Genuine row, a Fake row and an Error row for a corrupt file named "Ñandú – 夜.flac"
    And the Fake row is selected
    When I look at the main screen
    Then from top to bottom it shows the title bar "FakeFlac" with the buttons "Minimize", "Maximize" and "Close", the toolbar named "Main toolbar", the content area and the status bar
    And the content area shows the Albums panel named "Album summaries" on the left, and on its right the results table, the splitter and the detail panel, top to bottom
    And the results table shows the three rows, the Error row with the name "Ñandú – 夜.flac" exactly
    And the detail panel shows the spectrogram of the selected row

  Scenario: The empty state shows the window frame without the content regions
    Given the application has just started
    When I look at the main screen
    Then it shows the title bar "FakeFlac", the toolbar named "Main toolbar", the drop hint and the status bar "Ready"
    And it shows no Albums panel, no results table and no detail panel

  Scenario: Every toolbar command is an icon button with its label as tooltip
    Given the toolbar in English with every button enabled
    Then every button of the toolbar outside the search field shows an icon and no visible text
    And the tooltip of each of those buttons is its accessible name

  Scenario: The application declares one window without native decorations
    Given the file src-tauri/tauri.conf.json
    Then app.windows holds exactly one window, titled "FakeFlac", with decorations false, a minimum width of 900 and a minimum height of 500
    And no non-test source file under src/ or src-tauri/src/ builds another window (no WebviewWindowBuilder, WindowBuilder, "new WebviewWindow" or "new Window"; test files, stories and comments are not scanned)

  Scenario: Every shipped graphic file has a recorded origin
    Given the asset register of specs/ui/look-and-feel.md section 2.2
    When I list the files with extension png, ico, icns, svg, jpg, jpeg, gif, webp, bmp, woff, woff2, ttf or otf under src/, src-tauri/ (leaving out src-tauri/target/ and src-tauri/gen/) and public/
    Then the list equals the file rows of the register
    And the IconName values of src/lib/components/atoms/icons.ts equal the icon rows of the register
    And the origin of every row starts with one of the words of the register vocabulary: "own drawing", "derived from own drawing", "redrawn", "third party" or "unrecorded"
    And the number of rows whose origin is "unrecorded" is printed and does not fail the test (the owner closes them at G1: acceptance K2)

  Scenario: No shipped file copies a reference screenshot
    Given the files of docs/reference/ other than app-icon-source.png
    Then no file under src/, src-tauri/ (leaving out src-tauri/target/ and src-tauri/gen/) or public/ has the same SHA-256 (whole file, byte for byte: only a verbatim copy is found) as any of them
    And no import, CSS url(), HTML src or href, or value of src-tauri/tauri.conf.json names a path under docs/reference/ (plain comments are not scanned)

  Scenario: The owner compares the release app with the reference (M1)
    Given a release build installed on Windows 11 x64 and a folder of synthetic fixtures with a Genuine, a Fake and an Error row
    When the owner compares the app with the screenshots of docs/reference/ at 1280 by 720 and 900 by 500
    Then each element E1 to E7 of look-and-feel.md section 4 is recorded as pass, fail or a deliberate difference accepted at G1
    And the record holds the candidate commit, the date and the build, and no screenshot and no music file name or path
