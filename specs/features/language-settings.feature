@RF-13
Feature: Spanish and English interface with a persistent language preference
  The owner can choose the interface language and keep it for the next launch.
  Contract: specs/contracts/settings.md (proposed version 8).
  UI and text: specs/ui/language-settings.md and specs/ui/translations.md.

  Scenario Outline: First launch detects Windows language without creating a file
    Given the app configuration directory has no settings file
    And the Windows user language preferences are "<preferences>"
    When the app initializes its settings
    Then get_settings returns language "<language>" and loadStatus "default"
    And the main screen and document language use "<language>"
    And initialization creates no settings file or directory
    Examples:
      | preferences       | language |
      | es-ES,en-US       | es       |
      | en-GB,es-MX       | en       |
      | fr-FR,es-MX,en-US | es       |
      | ES-mx            | es       |
      | en               | en       |
      | es               | es       |
      | fr-FR,ja-JP       | en       |
      | empty            | en       |
      | unavailable      | en       |

  Scenario: Restore a saved language before enabling the main screen
    Given the settings file contains schemaVersion 1 and language "es"
    And the Windows user language preferences are "en-US"
    When the app mounts and its settings read is pending
    Then analysis and Settings actions are disabled
    And the language-neutral busy placeholder is shown
    When the read completes
    Then get_settings returns language "es" and loadStatus "loaded"
    And the first enabled main screen says "Abrir carpeta" and "Listo"
    And the document language is "es"

  Scenario Outline: An explicit Save survives a new app process
    Given the initialized app uses language "<before>"
    When I open Settings and select "<after>"
    Then the active interface still uses "<before>"
    When I Save and the write succeeds
    Then the mounted interface and document language use "<after>"
    And the dialog closes with focus on the translated Settings button
    And a new app process reads language "<after>" with loadStatus "loaded"
    Examples:
      | before | after |
      | en     | es    |
      | es     | en    |
      | en     | en    |

  Scenario Outline: Dismissing the draft does not persist it
    Given the active language is "en"
    And I have selected "es" in the open Settings panel
    When I dismiss the panel using "<action>"
    Then no set_settings call is made
    And the active language and saved bytes are unchanged
    And focus returns to the Settings toolbar button
    Examples:
      | action |
      | Cancel |
      | Escape |

  Scenario: A save already in progress cannot be duplicated or dismissed
    Given I have submitted a language change and the save reply is pending
    When I activate Save again and press Escape
    Then there is still exactly one set_settings request
    And the select and both actions are disabled
    And the dialog remains open and busy

  @RNF-04
  Scenario Outline: A failed save preserves the old preference and permits retry
    Given the saved and active language is "en"
    And I have selected "es" in Settings
    When the save fails at "<operation>"
    Then the previous settings file is byte-identical
    And get_settings still returns the previous snapshot
    And the interface and document language remain "en"
    And the panel keeps the Spanish draft and shows the localized save failure
    When I retry and the save succeeds
    Then the active and saved language is "es"
    Examples:
      | operation              |
      | directory creation     |
      | read before write      |
      | temporary file write   |
      | temporary file flush   |
      | destination replacement |

  @RNF-04
  Scenario Outline: Invalid persisted settings fall back without modifying the file
    Given the settings file contains "<content>"
    And the Windows user language preferences are "es-ES"
    When settings initialize
    Then the language is "en" and loadStatus is "invalid"
    And the settings file is byte-identical
    And the main screen works and Settings explains the fallback
    When I explicitly Save "es"
    Then the file contains valid schema 1 settings for "es"
    Examples:
      | content                                  |
      | malformed JSON                           |
      | schema 1 with language fr                 |
      | schema 1 with language ES                 |
      | schema 1 with missing language            |
      | schema 1 with an unknown field            |
      | schema 1 with a duplicate language field   |
      | schemaVersion 0                          |
      | schemaVersion 1.5                        |

  @RNF-04
  Scenario Outline: An unreadable configuration does not prevent analysis
    Given settings initialization encounters "<failure>"
    And the Windows user language preferences are "es-ES"
    When the main screen initializes
    Then English is active with the unavailable settings warning
    And folder analysis and its normal errors remain usable
    And opening Settings offers an explicit Save retry
    Examples:
      | failure                  |
      | config directory lookup  |
      | file access denied       |
      | settings path is a directory |
      | unexpected IPC rejection |

  Scenario: A newer settings version is preserved
    Given the settings file is a valid JSON object with schemaVersion 2
    And the Windows user language preferences are "es-ES"
    When settings initialize
    Then the language is "en" and loadStatus is "newerVersion"
    And Settings explains the newer version with its select and Save disabled
    And Close is enabled
    When set_settings is called directly with language "es"
    Then it returns SETTINGS_READ_ONLY and the file is byte-identical

  Scenario: A newer file appearing after startup is not overwritten
    Given settings initialized from a valid schema 1 file
    And another process has replaced it with a valid schema 2 file
    And no other process modifies it during the save attempt
    When I Save a language change
    And the pre-save version check observes that schema 2 file
    Then the command returns SETTINGS_READ_ONLY
    And the newer file and active language are unchanged
    And the panel shows the read-only error and permits Close

  Scenario Outline: Invalid IPC input cannot write a language preference
    Given settings have initialized normally
    When set_settings receives "<input>"
    Then the call is rejected
    And settings memory and disk are unchanged
    Examples:
      | input            |
      | language fr      |
      | language null    |
      | missing language |

  Scenario: The two catalogues cover all user-facing states explicitly
    Given the normative key tables in specs/ui/translations.md
    Then each key has explicit English and Spanish text
    And both texts have the specified identical placeholder multiset
    And no supported Spanish key depends on the English fallback
    And singular forms are selected exactly when their count is 1

  Scenario Outline: Existing mounted states translate after a successful Save
    Given the English main screen is in phase "<phase>"
    When I save Spanish
    Then all mounted visible texts, tooltips and accessible names use the Spanish catalogue
    And the run, rows, counts and phase are preserved
    Examples:
      | phase      |
      | ready      |
      | scanning   |
      | analysing  |
      | finished   |
      | cancelled  |
      | error      |

  @RF-08 @RNF-09
  Scenario: Language changes name ordering while preserving selection and view state
    Given the English table contains nube.flac, ñandú.flac and nora.flac
    And File ascending is active with ñandú.flac selected
    And the table order is ñandú.flac, nora.flac, nube.flac
    When I save Spanish
    Then the order becomes nora.flac, nube.flac, ñandú.flac
    And the same row index stays selected without an automatic reveal
    And query, verdict filters, sort direction, album order and splitter size are unchanged
    And the next arrow navigation uses the new visible order

  Scenario: Settings can change while progress and a spectrogram are pending
    Given an analysis has queued track events and a selected row with a pending spectrogram
    When Settings is open and I save Spanish while those replies arrive
    Then the analysis continues and all delivered events are applied normally
    And the same spectrogram reply is accepted for the selected row
    And subsequent render cycles use the active language
    And opening or saving Settings makes no analysis or spectrogram request

  @RNF-04 @RNF-09
  Scenario: Existing error and export notices translate without changing user data
    Given an English finished view contains a corrupt track named "Canción – 日本語.flac"
    And an export notice refers to "C:\Out\Álbum 日本語.csv"
    When I save Spanish
    Then the track's friendly error is "Flujo de audio dañado"
    And the export notice uses its Spanish sentence with the identical destination path
    And names, diagnostics and result data retain their original values

  @RNF-04
  Scenario: An analysis failure during a settings save remains an analysis failure
    Given analysis is running and a settings save is pending
    When the analysis returns INTERNAL and the settings save succeeds with Spanish
    Then the reported rows are retained and the phase is error
    And the status uses the Spanish analysis error text
    And the saved settings are Spanish

  Scenario: A cache notice already on screen translates
    Given an English cache-cleared notice is visible
    When I save Spanish
    Then the notice reads "Caché de resultados borrada"
    And no additional clear_cache command is issued

  @RNF-08
  Scenario: The language modal supports keyboard-only use
    Given the initialized main screen has keyboard focus on Settings
    When I activate Settings
    Then the labelled modal opens with focus on the language select
    And Tab and Shift+Tab remain within its controls
    And English and Español have their respective language annotations
    And background Ctrl+F, clicks and folder drops have no effect
    And analysis progress can still update the background view
    When I press Escape while not saving
    Then the modal closes and focus returns to Settings

  @RNF-05 @RNF-09
  Scenario: Settings writes are confined to the Unicode app configuration root
    Given an isolated configuration root with spaces and Unicode characters
    And generated audio fixtures and a results-cache file with recorded bytes
    When I save Spanish and read it from a fresh settings owner
    Then the preference is restored from that root's settings.json
    And the audio fixtures, cache and unrelated files are byte-identical

  Scenario: Language changes do not change exported results or detection identity
    Given a completed run with real, suspicious, fake and corrupt-track results
    When its results are exported with English and then Spanish active
    Then the CSV bytes are identical
    And the analysis DTOs and cache engine identity are identical

  Scenario: Native Windows dialogs retain their system language
    Given Spanish is active in FakeFlac on an English Windows installation
    When I use the native folder picker or CSV save dialog
    Then the app-owned trigger and result messages are Spanish
    And Windows controls and overwrite prompts retain their Windows language
    And the CSV filter name remains "CSV (*.csv)"
