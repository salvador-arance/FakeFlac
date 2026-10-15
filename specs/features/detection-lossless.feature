@RF-03 @RF-04 @RF-14
Feature: Verdict for lossless files (WAV, FLAC)
  The detection rules of specs/detection/rules.md, checked with the synthetic
  signals of specs/detection/fixtures.md and DetectionParams v1.
  Cutoff tolerance: ± 200 Hz.

  @RF-04
  Scenario Outline: Verdict by signal
    Given the synthetic signal "<signal>" saved as <format>
    When it is analysed with DetectionParams v1
    Then the verdict is "<verdict>"
    And the reasons are "<reasons>"

    Examples:
      | signal          | format | verdict    | reasons                 |
      | full_band       | FLAC   | real       |                         |
      | full_band       | WAV    | real       |                         |
      | lp16k           | FLAC   | fake       |                         |
      | lp19k           | FLAC   | fake       |                         |
      | lp20k           | WAV    | fake       |                         |
      | lp16k_96k       | FLAC   | fake       |                         |
      | bass_tone_lp16k | FLAC   | fake       |                         |
      | gradual         | FLAC   | suspicious | gradualRolloff          |
      | low_hf          | FLAC   | suspicious | lowHighFrequencyContent |
      | silence         | FLAC   | suspicious | lowHighFrequencyContent |

  @RF-03
  Scenario Outline: Cutoff estimate
    Given the synthetic signal "<signal>" saved as FLAC
    When it is analysed with DetectionParams v1
    Then the estimated cutoff is <cutoff> Hz ± 200 Hz

    Examples:
      | signal          | cutoff |
      | lp16k           | 16000  |
      | lp19k           | 19000  |
      | lp20k           | 20000  |
      | lp16k_96k       | 16000  |
      | bass_tone_lp16k | 16000  |

  @RF-03
  Scenario: Full-band signal reports the Nyquist frequency as cutoff
    Given the synthetic signal "full_band" saved as FLAC
    When it is analysed with DetectionParams v1
    Then the estimated cutoff is 22050 Hz

  @RNF-04
  Scenario: Too short
    Given the synthetic signal "short" saved as FLAC
    When it is analysed with DetectionParams v1
    Then the verdict is "error" with error "tooShort"

  @RNF-04
  Scenario: Corrupt file
    Given the file "corrupt" (FLAC truncated after the header)
    When it is analysed with DetectionParams v1
    Then the verdict is "error" with error "corruptStream"

  @RF-04
  Scenario: Deterministic result
    Given the synthetic signal "lp19k" saved as FLAC
    When it is analysed twice with DetectionParams v1
    Then both analyses are equal

  @RF-04
  Scenario: Parameters come from the versioned file
    Given the default DetectionParams of the code
    Then they are equal to specs/detection/detection-params.v1.toml
    And every TrackAnalysis carries algorithmVersion 1
