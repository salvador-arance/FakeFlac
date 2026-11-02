@RNF-07
Feature: Windows 11 x64 compatibility (closure matrix)
  Acceptance: specs/contracts/windows-platform.md (P1 to P4).
  These scenarios are manual installed-application checks, not CI runner claims.

  Scenario Outline: Install, analyse and uninstall on the Windows 11 closure matrix
    Given a clean "<os>" x64 test machine
    And the candidate "<installer>" installer and its SHA-256 are recorded
    When I install FakeFlac and launch it from Start
    Then the main screen opens and allows choosing a folder
    When I analyse the platform fixture folder
    Then each valid file completes with a real, suspicious or fake verdict and no track error
    And its corrupt file has an Error row without aborting the run
    When I close FakeFlac and uninstall it
    Then the uninstall succeeds
    And I record the OS build, app commit, WebView2 version and each result

    Examples:
      | os         | installer |
      | Windows 11 | NSIS      |
      | Windows 11 | MSI       |

  @RNF-04 @RNF-09
  Scenario: The platform fixture folder exercises errors and Unicode names
    Given each installed candidate in the platform matrix
    And a fixture folder containing a valid WAV or FLAC, a corrupt supported file and a valid file with a Unicode name
    When I analyse that folder
    Then each valid file completes with a real, suspicious or fake verdict and no track error
    And the valid files have their names preserved
    And the corrupt file produces an Error row
    And the analysis finishes

  @RNF-04
  Scenario: An empty-folder attempt after a completed run preserves its results
    Given each installed candidate in the platform matrix has completed the fixture analysis
    And I have selected a result row
    When I attempt to analyse a folder containing no supported audio
    Then the NO_AUDIO_FOUND message is shown in the status bar in the active language
    And the previous rows, selection and albums are kept
    And Export CSV remains enabled for the previous rows

  @RNF-04
  Scenario: An empty-folder attempt before any results leaves the table empty
    Given each installed candidate in the platform matrix is freshly launched with no analysis results
    When I attempt to analyse a folder containing no supported audio
    Then the NO_AUDIO_FOUND message is shown in the status bar in the active language
    And the table has no rows
    And Export CSV is disabled
