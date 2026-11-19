Feature: Agent process checks that run today
  Only behavior that an existing test or CI command checks is written here
  (specs/contracts/agent-process.md). The owner checks of RP-02, RP-05, RP-07
  and RP-08 are written in the contract, not as scenarios. RP-08 has no
  scenario: nothing measures a time automatically.

  # --- RP-02: agents never merge or push to main (tested in .claude/hooks/hooks.test.mjs,
  # --- "guard-bash: \"<command>\" -> exit <code>"; Codex payloads in scripts/codex-hooks.test.mjs,
  # --- "@RP-03 Preserve canonical shell and lifecycle hook behavior: exec_command <command>") ---

  @RP-02
  Scenario Outline: Agents cannot merge a pull request, push to main or force-push
    When guard-bash receives the command "<command>"
    Then it exits <code>

    Examples:
      | command                        | code |
      | gh pr merge 12 --squash        | 2    |
      | git push origin main           | 2    |
      | git push --force origin feat/x | 2    |
      | git push -f                    | 2    |
      | git push origin feat/RF-04     | 0    |
      | gh pr view 12                  | 0    |

  # --- RP-05: a fresh session starts from the newest progress block (tested in
  # --- scripts/codex-hooks.test.mjs, "@RP-03 Preserve canonical shell and lifecycle hook
  # --- behavior: same newest progress context") ---

  @RP-05
  Scenario: A new session receives the stage and the newest progress block
    Given the stage is implement
    And docs/agents/progress.md has a newest block and an older block
    When the session-context hook runs at session start
    Then its output names the stage implement
    And its output contains the newest block
    And its output does not contain the older block

  # --- RP-07: scenario tags trace to requirements (checked by `just trace` in the CI job
  # --- `specs` and in `just ci`; scripts/trace.mjs has no test of its own, contract Q10).
  # --- This is the only automated part of RP-07. It does NOT check that a PR body cites
  # --- the requirement, the spec and the issue: that is the owner check of the contract
  # --- (section 3, A2) ---

  @RP-07
  Scenario: Every scenario tag names an existing requirement
    Given the feature files in specs/features
    When just trace runs
    Then it exits 0
    And every @RF, @RNF and @RP tag in those files names a file in specs/requirements
