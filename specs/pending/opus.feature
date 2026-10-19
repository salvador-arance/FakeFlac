@pending-opus-decoder @RF-14 @RF-15
Feature: Opus files (pending the ADR-0030 decoder spike)
  PENDING. This file lives in specs/pending/, which neither "just trace"
  (scripts/trace.mjs reads specs/features/*.feature only) nor the test stage
  reads, so no test is derived from it yet. Symphonia does not decode Opus,
  and the decoder is chosen by the ADR-0030 spike (opus crate with bundled
  libopus, or a pure-Rust alternative). A later spec PR, after the spike,
  moves this file to specs/features/, removes the @pending-opus-decoder tag,
  adds the Opus rows of [lossy.cutoffs] (a new params version) and the Opus
  fixtures, and fills in the values marked <to be specified>.
  Until then the scanner skips ".opus" files (decode-lossy.feature) and the
  calibration tool lists Opus labels as not evaluated
  (specs/calibration/lossy.md). Rules: specs/detection/lossy-source.md.

  @RF-02 @RF-14
  Scenario: Opus files are found
    Given a folder "Library" containing "a.flac", "b.opus" and "c.OPUS"
    When I analyse "Library"
    Then 3 files are scanned: "a.flac", "b.opus" and "c.OPUS"

  @RF-14
  Scenario: Stream information of an Opus file
    Opus always decodes at 48 kHz, whatever input rate its header records.
    Given an Opus fixture encoded from "full_band" at 128 kbps
    When it is analysed
    Then the TrackInfo has format "opus", codec "opus", sample rate 48000 Hz and no bit depth
    And its declared bitrate is the average of its Ogg audio payload over its duration, pre-skip excluded, within 1 kbps
    And its duration is the fixture's within 20 ms

  @RF-15
  Scenario: An honest Opus file
    Given an Opus fixture encoded from "full_band" at 128 kbps
    When it is analysed with the params version that adds the Opus rows
    Then the verdict is "real"
    And the estimated source bitrate is 128

  @RF-15
  Scenario: An Opus file made from a lower-bitrate source
    Given an Opus fixture at 160 kbps whose content stops at <to be specified> Hz
    When it is analysed with the params version that adds the Opus rows
    Then the verdict is "fake"
    And the estimated source bitrate is <to be specified>

  @RF-14 @RNF-04
  Scenario: A corrupt Opus file becomes an error track
    Given an ".opus" file whose Ogg pages are truncated after the OpusHead header
    When it is analysed
    Then the verdict is "error" with error "corruptStream"

  @RNF-09
  Scenario: Unicode names of Opus files
    Given a folder "Álbum ″Ñandú″" with an Opus fixture saved as "01 – Canción ‘x’.opus"
    When I analyse "Álbum ″Ñandú″"
    Then the table shows the file name "01 – Canción ‘x’.opus" exactly
