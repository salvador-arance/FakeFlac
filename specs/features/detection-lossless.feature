@RF-03 @RF-04 @RF-14
Feature: Verdict for lossless files (WAV, FLAC)
  The detection rules of specs/detection/rules.md, checked with the synthetic
  signals of specs/detection/fixtures.md and DetectionParams v3 (the defaults).
  Cutoff tolerance: ± 200 Hz.
  Changed for v3 (the test author updates the derived tests listed in ADR-0038,
  "Consequences"): sfb21_gaps is now suspicious; "Deterministic result" uses
  shelf19k5_bursts instead of lp19k; the defaults are compared with
  detection-params.v3.toml and carry algorithmVersion 3.

  @RF-04
  Scenario Outline: Verdict by signal
    Given the synthetic signal "<signal>" saved as <format>
    When it is analysed with DetectionParams v3
    Then the verdict is "<verdict>"
    And the reasons are "<reasons>"

    Examples:
      | signal            | format | verdict    | reasons                 |
      | full_band         | FLAC   | real       |                         |
      | full_band         | WAV    | real       |                         |
      | lp16k             | FLAC   | fake       |                         |
      | lp19k             | FLAC   | fake       |                         |
      | lp20k             | WAV    | fake       |                         |
      | lp16k_96k         | FLAC   | fake       |                         |
      | bass_tone_lp16k   | FLAC   | fake       |                         |
      | gradual           | FLAC   | suspicious | gradualRolloff          |
      | low_hf            | FLAC   | suspicious | lowHighFrequencyContent |
      | silence           | FLAC   | suspicious | lowHighFrequencyContent |
      | lp16k_dither      | FLAC   | fake       |                         |
      | lp20k_floor       | FLAC   | fake       |                         |
      | sfb21_gaps        | FLAC   | suspicious | highBandGaps            |
      | sfb21_gaps_weak   | FLAC   | suspicious | highBandGaps            |
      | hf_bursts         | FLAC   | real       |                         |
      | sfb21_gaps_lp20k5 | FLAC   | fake       |                         |
      | sfb21_gaps_aa21k  | FLAC   | suspicious | highBandGaps            |
      | shelf19k5         | FLAC   | fake       |                         |
      | shelf19k5_bursts  | FLAC   | suspicious | ambiguousCutoff         |

  @RF-03
  Scenario Outline: Cutoff estimate
    Given the synthetic signal "<signal>" saved as FLAC
    When it is analysed with DetectionParams v3
    Then the estimated cutoff is <cutoff> Hz ± 200 Hz

    Examples:
      | signal            | cutoff |
      | lp16k             | 16000  |
      | lp19k             | 19000  |
      | lp20k             | 20000  |
      | lp16k_96k         | 16000  |
      | bass_tone_lp16k   | 16000  |
      | lp16k_dither      | 16000  |
      | lp20k_floor       | 20000  |
      | sfb21_gaps_lp20k5 | 20500  |
      | sfb21_gaps_aa21k  | 21000  |
      | shelf19k5         | 19500  |
      | shelf19k5_bursts  | 19500  |

  @RF-03
  Scenario Outline: Signal without a cutoff step reports the Nyquist frequency as cutoff
    Given the synthetic signal "<signal>" saved as FLAC
    When it is analysed with DetectionParams v3
    Then the estimated cutoff is 22050 Hz

    Examples:
      | signal     |
      | full_band  |
      | sfb21_gaps |
      | hf_bursts  |

  @RF-04
  Scenario Outline: Noise above the low-pass does not hide a transcode (v1 regression)
    The noise left above the cutoff of a decoded MP3 written at 16 bits made
    DetectionParams v1 report these signals as real.
    Given the synthetic signal "<signal>" saved as FLAC
    When it is analysed with DetectionParams v3
    Then the verdict is not "real"

    Examples:
      | signal       |
      | lp16k_dither |
      | lp20k_floor  |
      | sfb21_gaps   |

  @RF-04
  Scenario Outline: A stationary floor above the low-pass is not full band (v3 guard)
    The floor of a decoded transcode can sit less than 60 dB below the reference
    level, but it does not come and go like music. The full-band guard must not
    clear these transcodes.
    Given the synthetic signal "<signal>" saved as FLAC
    When it is analysed with DetectionParams v3
    Then the track is not full band
    And the verdict is "fake"

    Examples:
      | signal            |
      | lp16k_dither      |
      | lp20k_floor       |
      | sfb21_gaps_lp20k5 |
      | shelf19k5         |

  @RF-04
  Scenario Outline: Intermittent content above the lossy range is full band (v3 guard)
    Given the synthetic signal "<signal>" saved as FLAC
    When it is analysed with DetectionParams v3
    Then the track is full band
    And the verdict is "suspicious"

    Examples:
      | signal           |
      | sfb21_gaps       |
      | sfb21_gaps_aa21k |
      | shelf19k5_bursts |

  @RF-04
  Scenario Outline: A v2 params file still gives the v2 verdicts
    DetectionParams v2 has no [full_band] section, so the guard is disabled and
    the calibration tool can compare v2 and v3 in one run.
    Given the synthetic signal "<signal>" saved as FLAC
    When it is analysed with the parameters of specs/detection/detection-params.v2.toml
    Then the verdict is "fake"
    And the TrackAnalysis carries algorithmVersion 2

    Examples:
      | signal           |
      | sfb21_gaps       |
      | sfb21_gaps_aa21k |
      | shelf19k5_bursts |
      | lp16k_dither     |

  @RF-04
  Scenario: A v2 params file changes only the guarded verdicts
    Given every synthetic signal of specs/detection/fixtures.md saved as FLAC
    When each one is analysed with the parameters of specs/detection/detection-params.v2.toml and with DetectionParams v3
    Then the two results have the same verdict, reasons and cutoff for every signal except "sfb21_gaps", "sfb21_gaps_aa21k" and "shelf19k5_bursts"
    And those three are "fake" with no reasons with the v2 parameters, with the same cutoff as with v3

  @RNF-04
  Scenario: Too short
    Given the synthetic signal "short" saved as FLAC
    When it is analysed with DetectionParams v3
    Then the verdict is "error" with error "tooShort"

  @RNF-04
  Scenario: Corrupt file
    Given the file "corrupt" (FLAC truncated after the header)
    When it is analysed with DetectionParams v3
    Then the verdict is "error" with error "corruptStream"

  @RF-04
  Scenario: Deterministic result
    Given the synthetic signal "shelf19k5_bursts" saved as FLAC
    When it is analysed twice with DetectionParams v3
    Then both analyses are equal

  @RF-04
  Scenario: Parameters come from the versioned file
    Given the default DetectionParams of the code
    Then they are equal to specs/detection/detection-params.v3.toml
    And every TrackAnalysis carries algorithmVersion 3

  @RF-03 @RNF-04
  Scenario: Stream without a declared length
    A FLAC from a streaming encoder may leave total_samples at 0. The length is
    counted by decoding, so the track is analysed instead of reported as silent.
    Given the file "lp16k_unknown_length" (lp16k as FLAC with total_samples 0 in STREAMINFO)
    When it is analysed with DetectionParams v3
    Then the verdict is "fake"
    And the estimated cutoff is 16000 Hz ± 200 Hz

  @RF-03
  Scenario: A silent track reports no cutoff
    Given the synthetic signal "silence" saved as FLAC
    When it is analysed with DetectionParams v3
    Then the verdict is "suspicious"
    And the reasons are "lowHighFrequencyContent"
    And no cutoff is reported
