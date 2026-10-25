@RF-09
Feature: Export the results to CSV (H2)
  As a user who has analysed a folder
  I want to save the results as a CSV file
  So that I can keep them, share them or process them with other tools

  File format and layers: specs/contracts/csv-export.md (csv_format_version 1,
  ADR-0012: RFC 4180, UTF-8 without BOM, comma separator, CRLF line ends).
  Command: export_csv in specs/contracts/commands.md (version 4).
  UI: specs/ui/export-csv.md.
  The app writes the CSV only where the user chooses in the save dialog. The
  automated tests stub the dialog: they call the steps before and after it
  (run::begin_export, run::export_csv) with a destination inside a test
  folder, never in the audio corpus; the native dialog itself and the export
  time are checked by hand (ui/export-csv.md, M1 to M7).

  # --- Document (serialiser, stubbed TrackAnalysis values, no audio) ---------
  # "{LF}" stands for U+000A, "{CR}" for U+000D, "{TAB}" for U+0009 and "{SP}"
  # for U+0020 (Gherkin trims the spaces around a table cell). "none" is an
  # absent value in a given table and an empty field in an expected one.
  # Every expected document has its lines separated by CR LF and ends with
  # CR LF after its last line.

  Scenario: The header row
    Given one stubbed TrackAnalysis
    When the CSV document is built
    Then its first record is exactly "verdict,file,folder,format,sample_rate_hz,bit_depth,duration_ms,cutoff_hz,declared_bitrate_kbps,bitrate_mode,estimated_source_bitrate_kbps,reasons,error,error_detail,codec,channels,size_bytes,cutoff_steepness_db_per_khz,noise_floor_db,algorithm_version,path"
    And every record of the document has 21 fields

  @RNF-04 @RNF-09
  Scenario: The document for four stubbed tracks
    The first name holds '"', which Windows forbids in file names: the values
    are stubbed and the serialiser treats every field alike. The order given
    is the scan order of the four paths.
    Given these stubbed TrackAnalysis values, in this order, all with algorithmVersion 6 and folder the parent of their path:
      | path                                         | format | codec | rate  | bits | ch | ms     | bytes    | verdict    | cutoff  | steep | noise   | kbps | mode | est  | reasons             | error         | detail                      |
      | C:\Music\Album A\02 Live, "Encore".mp3       | mp3    | mp3   | 44100 | none | 2  | 180000 | 7200000  | suspicious | 18500   | 12.5  | -88.0   | 245  | vbr  | 192  | cutoffBelowDeclared | none          | none                        |
      | C:\Music\Album A\corrupt.flac                | flac   | flac  | 0     | none | 0  | 0      | 1024     | error      | none    | none  | none    | none | none | none | none                | corruptStream | bad frame header, sync lost |
      | C:\Music\Album A\lp16k.flac                  | flac   | flac  | 44100 | 16   | 2  | 30000  | 5000000  | fake       | 16000.4 | 42.06 | -101.94 | none | none | none | none                | none          | none                        |
      | C:\Music\Álbum ″Ñandú″\01 – Canción ‘x’.m4a  | m4a    | alac  | 48000 | 24   | 2  | 245000 | 30000000 | real       | 24000   | 0.0   | -120.0  | none | none | none | none                | none          | none                        |
    When the CSV document is built
    Then it is exactly:
      ```
      verdict,file,folder,format,sample_rate_hz,bit_depth,duration_ms,cutoff_hz,declared_bitrate_kbps,bitrate_mode,estimated_source_bitrate_kbps,reasons,error,error_detail,codec,channels,size_bytes,cutoff_steepness_db_per_khz,noise_floor_db,algorithm_version,path
      suspicious,"02 Live, ""Encore"".mp3",C:\Music\Album A,mp3,44100,,180000,18500,245,vbr,192,cutoffBelowDeclared,,,mp3,2,7200000,12.5,-88.0,6,"C:\Music\Album A\02 Live, ""Encore"".mp3"
      error,corrupt.flac,C:\Music\Album A,flac,,,,,,,,,corruptStream,"bad frame header, sync lost",flac,,1024,,,6,C:\Music\Album A\corrupt.flac
      fake,lp16k.flac,C:\Music\Album A,flac,44100,16,30000,16000,,,,,,,flac,2,5000000,42.1,-101.9,6,C:\Music\Album A\lp16k.flac
      real,01 – Canción ‘x’.m4a,C:\Music\Álbum ″Ñandú″,m4a,48000,24,245000,24000,,,,,,,alac,2,30000000,0.0,-120.0,6,C:\Music\Álbum ″Ñandú″\01 – Canción ‘x’.m4a
      ```

  Scenario Outline: A field is quoted only when it must be (RFC 4180)
    <field> is the exact text written between the separators around the
    error_detail field (the comma before it and the comma after it), quotes
    included; <quoted> says whether it starts and ends with '"'.
    Given a stubbed TrackAnalysis whose error detail is "<value>"
    When the CSV document is built
    Then the bytes of its error_detail field are exactly <field>
    And the field is quoted: <quoted>

    Examples:
      | value               | field                 | quoted |
      | plain text          | plain text            | no     |
      | a, b                | "a, b"                | yes    |
      | say "hi"            | "say ""hi"""          | yes    |
      | "                   | """"                  | yes    |
      | line{LF}break       | "line{LF}break"       | yes    |
      | line{CR}break       | "line{CR}break"       | yes    |
      | line{CR}{LF}break   | "line{CR}{LF}break"   | yes    |
      | {SP}leading space   | {SP}leading space     | no     |
      | trailing space{SP}  | trailing space{SP}    | no     |
      | tab{TAB}inside      | tab{TAB}inside        | no     |
      | semi;colon          | semi;colon            | no     |
      | it's                | it's                  | no     |
      | =1+1                | =1+1                  | no     |
      | Ñandú ″x″           | Ñandú ″x″             | no     |
      | none                | none                  | no     |

  Scenario: File names are quoted by the same rule
    Given a stubbed TrackAnalysis of "C:\Music\Rock, Pop\Live, 1999.flac"
    When the CSV document is built
    Then its file field is written as "Live, 1999.flac" in double quotes
    And its folder field is written as "C:\Music\Rock, Pop" in double quotes
    And its path field is written as "C:\Music\Rock, Pop\Live, 1999.flac" in double quotes

  Scenario Outline: Numbers do not depend on the locale
    Given a stubbed TrackAnalysis with <field> <stored>
    When the CSV document is built
    Then its <column> field is "<cell>"

    Examples:
      | field                     | stored   | column                      | cell     |
      | cutoff frequency (Hz)     | 16000.4  | cutoff_hz                   | 16000    |
      | cutoff frequency (Hz)     | 16000.5  | cutoff_hz                   | 16001    |
      | cutoff steepness (dB/kHz) | 42.06    | cutoff_steepness_db_per_khz | 42.1     |
      | cutoff steepness (dB/kHz) | -0.04    | cutoff_steepness_db_per_khz | 0.0      |
      | noise floor (dB)          | -101.94  | noise_floor_db              | -101.9   |
      | noise floor (dB)          | -120     | noise_floor_db              | -120.0   |
      | size in bytes             | 1234567  | size_bytes                  | 1234567  |
      | duration in ms            | 3723456  | duration_ms                 | 3723456  |
      | sample rate (Hz)          | 192000   | sample_rate_hz              | 192000   |

  Scenario: Values are contract identifiers, not interface texts
    Given a stubbed TrackAnalysis with verdict "real", format "m4a", codec "aac", bitrate mode "cbr" and no reasons
    When the CSV document is built
    Then its verdict field is "real", not "Genuine"
    And its format field is "m4a" and its codec field is "aac", in lower case
    And the serialiser has no locale or language input, so the document is the same whatever the interface language and the Windows regional settings

  Scenario Outline: Reasons and errors
    Given a stubbed TrackAnalysis with verdict "<verdict>", reasons "<reasons>" and error "<error>" with detail "<detail>"
    When the CSV document is built
    Then its reasons field is "<reasons_cell>"
    And its error field is "<error_cell>" and its error_detail field is "<detail_cell>"

    Examples:
      | verdict    | reasons                      | error             | detail             | reasons_cell                 | error_cell        | detail_cell        |
      | suspicious | highBandGaps,ambiguousCutoff | none              | none               | highBandGaps;ambiguousCutoff | none              | none               |
      | suspicious | lowHighFrequencyContent      | none              | none               | lowHighFrequencyContent      | none              | none               |
      | fake       | none                         | none              | none               | none                         | none              | none               |
      | error      | none                         | tooShort          | none               | none                         | tooShort          | none               |
      | error      | none                         | unsupportedFormat | HE-AAC             | none                         | unsupportedFormat | HE-AAC             |
      | error      | none                         | unreadable        | permission denied  | none                         | unreadable        | permission denied  |

  @RNF-04
  Scenario: Unknown values of an error track are empty cells
    Given a stubbed TrackAnalysis of an M4A file that could not be probed: codec none, sample rate 0, channels 0, duration 0, bit depth none, declared bitrate none, bitrate mode none, size 2048, verdict "error" and error "corruptStream"
    When the CSV document is built
    Then its codec, sample_rate_hz, bit_depth, channels and duration_ms fields are empty
    And its declared_bitrate_kbps and bitrate_mode fields are empty, as their stored values are none
    And its cutoff_hz, cutoff_steepness_db_per_khz, noise_floor_db and estimated_source_bitrate_kbps fields are empty
    And its size_bytes field is "2048" and its format field is "m4a"

  @RNF-04
  Scenario: An error track keeps the values learnt before the error
    Given a stubbed TrackAnalysis of an MP3 file that is too short: codec "mp3", sample rate 44100, channels 1, duration 2011, declared bitrate 320, bitrate mode "cbr", no cutoff, no estimate, verdict "error" and error "tooShort"
    When the CSV document is built
    Then its codec field is "mp3", its sample_rate_hz field "44100", its channels field "1" and its duration_ms field "2011"
    And its declared_bitrate_kbps field is "320" and its bitrate_mode field is "cbr"
    And its cutoff_hz, cutoff_steepness_db_per_khz, noise_floor_db, estimated_source_bitrate_kbps and reasons fields are empty
    And its verdict field is "error", its error field "tooShort" and its error_detail field is empty

  Scenario: A non-finite cutoff is written as no cutoff
    Given a stubbed TrackAnalysis whose cutoff has frequency 16000 Hz and a NaN noise floor
    When the CSV document is built
    Then its cutoff_hz, cutoff_steepness_db_per_khz and noise_floor_db fields are empty

  @RNF-09
  Scenario: UTF-8 without BOM and CRLF line ends
    Given a stubbed TrackAnalysis of "C:\Music\Álbum ″Ñandú″\01 – Canción ‘x’ 🎵.flac"
    When the CSV document is built
    Then its first bytes are 76 65 72 ("ver") and not the BOM EF BB BF
    And the whole document is valid UTF-8
    And "″" is encoded as E2 80 B3 and "🎵" as F0 9F 8E B5
    And every record ends with 0D 0A, the last one included, and no LF appears without a CR before it
    And there is no empty line after the last record

  # --- Default file name (fakeflac-export, pure: default_file_name) ----------
  # csv-export.md section 5.1. No dialog and no file system: the function only
  # reads the path it is given.

  @RNF-09
  Scenario Outline: Default file name
    When default_file_name is called with the root "<root>"
    Then it returns "<name>"

    Examples:
      | root                     | name                         |
      | C:\Music\Album A         | FakeFlac - Album A.csv       |
      | C:\Music\Album A\        | FakeFlac - Album A.csv       |
      | C:\Music\Album A\\       | FakeFlac - Album A.csv       |
      | C:/Music/Album A/        | FakeFlac - Album A.csv       |
      | C:\Music\Álbum ″Ñandú″   | FakeFlac - Álbum ″Ñandú″.csv |
      | C:\Music\02 «Ça va» 🎵   | FakeFlac - 02 «Ça va» 🎵.csv |
      | C:\Music\Rock, Pop       | FakeFlac - Rock, Pop.csv     |
      | D:\                      | FakeFlac.csv                 |
      | D:                       | FakeFlac.csv                 |
      | \\nas\music              | FakeFlac.csv                 |
      | \\nas\music\             | FakeFlac.csv                 |
      | \\nas\music\Album A      | FakeFlac - Album A.csv       |
      | \\?\C:\Music\Album A     | FakeFlac - Album A.csv       |
      | \\?\UNC\nas\music        | FakeFlac.csv                 |

  Scenario: The default file name does not touch the file system
    Given the root "Gone" inside the test folder, which does not exist
    When default_file_name is called with its absolute path
    Then it returns "FakeFlac - Gone.csv"
    And no file or folder is created

  # --- Use case (fakeflac-core, fake exporter) --------------------------------

  Scenario: Nothing to export
    Given no analysed track
    When the export use case runs with the destination "C:\Out\results.csv"
    Then it fails with error "NOT_ANALYZED"
    And the exporter is not called

  Scenario Outline: The destination must be an absolute .csv path
    Given 2 analysed tracks
    When the export use case runs with the destination "<dest>"
    Then <outcome>

    Examples:
      | dest                         | outcome                                                  |
      | C:\Out\results.csv           | the exporter is called once with the 2 tracks in order and the use case returns 2 |
      | C:\Out\RESULTS.CSV           | the exporter is called once and the use case returns 2   |
      | C:\Out\results.txt           | it fails with error "EXPORT_FAILED" and the exporter is not called |
      | C:\Out\results               | it fails with error "EXPORT_FAILED" and the exporter is not called |
      | C:\Out\results.csv.flac      | it fails with error "EXPORT_FAILED" and the exporter is not called |
      | C:\Music\Album A\lp16k.flac  | it fails with error "EXPORT_FAILED" and the exporter is not called |
      | out\results.csv              | it fails with error "EXPORT_FAILED" and the exporter is not called |

  # --- Command (backend around the dialog: run::begin_export, run::export_csv,
  #     test folders) ----------------------------------------------------------
  # Audio files here are generated inside the test, never taken from the
  # corpus: "<id>.flac" / "<id>.wav" are the lossless fixtures of
  # specs/detection/fixtures.md and "<id>.mp3" / "<id>.m4a" the lossy fixtures
  # written by the writers of specs/detection/lossy-fixtures.md, both with the
  # id named. A file named otherwise ("a.flac", "01 – Canción ‘x’.flac") holds
  # the samples of full_band: its content does not matter. These scenarios check
  # that the CSV carries what the analysis reported; the detection outcome of
  # each fixture is covered by detection-lossless.feature, decode-lossy.feature
  # and detection-lossy.feature.
  # "I export" means: run::begin_export on the results kept, then, with the
  # dialog stubbed to return the destination named, run::export_csv.

  @RNF-04
  Scenario: Export after a finished analysis
    Given a test folder "Mixed" holding "full_band.flac", "corrupt.flac" and "lp16k.flac" (fixtures full_band, corrupt and lp16k of specs/detection/fixtures.md) and "mp3_320_lp17k.mp3" (fixture mp3_320_lp17k of specs/detection/lossy-fixtures.md)
    And an analysis of "Mixed" has finished with the default DetectionParams, and its trackDone events were recorded
    When I export and choose "results.csv" in an empty output folder
    Then the export resolves with rows 4 and the absolute path of "results.csv"
    And "results.csv" reads back as the header of csv-export.md section 3 and 4 records of 21 fields each
    And the records are in the index order of their trackDone events
    And in each record the verdict, cutoff_hz, declared_bitrate_kbps, bitrate_mode, estimated_source_bitrate_kbps, reasons and error fields equal the trackDone values of that file, encoded as in csv-export.md sections 3 and 4
    And in each record the file, folder, path, format, codec and size_bytes fields equal the trackDone TrackInfo of that file
    And the "corrupt.flac" record is an error row: its verdict field is "error" and its cutoff_hz, cutoff_steepness_db_per_khz and noise_floor_db fields are empty
    And the "mp3_320_lp17k.mp3" record has non-empty declared_bitrate_kbps and bitrate_mode fields, and the "full_band.flac" record has empty declared_bitrate_kbps, bitrate_mode and estimated_source_bitrate_kbps fields
    And every record's algorithm_version field is the algorithm_version of the DetectionParams the analysis used

  Scenario: Rows follow the scan order, not the arrival order or the table sort
    Given an analysis of a folder with "a.flac", "b.flac" and "c.flac" whose trackDone events arrived in the order "c.flac", "a.flac", "b.flac"
    And the table is sorted by state
    When I export
    Then the records are in the order "a.flac", "b.flac", "c.flac"

  Scenario: A cancelled analysis exports the tracks done
    Given a folder "Big" with 200 FLAC files
    And an analysis of "Big" was cancelled after 18 tracks were done
    When I export
    Then the CSV has the header and 18 records, one per track done, in scan order
    And the export resolves with rows 18

  @RNF-04
  Scenario: A run that failed unexpectedly exports the tracks it reported
    Given an analysis of "Album A" reported 2 tracks and then failed with error "INTERNAL"
    When I export
    Then the CSV has the header and 2 records

  Scenario: A new analysis replaces the results
    Given an analysis of "Album A" with 3 files has finished
    And an analysis of "Album B" with 2 files has finished after it
    When I export
    Then the CSV has the 2 records of "Album B" and none of "Album A"
    And the snapshot's root is the folder "Album B", so the dialog is given default_file_name of it, "FakeFlac - Album B.csv"

  Scenario: An analysis that fails before it starts keeps the previous results
    Given an analysis of "Album A" with 3 files has finished
    And an analysis of the folder "Docs" failed with error "NO_AUDIO_FOUND"
    When I export
    Then the CSV has the 3 records of "Album A"
    And the snapshot's root is the folder "Album A", so the dialog is given default_file_name of it, "FakeFlac - Album A.csv"

  Scenario: No export while an analysis is running
    Given an analysis of "Big" is running
    When I call export_csv
    Then it fails with error "ANALYSIS_RUNNING"
    And the save dialog does not open and no file is written

  Scenario: No export before any analysis
    Given the application has just started
    When I call export_csv
    Then it fails with error "NOT_ANALYZED"
    And the save dialog does not open

  Scenario: No export when the latest run reported no track
    Given an analysis of "Big" was cancelled before any track was done
    When I call export_csv
    Then it fails with error "NOT_ANALYZED"

  Scenario: Cancelling the save dialog writes nothing
    Given an analysis of "Album A" has finished
    When I export and cancel the save dialog
    Then export_csv resolves with null
    And no file is written

  Scenario: A second export while one is in progress
    Given an analysis of "Album A" with 3 files has finished
    And run::begin_export has returned a snapshot and its export guard, as when the dialog is open
    When run::begin_export is called again while that guard lives
    Then the second call fails with error "INTERNAL" and the message "an export is already in progress"
    And no second dialog would open and no file is written for it
    When the first export goes on and I choose "results.csv"
    Then "results.csv" has the 3 records of "Album A" and the first export resolves with rows 3

  Scenario Outline: The export guard is released however the export ends
    Given an analysis of "Album A" with 3 files has finished
    When an export ends by <ending>
    Then a new run::begin_export succeeds

    Examples:
      | ending                                          |
      | resolving with null (dialog cancelled)          |
      | resolving with rows 3                           |
      | rejecting with "EXPORT_FAILED" (results.txt)    |

  Scenario: An export in progress is reported before a running analysis
    Given an analysis of "Album A" with 3 files has finished
    And run::begin_export has returned a snapshot and its export guard
    And an analysis of "Album B" is running
    When run::begin_export is called again
    Then it fails with error "INTERNAL", not "ANALYSIS_RUNNING"

  Scenario: Replacing an existing file
    Given an analysis of "Album A" with 2 files has finished
    And the output folder has a file "results.csv" of 100 000 bytes
    When I export and choose "results.csv", confirming the replacement in the dialog
    Then "results.csv" holds exactly the new document, with nothing of the old file left at its end
    And the output folder holds no other new file

  Scenario: The temporary file never overwrites an existing file
    The first temporary name of an export is "results.csv.<pid>-0.tmp", <pid>
    being the test process id (csv-export.md section 6).
    Given an analysis of "Album A" with 2 files has finished
    And the output folder has a file "results.csv.<pid>-0.tmp" with known content
    When I export and choose "results.csv"
    Then the export resolves with rows 2
    And "results.csv" holds exactly the new document
    And "results.csv.<pid>-0.tmp" keeps its known content
    And the output folder holds no other new file

  Scenario: All temporary names taken
    Given an analysis of "Album A" with 2 files has finished
    And the output folder has the files "results.csv.<pid>-0.tmp" to "results.csv.<pid>-15.tmp", each with known content
    When I export and choose "results.csv"
    Then it fails with error "EXPORT_FAILED"
    And no "results.csv" is created and the 16 files keep their content

  Scenario Outline: A file that cannot be written is a typed error
    The read-only and folder rows fail at the rename, after the temporary
    file was written: it must be deleted (csv-export.md section 6, Cleanup).
    Given an analysis of "Album A" with 2 files has finished
    And <setup>
    When I export and choose "<dest>"
    Then it fails with error "EXPORT_FAILED"
    And <after>
    And no temporary file is left in the output folder

    Examples:
      | setup                                                           | dest                   | after                                                  |
      | the output folder has no subfolder "missing"                    | missing\results.csv    | no file or folder "missing" is created                 |
      | the output folder has a read-only "results.csv" with known content | results.csv         | "results.csv" keeps its content and its read-only attribute |
      | the output folder has a folder named "results.csv"              | results.csv            | the folder "results.csv" is unchanged                  |
      | nothing else                                                    | results.txt            | no file is created                                     |

  Scenario: A run started while the dialog is open does not change the export
    Given an analysis of "Album A" with 3 files has finished
    When I call export_csv
    And an analysis of "Album B" starts before I choose "results.csv" in the dialog
    Then "results.csv" has the 3 records of "Album A"

  @RNF-05
  Scenario: Exporting never touches the audio files
    Given a test folder "Album A" holding "full_band.flac" and "lp16k.flac" (fixtures full_band and lp16k of specs/detection/fixtures.md) and "mp3_320_lp20k5.mp3" (fixture mp3_320_lp20k5 of specs/detection/lossy-fixtures.md)
    And an analysis of "Album A" has finished
    When I export and choose "Album A\results.csv", inside the analysed folder
    Then the size, modification time and content hash of the 3 audio files are unchanged
    And the only new file in "Album A" is "results.csv"

  @RNF-05
  Scenario: Exporting reads no audio file
    Given a test folder "Album A" holding "lp16k.flac" (fixture lp16k of specs/detection/fixtures.md)
    And an analysis of "Album A" has finished and its trackDone event was recorded
    And "lp16k.flac" was deleted after the analysis
    When I export
    Then the export resolves with rows 1
    And the CSV has the "lp16k.flac" record whose verdict, cutoff_hz and size_bytes fields equal the recorded trackDone values

  @RNF-09
  Scenario: Unicode names survive a round-trip
    Given a test folder "Álbum ″Ñandú″" with the files "01 – Canción ‘x’.flac", "02 «Ça va» 🎵.flac" and "03 Live, 1999.flac", each holding the samples of full_band (specs/detection/fixtures.md)
    And an analysis of "Álbum ″Ñandú″" has finished
    When I export and choose "résultats ″1″.csv"
    And I read the file back as UTF-8 with an RFC 4180 reader
    Then the file fields are "01 – Canción ‘x’.flac", "02 «Ça va» 🎵.flac" and "03 Live, 1999.flac", code point for code point
    And each folder field equals the analysed folder path and each path field the analysed file path, code point for code point

  Scenario: Large result sets
    Structure only: no automated test asserts wall-clock time. The 1-second
    budget is checked by hand in a release build (ui/export-csv.md, M7;
    csv-export.md section 7, criterion 5).
    Given 10 000 stubbed TrackAnalysis values with distinct paths
    When they are exported to a file in the output folder
    Then the export resolves with rows 10000
    And the file reads back as the header and exactly 10 000 records of 21 fields
    And the path fields are in the order given

  # --- Interface (stubbed API client; specs/ui/export-csv.md) -------------------

  Scenario Outline: When Export CSV is enabled
    Given the screen is in state "<state>" with <rows> rows in the table
    Then the toolbar button "Export CSV" is <enabled>

    Examples:
      | state     | rows | enabled  |
      | Empty     | 0    | disabled |
      | Scanning  | 0    | disabled |
      | Analysing | 5    | disabled |
      | Finished  | 31   | enabled  |
      | Cancelled | 18   | enabled  |
      | Cancelled | 0    | disabled |
      | Error     | 2    | enabled  |
      | Error     | 0    | disabled |

  Scenario: A successful export is confirmed in the status bar
    Given an analysis of "Album A" has finished with 31 tracks
    When I click "Export CSV" and the export resolves with rows 31 and path "C:\Out\FakeFlac - Album A.csv"
    Then the status bar ends with "· Saved 31 rows to FakeFlac - Album A.csv"
    And the table, the selection and the detail panel are unchanged

  Scenario: A single row uses the singular
    Given an analysis has finished with 1 track
    When I click "Export CSV" and the export resolves with rows 1 and path "C:\Out\one.csv"
    Then the status bar ends with "· Saved 1 row to one.csv"

  Scenario: Export CSV is disabled while the export is in progress
    Given an analysis has finished with 3 tracks
    When I click "Export CSV" and the export has not resolved yet
    Then the toolbar button "Export CSV" is disabled
    And it is enabled again when the export resolves

  Scenario: Cancelling the dialog changes nothing
    Given an analysis has finished and an earlier export showed "Saved 3 rows to a.csv"
    When I click "Export CSV" and the export resolves with null
    Then the status bar still ends with "· Saved 3 rows to a.csv"

  Scenario Outline: A failed export is reported in the status bar
    Given an analysis has finished with 3 tracks
    When I click "Export CSV" and the export rejects with "<code>"
    Then the status bar ends with "· <text>"
    And the table is unchanged

    Examples:
      | code           | text                           |
      | EXPORT_FAILED  | Cannot save the CSV file       |
      | INTERNAL       | Cannot save the CSV file       |
      | NOT_ANALYZED   | There are no results to export |
      | ANALYSIS_RUNNING | An analysis is already running |

  Scenario: The notice is cleared when a new analysis starts
    Given the status bar shows "· Saved 3 rows to a.csv"
    When I start an analysis of "Album B"
    Then the status bar shows "Scanning…" without the notice

  Scenario Outline: A start that fails before Scanning replaces the notice with its error
    Given an analysis has finished with 3 tracks and the status bar ends with "· Saved 3 rows to a.csv"
    When I open the folder "Docs" and start_analysis rejects with "<code>" before any event
    Then the status bar shows the error text of "<code>" (main-screen.md, state Error) and not "Saved 3 rows to a.csv"
    And the table keeps its 3 rows and the toolbar button "Export CSV" is enabled

    Examples:
      | code              |
      | NO_AUDIO_FOUND    |
      | FOLDER_UNREADABLE |
      | NOT_A_FOLDER      |

  Scenario: A drop of files only keeps the notice
    Given an analysis has finished with 3 tracks and the status bar ends with "· Saved 3 rows to a.csv"
    When I drop two files and start_analysis rejects with "NOT_A_FOLDER" for both
    Then the status bar still ends with "· Saved 3 rows to a.csv"
