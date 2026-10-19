@RF-07
Feature: Inspect a track's spectrogram
  As a user who doubts a verdict
  I want to see the spectrogram of a track
  So that I can check the verdict with my own eyes

  Contract: specs/contracts/commands.md (get_spectrogram)

  Scenario: Selecting a row shows its spectrogram
    Given an analysis of "Album A" has finished with "lp16k.flac" in the table
    When I select the row "lp16k.flac"
    Then the detail panel shows the spectrogram of "lp16k.flac" within 2 seconds
    And a horizontal line marks the estimated cutoff at 16.0 kHz
    And the panel shows "Cutoff 16.0 kHz · Fake"

  Scenario: A Suspicious track shows its reason in plain words
    Given an analysis has finished with a track whose verdict is "Suspicious" with reason "highBandGaps" and cutoff 17.0 kHz
    When I select that row
    Then the panel shows "Cutoff 17.0 kHz · Suspicious · Frequent gaps above 16 kHz, as in an MP3"

  Scenario: Spectrogram shape
    Given the synthetic file "lp16k.flac" of 30 s at 44.1 kHz
    When its spectrogram is requested
    Then it has height 512 and a width of at most 1200
    And maxFrequencyHz is 22050
    And the rows above 16.2 kHz are near the bottom of dbRange

  Scenario: Changing the selection
    Given the spectrogram of "a.flac" is shown
    When I select "b.flac"
    Then the spectrogram of "b.flac" replaces it
    And a late response for "a.flac" is discarded

  Scenario: Track with an error
    Given "corrupt.flac" has state "Error"
    When I select it
    Then the detail panel shows the error reason instead of a spectrogram

  Scenario: The spectrogram cannot be read
    Given an analysis has finished with "lp16k.flac" in the table
    And "lp16k.flac" was deleted after the analysis
    When I select the row "lp16k.flac"
    Then the panel shows "Cutoff 16.0 kHz · Fake"
    And the panel shows "Cannot show the spectrogram of this file" instead of a spectrogram

  Scenario: Keyboard navigation
    Given the table has focus and a row is selected
    When I press the down arrow
    Then the next row is selected and its spectrogram is shown
