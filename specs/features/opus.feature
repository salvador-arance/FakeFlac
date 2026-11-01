@RF-14 @RF-15
Feature: Opus files (detection v8)
  Opus in Ogg (.opus) is decoded by the audio adapter with rusty-opus =1.0.2,
  without external tools, and gives the same TrackAnalysis contract as the other
  formats (ADR-0047; specs/contracts/commands.md, "Opus files (detection v8)").
  The decoder limits its f32 output to [-1, 1] (lossy-source.md section 2.4.3,
  accepted at G1); the fixtures' input peaks at -1 dBFS, and their truths were
  re-measured at G1 with this decoder (lossy-source.md section 12.8 F8).
  The adapter reads the Ogg layer itself: pages with their CRC-32, packets,
  OpusHead and OpusTags, granule positions (specs/detection/lossy-source.md
  section 2.4). Only channel mapping family 0 (mono and stereo) is decoded;
  every other family is unsupportedFormat. Only ".opus" is scanned.
  Rules: lossy-source.md section 12 with DetectionParams v8 (the defaults,
  detection-params.v8.toml = v7 plus the "opus" rows of [lossy.cutoffs]):
  every Opus row from 48 to 192 kbps has cutoff 20000 Hz, there is no row
  below 48 kbps, and the rest is the v5 lossy rules (detection-lossy.feature):
  fc = fs when D >= 7 dB, otherwise fe; c_exp = min(table row of the declared
  bitrate, nyquist - 400) (Opus has no tag low-pass); s = c_exp - fc;
  L3 fake when s >= 2500 Hz and D >= 10 dB; L4 suspicious (cutoffBelowDeclared)
  when s >= 1000 Hz and D >= 7 dB; L5 suspicious (gradualRolloff) when
  s >= 1000 Hz; L2 suspicious (lowHighFrequencyContent) when H < -60 dB; L6a
  never applies to Opus ("opus" cannot be listed in lossy.streaming.codecs).
  The estimate of an Opus track is its declared bitrate when it is real, and
  none (null) when L2-L5 fire (lossy-source.md section 12.4, Q29).
  The Opus rows were measured before G1 and their first row decided at G1
  (lossy-source.md Q30, sections 12.3 and 12.8: honest libopus encodes report
  fc 20050 or 20150 Hz at 16-320 kbps; the table starts at 48 kbps); acceptance
  O checks them on every run (section 12.7). At 160 kbps or less libopus hides
  a band-limited source's edge by band folding unless the decoder's output
  limit clips the decoded signal (section 12.6), so the rule-level cases "from
  an MP3 128" and "from an MP3 192" below use the cutoffs measured on Opus 256
  cascades (the same medians with either decoder, section 12.8 F8).
  Test files: the Opus fixtures of specs/detection/lossy-fixtures.md section
  2.2, encoded inside the tests by rusty-opus's own encoder and written to Ogg
  by a test-support writer that shares no code with the adapter's reader
  (ADR-0047 point 3). The test stage first confirms the checks of
  lossy-fixtures.md section 3 for them and returns to G1 if one fails.
  Steps "When the adapter decodes them" look at the samples the adapter feeds
  to the spectrum stage, per channel and before the mono mix (lossy-source.md
  section 2.4.3). The public API only returns the AudioProbe, so the test stage
  chooses how to reach them (for example a hidden test-only function of the
  Opus module, or unit tests in crates/fakeflac-audio/src/ fed with fixture
  bytes) and declares it in its commit; no verdict depends on that choice.
  In the measurement tables, fs "—" means fs is undefined (D = 0), c_exp "none"
  means the expected cutoff is undefined and an estimate "none" means no
  estimated source bitrate is reported (null). The rows "s exactly
  fake_shortfall_hz" and "s exactly suspicious_shortfall_hz" give fs values
  (17500, 19000) that a real analysis never reports (with D >= 7 dB fs is a band
  centre ending in 50); they test the comparisons as unit boundaries.
  The cutoffs, steps and H of the encoded fixtures were measured before G1
  with the fixture recipes (lossy-source.md section 12.8 F4, F5; lossy-fixtures.md
  section 2.2); a check of lossy-fixtures.md section 3 that contradicts a row of
  this feature returns it to G1. The fixtures use pre-skip 120, the measured
  delay of rusty-opus 1.0.2's encoder (section 12.8 F7).

  # --- Scan ---------------------------------------------------------------------

  @RF-02 @RF-14
  Scenario: Opus files are found; other Ogg extensions are not
    Given a folder "Library" containing "a.flac", "b.opus", "c.OPUS", "d.ogg", "e.oga" and "notes.txt", and a subfolder "CD2" containing "f.Opus"
    When I analyse "Library"
    Then 4 files are scanned: "a.flac", "b.opus", "c.OPUS" and "CD2/f.Opus"

  @RF-02 @RF-14
  Scenario: A folder with only Opus files is analysed
    Given a folder "Opus only" with the Opus fixture "opus_128_fb" saved as "a.opus"
    When I analyse "Opus only"
    Then the analysis finishes with 1 track and errors 0

  # --- Stream information (lossy-source.md section 2.4) ----------------------------

  @RF-14
  Scenario Outline: Stream information of Opus fixtures
    Opus always decodes at 48 kHz, whatever input rate its OpusHead records.
    The declared bitrate is the total size of the audio packets (Ogg page
    headers and the two header packets excluded) over the duration
    (granule position of the last page minus pre-skip, at 48 kHz).
    Given the Opus fixture "<fixture>"
    When it is analysed
    Then the TrackInfo has format "opus", codec "opus", sample rate 48000 Hz, <channels> channel(s), no bit depth and no bitrate mode
    And its durationMs is <duration_ms>
    And its declared bitrate is round(8 x the fixture's audio packet bytes / its duration in s / 1000), which is within 1 kbps of <kbps>

    Examples:
      | fixture             | channels | duration_ms | kbps |
      | opus_128_fb         | 1        | 30000       | 128  |
      | opus_128_fb_stereo  | 2        | 30000       | 128  |
      | opus_64_fb          | 1        | 30000       | 64   |
      | opus_160_src16k     | 1        | 30000       | 160  |
      | opus_128_fb_preskip | 1        | 29980       | 128  |

  @RF-14
  Scenario: The input sample rate of OpusHead is informational only
    opus_128_fb_rate44k is opus_128_fb with 44100 in the input sample rate
    field of its OpusHead.
    Given the Opus fixtures "opus_128_fb" and "opus_128_fb_rate44k"
    When both are analysed
    Then the TrackInfo of "opus_128_fb_rate44k" has sample rate 48000 Hz
    And their TrackAnalysis are equal except for the path

  @RF-14
  Scenario: Pre-skip and end trim are applied
    opus_128_fb has 1501 packets of 960 samples (1 440 960 samples), pre-skip
    120 and a last granule position of 1 440 120, so the end trim is 840 samples.
    opus_128_fb_preskip is the same stream with pre-skip 1080 (120 + 960) and
    the same granule positions.
    Given the Opus fixtures "opus_128_fb" and "opus_128_fb_preskip"
    When the adapter decodes them
    Then "opus_128_fb" gives exactly 1 440 000 samples per channel: the decoder output with its first 120 and last 840 samples removed
    And "opus_128_fb_preskip" gives exactly 1 439 040 samples per channel, equal to those of "opus_128_fb" from sample 960 on

  @RF-14
  Scenario: The output gain of OpusHead is applied to every sample
    The gain is a Q7.8 value in dB: -5120 is -20 dB, a factor of 0.1.
    Given the Opus fixtures "opus_128_fb" and "opus_128_fb_gain_m20"
    When the adapter decodes them
    Then every decoded sample of "opus_128_fb_gain_m20" equals 0.1 times the same sample of "opus_128_fb" within 1e-6 (absolute, full scale 1.0)
    And both have the same verdict, reasons and cutoff

  @RF-14
  Scenario: An output gain that makes the track silent
    The gain of -15360 (-60 dB) puts every frame of the signal (about -15 dBFS
    RMS: -12 dBFS scaled to a -1 dBFS peak, lossy-fixtures.md section 1.3) near
    -75 dBFS, below silence_dbfs (-60 dBFS): the gain is applied before the
    analysis, so the track has no frame to analyse.
    Given the Opus fixture "opus_128_fb_gain_m60"
    When it is analysed
    Then the verdict is "suspicious" with reasons "lowHighFrequencyContent"
    And no cutoff and no estimated source bitrate are reported

  @RF-14
  Scenario: A packet split across pages is reassembled
    opus_128_fb_split holds the packets of opus_128_fb in pages whose bodies are
    closed before they would pass 1000 bytes, whatever the packet boundaries
    (lossy-fixtures.md section 1.4), so most pages end inside a packet.
    Given the Opus fixtures "opus_128_fb" and "opus_128_fb_split"
    When both are analysed
    Then their TrackAnalysis are equal except for the path and the size
    And the adapter decodes the same samples from both

  # --- Verdicts with DetectionParams v8 (lossy-source.md section 12) ----------------

  @RF-15
  Scenario Outline: Verdict of Opus fixtures
    Given the Opus fixture "<fixture>"
    When it is analysed with DetectionParams v8
    Then the verdict is "<verdict>"
    And the reasons are "<reasons>"
    And the estimated source bitrate is <estimate>
    And the estimated cutoff is <cutoff>
    And the TrackAnalysis carries algorithmVersion 8

    Examples:
      | fixture            | verdict    | reasons                 | estimate | cutoff             |
      | opus_128_fb        | real       |                         | 128      | 20000 Hz ± 200 Hz  |
      | opus_128_fb_stereo | real       |                         | 128      | 20000 Hz ± 200 Hz  |
      | opus_64_fb         | real       |                         | 64       | 20000 Hz ± 200 Hz  |
      | opus_160_src16k    | fake       |                         | none     | 16000 Hz ± 200 Hz  |
      | opus_160_swb       | suspicious | lowHighFrequencyContent | none     | 12000 Hz ± 200 Hz  |

  @RF-15 @RF-16
  Scenario: An honest Opus file is never flagged by the streaming sample rate rule
    Opus is natively 48 kHz and its full band ends inside the L6a window
    [19800, 20200) Hz, but "opus" is not in lossy.streaming.codecs.
    Given the Opus fixture "opus_128_fb"
    When it is analysed with DetectionParams v8
    Then the reasons do not contain "streamingSampleRate"

  @RF-15
  Scenario: An Opus file analysed with DetectionParams v7 is an error
    detection-params.v7.toml has no Opus row (rule L0).
    Given the Opus fixture "opus_128_fb"
    When it is analysed with the parameters of specs/detection/detection-params.v7.toml
    Then the verdict is "error" with error "unsupportedFormat" and detail "no [lossy.cutoffs] rows for opus"
    And the TrackAnalysis carries algorithmVersion 7

  @RF-15
  Scenario Outline: Opus verdicts from given measurements
    Given a lossy track with codec "opus", sample rate 48000 Hz, declared bitrate <b> kbps and tag low-pass — Hz
    And the measurements H = <H> dB, G = 0 dB, D = <D> dB, fs = <fs> Hz and fe = <fe> Hz
    When the verdict of DetectionParams v8 is computed for the track
    Then the expected cutoff is <c_exp>
    And the verdict is "<verdict>"
    And the reasons are "<reasons>"
    And the estimated source bitrate is <estimate>

    Examples:
      | case                                       | b   | H   | D  | fs    | fe    | c_exp | verdict    | reasons                 | estimate |
      | honest full band                           | 128 | -20 | 40 | 19950 | 24000 | 20000 | real       |                         | 128      |
      | honest, edge reported above 20 kHz          | 160 | -20 | 40 | 20150 | 24000 | 20000 | real       |                         | 160      |
      | Opus 256 from an MP3 128 (measured median) | 256 | -20 | 40 | 16650 | 24000 | 20000 | fake       |                         | none     |
      | s exactly fake_shortfall_hz                | 160 | -20 | 40 | 17500 | 24000 | 20000 | fake       |                         | none     |
      | s just below fake_shortfall_hz             | 160 | -20 | 40 | 17550 | 24000 | 20000 | suspicious | cutoffBelowDeclared     | none     |
      | Opus 256 from an MP3 192 (measured median) | 256 | -20 | 40 | 18750 | 24000 | 20000 | suspicious | cutoffBelowDeclared     | none     |
      | s exactly suspicious_shortfall_hz          | 128 | -20 | 40 | 19000 | 24000 | 20000 | suspicious | cutoffBelowDeclared     | none     |
      | s just below suspicious_shortfall_hz       | 128 | -20 | 40 | 19050 | 24000 | 20000 | real       |                         | 128      |
      | large shortfall, step not deep enough      | 128 | -20 | 8  | 16050 | 24000 | 20000 | suspicious | cutoffBelowDeclared     | none     |
      | no step, content fades at 18.5 kHz         | 128 | -20 | 0  | —     | 18500 | 20000 | suspicious | gradualRolloff          | none     |
      | little treble                              | 128 | -70 | 40 | 12050 | 24000 | 20000 | suspicious | lowHighFrequencyContent | none     |
      | first row                                  | 48  | -20 | 40 | 19950 | 24000 | 20000 | real       |                         | 48       |
      | just below the first row: not judged       | 40  | -20 | 40 | 16650 | 24000 | none  | real       |                         | 40       |
      | below the first row: not judged            | 16  | -20 | 40 | 12050 | 24000 | none  | real       |                         | 16       |
      | between rows uses the lower row            | 140 | -20 | 40 | 17050 | 24000 | 20000 | fake       |                         | none     |
      | above the last row uses the last row       | 300 | -20 | 40 | 19950 | 24000 | 20000 | real       |                         | 300      |
      | 48 kHz edge in the L6a window              | 256 | -20 | 40 | 20050 | 24000 | 20000 | real       |                         | 256      |

  # --- Parameter files ----------------------------------------------------------------

  @RF-15
  Scenario: The defaults are the v8 file
    Given the default DetectionParams of the code
    Then they are equal to specs/detection/detection-params.v8.toml
    And their sections and values other than lossy.cutoffs.opus are equal to those of specs/detection/detection-params.v7.toml
    And lossy.cutoffs.opus has 6 rows, for 48, 64, 96, 128, 160 and 192 kbps, each with cutoff_hz 20000
    And every TrackAnalysis computed with them carries algorithmVersion 8

  @RF-15 @RF-04
  Scenario: v8 gives the v7 results on every file that is not Opus
    Given every synthetic signal of specs/detection/fixtures.md saved as FLAC and every MP3 and M4A fixture of specs/detection/lossy-fixtures.md
    When each one is analysed with the parameters of specs/detection/detection-params.v7.toml and with DetectionParams v8
    Then the two results have the same verdict, reasons, cutoff and estimated source bitrate for every file
    And they carry algorithmVersion 7 and 8

  @RF-15
  Scenario Outline: Invalid Opus rows are rejected
    Given the text of specs/detection/detection-params.v8.toml with <change>
    When it is deserialised into DetectionParams and validated
    Then it is rejected by validation
    And the error names "<what>"

    Examples:
      | change                                                    | what                                     |
      | the opus rows for 48 and 64 kbps swapped                  | lossy.cutoffs.opus: bitrate_kbps increases |
      | the cutoff_hz of the opus row for 192 kbps set to 19000.0 | lossy.cutoffs.opus: cutoff_hz does not decrease |
      | the cutoff_hz of the opus row for 48 kbps set to 0.0      | lossy.cutoffs.opus: cutoff_hz > 0         |

  # --- Corrupt and unsupported files (RNF-04; lossy-source.md section 2.4.4) ----------

  @RF-14 @RNF-04
  Scenario Outline: Broken Ogg or Opus structure is corruptStream
    The whole Ogg structure is read and checked before any packet is decoded;
    the first failing check gives the detail. Every fixture is a variant of
    opus_128_fb (or of opus_128_fb_split) described in lossy-fixtures.md
    section 2.2.
    Given the Opus fixture "<fixture>"
    When it is analysed
    Then the verdict is "error" with error "corruptStream" and a detail that contains "<detail>"
    And the TrackInfo has format "opus" and codec "opus"
    And no cutoff and no estimated source bitrate are reported

    Examples:
      | fixture                    | detail                                     |
      | opus_not_ogg               | not an Ogg stream                          |
      | opus_bad_crc               | Ogg page CRC mismatch                      |
      | opus_page_version          | unsupported Ogg page version               |
      | opus_truncated_page        | truncated Ogg page                         |
      | opus_sequence_gap          | Ogg page out of sequence                   |
      | opus_no_bos                | first Ogg page is not a beginning of stream |
      | opus_after_eos             | Ogg page after end of stream               |
      | opus_continuation          | Ogg packet continuation mismatch           |
      | opus_unfinished_packet     | unfinished Ogg packet at end of stream     |
      | opus_no_head               | no OpusHead                                |
      | opus_head_version          | invalid OpusHead                           |
      | opus_head_channels         | invalid OpusHead                           |
      | opus_head_only             | no OpusTags                                |
      | opus_no_tags               | no OpusTags                                |
      | opus_tags_overrun          | invalid OpusTags                           |
      | opus_tags_shared_page      | audio data on the OpusTags page            |
      | opus_no_audio              | no Opus audio packets                      |
      | opus_granule_decreases     | Ogg granule position decreases             |
      | opus_granule_below_preskip | granule position below pre-skip            |
      | opus_granule_beyond_audio  | granule position beyond the audio          |
      | opus_bad_packet            | Opus packet 100                            |

  @RF-14 @RNF-04
  Scenario Outline: Recognised but unsupported Opus streams are unsupportedFormat
    Given the Opus fixture "<fixture>"
    When it is analysed
    Then the verdict is "error" with error "unsupportedFormat" and detail "<detail>"
    And no cutoff and no estimated source bitrate are reported

    Examples:
      | fixture                | detail                                                 |
      | opus_family1_6ch       | channel mapping family 1 is not supported              |
      | opus_family255_3ch     | channel mapping family 255 is not supported            |
      | opus_serial_change     | multiplexed or chained Ogg streams are not supported   |

  @RF-14 @RNF-04
  Scenario: A short Opus file is too short
    Given the Opus fixture "opus_short"
    When it is analysed
    Then the verdict is "error" with error "tooShort"

  @RNF-04
  Scenario: A corrupt Opus file does not stop the analysis
    Given a folder "Mixed" with "opus_128_fb.opus", "opus_bad_crc.opus" and "mp3_320_lp20k5.mp3"
    When I analyse "Mixed"
    Then "opus_bad_crc.opus" has state "Error" with the reason "Corrupt stream"
    And the other two files are analysed normally
    And the last event is "finished" with errors 1

  @RF-07 @RF-14
  Scenario: An unsupported Opus file shows its error text
    Given a stubbed TrackAnalysis with format "opus", codec "opus", verdict "error" and error "unsupportedFormat"
    When I select its row
    Then the panel shows "Unsupported audio format" instead of a spectrogram

  # --- In the app ---------------------------------------------------------------------

  @RF-14 @RF-06
  Scenario: Opus rows in the table
    Given a folder "Opus" with the Opus fixtures "opus_128_fb.opus" and "opus_160_src16k.opus"
    When I analyse "Opus"
    Then every trackDone event carries a complete TrackAnalysis with algorithmVersion 8
    And the table shows "opus_128_fb.opus" with format "OPUS", samplerate "48000", bitrate "128 kbps", real bitrate "128 kbps" and state "Real"
    And the table shows "opus_160_src16k.opus" with format "OPUS", bitrate "160 kbps", real bitrate "—" and state "Fake", and a cutoff within 200 Hz of 16.0 kHz

  @RF-07 @RF-15
  Scenario Outline: Detail line of an Opus track
    Given a stubbed TrackAnalysis with format "opus", codec "opus", declared bitrate <declared> kbps, no bitrate mode, estimated source bitrate <estimate>, cutoff <cutoff> Hz, verdict "<verdict>" and reasons "<reasons>"
    When I select its row
    Then the panel shows "<line>"

    Examples:
      | declared | estimate | cutoff | verdict    | reasons             | line                                                                                 |
      | 128      | 128 kbps | 20000  | real       |                     | Cutoff 20.0 kHz · 128 kbps · Genuine                                                 |
      | 160      | none     | 16000  | fake       |                     | Cutoff 16.0 kHz · 160 kbps · Fake                                                    |
      | 128      | none     | 18500  | suspicious | cutoffBelowDeclared | Cutoff 18.5 kHz · 128 kbps · Suspicious · Cutoff lower than expected for its bitrate |

  @RF-14 @RF-07
  Scenario: The spectrogram of an Opus file
    The band spectrum of this recipe, measured before G1 (rusty-opus 1.0.2, mono,
    128 kbps), rolls off gradually above the 20 kHz band edge: about -38 dB
    below L_ref at 20.45 kHz, -64 dB at 21.05 kHz and -72 dB at 21.55 kHz. So the
    bound is 21.5 kHz, not the 200 Hz margin of the MP3 precedent
    (decode-lossy.feature). "Near the bottom" is the existing test criterion: the
    mean byte of every such row is at most 63.
    Given the Opus fixture "opus_128_fb" saved as "a.opus"
    When its spectrogram is requested
    Then it has height 512 and a width of at most 1200
    And maxFrequencyHz is 24000
    And the rows above 21.5 kHz are near the bottom of dbRange

  @RNF-05
  Scenario: Analysed Opus files are never modified
    Given a folder "Album A" with the Opus fixtures "opus_128_fb.opus" and "opus_bad_crc.opus"
    When I analyse "Album A"
    Then the size, modification time and content hash of both files are unchanged

  @RNF-09
  Scenario: Unicode names of Opus files
    Given a folder "Álbum ″Ñandú″" with the Opus fixture "opus_128_fb" saved as "01 – Canción ‘x’.opus" and the Opus fixture "opus_bad_crc" saved as "02 – Ópera «Ça va» 🎵.opus"
    When I analyse "Álbum ″Ñandú″"
    Then the table shows the file names "01 – Canción ‘x’.opus" and "02 – Ópera «Ça va» 🎵.opus" exactly
    And both paths in the trackDone events equal the names on disk code point for code point
    And "01 – Canción ‘x’.opus" has verdict "real" and "02 – Ópera «Ça va» 🎵.opus" has verdict "error" with error "corruptStream"

  # --- Test files (lossy-fixtures.md section 1.3) ----------------------------------------

  @RF-14
  Scenario: The fixture encoder is deterministic on one machine
    lossy-fixtures.md section 1.3 (a); lossy-source.md Q42. Measured before G1
    (section 12.8 F7): rusty-opus 1.0.2 gives the same bytes on the same SIMD
    level, run to run.
    Given every encoded Opus fixture of lossy-fixtures.md section 2.2 (opus_128_fb, opus_128_fb_stereo, opus_64_fb, opus_160_src16k, opus_160_swb and opus_short)
    When each one is written twice in the test process, and once in a child process of the same test binary with the same environment
    Then the three files of each fixture are byte-identical

  @RF-14
  Scenario: Opus fixtures encoded on the scalar path keep their truth
    lossy-fixtures.md section 1.3 (b); lossy-source.md Q42 (ADR-0047 point 3 as
    amended). Measured before G1 (section 12.8 F7): the scalar path gives other
    bytes than the AVX2 path for mono fixtures, with the same fc, D and H. So the
    truth is compared, never the bytes, and no step compares a fixture with a
    stored hash.
    Given every encoded Opus fixture of lossy-fixtures.md section 2.2 (opus_128_fb, opus_128_fb_stereo, opus_64_fb, opus_160_src16k, opus_160_swb and opus_short)
    When each one is encoded in a child process of the same test binary run with the environment variable RUSTY_OPUS_ISA=scalar
    And the file it writes is analysed in the test process with DetectionParams v8
    Then each has the verdict, reasons and estimated source bitrate of its row in lossy-fixtures.md section 2.2 ("error" with error "tooShort" for opus_short)
    And each except opus_short has a cutoff within 200 Hz of its edge
    And each has the durationMs, packet count and decoded samples per channel of lossy-fixtures.md section 1.3 (30000 ms, 1501 packets and 1 440 000 samples; 2000 ms, 101 packets and 96 000 samples for opus_short)
    And the declared bitrate of each except opus_short is within 1 kbps of its target bitrate

  @RF-14
  Scenario: The Ogg writer of the tests is not the adapter's reader
    Given the test-support Ogg writer (crates/fakeflac-audio/tests/common/)
    Then it imports nothing from the Ogg or Opus modules of crates/fakeflac-audio/src, its CRC-32 table included
