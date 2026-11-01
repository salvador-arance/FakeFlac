// Tests for the harness hooks (3.9): sample JSON payloads in, exit code and message out.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))

function run(script, payload, { stage, env = {} } = {}) {
  const proj = mkdtempSync(join(tmpdir(), 'ff-hook-'))
  try {
    if (stage) {
      mkdirSync(join(proj, '.claude'), { recursive: true })
      writeFileSync(join(proj, '.claude', 'stage'), stage)
    }
    const res = spawnSync('node', [join(here, script)], {
      input: JSON.stringify(payload),
      encoding: 'utf8',
      env: { ...process.env, CLAUDE_PROJECT_DIR: proj, FAKEFLAC_CORPUS: '', ...env },
    })
    return { code: res.status, stderr: res.stderr, proj }
  } finally {
    rmSync(proj, { recursive: true, force: true })
  }
}

const edit = (proj, rel) => ({ tool_name: 'Edit', tool_input: { file_path: join(proj ?? 'X:/nowhere', rel) } })

test('guard-protected: blocks writes outside the repo', () => {
  const r = run('guard-protected.mjs', { tool_input: { file_path: join(tmpdir(), 'elsewhere.txt') } })
  assert.equal(r.code, 2)
  assert.match(r.stderr, /outside the repository/)
})

test('guard-protected: allows writes inside the repo when no stage is set', () => {
  const proj = mkdtempSync(join(tmpdir(), 'ff-hook-'))
  const res = spawnSync('node', [join(here, 'guard-protected.mjs')], {
    input: JSON.stringify(edit(proj, 'specs/a.md')),
    encoding: 'utf8',
    env: { ...process.env, CLAUDE_PROJECT_DIR: proj },
  })
  rmSync(proj, { recursive: true, force: true })
  assert.equal(res.status, 0)
})

for (const [stage, rel, expected] of [
  ['implement', 'specs/features/a.feature', 2],
  ['implement', 'crates/fakeflac-core/tests/verdict.rs', 2],
  ['implement', 'src/lib/components/atoms/Button.test.ts', 2],
  ['implement', 'crates/fakeflac-core/src/domain.rs', 0],
  ['spec', 'specs/features/a.feature', 0],
  ['review', 'crates/fakeflac-core/src/domain.rs', 2],
  ['review', 'docs/agents/progress.md', 0],
]) {
  test(`guard-protected: stage=${stage} ${rel} -> exit ${expected}`, () => {
    const proj = mkdtempSync(join(tmpdir(), 'ff-hook-'))
    mkdirSync(join(proj, '.claude'), { recursive: true })
    writeFileSync(join(proj, '.claude', 'stage'), stage)
    const res = spawnSync('node', [join(here, 'guard-protected.mjs')], {
      input: JSON.stringify(edit(proj, rel)),
      encoding: 'utf8',
      env: { ...process.env, CLAUDE_PROJECT_DIR: proj },
    })
    rmSync(proj, { recursive: true, force: true })
    assert.equal(res.status, expected, res.stderr)
  })
}

for (const [cmd, expected] of [
  ['git push --force origin feat/x', 2],
  ['git push -f', 2],
  ['git push origin main', 2],
  ['git push origin feat/RF-04', 0],
  ['gh pr merge 12 --squash', 2],
  ['gh pr view 12', 0],
  ['rm -rf node_modules', 2],
  ['Remove-Item -Recurse -Force C:\\x', 2],
  ['git reset --hard HEAD~1', 2],
  ['git commit --no-verify -m x', 2],
  ['just check', 0],
  ['cargo test --workspace', 0],
  // Agents never release (RNF-11, Q11; specs/contracts/release.md 8.1; release.feature
  // "Agents cannot push a release tag or create a release" @RNF-11).
  ['git push origin v0.1.0', 2],
  ['git push --delete origin v0.1.0', 2],
  ['git push origin :v0.1.0', 2],
  ['git push origin +v0.1.0', 2],
  ['git push origin refs/tags/v0.1.0', 2],
  ['git push --tags', 2],
  ['git push --follow-tags origin feat/RNF-11', 2],
  ['gh release create v0.1.0', 2],
  ['gh release edit v0.1.0 --draft=false', 2],
  ['gh release upload v0.1.0 SHA256SUMS.txt', 2],
  ['gh release delete v0.1.0', 2],
  ['git push -u origin feat/v2-ui', 0],
  ['git push origin chore/release-0.1.0', 0],
  ['git push origin feat/RNF-11', 0],
  ['git tag --list', 0],
  ['gh release view v0.1.0', 0],
  ['gh release list', 0],
]) {
  test(`guard-bash: "${cmd}" -> exit ${expected}`, () => {
    assert.equal(run('guard-bash.mjs', { tool_input: { command: cmd } }).code, expected)
  })
}

test('guard-bash: corpus is read-only', () => {
  const r = run('guard-bash.mjs', { tool_input: { command: 'Remove-Item D:\\music\\corpus\\a.flac' } }, { env: { FAKEFLAC_CORPUS: 'D:\\music\\corpus' } })
  assert.equal(r.code, 2)
  const ok = run('guard-bash.mjs', { tool_input: { command: 'ls D:\\music\\corpus' } }, { env: { FAKEFLAC_CORPUS: 'D:\\music\\corpus' } })
  assert.equal(ok.code, 0)
})

test('stop-check: skips when stop_hook_active is set (no infinite loop)', () => {
  assert.equal(run('stop-check.mjs', { stop_hook_active: true }).code, 0)
})

test('stop-check: skips in the test stage (tests are expected to be red)', () => {
  assert.equal(run('stop-check.mjs', {}, { stage: 'test' }).code, 0)
})

// guard-agents: a fake session whose subagent transcript ends with a call of `tokens` input tokens.
function sendTo(agent, tokens, { env = {}, to = agent } = {}) {
  const home = mkdtempSync(join(tmpdir(), 'ff-agents-'))
  try {
    const session = join(home, 'session')
    if (tokens !== null) {
      mkdirSync(join(session, 'subagents'), { recursive: true })
      const usage = (n) => JSON.stringify({ message: { usage: { input_tokens: 10, cache_read_input_tokens: n - 10, output_tokens: 5 } } })
      writeFileSync(join(session, 'subagents', `agent-${agent}.jsonl`), [usage(1000), usage(tokens), 'not json'].join('\n'))
    }
    return run('guard-agents.mjs', { tool_name: 'SendMessage', transcript_path: `${session}.jsonl`, tool_input: { to, message: 'go on' } }, { env })
  } finally {
    rmSync(home, { recursive: true, force: true })
  }
}

test('guard-agents: blocks resuming a subagent over the context limit', () => {
  const r = sendTo('a1b2', 283_000)
  assert.equal(r.code, 2)
  assert.match(r.stderr, /283k tokens of context/)
})

test('guard-agents: allows resuming a subagent under the limit', () => {
  assert.equal(sendTo('a1b2', 90_000).code, 0)
})

test('guard-agents: the limit comes from FAKEFLAC_AGENT_CONTEXT_LIMIT', () => {
  assert.equal(sendTo('a1b2', 90_000, { env: { FAKEFLAC_AGENT_CONTEXT_LIMIT: '50000' } }).code, 2)
})

test('guard-agents: an agent with no transcript in this session is not checked', () => {
  assert.equal(sendTo('a1b2', null).code, 0)
})

test('guard-agents: a recipient that is not a plain id is not checked', () => {
  assert.equal(sendTo('a1b2', 283_000, { to: '../a1b2' }).code, 0)
})
