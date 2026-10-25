@RF-10
Feature: Album summaries in the application (H2)
  As a user who analyses a folder with several albums
  I want one summary verdict per album while and after the analysis runs
  So that I can tell at a glance which albums have fake or suspicious tracks

  Rule (unchanged, H0): specs/detection/album-summary.md and its scenarios in
  album-summary.feature. This file covers how the application uses it:
  contract specs/contracts/commands.md ("Album summary (version 5)"), UI
  specs/ui/album-summary.md. An album is the set of scanned files with the
  same direct parent folder.

  # --- Use case (fakeflac-core analyze_folder, fake scanner and fake analyzer) --
  # The fake scanner returns the files named, under the root named; the fake
  # analyzer gives each file the verdict named (an "error" file gets
  # corruptStream). "album update" is the `album` field of a trackDone event.
  # Paths use "\" and are absolute under "C:\Music" unless stated. These are
  # Windows paths, so these tests are #[cfg(windows)] (commands.md name rule);
  # CI runs the Rust tests on windows-latest. The test sink only records events.

  Scenario: Every trackDone carries the summary of its album
    Given the root "C:\Music\Album A" with the files "01.flac" real, "02.flac" fake and "03.flac" real
    When the use case runs with 1 worker
    Then each of the 3 trackDone events has an album update with index 0, name "Album A" and files 3
    And the album update of the last trackDone has total 3, real 2, suspicious 0, fake 1, errors 0 and verdict "fake"
    And its summary folder is "C:\Music\Album A", equal to the analysis folder of that event

  Scenario: A folder tree, the root's own tracks and a folder without files of its own
    Given the root "C:\Music\Artist" with these files and verdicts:
      | file                   | verdict    |
      | intro.flac             | real       |
      | Album 1\01.flac        | real       |
      | Album 1\02.flac        | suspicious |
      | Album 1\CD2\01.flac    | fake       |
      | Album 2\01.mp3         | real       |
      | Box\Disc 1\01.flac     | real       |
    When the use case runs
    Then the album updates have exactly these albums:
      | index | name                  | folder                            | files | final verdict |
      | 0     | Artist                | C:\Music\Artist                   | 1     | real          |
      | 1     | Artist\Album 1        | C:\Music\Artist\Album 1           | 2     | suspicious    |
      | 2     | Artist\Album 1\CD2    | C:\Music\Artist\Album 1\CD2       | 1     | fake          |
      | 3     | Artist\Album 2        | C:\Music\Artist\Album 2           | 1     | real          |
      | 4     | Artist\Box\Disc 1     | C:\Music\Artist\Box\Disc 1        | 1     | real          |
    And no album update has the folder "C:\Music\Artist\Box", which holds no file of its own
    And the "fake" of "Album 1\CD2" does not change the verdict of "Album 1"

  Scenario: The album index does not depend on which track finishes first
    Given the root "C:\Music\Lib" with "B\01.flac", "A\01.flac" and "A\02.flac"
    And the fake analyzer finishes "B\01.flac" first, then "A\02.flac", then "A\01.flac"
    When the use case runs with 3 workers
    Then every album update of folder "C:\Music\Lib\A" has index 0
    And every album update of folder "C:\Music\Lib\B" has index 1

  Scenario: Folder order is component by component and case-insensitive
    Given the root "C:\Music\Lib" with "album (deluxe)\01.flac", "Album\02.flac", "Album\CD2\01.flac" and "ALBUM B\01.flac"
    When the use case runs
    Then the album indexes are 0 "Lib\Album", 1 "Lib\Album\CD2", 2 "Lib\album (deluxe)", 3 "Lib\ALBUM B"

  Scenario: Summaries grow one track at a time in event order
    Given the root "C:\Music\Album A" with 4 files whose verdicts, in the order their trackDone is sent, are "real, real, fake, real"
    When the use case runs
    Then the album updates of the 4 trackDone events, in channel order, have totals 1, 2, 3, 4
    And their verdicts are "real", "real", "fake", "fake"

  Scenario Outline: The order holds with parallel workers
    Given the root "C:\Music\Big" with 50 files directly in it and 30 files in its subfolder "CD2", with mixed verdicts
    When the use case runs with <workers> workers
    Then for each album, the n-th trackDone of that album in channel order has an album update with total n
    And every album update equals summarize_album of its folder over the tracks of that album whose trackDone was sent up to and including it
    And the last album update of "Big" has total 50 and files 50, and that of "Big\CD2" total 30 and files 30

    Examples:
      | workers |
      | 1       |
      | 4       |
      | 8       |

  @RNF-04
  Scenario: A folder whose files all fail has an error summary
    Given the root "C:\Music\Lib" with "Broken\01.flac" error, "Broken\02.flac" error and "Good\01.flac" real
    When the use case runs
    Then the last album update of "Lib\Broken" has total 2, errors 2 and verdict "error"
    And the last album update of "Lib\Good" has verdict "real"
    And the run ends with "finished"

  @RNF-04
  Scenario: An album can leave the error verdict when a readable track arrives
    Given the root "C:\Music\Album A" with "01.flac" error and "02.flac" fake, and "01.flac" finishes first
    When the use case runs
    Then the album updates in channel order have verdicts "error" then "fake"
    And the last one has total 2, fake 1, errors 1

  Scenario: A finished run completes every album
    Given the root "C:\Music\Artist" with these files and verdicts:
      | file                   | verdict    |
      | intro.flac             | real       |
      | Album 1\01.flac        | real       |
      | Album 1\02.flac        | suspicious |
      | Album 1\03.flac        | error      |
      | Album 1\CD2\01.flac    | fake       |
      | Album 2\01.mp3         | real       |
      | Album 2\02.mp3         | fake       |
    When the use case runs with 4 workers
    Then the run ends with "finished"
    And for every album, its last album update in channel order has total equal to its files
    And the totals of those last album updates add up to the done count of the AnalysisSummary returned

  Scenario: A cancelled run leaves partial albums
    Given the root "C:\Music\Big" with 12 files in "Album 1" and 12 files in "Album 2", all real
    And the test sink fires the run's cancel token when it records the 3rd trackDone
    When the use case runs with 1 worker
    Then the recorded events have exactly 3 trackDone events, all of files in "Album 1"
    And the last event is "cancelled"
    And the album updates recorded are all of index 0, folder "C:\Music\Big\Album 1", with totals 1, 2, 3 in channel order and files 12
    And no album update has the folder "C:\Music\Big\Album 2"

  Scenario: Other events carry no album data
    Given the root "C:\Music\Album A" with 2 files
    When the use case runs
    Then the "scanned", "trackStarted" and "finished" events are unchanged from version 4
    And the AnalysisSummary returned has the same fields as in version 4

  Scenario Outline: Album name
    These rows are the table of commands.md ("Name rule"), row for row: the
    table is tested here, one case per row. Windows paths: #[cfg(windows)].
    Given the root "<root>" with a file directly in the folder "<folder>"
    When the use case runs
    Then the album update of that folder has the name "<name>"

    Examples:
      | root                    | folder                        | name                 |
      | C:\Music\Artist         | C:\Music\Artist               | Artist               |
      | C:\Music\Artist         | C:\Music\Artist\Album 1       | Artist\Album 1       |
      | C:\Music\Artist\        | C:\Music\Artist\Album 1\CD2   | Artist\Album 1\CD2   |
      | C:\Music\Álbum ″Ñandú″  | C:\Music\Álbum ″Ñandú″        | Álbum ″Ñandú″        |
      | \\?\C:\Music\Album A    | \\?\C:\Music\Album A          | Album A              |
      | \\?\C:\                 | \\?\C:\                       | \\?\C:\              |
      | \\?\C:\                 | \\?\C:\Album A                | \\?\C:\Album A       |
      | \\nas\music\Rock        | \\nas\music\Rock\Album B      | Rock\Album B         |
      | D:\                     | D:\                           | D:\                  |
      | D:\                     | D:\Album A                    | D:\Album A           |
      | \\nas\music             | \\nas\music                   | \\nas\music          |
      | \\nas\music             | \\nas\music\Album B           | \\nas\music\Album B  |

  @RNF-09
  Scenario: Unicode folder names
    Given the root "C:\Music\Álbum ″Ñandú″" with "01 – Canción ‘x’.flac" real and "Disco 2 «Ça va» 🎵\01.flac" suspicious
    When the use case runs
    Then the album names are "Álbum ″Ñandú″" and "Álbum ″Ñandú″\Disco 2 «Ça va» 🎵", code point for code point
    And their summary folders equal the folders of their tracks, code point for code point

  # --- Contract and real run (src-tauri DTO, test folders) ---------------------
  # Audio files are generated inside the test, never taken from the corpus
  # ("<id>.flac": fixture <id> of specs/detection/fixtures.md).

  Scenario: The trackDone event in JSON
    Given a core trackDone event for index 0 of "C:\Music\Album A\01.flac" with verdict "real" and its album update index 0, name "Album A", files 3, total 1, real 1
    When it is converted to the contract type and serialised
    Then its JSON object has the keys "kind", "index", "analysis" and "album"
    And "album" is {"index":0,"name":"Album A","files":3,"summary":{"folder":"C:\\Music\\Album A","total":1,"real":1,"suspicious":0,"fake":0,"errors":0,"verdict":"real"}}

  @RNF-04
  Scenario: A real run with a corrupt file
    Given a test folder "Mixed" holding "full_band.flac", "corrupt.flac" and "lp16k.flac"
    When an analysis of "Mixed" finishes
    Then every trackDone event has an album update with index 0, name "Mixed" and files 3
    And the last album update has total 3, real 1, fake 1, errors 1 and verdict "fake"

  @RNF-09
  Scenario: A real run in a folder with a Unicode name
    Given a test folder "Álbum ″Ñandú″" holding "full_band.flac" and "Disco 2 «Ça va» 🎵\lp16k.flac"
    When an analysis of "Álbum ″Ñandú″" finishes
    Then the album updates have the indexes 0 and 1 with the names "Álbum ″Ñandú″" and "Álbum ″Ñandú″\Disco 2 «Ça va» 🎵", code point for code point
    And each album's summary folder equals the folder of its track, code point for code point
    And the last album updates have the verdicts "real" and "fake"

  @RNF-09
  Scenario: Folders whose names are not valid Unicode stay separate albums
    Windows only (#[cfg(windows)]): a src-tauri test that runs the core use case
    with fakes of the ports and converts its events to the contract types. The
    two folder names are built from UTF-16 with OsString::from_wide.
    Given the root "C:\Music\Lib" with "01.flac" real in the folder "Disc " followed by the unpaired surrogate U+D800
    And "01.flac" fake in the folder "Disc " followed by the unpaired surrogate U+DC00
    When the use case runs and its events are converted to the contract types
    Then there are album updates with exactly two indexes, 0 and 1
    And both names are "Lib\Disc �" and both summary folders are "C:\Music\Lib\Disc �", equal strings
    And the album with one index has the verdict "real" and the other "fake", each with files 1 and total 1

  Scenario: The CSV export does not include album summaries
    Given a test folder "Lib" holding "Album 1\full_band.flac" and "Album 2\lp16k.flac"
    And an analysis of "Lib" has finished
    When I export
    Then the CSV has the header of csv-export.md section 3 and exactly 2 records, one per track
    And no record or column holds an album summary

  # --- Interface (stubbed API client and events; specs/ui/album-summary.md) ----
  # "an album update for <name>" is a stubbed trackDone whose `album` has that
  # name; indexes follow the order the albums are listed in the Given.

  Scenario: Album summaries are shown after the analysis
    Given an analysis of "Artist" finished with these last album updates:
      | index | name           | files | total | real | suspicious | fake | errors | verdict    |
      | 0     | Artist         | 1     | 1     | 1    | 0          | 0    | 0      | real       |
      | 1     | Artist\Album 1 | 12    | 12    | 11   | 0          | 1    | 0      | fake       |
      | 2     | Artist\Album 2 | 9     | 9     | 7    | 2          | 0    | 0      | suspicious |
    Then the Albums panel header reads "Albums (3)"
    And the panel lists, in this order:
      | name           | verdict    | counts                                |
      | Artist         | Genuine    | 1 track · 1 real                      |
      | Artist\Album 1 | Fake       | 12 tracks · 11 real · 1 fake          |
      | Artist\Album 2 | Suspicious | 9 tracks · 7 real · 2 suspicious      |
    And each verdict is shown with its icon, its text and its colour, as in the table's State column
    And the table still has one row per track and is not grouped

  Scenario: Albums appear in path order, whatever order they arrive in
    Given an analysis is running
    When a batch brings the first trackDone of the album with index 2, "Lib\B"
    And a later batch brings the first trackDone of the album with index 0, "Lib\A"
    Then the panel lists "Lib\A" above "Lib\B"

  Scenario: Summaries update during the analysis
    Given an analysis of "Album A" is running, whose album has files 12
    When batches bring trackDone events with album totals 1, 2 and 3, all real
    Then the item "Album A" shows "Genuine" and "3 of 12 tracks · 3 real"
    When a batch brings a fake track with album total 4
    Then the item "Album A" shows "Fake" and "4 of 12 tracks · 3 real · 1 fake"
    And the panel and the table were updated in the same flushes, at most once every 100 ms

  Scenario: Panel counts always match the table rows
    Given an analysis is running
    When 200 stubbed trackDone events for 7 albums arrive in a shuffled order, in batches
    Then after every flush each album item's counts equal the counts by verdict of the table rows with that album index
    And after "finished" the totals of the items add up to the done count of the summary

  Scenario: A cancelled run keeps partial albums
    Given an analysis of "Big" is running with the albums "Big\Album 1" (files 12) and "Big\Album 2" (files 12)
    And 3 tracks of "Big\Album 1" are done and none of "Big\Album 2"
    When I click "Cancel" and the run ends with "cancelled"
    Then the panel lists only "Big\Album 1", with "3 of 12 tracks · 3 real"
    And the status bar reads "Cancelled · 3 of 24"

  Scenario: A cancel before any track leaves the panel empty
    Given an analysis is running
    When the run ends with "cancelled" before any trackDone
    Then the Albums panel header reads "Albums" and the panel has no items

  @RNF-04
  Scenario: A folder whose files all failed
    Given an analysis finished with the album "Lib\Broken" whose last update has files 2, total 2, errors 2 and verdict "error"
    Then the item "Lib\Broken" shows "Error" with the error icon and "2 tracks · 2 errors"

  @RNF-04
  Scenario: A run that fails unexpectedly keeps the albums it reported
    Given an analysis of "Album A" with files 5 reported 2 real tracks, the second still in the pending batch
    When start_analysis rejects with "INTERNAL"
    Then the panel lists "Album A" with "2 of 5 tracks · 2 real"
    And the table has the 2 rows

  @RNF-09
  Scenario: Two albums with the same name stay two items
    Given an analysis finished with two album updates named "Lib\Disc �" with the summary folder "C:\Music\Lib\Disc �": index 0 with 1 real track and index 1 with 1 fake track
    Then the panel lists two items "Lib\Disc �", the first "Genuine" and the second "Fake"
    When I click the second item
    Then the row of the fake track is selected and the second item is the current album

  @RNF-09
  Scenario: Unicode folder names in the panel
    Given an analysis finished with the albums "Álbum ″Ñandú″" and "Álbum ″Ñandú″\Disco 2 «Ça va» 🎵" whose summary folders are "C:\Music\Álbum ″Ñandú″" and "C:\Music\Álbum ″Ñandú″\Disco 2 «Ça va» 🎵"
    Then the panel shows both names exactly, code point for code point
    And the tooltip of the second item is "C:\Music\Álbum ″Ñandú″\Disco 2 «Ça va» 🎵"

  Scenario: A long name is cut with an ellipsis and shown whole in the tooltip
    Given an analysis finished with the album "Artist\A very long album name that does not fit in the panel (Remastered 2024 Deluxe Edition)"
    Then its name is shown on one line, ending with an ellipsis
    And its tooltip is the full folder path

  Scenario: A new analysis replaces the album summaries
    Given an analysis of "Lib A" finished with the albums "Lib A\X" and "Lib A\Y"
    When I start an analysis of "Lib B"
    Then while it is scanning the panel header reads "Albums" and the panel has no items
    When "Lib B" finishes with the album "Lib B"
    Then the panel lists only "Lib B"

  Scenario Outline: A start that fails before Scanning keeps the albums
    Given an analysis finished with the albums "Lib\A" and "Lib\B", and the row of "Lib\B\01.flac" is selected
    When I open the folder "Docs" and start_analysis rejects with "<code>" before any event
    Then the panel still lists "Lib\A" and "Lib\B" with the same counts
    And "Lib\B" is still the current album

    Examples:
      | code              |
      | NO_AUDIO_FOUND    |
      | FOLDER_UNREADABLE |
      | NOT_A_FOLDER      |
      | ANALYSIS_RUNNING  |

  Scenario: The panel is not shown in the empty state
    Given the application has just started
    Then no Albums panel is shown and the drop hint is shown

  Scenario: Selecting an album selects its first track
    Given an analysis finished with the album "Lib\B" whose rows have the indexes 7, 4 and 9
    And the table is sorted by file in descending order
    When I click the item "Lib\B"
    Then the row with index 4 is selected and is within the table's visible area
    And the detail panel requests the spectrogram of that row's path
    And "Lib\B" is the current album

  Scenario: Selecting a track marks its album as current
    Given an analysis finished with the albums "Lib\A" and "Lib\B"
    When I click a row of "Lib\B" in the table
    Then the item "Lib\B" has the selected background and aria-selected "true"
    And the item "Lib\A" does not
    When I press ↑ in the table until a row of "Lib\A" is selected
    Then "Lib\A" becomes the current album

  Scenario: Keyboard in the Albums panel
    Given an analysis finished with the albums "Lib\A", "Lib\B" and "Lib\C" and no row selected
    When I focus the Albums panel and press ↓
    Then the first row of "Lib\A" is selected
    When I press End
    Then the first row of "Lib\C" is selected
    When I press ↑
    Then the first row of "Lib\B" is selected and the focus is still in the panel

  @RF-06
  Scenario: Moving the selection in the table keeps the selected row visible
    Proposed with RF-10 (components.md, ResultsTable): the table scrolls the
    selected row into view whatever changed the selection.
    Given an analysis finished with 500 tracks and the first row selected
    When I press End in the table
    Then the last row is selected and is within the table's visible area
    When I press Home
    Then the first row is selected and is within the table's visible area
    When I press ↓ until the selection passes the last visible row
    Then the selected row is within the table's visible area after every key press

  Scenario: Sorting the table does not change the panel
    Given an analysis finished with the albums "Lib\A", "Lib\B" and "Lib\C"
    When I sort the table by state, then by folder in descending order
    Then the panel still lists "Lib\A", "Lib\B", "Lib\C" in this order with the same counts

  Scenario: The panel's accessible names
    Given an analysis finished with the album "Artist\Album 1" (12 tracks, 11 real, 1 fake)
    Then the Albums panel is a listbox named "Album summaries"
    And the item's accessible name is "Artist\Album 1, Fake, 12 tracks · 11 real · 1 fake"
