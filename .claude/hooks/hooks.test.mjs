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
