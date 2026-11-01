@RP-01 @RP-03 @RP-04 @RP-06 @RP-10
Feature: Codex follows the canonical Claude harness
  The owner maintains Claude configuration once.
  Codex consumes derived compatibility files and the same workflow policies.

  Scenario: Generate Codex configuration from Claude sources
    Given canonical root and scoped Claude instructions, roles, skills and hooks
    When the agent runs just agents-sync
    Then Codex instruction bodies and skills preserve the canonical content
    And Codex roles preserve their canonical names, descriptions and instruction bodies
    And both harnesses use .claude/stage
    And Claude sources and model assignments are unchanged

  Scenario: One canonical edit updates the derived configuration
    Given synchronized harness configuration
    When a canonical instruction, role, skill or hook registration changes
    Then just agents-check fails and names the stale relative output paths
    And check mode writes no files
    When the agent runs just agents-sync
    Then just agents-check passes without a manual Codex edit
    And running synchronization again changes no file

  Scenario: Manage source additions and removals safely
    Given synchronized harness configuration and an unrelated local file
    When a canonical source is added and another is removed
    And the agent runs just agents-sync
    Then the new generated output exists and the obsolete managed output is removed
    And the unrelated local file and personal settings are untouched

  Scenario: Reject invalid input before changing outputs
    Given malformed canonical metadata or an output path escaping the repository
    When the agent runs just agents-sync
    Then synchronization fails with a clear error
    And no generated output is changed

  Scenario: Protect every path in a Codex patch
    Given the shared stage is implement
    And a Codex patch changes an allowed production file and a protected test
    When the patch reaches the compatibility adapter
    Then the canonical guard blocks the entire call before mutation
    And the same result applies when the protected path is a move destination

  Scenario: Preserve canonical shell and lifecycle hook behavior
    Given equivalent Claude and Codex hook payloads
    When the adapter invokes the canonical shell, context or stop hook
    Then the canonical output and exit status are preserved
    And the canonical stop skip conditions remain unchanged

  Scenario: Fail existing checks on generated drift
    Given a generated Codex file has a manual change
    When just check-fast runs locally or in CI
    Then agents-check fails and requests regeneration from Claude sources

  Scenario: Report host-specific enforcement limits honestly
    Given a Claude permission or transcript format is unsupported by the Codex host
    When the compatibility behavior is validated
    Then the limitation is reported explicitly
    And no unsupported permission key or automatic approval is generated
    And instruction parity is not reported as verified enforcement parity

  Scenario: Generate the owner-approved Codex model assignments
    Given the canonical Codex model source assigns every canonical role a model and reasoning effort
    When the agent runs just agents-sync
    Then each Codex role declares that model and reasoning effort
    And Claude model defaults and role instruction bodies remain unchanged
    And changing one assignment makes read-only agents-check fail until regeneration

  Scenario: Reject invalid Codex model assignments before changing outputs
    Given the canonical model source has an invalid structure, role, field, model id or reasoning effort
    When the agent runs just agents-sync
    Then synchronization fails before any output changes
