@RF-08
Feature: Sort and filter the results table (H2)
  As a user reviewing the results of a large folder
  I want to sort the table by verdict or by name and narrow it down by verdict and by name
  So that I can find the fake and suspicious tracks, or one album or track, quickly

  UI spec: specs/ui/sort-filter.md. Frontend only: the contract
  (specs/contracts/commands.md, version 5) does not change.

  # --- Interface (stubbed API client and events) ------------------------------
  # "an analysis finished with these rows": stubbed trackDone events, one per
  # row, then "finished". The "name" column is "<album name>\<file name>": the
  # album name is the AlbumUpdate.name of the row's album (one album per distinct
  # folder part, indexes in path order), the file name is the last component of
  # TrackInfo.path, and TrackInfo.path is "C:\Music\" followed by the name. Rows
  # get their index in the order listed unless an "index" column says otherwise.
  # "the table shows" lists the File cells (or the name, when the folder matters)
  # from top to bottom; "visible" rows are the rows the filters let through.
  # "the table's row order" is the order of every row the table shows, top to
  # bottom (the table is virtualised: tests may read it through the rendered
  # window or by scrolling).
  # Verdict texts are the English ones: Genuine, Suspicious, Fake, Error.
  #
  # "an analysis finished with N tracks[: a real, b suspicious, c fake and d
  # errors]": the rows "Lib\A\01.flac" to "Lib\A\NN.flac" (two-digit numbers,
  # one album "Lib\A"), index = number - 1, verdicts given in the order real,
  # suspicious, fake, error (so with "28 real, 2 suspicious and 1 fake" the
  # fake one is "31.flac"); with no breakdown every track is real. Duration =
  # number x 1000 ms. An album given as "Lib\X" (n tracks, ...) follows the
  # same rule with its own album name; albums are numbered in path order.
  #
  # Characters: every non-ASCII character in this file is the literal code
  # point written, letters with accents precomposed (NFC: Á U+00C1, ñ U+00F1,
  # Ñ U+00D1, ú U+00FA, ó U+00F3, Ó U+00D3, Ç U+00C7), "–" U+2013, "″" U+2033,
  # "‘" U+2018, "’" U+2019, "«" U+00AB, "»" U+00BB, "🎵" U+1F3B5 (no variation
  # selector), "�" U+FFFD. Quotes around a step argument are delimiters, not
  # part of it; an argument that contains an ASCII quote is given in a table.

  Background:
    Given the UI locale is English

  # --- Sorting -----------------------------------------------------------------

  Scenario: The default order is by name, natural and case-insensitive
    Given an analysis finished with these rows:
      | name                    | verdict |
      | Album\Track 10.flac     | real    |
      | Album\track 2.flac      | real    |
      | Album\Track 1.flac      | real    |
    Then the table shows "Track 1.flac", "track 2.flac", "Track 10.flac"
    And the File header has aria-sort "ascending"

  Scenario: A folder's tracks come before a sibling folder whose name extends it
    Given an analysis finished with these rows:
      | name                        | verdict |
      | Lib\Album (deluxe)\01.flac  | real    |
      | Lib\Album\CD2\01.flac       | real    |
      | Lib\Album\02.flac           | real    |
    Then the table shows the names "Lib\Album\02.flac", "Lib\Album\CD2\01.flac", "Lib\Album (deluxe)\01.flac"

  Scenario: A subfolder sorts among its parent folder's files by its name
    Intended: the path is compared component by component, so the subfolder
    "Bonus" sorts between the files "01.flac" and "Z.flac" (ui/sort-filter.md
    section 3.1, Q3); files are not put before subfolders.
    Given an analysis finished with these rows:
      | name                     | verdict |
      | Lib\Album\Z.flac         | real    |
      | Lib\Album\Bonus\x.flac   | real    |
      | Lib\Album\01.flac        | real    |
    Then the table shows the names "Lib\Album\01.flac", "Lib\Album\Bonus\x.flac", "Lib\Album\Z.flac"
    When I click the File header
    Then the table shows the names "Lib\Album\Z.flac", "Lib\Album\Bonus\x.flac", "Lib\Album\01.flac"

  Scenario: Sorting by state puts the tracks that need attention first
    Given an analysis finished with these rows:
      | name        | verdict    |
      | A\a.flac    | real       |
      | A\b.flac    | error      |
      | A\c.flac    | fake       |
      | A\d.flac    | suspicious |
    When I click the State header
    Then the State column reads "Fake", "Suspicious", "Error", "Genuine" from top to bottom
    And the State header has aria-sort "ascending"
    When I click the State header again
    Then the State column reads "Genuine", "Error", "Suspicious", "Fake" from top to bottom
    And the State header has aria-sort "descending"

  Scenario: Rows with the same verdict stay in name order in both directions
    Given an analysis finished with these rows:
      | name       | verdict | index |
      | A\b.flac   | fake    | 0     |
      | A\a.flac   | fake    | 1     |
      | A\c.flac   | real    | 2     |
    And their trackDone events arrived in the order "c.flac", "b.flac", "a.flac"
    When I sort the table by state ascending
    Then the table shows "a.flac", "b.flac", "c.flac"
    When I sort the table by state descending
    Then the table shows "c.flac", "a.flac", "b.flac"

  @RNF-09
  Scenario: Two rows with the same path keep their scan order
    Two files whose names were not valid Unicode arrive with the same path
    (U+FFFD, commands.md "Events"); the table keys rows by index.
    Given an analysis finished with the rows "Lib\Disc �\01.flac" index 5 real and "Lib\Disc �\01.flac" index 3 fake
    Then the first row is the one with index 3 and the second the one with index 5
    When I click the File header
    Then the first row is still the one with index 3

  @RNF-04
  Scenario: Empty cells go last in both directions
    Given an analysis finished with these rows:
      | name            | verdict | cutoff hz |
      | A\low.flac      | fake    | 16000     |
      | A\corrupt.flac  | error   |           |
      | A\full.flac     | real    | 20000     |
    When I sort the table by cutoff ascending
    Then the table shows "low.flac", "full.flac", "corrupt.flac"
    When I sort the table by cutoff descending
    Then the table shows "full.flac", "low.flac", "corrupt.flac"

  Scenario: A row with an empty folder sorts by the folder its cell shows
    The Folder cell falls back to the folder part of the path when
    TrackInfo.folder is empty (ui/sort-filter.md section 3.1). The "folder"
    column gives TrackInfo.folder; an empty cell is the empty string. (Where
    the column is absent, TrackInfo.folder is the path's folder.)
    Given an analysis finished with these rows:
      | name        | verdict | folder       |
      | L\B\01.flac | real    |              |
      | L\A\01.flac | real    | C:\Music\L\A |
      | L\C\01.flac | real    | C:\Music\L\C |
    When I click the Folder header
    Then the table shows the names "L\A\01.flac", "L\B\01.flac", "L\C\01.flac"
    And the Folder cell of "L\B\01.flac" shows its path's folder

  @RNF-09
  Scenario: Unicode names sort by letter, accents only breaking ties
    Given an analysis finished with the rows "A\zebra.flac", "A\Canción.flac", "A\apple.flac", "A\Ça va.flac", "A\Cancion.flac" and "A\Álbum ″Ñandú″.flac", all real
    Then the table shows "Álbum ″Ñandú″.flac", "apple.flac", "Ça va.flac", "Cancion.flac", "Canción.flac", "zebra.flac"
    And each File cell shows its name code point for code point

  Scenario: New rows take their sorted place during a run without moving the selection or the scroll
    Given an analysis is running whose scan found 520 tracks
    And a batch has brought the 500 rows "A\0000.flac" to "A\0499.flac" (four-digit numbers), index equal to the number, fake when the number is odd and real when it is even
    And the table is sorted by state ascending
    And I click the row "A\0100.flac" and then scroll the table to half its maximum scroll position
    When a batch brings the 20 rows "A\0500.flac" to "A\0519.flac", index equal to the number, fake when the number is odd and real when it is even
    Then the table's row order is the 260 fake rows "A\0001.flac", "A\0003.flac", …, "A\0519.flac" followed by the 260 genuine rows "A\0000.flac", "A\0002.flac", …, "A\0518.flac"
    And "A\0100.flac" is still selected
    And the table's scroll position did not change

  Scenario: The sort is kept for the next run
    Given an analysis of "Lib A" finished and the table is sorted by state descending
    When I start an analysis of "Lib B" and it finishes
    Then the table is sorted by state descending
    And the State header has aria-sort "descending"

  # --- Verdict filter --------------------------------------------------------

  Scenario: One verdict button shows only that verdict
    Given an analysis finished with 31 tracks: 28 real, 2 suspicious and 1 fake
    When I press the "Show fake tracks" button
    Then the table shows only "31.flac"
    And the "Show fake tracks" button has aria-pressed "true" and the other three "false"
    And the status bar reads "31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors · 1 shown"

  Scenario: Several verdict buttons show the union
    Given an analysis finished with 31 tracks: 28 real, 2 suspicious and 1 fake
    When I press the "Show fake tracks" and "Show suspicious tracks" buttons
    Then the table shows "29.flac", "30.flac", "31.flac"
    When I press the "Show fake tracks" button again
    Then the table shows "29.flac", "30.flac"

  Scenario: No verdict button pressed shows every verdict
    Given an analysis finished with 31 tracks: 28 real, 2 suspicious and 1 fake
    And the "Show suspicious tracks" button is pressed
    When I press the "Show suspicious tracks" button again
    Then the table shows all 31 tracks
    And the status bar reads "31 tracks · 28 real · 2 suspicious · 1 fake · 0 errors"

  Scenario: Every verdict button pressed shows every track
    Given an analysis finished with 31 tracks: 28 real, 2 suspicious and 1 fake
    When I press all four verdict buttons
    Then the table shows all 31 tracks
    And the status bar ends with "· 31 shown"

  @RNF-04
  Scenario: The Error button shows the files that could not be analysed
    Given an analysis finished with these rows:
      | name             | verdict | error         |
      | A\ok.flac        | real    |               |
      | A\corrupt.flac   | error   | corruptStream |
      | A\short.flac     | error   | tooShort      |
      | A\fake.flac      | fake    |               |
    When I press the "Show tracks with errors" button
    Then the table shows "corrupt.flac", "short.flac"
    And hovering "corrupt.flac" shows the tooltip "Corrupt stream"
    When I press the "Show fake tracks" button
    Then the table shows "corrupt.flac", "fake.flac", "short.flac"

  @RNF-08
  Scenario: The verdict buttons do not rely on colour alone
    Given an analysis finished with 3 tracks
    Then the toolbar has a group named "Filter by verdict" with the toggle buttons, in this order, "Show fake tracks", "Show suspicious tracks", "Show genuine tracks", "Show tracks with errors"
    And each button shows the icon of its verdict in the State column and has its label as tooltip and accessible name

  # --- Name search -----------------------------------------------------------

  Scenario: Search by part of the file name, case-insensitive
    Given an analysis finished with the rows "Artist\Album 1\01 – Intro.flac", "Artist\Album 1\02 – Outro.flac" and "Artist\Album 2\01 – Intro.flac"
    When I type "INTRO" in the search field
    Then the table shows the names "Artist\Album 1\01 – Intro.flac", "Artist\Album 2\01 – Intro.flac"
    And the status bar ends with "· 2 shown"

  Scenario: Search by the album folder, but not by where the analysed folder is
    Given the analysed root is "C:\Music\Artist"
    And an analysis finished with the rows "Artist\Album 1\Intro.flac", "Artist\Album 10\Intro.flac" and "Artist\Album 2\Intro.flac"
    When I type "album 1" in the search field
    Then the table shows the names "Artist\Album 1\Intro.flac", "Artist\Album 10\Intro.flac"
    When I replace the search text with "music"
    Then the table shows no row and the no-match message

  Scenario: Every word of the search must appear, in any order
    Given an analysis finished with the rows "The Beatles\Help!\01 – Help!.flac" and "The Beatles\Abbey Road\01 – Come Together.flac"
    When I type "help beatles" in the search field
    Then the table shows the name "The Beatles\Help!\01 – Help!.flac"
    When I replace the search text with "beatles/abbey"
    Then the table shows the name "The Beatles\Abbey Road\01 – Come Together.flac"

  @RNF-09
  Scenario Outline: Search ignores case and accents, not other symbols
    The step types the "query" cell verbatim: the cell's text exactly as written
    between the bars (Gherkin trims only the spaces next to the bars), with no
    delimiters of its own. So the ASCII " and ' in the last rows are part of the
    query, and the cells hold the literal code points listed in the Interface
    comment ("″" is U+2033, "‘x’" is U+2018 x U+2019, "🎵" is U+1F3B5).
    Given an analysis finished with the rows "Álbum ″Ñandú″\01 – Canción ‘x’.flac" and "Álbum ″Ñandú″\Disco 2 «Ça va» 🎵\01.flac"
    When I type the query cell <query> in the search field
    Then the table shows <count> rows

    Examples:
      | query       | count |
      | cancion     | 1     |
      | CANCIÓN     | 1     |
      | nandu       | 2     |
      | ñandú       | 2     |
      | ÇA VA       | 1     |
      | ca va       | 1     |
      | 🎵          | 1     |
      | ″ñandú″     | 2     |
      | "nandu"     | 0     |
      | ‘x’         | 1     |
      | 'x'         | 0     |

  @RNF-09
  Scenario: The search field keeps the text as typed
    Given an analysis finished with 3 tracks
    When I type "  Álbum ″Ñandú″ " in the search field
    Then the search field's value is "  Álbum ″Ñandú″ ", code point for code point

  Scenario: The verdict filter and the search combine
    Given an analysis finished with these rows:
      | name                | verdict |
      | Lib\A\01.flac       | fake    |
      | Lib\A\02.flac       | real    |
      | Lib\B\01.flac       | fake    |
    When I press the "Show fake tracks" button and type "lib\a" in the search field
    Then the table shows the name "Lib\A\01.flac"

  Scenario: Escape and the clear button empty the search field
    Given an analysis finished with 31 tracks and "intro" typed in the search field
    When I press Escape in the search field
    Then the search field is empty and still focused
    And the table shows all 31 tracks
    When I type "intro" again and click the "Clear search" button
    Then the search field is empty and focused
    And the "Clear search" button is not shown

  Scenario: Ctrl+F focuses the search field
    Given an analysis finished with 31 tracks and the table focused
    And "intro" typed earlier in the search field
    When I press Ctrl+F
    Then the search field is focused and its text "intro" is selected
    And the default action of the Ctrl+F key event was prevented

  # --- No match, empty state, runs -------------------------------------------

  Scenario: No track matches the filters
    Given an analysis finished with 31 tracks: 29 real and 2 suspicious
    And the table is sorted by duration descending
    When I press the "Show fake tracks" button
    Then the table keeps its header and its body reads "No tracks match the filters"
    And the status bar ends with "· 0 shown"
    When I click "Clear filters"
    Then the table shows all 31 tracks, "31.flac", "30.flac", …, "01.flac" from top to bottom
    And no verdict button is pressed and the search field is empty
    And the search field is focused
    And the selection did not change

  Scenario: The filters are disabled in the empty state
    Given the application has just started
    Then the four verdict buttons and the search field are disabled
    And no verdict button is pressed and the search field shows its placeholder
    When I press Ctrl+F
    Then the search field is not focused
    And the default action of the Ctrl+F key event was prevented

  Scenario: A filter kept into the empty state is shown on the disabled controls
    The empty state can come back with a filter set only after a run that left
    no row: here a cancelled run, then a start rejected with NOT_A_FOLDER, which
    puts the previous state back and then resets it (phase ready, no row).
    Given an analysis is running whose scan found 10 tracks and no track is done yet
    And the "Show fake tracks" button is pressed and "01" is typed in the search field
    And the run is cancelled before any track is done
    When I start an analysis and start_analysis rejects with "NOT_A_FOLDER" before any event
    Then the drop hint of the empty state is shown and the table is not
    And the "Show fake tracks" button is disabled with aria-pressed "true" and the other three are disabled with aria-pressed "false"
    And the search field is disabled and reads "01", and the "Clear search" button is not shown
    When I start an analysis and it is scanning
    Then the "Show fake tracks" button is enabled and pressed and the search field is enabled and reads "01"

  Scenario: Filters apply to the rows that arrive during a run
    Given an analysis is running and the "Show fake tracks" button is pressed
    When a batch brings these rows:
      | name       | verdict |
      | A\03.flac  | fake    |
      | A\01.flac  | real    |
      | A\02.flac  | real    |
      | A\04.flac  | real    |
    Then the table shows only "03.flac"
    And the status bar ends with "· 1 shown"
    When a later batch brings the row "A\00.flac", fake
    Then the table shows "00.flac", "03.flac"
    And the status bar ends with "· 2 shown"

  Scenario: While scanning with a filter, the table shows no no-match message
    Given an analysis finished and the "Show fake tracks" button is pressed
    When I start a new analysis and it is scanning
    Then the "Show fake tracks" button is still pressed
    And the table is empty without the no-match message
    And the status bar does not contain "shown"

  Scenario: The filters are kept for the next run
    Given an analysis of "Lib A" finished with 31 tracks: 30 real and 1 fake
    And the "Show fake tracks" button is pressed and "01" is typed in the search field
    When I start an analysis of "Lib B" and it finishes with these rows:
      | name             | verdict |
      | Lib B\01.flac    | fake    |
      | Lib B\10.flac    | fake    |
      | Lib B\01b.flac   | real    |
    Then the "Show fake tracks" button is still pressed and the search field still reads "01"
    And the table shows the name "Lib B\01.flac"

  Scenario: The shown count after a cancel
    Given an analysis is running whose scan found 10 tracks, with the "Show fake tracks" button pressed
    And a batch brought the rows "A\01.flac" real, "A\02.flac" fake and "A\03.flac" real
    When the run is cancelled with 3 of 10 tracks done
    Then the table shows only "02.flac"
    And the status bar reads "Cancelled · 3 of 10 · 1 shown"

  Scenario: No shown count while the store has no row
    Given an analysis is running whose scan found 10 tracks and no track is done yet
    And the "Show fake tracks" button is pressed
    When the run is cancelled with 0 of 10 tracks done
    Then the status bar reads "Cancelled · 0 of 10"

  @RNF-04
  Scenario: A run that fails unexpectedly keeps the filters
    Given an analysis of "Album A" is running with the "Show tracks with errors" button pressed
    And it reported the rows "Album A\01.flac" real and "Album A\02.flac" error
    When start_analysis rejects with "INTERNAL"
    Then the table shows only "02.flac"
    And the "Show tracks with errors" button is still pressed
    And the status bar reads "The analysis stopped because of an unexpected error", with no shown count

  # --- Selection and keyboard ------------------------------------------------

  Scenario: A selected track hidden by a filter stays selected
    Given an analysis finished with these rows:
      | name       | verdict |
      | A\a.flac   | real    |
      | A\b.flac   | fake    |
    And I click the row "a.flac"
    When I press the "Show fake tracks" button
    Then no row of the table is highlighted
    And the detail panel still shows "a.flac"
    And no new spectrogram request was made
    When I press the "Show fake tracks" button again
    Then the row "a.flac" is highlighted as selected

  Scenario: The keyboard moves over the visible rows only
    Given an analysis finished with these rows:
      | name       | verdict |
      | A\a.flac   | fake    |
      | A\b.flac   | real    |
      | A\c.flac   | fake    |
    And the "Show fake tracks" button is pressed
    When I focus the table and press ↓
    Then "a.flac" is selected
    When I press ↓
    Then "c.flac" is selected
    When I press Home
    Then "a.flac" is selected

  Scenario: ↓ with a hidden selection selects the first visible row
    Given an analysis finished with these rows:
      | name       | verdict |
      | A\a.flac   | real    |
      | A\b.flac   | fake    |
      | A\c.flac   | fake    |
    And I click the row "a.flac" and then press the "Show fake tracks" button
    When I focus the table and press ↓
    Then "b.flac" is selected
    And the detail panel requests the spectrogram of "b.flac"

  # --- Albums panel (RF-10) --------------------------------------------------

  Scenario: The Albums panel ignores the filters
    Given an analysis finished with the albums "Lib\A" (12 tracks, 11 real, 1 fake) and "Lib\B" (9 tracks, 9 real)
    When I press the "Show fake tracks" button and type "zzz" in the search field
    Then the panel still lists "Lib\A" with "12 tracks · 11 real · 1 fake" and "Lib\B" with "9 tracks · 9 real"

  Scenario: Selecting an album selects its lowest-index visible track
    Given an analysis finished with these rows:
      | name           | verdict | index |
      | Lib\B\01.flac  | real    | 4     |
      | Lib\B\02.flac  | fake    | 7     |
      | Lib\B\03.flac  | fake    | 9     |
    And the "Show fake tracks" button is pressed
    When I click the item "Lib\B"
    Then the row with index 7 is selected and is within the table's visible area
    And "Lib\B" is the current album

  Scenario: Under the State sort the lowest-index visible track is not the first shown
    The rule is the lowest index among the shown rows, not the first row in the
    display order (ui/sort-filter.md section 6).
    Given an analysis finished with these rows:
      | name           | verdict    | index |
      | Lib\B\01.flac  | real       | 4     |
      | Lib\B\02.flac  | suspicious | 7     |
      | Lib\B\03.flac  | fake       | 9     |
    And the table is sorted by state ascending
    And the "Show fake tracks" and "Show suspicious tracks" buttons are pressed
    Then the table shows "03.flac", "02.flac"
    When I click the item "Lib\B"
    Then the row with index 7 ("02.flac", the second row shown) is selected
    When I release both verdict buttons and click the item "Lib\B" again
    Then the table shows "03.flac", "02.flac", "01.flac"
    And the row with index 4 ("01.flac", the last row shown) is selected

  Scenario: Selecting an album whose tracks are all hidden
    Given an analysis finished with the albums "Lib\A" (fake tracks only) and "Lib\B" whose rows are index 4 real and index 9 real
    And the "Show fake tracks" button is pressed
    When I click the item "Lib\B"
    Then the row with index 4 is selected but not shown in the table
    And the detail panel requests the spectrogram of that row's path
    And "Lib\B" is the current album
    And the "Show fake tracks" button is still pressed

  # --- Export (RF-09) ----------------------------------------------------------

  Scenario: Export ignores the sort and the filters
    Given an analysis finished with 31 tracks: 30 real and 1 fake
    And the table is sorted by state descending and only the "Show fake tracks" button is pressed
    Then the "Export CSV" button is enabled
    When I click "Export CSV" and the stubbed export resolves with rows 31 and the path "C:\Out\results.csv"
    Then exportCsv was called once, with no argument
    And the status bar ends with "· 1 shown · Saved 31 rows to results.csv"

  # --- Size ------------------------------------------------------------------

  Scenario: Ten thousand rows
    Timing is checked by hand (ui/sort-filter.md, P1); this scenario only checks
    the result. Rows i = 0 to 9999: name "Lib\Album AA\IIII – Track.flac", where
    AA is floor(i / 100) with two digits and IIII is i with four digits (100
    albums of 100 rows, "Lib\Album 00" to "Lib\Album 99"), index i, verdict by
    i mod 4 (0 real, 1 suspicious, 2 fake, 3 error), duration (i + 1) x 1000 ms.
    The suspicious rows whose searched name contains "00" are those of
    "Lib\Album 00" (i = 1, 5, …, 97: 25 rows) and those whose four digits have
    "00" in the middle (i = d001, d005, d009 for d = 0 to 9: 30 rows, 3 of them
    in "Lib\Album 00"): 52 rows.
    Given an analysis finished with these 10 000 stubbed tracks
    When I sort the table by state ascending, press the "Show suspicious tracks" button and type "00" in the search field
    Then the status bar ends with "· 52 shown"
    And the table's row order is the 25 rows "Lib\Album 00\0001 – Track.flac", "Lib\Album 00\0005 – Track.flac", …, "Lib\Album 00\0097 – Track.flac", then for each of "Lib\Album 10", "Lib\Album 20", …, "Lib\Album 90" its rows d001, d005, d009 (for example "Lib\Album 10\1001 – Track.flac", "Lib\Album 10\1005 – Track.flac", "Lib\Album 10\1009 – Track.flac")
    And the first row rendered is "Lib\Album 00\0001 – Track.flac" and, after End in the table, the selected row is "Lib\Album 90\9009 – Track.flac"
