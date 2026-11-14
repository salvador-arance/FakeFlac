@RF-17
Feature: Explain a Fake verdict (evidence and album context)
  As a user who finds Fake tracks in my library
  I want to see what evidence made each one Fake and how strong it is
  So that I can judge the verdict instead of trusting it blindly

  Spec: specs/detection/fake-evidence.md (the rule, invariants, cache, CSV,
  acceptance X1-X7 and M1-M2, G1 questions Q1-Q16). Contract:
  specs/contracts/commands.md, "Fake evidence (version 9)". Interface and
  texts: specs/ui/fake-evidence.md. RF-17 changes no verdict, reason, cutoff,
  estimate, threshold or version: the default DetectionParams stay
  detection-params.v8.toml and every analysis carries algorithmVersion 8.
  Thresholds of the defaults: gap_fake_db 6 dB, step_fake_db 10 dB (lossless,
  rules V3 and V4), lossy.fake_shortfall_hz 2500 Hz and
  lossy.fake_min_step_db 10 dB (lossy, rule L3). Lossless FullBand: U >= -70,
  B >= 3 and not LossyStep (fs in [14500, 20300) Hz and D >= 3 dB). EdgeStep:
  fs in that range, D_Q < 3, R >= 15 and D < 16 (rules.md section 2).
  Notation. "evidence" lists the items in order, separated by "; ", each
  written "<kind> <value>/<threshold>/<margin>" (dB for gapStep and
  cutoffStep, Hz for shortfall); "—" means an empty evidence. Rule-level
  values are compared within 0.01. "the evidence kinds" lists only the kinds,
  in order. An album note "none" means that the panel shows no album note. The
  "audio note" is the text evidence.audioNote, the last line of every fake
  track's evidence (specs/ui/fake-evidence.md section 2). An
  estimate "none" means no estimated source bitrate (null). A stubbed album
  summary gives the counts of the latest album update of the row's album. Audio files are generated inside the tests
  (specs/detection/fixtures.md, specs/detection/lossy-fixtures.md); every
  cache file is inside the test's own temporary folder.

  # --- The rule from given measurements: lossless (fake-evidence.md 2.1) -------

  Scenario Outline: Evidence of a lossless track from given measurements
    Given a lossless track with codec "flac" and sample rate 44100 Hz
    And the measurements H = <H> dB, G = <G> dB, D = <D> dB, fs = <fs> Hz, fe = <fe> Hz, U = <U> dB, B = <B> dB, D_Q = <D_Q> dB and R = <R> dB
    When the verdict rules and the evidence of the default DetectionParams are applied
    Then the verdict is "<verdict>"
    And the reasons are "<reasons>"
    And the reported cutoff is <cutoff> Hz
    And the evidence is "<evidence>"

    Examples:
      | case                                                   | H   | G     | D     | fs    | fe    | U      | B    | D_Q  | R     | verdict    | reasons                 | cutoff | evidence                                            |
      | gap only, cutoff at full band                          | -20 | 9.6   | 5     | 16050 | 22050 | -90    | 0.2  | 0    | 0     | fake       |                         | 22050  | gapStep 9.60/6.00/3.60                              |
      | gap only, upper content, the lossy step removes guard  | -20 | 8     | 5     | 16050 | 22050 | -50    | 10   | 0    | 0     | fake       |                         | 22050  | gapStep 8.00/6.00/2.00                              |
      | gap only, deep step above the lossy range              | -20 | 19    | 60    | 20450 | 20400 | -100   | 0.1  | 0    | 0     | fake       |                         | 20450  | gapStep 19.00/6.00/13.00                            |
      | step only                                              | -20 | 0.5   | 13.2  | 19450 | 22050 | -90    | 0.2  | 10   | 2     | fake       |                         | 19450  | cutoffStep 13.20/10.00/3.20                         |
      | both: V3 decides, V4 also holds                        | -20 | 12    | 24.7  | 16050 | 22050 | -90    | 0.2  | 10   | 0     | fake       |                         | 16050  | gapStep 12.00/6.00/6.00; cutoffStep 24.70/10.00/14.70 |
      | both at their thresholds                               | -20 | 6     | 10    | 19450 | 22050 | -90    | 0.2  | 10   | 0     | fake       |                         | 19450  | gapStep 6.00/6.00/0.00; cutoffStep 10.00/10.00/0.00 |
      | both just below their thresholds                       | -20 | 5.99  | 9.99  | 19450 | 22050 | -90    | 0.2  | 10   | 0     | suspicious | highBandGaps            | 19450  | —                                                   |
      | gap with an edge step: V3 listed, V4 not               | -20 | 7     | 14.22 | 19450 | 22050 | -47.06 | 2.86 | 0.32 | 17.48 | fake       |                         | 19450  | gapStep 7.00/6.00/1.00                              |
      | W4: the edge step keeps it from Fake                   | -20 | -0.25 | 14.22 | 19450 | 22050 | -47.06 | 2.86 | 0.32 | 17.48 | suspicious | ambiguousCutoff         | 19450  | —                                                   |
      | FP2: the full-band guard keeps it from Fake            | -20 | 13.18 | 14.79 | 21450 | 21400 | -44.74 | 30.6 | 0    | 0     | suspicious | highBandGaps            | 21450  | —                                                   |
      | little treble: V2 comes first                          | -65 | 12    | 24.7  | 16050 | 22050 | -90    | 0.2  | 10   | 0     | suspicious | lowHighFrequencyContent | 16050  | —                                                   |
      | deep step below the lossy range                        | -20 | 0     | 30    | 14450 | 22050 | -90    | 0.2  | 10   | 0     | suspicious | ambiguousCutoff         | 14450  | —                                                   |
      | deep step just above the lossy range                   | -20 | 0     | 30    | 20350 | 22050 | -90    | 0.2  | 10   | 0     | suspicious | ambiguousCutoff         | 20350  | —                                                   |
      | full band, small step                                  | -20 | 0     | 2     | 16050 | 22050 | -50    | 10   | 0    | 0     | real       |                         | 22050  | —                                                   |

  Scenario Outline: The evidence follows the parameters the track was analysed with
    Older params files give their own verdict and their own evidence
    (fake-evidence.md D2); a changed threshold is the threshold of the item.
    Given a lossless track with codec "flac" and sample rate 44100 Hz
    And the measurements H = -20 dB, G = <G> dB, D = <D> dB, fs = <fs> Hz, fe = 22050 Hz, U = <U> dB, B = <B> dB, D_Q = <D_Q> dB and R = <R> dB
    When the verdict rules and the evidence of <params> are applied
    Then the verdict is "<verdict>"
    And the evidence is "<evidence>"

    Examples:
      | case                         | G     | D     | fs    | U      | B    | D_Q  | R     | params                                                               | verdict    | evidence                     |
      | W4 with v5 (no [edge])       | -0.25 | 14.22 | 19450 | -47.06 | 2.86 | 0.32 | 17.48 | the parameters of specs/detection/detection-params.v5.toml           | fake       | cutoffStep 14.22/10.00/4.22  |
      | W4 with v6                   | -0.25 | 14.22 | 19450 | -47.06 | 2.86 | 0.32 | 17.48 | the parameters of specs/detection/detection-params.v6.toml           | suspicious | —                            |
      | FP2 with v2 (no [full_band]) | 13.18 | 14.79 | 21450 | -44.74 | 30.6 | 0    | 0     | the parameters of specs/detection/detection-params.v2.toml           | fake       | gapStep 13.18/6.00/7.18      |
      | a raised gap threshold       | 9.6   | 5     | 16050 | -90    | 0.2  | 0    | 0     | the text of specs/detection/detection-params.v8.toml with verdict.gap_fake_db set to 8.0 | fake | gapStep 9.60/8.00/1.60 |

  # --- The rule from given measurements: lossy (fake-evidence.md 2.2) ----------

  Scenario Outline: Evidence of a lossy track from given measurements
    Given a lossy track with codec "<codec>", sample rate <rate> Hz, declared bitrate <b> kbps and tag low-pass — Hz
    And the measurements H = <H> dB, G = <G> dB, D = <D> dB, fs = <fs> Hz and fe = <fe> Hz
    When the lossy rules and the evidence of the default DetectionParams are applied
    Then the expected cutoff is <c_exp> Hz
    And the verdict is "<verdict>"
    And the reasons are "<reasons>"
    And the estimated source bitrate is <estimate>
    And the evidence is "<evidence>"

    Examples:
      | case                                      | codec | rate  | b   | H   | G  | D    | fs    | fe    | c_exp | verdict    | reasons                 | estimate | evidence                                                 |
      | MP3 320 from 128                          | mp3   | 44100 | 320 | -20 | 0  | 40   | 17050 | 22050 | 20500 | fake       |                         | 128      | shortfall 3450.00/2500.00/950.00; cutoffStep 40.00/10.00/30.00 |
      | a large G adds no gapStep to a lossy file | mp3   | 44100 | 320 | -20 | 19 | 40   | 17050 | 22050 | 20500 | fake       |                         | 128      | shortfall 3450.00/2500.00/950.00; cutoffStep 40.00/10.00/30.00 |
      | both at their thresholds                  | mp3   | 44100 | 320 | -20 | 0  | 10   | 18000 | 22050 | 20500 | fake       |                         | 160      | shortfall 2500.00/2500.00/0.00; cutoffStep 10.00/10.00/0.00 |
      | shortfall just below its threshold        | mp3   | 44100 | 320 | -20 | 0  | 40   | 18001 | 22050 | 20500 | suspicious | cutoffBelowDeclared     | 160      | —                                                        |
      | step just below its threshold             | mp3   | 44100 | 320 | -20 | 0  | 9.99 | 17050 | 22050 | 20500 | suspicious | cutoffBelowDeclared     | 128      | —                                                        |
      | large shortfall without a step: L5        | mp3   | 44100 | 320 | -20 | 0  | 3    | 18050 | 17000 | 20500 | suspicious | gradualRolloff          | 128      | —                                                        |
      | AAC 256 from 128                          | aac   | 44100 | 256 | -20 | 0  | 40   | 15350 | 22050 | 19000 | fake       |                         | 128      | shortfall 3650.00/2500.00/1150.00; cutoffStep 40.00/10.00/30.00 |
      | Opus 160 from a 16 kHz source             | opus  | 48000 | 160 | -20 | 0  | 33   | 16050 | 24000 | 20000 | fake       |                         | none     | shortfall 3950.00/2500.00/1450.00; cutoffStep 33.00/10.00/23.00 |
      | honest MP3 320                            | mp3   | 44100 | 320 | -20 | 0  | 40   | 20450 | 22050 | 20500 | real       |                         | 320      | —                                                        |
      | 48 kHz streaming rip: L6a                 | mp3   | 48000 | 320 | -20 | 0  | 40   | 20050 | 24000 | 20500 | suspicious | streamingSampleRate     | none     | —                                                        |
      | little treble: L2 comes first             | mp3   | 44100 | 320 | -65 | 0  | 40   | 17050 | 22050 | 20500 | suspicious | lowHighFrequencyContent | none     | —                                                        |

  # --- Fixtures ----------------------------------------------------------------

  @RF-04
  Scenario Outline: Evidence kinds of lossless fixtures with the default parameters
    Given the synthetic signal "<id>" saved as FLAC
    When it is analysed with the default parameters
    Then the verdict is "<verdict>"
    And the evidence kinds are "<kinds>"
    And every evidence item has the threshold of its kind in the default DetectionParams, a margin of at least 0, and a margin equal to its value minus its threshold within 0.001

    Examples:
      | id                   | verdict    | kinds      |
      | sfb21_gaps           | fake       | gapStep    |
      | sfb21_gaps_lp20k5    | fake       | gapStep    |
      | shelf19k5            | fake       | cutoffStep |
      | shelf19k5_bursts     | fake       | cutoffStep |
      | lp19k                | fake       | cutoffStep |
      | holes_lp19k5         | fake       | cutoffStep |
      | edge_bump19k3_lp19k5 | fake       | cutoffStep |
      | full_band            | real       | —          |
      | hf_bursts            | real       | —          |
      | gradual              | suspicious | —          |
      | low_hf               | suspicious | —          |
      | silence              | suspicious | —          |
      | sfb21_gaps_weak      | suspicious | —          |
      | sfb21_gaps_aa21k     | suspicious | —          |
      | edge_bump19k3        | suspicious | —          |

  @RF-04
  Scenario: The gap-only fixture with a full-band cutoff
    The case of the 19 isolated gap-only fakes of fake-evidence.md section 1:
    the reported cutoff is the content extent, at Nyquist, and the evidence is
    the gap step at 16 kHz.
    Given the synthetic signal "sfb21_gaps" saved as FLAC
    When it is analysed with the default parameters
    Then the verdict is "fake" with no reasons
    And the reported cutoff is 22050 Hz
    And the evidence is one gapStep item with threshold 6.0 and a value of at least 6.0

  @RF-04 @RF-14
  Scenario Outline: Lossless fixtures whose step is the evidence
    The gap step of these fixtures is not specified by their fixture spec, so
    only the step is asserted.
    Given the <fixture>
    When it is analysed with the default parameters
    Then the verdict is "fake"
    And the evidence kinds contain "cutoffStep" and no kind other than "gapStep" and "cutoffStep"

    Examples:
      | fixture                                         |
      | synthetic signal "lp16k" saved as FLAC          |
      | synthetic signal "lp16k_dither" saved as FLAC   |
      | synthetic signal "lp16k_96k" saved as FLAC      |
      | synthetic signal "bass_tone_lp16k" saved as FLAC |
      | lossy fixture "m4a_alac_lp16k"                  |

  @RF-15
  Scenario Outline: Evidence of lossy fixtures with the default parameters
    Given the lossy fixture "<fixture>"
    When it is analysed with the default parameters
    Then the verdict is "<verdict>"
    And the evidence kinds are "<kinds>"
    And for a "fake" verdict the shortfall value equals <c_exp> Hz minus the reported cutoff within 0.5 Hz, with threshold 2500, and the cutoffStep threshold is 10.0
    And every evidence item has a margin of at least 0, equal to its value minus its threshold within 0.001

    Examples:
      | fixture            | verdict    | kinds                 | c_exp |
      | mp3_320_lp17k      | fake       | shortfall, cutoffStep | 20500 |
      | m4a_aac_256_lp15k3 | fake       | shortfall, cutoffStep | 19000 |
      | opus_160_src16k    | fake       | shortfall, cutoffStep | 20000 |
      | mp3_320_lp20k5     | real       | —                     | 20500 |
      | mp3_320_lp18k5     | suspicious | —                     | 20500 |
      | mp3_320_48k_lp20k  | suspicious | —                     | 20500 |
      | mp3_320_low_hf     | suspicious | —                     | 20500 |
      | mp3_silence        | suspicious | —                     | 20500 |
      | opus_128_fb        | real       | —                     | 20000 |
      | opus_160_swb       | suspicious | —                     | 20000 |

  @RF-04
  Scenario: An older params file gives its own evidence on a fixture
    Given the synthetic signal "edge_bump19k3" saved as FLAC
    When it is analysed with the parameters of specs/detection/detection-params.v5.toml and with the default parameters
    Then with v5 the verdict is "fake" and the evidence kinds are "cutoffStep"
    And with the defaults the verdict is "suspicious" with reasons "ambiguousCutoff" and the evidence is "—"

  @RNF-04
  Scenario Outline: Error tracks carry no evidence
    Given the <fixture>
    When it is analysed with the default parameters
    Then the verdict is "error" with error "<error>"
    And the evidence is "—"

    Examples:
      | fixture                            | error         |
      | synthetic signal "short" saved as FLAC   | tooShort      |
      | synthetic signal "corrupt" saved as FLAC | corruptStream |
      | lossy fixture "mp3_corrupt"        | corruptStream |
      | lossy fixture "m4a_corrupt"        | corruptStream |
      | lossy fixture "mp3_320_48k_short"  | tooShort      |
      | lossy fixture "opus_short"         | tooShort      |

  # --- Invariants and unchanged results (fake-evidence.md 2.4 and 5) ----------

  @RF-04 @RF-15
  Scenario: The evidence agrees with the measurements on every fixture and params file
    Given every synthetic signal of specs/detection/fixtures.md saved as FLAC and every lossy fixture of specs/detection/lossy-fixtures.md sections 2 and 2.2
    When each one is analysed with the default parameters and with the parameters of each of specs/detection/detection-params.v2.toml to specs/detection/detection-params.v7.toml
    Then in every result the evidence is not empty exactly when the verdict is "fake"
    And its kinds are exactly the rules of fake-evidence.md section 2 whose conditions hold on the measurements of that analysis with those parameters, in the order of that section, without repetition
    And every item has a finite value, the threshold of its kind in those parameters, and a margin of at least 0 equal to its value minus its threshold within 0.001

  @RF-04 @RF-15
  Scenario: RF-17 changes no verdict, reason, cutoff, estimate or version of any fixture
    Given every synthetic signal of specs/detection/fixtures.md saved as FLAC and every lossy fixture of specs/detection/lossy-fixtures.md sections 2 and 2.2
    When each one is analysed with the default parameters
    Then its verdict, reasons, error, estimated source bitrate and cutoff (within the tolerance of its fixture spec) are the truth its fixture spec gives
    And every analysis carries algorithmVersion 8
    And the default DetectionParams of the code are equal to specs/detection/detection-params.v8.toml

  # --- In the app: a real run (src-tauri) ---------------------------------------

  @RNF-09 @RNF-05 @RNF-04 @RF-10
  Scenario: A gap-only fake in a real run with Unicode names, an error track and its album
    Given a folder "Álbum ″Ñandú″" with the synthetic signal "sfb21_gaps" saved as "01 – Canción ‘x’.flac", the synthetic signal "full_band" saved as "02 – Ópera «Ça va» 🎵.flac" and as "03 – Ñandú.flac", and the synthetic signal "corrupt" saved as "04 – Roto.flac"
    When I analyse "Álbum ″Ñandú″" with the default parameters
    Then the trackDone event of "01 – Canción ‘x’.flac" carries verdict "fake", no reasons, the evidence kinds "gapStep" and algorithmVersion 8
    And the trackDone events of "02 – Ópera «Ça va» 🎵.flac" and "03 – Ñandú.flac" carry verdict "real" and an empty evidence
    And the trackDone event of "04 – Roto.flac" carries verdict "error" with error "corruptStream" and an empty evidence
    And the album update of "Álbum ″Ñandú″" after the four tracks has real 2, suspicious 0, fake 1, errors 1 and verdict "fake"
    And every path in the events equals the name on disk code point for code point
    And the size, modification time and content hash of every file are unchanged

  # --- Contract (version 9) -----------------------------------------------------

  Scenario: The contract of version 9
    Given a core TrackAnalysis with verdict "fake" and evidence "gapStep 9.5/6.0/3.5; cutoffStep 13.25/10.0/3.25"
    And a core TrackAnalysis with verdict "real" and an empty evidence
    When each is mapped to the DTO and serialised to JSON
    Then the first has "evidence":[{"kind":"gapStep","value":9.5,"threshold":6.0,"margin":3.5},{"kind":"cutoffStep","value":13.25,"threshold":10.0,"margin":3.25}]
    And the second has "evidence":[]
    And both still have the keys "reasons", "cutoff", "estimatedSourceBitrateKbps", "error" and "algorithmVersion" with their version 8 meaning
    And the generated bindings declare EvidenceKind as "gapStep" | "cutoffStep" | "shortfall", FakeEvidence with the fields kind, value, threshold and margin, and the field evidence of TrackAnalysis as FakeEvidence[]
    And SuspicionReason has the same members as in contract version 8

  # --- Result cache (fake-evidence.md 6.3, result-cache.md section 3) ----------

  @RF-11
  Scenario Outline: A cached analysis is reused only when its evidence matches its verdict
    Given a scan entry "a.flac" with size 1000, mtime 1700000000 and format flac
    And the store fake returns for it an analysis with the run's algorithm_version, verdict "<verdict>" and evidence "<evidence>"
    When analyze_folder_cached runs with ResultCache::Store of the fake
    Then the decoder spy is called <calls> for "a.flac"
    And the summary has fromCache <from cache> and cache "ok"

    Examples:
      | verdict    | evidence                    | calls | from cache |
      | real       | —                           | never | 1          |
      | suspicious | —                           | never | 1          |
      | error      | —                           | never | 1          |
      | fake       | cutoffStep 24.7/10.0/14.7   | never | 1          |
      | fake       | —                           | once  | 0          |
      | real       | gapStep 9.6/6.0/3.6         | once  | 0          |

  @RF-11
  Scenario: A cached fake without evidence is analysed again and replaced
    Given the store fake holds for "a.flac" an analysis with verdict "fake", an empty evidence and the entry's size, mtime, format and algorithm_version
    And the decoder spy returns for "a.flac" the band spectra of a cliff at 16 kHz
    When analyze_folder_cached runs with ResultCache::Store of the fake
    Then the trackDone of "a.flac" carries verdict "fake" and a non-empty evidence
    And the store fake received exactly 1 put, for "a.flac", with the analysis sent in its trackDone

  @RF-11
  Scenario: A stored row without the evidence key parses with an empty evidence
    Rows written before RF-17 have no "evidence" key in analysis_json.
    Given a SqliteResultStore whose row for "a.flac", under the current engine, holds the analysis_json of a "real" analysis without the key "evidence"
    When the store's get is called for "a.flac" with the row's size and mtime
    Then it returns that analysis with an empty evidence, and no failure

  @RF-11 @RNF-09
  Scenario: A warm run over rows written before RF-17 analyses only the fake tracks again
    Given a folder "Álbum ″Ñandú″" with the synthetic signal "sfb21_gaps" saved as "01 – Canción ‘x’.flac" and the synthetic signal "full_band" saved as "02 – Ñandú.flac"
    And run 1 with the default parameters and a cache file has stored both tracks
    And the key "evidence" is removed from the analysis_json of both rows
    When run 2 analyses the folder with the same cache file
    Then run 2 makes exactly 1 analyze call, for "01 – Canción ‘x’.flac"
    And run 2's summary has fromCache 1 and cache "ok"
    And the trackDone events of run 2 equal those of run 1, evidence included
    And afterwards the row of "01 – Canción ‘x’.flac" holds the key "evidence" again

  @RF-11
  Scenario: Evidence round-trips exactly through the cache
    Given stubbed fake analyses whose evidence values, thresholds and margins include 9.6, 6.0, 3.6000004, 3450.0, 2500.0, 950.0 and 1e-7
    When each is put and then got back through a SqliteResultStore
    Then each one read back is equal (==) to the one stored, every f32 bit for bit

  # --- CSV export: unchanged (fake-evidence.md Q8) ------------------------------

  @RF-09 @RNF-09
  Scenario: The evidence of a fake track is not exported
    Given a stubbed TrackAnalysis with path "C:\Music\Álbum ″Ñandú″\01 – Canción ‘x’.flac", folder "C:\Music\Álbum ″Ñandú″", format "flac", codec "flac", sample rate 44100, bit depth 16, 2 channels, duration 240000 ms, size 30000000 bytes, no declared bitrate, no bitrate mode, no estimated source bitrate, cutoff 22050 Hz with steepness 0.0 dB/kHz and noise floor -95.0 dB, verdict "fake", no reasons, evidence "gapStep 9.6/6.0/3.6", no error and algorithmVersion 8
    When the CSV document of that track is built
    Then its second record is exactly:
      """
      fake,01 – Canción ‘x’.flac,C:\Music\Álbum ″Ñandú″,flac,44100,16,240000,22050,,,,,,,flac,2,30000000,0.0,-95.0,8,C:\Music\Álbum ″Ñandú″\01 – Canción ‘x’.flac
      """
    And every record of the document has 21 fields
    And the document is byte-identical to the one built for the same track with an empty evidence

  # --- Calibration: unchanged (fake-evidence.md 6.4) ----------------------------

  @RP-11
  Scenario: The calibration reports do not change
    Given the sample corpus of specs/calibration/sample-corpus.md
    When the calibration tool runs with its default params file
    Then the header of tracks.csv has no column whose name contains "evidence"
    And the reasons cell of every row whose verdict is "fake" is empty
    And summary.md names no file, folder or artist

  # --- Interface (specs/ui/fake-evidence.md) -----------------------------------

  @RF-07
  Scenario: Detail panel of a gap-only fake whose cutoff is near full band
    Given a stubbed TrackAnalysis of "01 – Canción ‘x’.flac" with format "flac", sample rate 44100, cutoff 22000 Hz, verdict "fake", no reasons and evidence "gapStep 9.6/6.0/3.6"
    And the summary of its album has real 13, suspicious 0, fake 1 and errors 0
    And the interface language is English
    When I select its row
    Then the panel shows "Cutoff 22.0 kHz · Fake"
    And below it the panel shows these lines, in this order:
      | Gaps above 16 kHz, as in an MP3: gap step 9.6 dB, 3.6 dB over the Fake threshold (6.0 dB)                          |
      | The evidence is the gap pattern at 16 kHz, not the cutoff.                                                          |
      | The only Fake of the 14 analysed tracks in this album. Transcodes usually affect whole albums: check its spectrogram. |
      | These traces are in the audio itself. They cannot tell a converted file from a genuine release that was mastered from lossy audio. |

  @RF-07 @RF-13
  Scenario: The same detail panel in Spanish
    Given a stubbed TrackAnalysis of "01 – Canción ‘x’.flac" with format "flac", sample rate 44100, cutoff 22000 Hz, verdict "fake", no reasons and evidence "gapStep 9.6/6.0/3.6"
    And the summary of its album has real 13, suspicious 0, fake 1 and errors 0
    And the interface language is Spanish
    When I select its row
    Then the panel shows "Corte 22.0 kHz · Falso"
    And below it the panel shows these lines, in this order:
      | Huecos por encima de 16 kHz, como en un MP3: escalón de huecos de 9.6 dB, 3.6 dB por encima del umbral de Falso (6.0 dB) |
      | La prueba es el patrón de huecos a partir de 16 kHz, no el corte.                                                         |
      | Es la única pista falsa de las 14 analizadas en este álbum. Las transcodificaciones suelen afectar a álbumes enteros: revisa su espectrograma. |
      | Estas huellas están en el propio audio. No distinguen un archivo convertido de una edición auténtica cuyo máster se hizo con audio con pérdida. |

  @RF-07
  Scenario: Detail panel of a fake with both kinds in an album where every track is fake
    Given a stubbed TrackAnalysis with format "flac", sample rate 44100, cutoff 16000 Hz, verdict "fake", no reasons and evidence "gapStep 12.0/6.0/6.0; cutoffStep 24.7/10.0/14.7"
    And the summary of its album has real 0, suspicious 0, fake 12 and errors 0
    When I select its row
    Then the panel shows "Cutoff 16.0 kHz · Fake"
    And below it the panel shows these lines, in this order:
      | Gaps above 16 kHz, as in an MP3: gap step 12.0 dB, 6.0 dB over the Fake threshold (6.0 dB)          |
      | Sharp cutoff where lossy encoders cut: step 24.7 dB, 14.7 dB over the Fake threshold (10.0 dB)       |
      | These traces are in the audio itself. They cannot tell a converted file from a genuine release that was mastered from lossy audio. |
    And the panel shows no gap-only sentence and no album note

  @RF-07 @RF-15
  Scenario: Detail panel of a lossy fake
    Given a stubbed TrackAnalysis with format "mp3", codec "mp3", sample rate 44100, declared bitrate 320 kbps, bitrate mode "cbr", estimated source bitrate 128, cutoff 17000 Hz, verdict "fake", no reasons and evidence "shortfall 3500/2500/1000; cutoffStep 40.0/10.0/30.0"
    And the summary of its album has real 9, suspicious 0, fake 3 and errors 0
    When I select its row
    Then the panel shows "Cutoff 17.0 kHz · 320 kbps · ≈128 kbps source · Fake"
    And below it the panel shows these lines, in this order:
      | Content ends 3.5 kHz below what its bitrate keeps: 1.0 kHz over the Fake threshold (2.5 kHz)                     |
      | Sharp cutoff where lossy encoders cut: step 40.0 dB, 30.0 dB over the Fake threshold (10.0 dB)                     |
      | 3 of the 12 analysed tracks in this album are Fake. Transcodes usually affect whole albums: check their spectrograms. |
      | These traces are in the audio itself. They cannot tell a converted file from a genuine release that was mastered from lossy audio. |

  @RF-07 @RF-10
  Scenario Outline: The album note of a fake track
    Given a stubbed TrackAnalysis with verdict "fake" and evidence "cutoffStep 13.2/10.0/3.2"
    And the summary of its album has real <real>, suspicious <suspicious>, fake <fake> and errors <errors>
    When I select its row
    Then the album note is "<note>"

    Examples:
      | real | suspicious | fake | errors | note                                                                                                                  |
      | 13   | 0          | 1    | 0      | The only Fake of the 14 analysed tracks in this album. Transcodes usually affect whole albums: check its spectrogram.  |
      | 0    | 1          | 1    | 0      | The only Fake of the 2 analysed tracks in this album. Transcodes usually affect whole albums: check its spectrogram.   |
      | 1    | 0          | 1    | 3      | The only Fake of the 2 analysed tracks in this album. Transcodes usually affect whole albums: check its spectrogram.   |
      | 9    | 2          | 3    | 0      | 3 of the 14 analysed tracks in this album are Fake. Transcodes usually affect whole albums: check their spectrograms.  |
      | 1    | 0          | 11   | 0      | 11 of the 12 analysed tracks in this album are Fake. Transcodes usually affect whole albums: check their spectrograms. |
      | 0    | 0          | 12   | 0      | none                                                                                                                  |
      | 0    | 0          | 12   | 2      | none                                                                                                                  |
      | 0    | 0          | 1    | 0      | none                                                                                                                  |
      | 0    | 0          | 1    | 4      | none                                                                                                                  |

  @RF-07
  Scenario: Margins are written with one decimal
    Given a stubbed TrackAnalysis with verdict "fake" and evidence "cutoffStep 10.04/10.0/0.04"
    When I select its row
    Then below the verdict line the panel shows "Sharp cutoff where lossy encoders cut: step 10.0 dB, 0.0 dB over the Fake threshold (10.0 dB)"

  @RF-07
  Scenario: Tracks that are not fake show no evidence and no album note
    Given a stubbed TrackAnalysis with format "flac", sample rate 44100, cutoff 21500 Hz, verdict "suspicious", reasons "highBandGaps" and an empty evidence
    And the summary of its album has real 13, suspicious 1, fake 0 and errors 0
    When I select its row
    Then the panel shows "Cutoff 21.5 kHz · Suspicious · Frequent gaps above 16 kHz, as in an MP3"
    And the panel shows no evidence line, no gap-only sentence, no album note and no audio note

  @RF-07
  Scenario: Empty state: with no row selected there is no evidence and no note
    Given a finished run whose rows include a fake track with evidence
    And no row is selected
    Then the detail panel shows its nothing-selected state, with no evidence line and no album note

  @RF-07
  Scenario: The evidence stays when the spectrogram cannot be shown
    Given the selected row is a fake track with evidence "cutoffStep 13.2/10.0/3.2" whose album has real 13 and fake 1
    When its spectrogram request fails
    Then the panel shows, in this order, the verdict line, the evidence line, the album note, the audio note and "Cannot show the spectrogram of this file"

  @RF-07 @RF-10
  Scenario: The album note follows the album's latest update during a run
    Given a run is analysing "Album A", whose row "01.flac" is fake with evidence and is selected
    And the latest album update of "Album A" has real 0, suspicious 0, fake 1 and errors 0
    Then the panel shows no album note
    When the next flush applies an album update of "Album A" with real 1, suspicious 0, fake 1 and errors 0
    Then the album note is "The only Fake of the 2 analysed tracks in this album. Transcodes usually affect whole albums: check its spectrogram."

  @RF-07 @RF-08
  Scenario: Filters and sort do not change the album note
    Given a finished run with "Album A" of 13 real tracks and 1 fake track with evidence
    And the verdict filter shows only fake tracks and the table is sorted by cutoff
    When I select the fake row
    Then the album note is "The only Fake of the 14 analysed tracks in this album. Transcodes usually affect whole albums: check its spectrogram."

  @RF-06
  Scenario: The State cell's tooltip of a fake row shows its evidence
    Given stubbed rows "a.flac" with verdict "fake" and evidence "gapStep 12.0/6.0/6.0; cutoffStep 24.7/10.0/14.7", and "b.flac" with verdict "suspicious", reasons "ambiguousCutoff" and an empty evidence
    When the results table shows them
    Then hovering the state cell of "a.flac" shows a tooltip that contains "Gaps above 16 kHz, as in an MP3: gap step 12.0 dB, 6.0 dB over the Fake threshold (6.0 dB)" and "Sharp cutoff where lossy encoders cut: step 24.7 dB, 14.7 dB over the Fake threshold (10.0 dB)"
    And hovering the state cell of "b.flac" shows a tooltip that contains "Sharp cutoff at a high frequency"
    And both state cells keep their label, icon and colour token ("Fake" and "Suspicious")

  @RF-10
  Scenario: The Albums panel does not change
    Given a finished run with the album "Artist\Album 1" of 13 real tracks and 1 fake track with evidence
    Then its item shows the verdict "Fake" and the counts "14 tracks · 13 real · 1 fake"
    And its verdict cell has no evidence in its tooltip
