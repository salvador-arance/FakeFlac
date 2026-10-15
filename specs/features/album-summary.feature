@RF-10
Feature: Album summary verdict
  As a user reviewing a folder of music
  I want one verdict per album
  So that I can tell at a glance whether the album has fake tracks

  Spec: specs/detection/album-summary.md

  Scenario: All tracks are real
    Given a folder "Album A" with tracks whose verdicts are "real, real, real"
    When the album summary is computed
    Then the summary verdict is "real"
    And the counts are total 3, real 3, suspicious 0, fake 0, errors 0

  Scenario: One fake track makes the album fake
    Given a folder "Album A" with tracks whose verdicts are "real, fake, real"
    When the album summary is computed
    Then the summary verdict is "fake"
    And the counts are total 3, real 2, suspicious 0, fake 1, errors 0

  Scenario: Suspicious without fakes makes the album suspicious
    Given a folder "Album A" with tracks whose verdicts are "real, suspicious, real"
    When the album summary is computed
    Then the summary verdict is "suspicious"

  Scenario: Fake wins over suspicious
    Given a folder "Album A" with tracks whose verdicts are "suspicious, fake, suspicious"
    When the album summary is computed
    Then the summary verdict is "fake"

  Scenario: Errors are counted but do not decide the verdict
    Given a folder "Album A" with tracks whose verdicts are "real, error, real"
    When the album summary is computed
    Then the summary verdict is "real"
    And the counts are total 3, real 2, suspicious 0, fake 0, errors 1

  Scenario: Only errors
    Given a folder "Album A" with tracks whose verdicts are "error, error"
    When the album summary is computed
    Then the summary verdict is "error"
    And the counts are total 2, real 0, suspicious 0, fake 0, errors 2

  Scenario: Empty folder
    Given a folder "Album A" with no tracks
    When the album summary is computed
    Then there is no summary

  Scenario: Order does not matter
    Given a folder "Album A" with tracks whose verdicts are "fake, real, suspicious"
    And the same tracks in the order "suspicious, fake, real"
    When the album summary is computed for both orders
    Then both summaries are equal

  Scenario: Tracks from another folder are ignored
    Given a folder "Album A" with tracks whose verdicts are "real, real"
    And a track from folder "Album B" whose verdict is "fake"
    When the album summary of "Album A" is computed with all three tracks
    Then the summary verdict is "real"
    And the counts are total 2, real 2, suspicious 0, fake 0, errors 0

  Scenario: Unicode folder names
    Given a folder "Álbum ″Ñandú″" with tracks whose verdicts are "real, suspicious"
    When the album summary is computed
    Then the summary folder is "Álbum ″Ñandú″"
    And the summary verdict is "suspicious"
