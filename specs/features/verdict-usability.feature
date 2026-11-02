@RNF-08
Feature: Understand verdicts without documentation
  Acceptance: specs/ui/verdict-usability.md (U1 to U4).
  Exact translations: specs/ui/translations.md.

  @RF-06 @RF-13
  Scenario Outline: Every verdict has text and a distinct icon in each language
    Given the interface language is "<language>"
    And the table contains real, suspicious, fake and error rows
    When I inspect their State cells without hovering or selecting them
    Then the real, suspicious, fake and error rows respectively show the labels "<labels>"
    And each verdict has its distinct icon and colour family
    And each icon is decorative for assistive technology

    Examples:
      | language | labels                       |
      | en       | Genuine,Suspicious,Fake,Error |
      | es       | Real,Sospechoso,Falso,Error   |

  @RF-06
  Scenario Outline: Verdict text remains readable in every row state
    Given each of the four verdict labels in each interface language
    When its table row is "<state>"
    Then its text contrast against the rendered background is at least 4.5 to 1
    And its whole label remains readable at a window size of 900 by 500

    Examples:
      | state    |
      | normal   |
      | hovered  |
      | selected |

  @RF-07 @RNF-04 @RNF-09
  Scenario: Doubt and failure have explanations in the selected language
    Given each supported interface language
    And a Suspicious row with a reason and an Error row for a corrupt file with a Unicode name
    When I select the Suspicious row
    Then the detail panel shows its localized verdict and reason in words
    When I select the Error row
    Then the detail panel shows its friendly localized error explanation
    And the table preserves its Unicode filename

  Scenario: An empty table has no invented verdict
    Given the application has no analysis results
    When I inspect the result table and detail panel
    Then neither shows a quality verdict for a nonexistent track

  Scenario: The owner understands all states through the interface
    Given the rendered candidate contains all four verdicts with a suspicious reason and a track error
    When the owner completes U4 in English and Spanish without project documentation
    Then each verdict meaning is correctly explained in each language
    And Suspicious is distinguished from Fake and Error
    And the reason and error explanation are found through the interface
    And the observations are recorded with the candidate commit
