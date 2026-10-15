@RF-01 @RF-02 @RF-05 @RF-06 @RF-14
Feature: Analyse a folder
  As a user who downloads music
  I want to drop a folder and get a verdict for every audio file in it
  So that I know which files are genuine

  Contract: specs/contracts/commands.md
  Detection rules: specs/detection/rules.md

  Background:
    Given the application is open and no analysis is running

  @RF-01
  Scenario: Choose a folder with the dialog
    When I click "Open folder" and pick the folder "Album A"
    Then an analysis of "Album A" starts

  @RF-01
  Scenario: Cancel the folder dialog
    When I click "Open folder" and cancel the dialog
    Then no analysis starts

  @RF-01
  Scenario: Drop a folder on the window
    When I drop the folder "Album A" anywhere on the window
    Then an analysis of "Album A" starts

  @RF-02 @RF-14
  Scenario: Supported files are found recursively
    Given a folder "Library" containing "a.flac", "b.WAV", "notes.txt", "cover.jpg" and a subfolder "CD2" containing "c.flac"
    When I analyse "Library"
    Then 3 files are scanned: "a.flac", "b.WAV" and "CD2/c.flac"

  @RF-02
  Scenario: Hidden files and folders are skipped
    Given a folder "Library" containing "a.flac", ".hidden.flac" and a hidden subfolder ".trash" containing "b.flac"
    When I analyse "Library"
    Then 1 file is scanned: "a.flac"

  @RF-02
  Scenario: Folder without supported audio
    Given a folder "Docs" containing only "notes.txt"
    When I analyse "Docs"
    Then the analysis fails with error "NO_AUDIO_FOUND"
    And the table stays empty

  @RF-05
  Scenario: Progress is reported per track and globally
    Given a folder "Album A" with 4 FLAC files
    When I analyse "Album A"
    Then I first receive a "scanned" event with total 4
    And I receive "trackStarted" and "trackDone" for each of the 4 files
    And the last event is "finished" with done 4
    And the progress bar shows 4 of 4

  @RF-05
  Scenario: Cancel a running analysis
    Given a folder "Big" with 200 FLAC files
    And an analysis of "Big" is running
    When I click "Cancel"
    Then the analysis stops within 500 ms
    And the last event is "cancelled"
    And the tracks already analysed stay in the table

  @RF-05
  Scenario: Only one analysis at a time
    Given an analysis of "Album A" is running
    When I start an analysis of "Album B"
    Then it fails with error "ANALYSIS_RUNNING"

  @RF-06
  Scenario: Results appear in the table as they finish
    Given a folder "Album A" with the synthetic files "full_band.flac" and "lp16k.flac"
    When I analyse "Album A"
    Then the table shows a row for "full_band.flac" with state "Real", format "FLAC", 44100 Hz, 16 bits and its duration
    And the table shows a row for "lp16k.flac" with state "Fake" and cutoff "16.0 kHz"
    And each state is shown with an icon, a text and a colour

  @RNF-04
  Scenario: A corrupt file does not stop the analysis
    Given a folder "Mixed" with "full_band.flac", "corrupt.flac" and "lp16k.flac"
    When I analyse "Mixed"
    Then "corrupt.flac" has state "Error" with the reason "Corrupt stream"
    And "full_band.flac" and "lp16k.flac" are analysed normally
    And the last event is "finished" with errors 1

  @RNF-05
  Scenario: Analysed files are never modified
    Given a folder "Album A" with 2 FLAC files
    When I analyse "Album A"
    Then the size, modification time and content hash of both files are unchanged

  @RNF-09
  Scenario: Unicode names
    Given a folder "Álbum ″Ñandú″" with the file "01 – Canción ‘x’.flac"
    When I analyse "Álbum ″Ñandú″"
    Then the table shows the file name "01 – Canción ‘x’.flac" exactly
