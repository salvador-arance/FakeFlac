// @RP-01 @RP-03 @RP-04 @RP-06 @RP-10: specs/features/agent-harness.feature.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const sourceAdapter = join(repo, '.codex/hooks/dispatch.mjs')
const scratch = join(repo, 'target', 'agent-harness-tests')
mkdirSync(scratch, { recursive: true })

function put(root, path, text) {
  mkdirSync(dirname(join(root, path)), { recursive: true })
  writeFileSync(join(root, path), text)
}
function fixture(t, stage = '') {
  assert.ok(existsSync(sourceAdapter), 'Codex payload adapter is not implemented')
  const root = mkdtempSync(join(scratch, 'hooks-'))
  t.after(() => rmSync(root, { recursive: true, force: true }))
  cpSync(join(repo, '.claude/hooks'), join(root, '.claude/hooks'), { recursive: true })
  put(root, 'scripts/test-paths.mjs', readFileSync(join(repo, 'scripts/test-paths.mjs'), 'utf8'))
  put(root, '.codex/hooks/dispatch.mjs', readFileSync(sourceAdapter, 'utf8'))
  put(root, '.claude/stage', stage)
  return root
}
function invoke(root, hook, payload, direct = false) {
  const script = direct ? join(root, '.claude/hooks', `${hook}.mjs`) : join(root, '.codex/hooks/dispatch.mjs')
  const args = direct ? [script] : [script, hook]
  return spawnSync(process.execPath, args, {
    cwd: root,
    input: JSON.stringify({ cwd: root, ...payload }),
    encoding: 'utf8',
    env: { ...process.env, CLAUDE_PROJECT_DIR: root, FAKEFLAC_CORPUS: '' },
  })
}
function patch(path, operation = 'Update') {
  if (operation === 'Delete') return `*** Begin Patch\n*** Delete File: ${path}\n*** End Patch\n`
  if (operation === 'Add') return `*** Begin Patch\n*** Add File: ${path}\n+new\n*** End Patch\n`
  return `*** Begin Patch\n*** Update File: ${path}\n@@\n-before\n+after\n*** End Patch\n`
}
function codexPatch(command) { return { tool_name: 'apply_patch', tool_input: { command } } }

for (const [stage, path, expected] of [
  ['implement', 'specs/features/example.feature', 2],
  ['implement', 'crates/fakeflac-core/tests/example.rs', 2],
  ['implement', 'src/lib/example.test.ts', 2],
  ['implement', 'src/lib/example.ts', 0],
  ['review', 'src/lib/example.ts', 2],
  ['review', 'docs/review.md', 0],
  ['', '../outside.md', 2],
]) {
  test(`@RP-04 @RP-06 Protect every path in a Codex patch: ${stage || 'free'} ${path}`, (t) => {
    const root = fixture(t, stage)
    const canonical = invoke(root, 'guard-protected', { tool_name: 'Edit', tool_input: { file_path: resolve(root, path) } }, true)
    const adapted = invoke(root, 'guard-protected', codexPatch(patch(path)))
    assert.equal(canonical.status, expected, canonical.stderr)
    assert.equal(adapted.status, canonical.status, adapted.stderr)
    if (expected === 2) assert.equal(adapted.stderr, canonical.stderr)
  })
}

for (const operation of ['Add', 'Update', 'Delete']) {
  test(`@RP-04 Protect every path in a Codex patch: protected ${operation.toLowerCase()} after an allowed file`, (t) => {
    const root = fixture(t, 'implement')
    put(root, 'src/allowed.ts', 'before\n')
    put(root, 'src/hidden.test.ts', 'before\n')
    const command = patch('src/allowed.ts').replace('*** End Patch\n', '') + patch('src/hidden.test.ts', operation).replace('*** Begin Patch\n', '')
    const result = invoke(root, 'guard-protected', codexPatch(command))
    assert.equal(result.status, 2, result.stderr)
    assert.match(result.stderr, /src\/hidden\.test\.ts/)
    assert.equal(readFileSync(join(root, 'src/allowed.ts'), 'utf8'), 'before\n')
    assert.equal(readFileSync(join(root, 'src/hidden.test.ts'), 'utf8'), 'before\n')
  })
}

test('@RP-04 Protect every path in a Codex patch: protected move destination and source', (t) => {
  const root = fixture(t, 'implement')
  for (const [source, destination] of [['src/plain.ts', 'src/moved.test.ts'], ['src/original.test.ts', 'src/plain.ts']]) {
    const result = invoke(root, 'guard-protected', codexPatch(`*** Begin Patch\n*** Update File: ${source}\n*** Move to: ${destination}\n@@\n-old\n+new\n*** End Patch\n`))
    assert.equal(result.status, 2, result.stderr)
    assert.match(result.stderr, /protected test/)
  }
})

for (const [command, expected] of [['git push origin main', 2], ['gh pr merge 7', 2], ['git status', 0]]) {
  test(`@RP-03 Preserve canonical shell and lifecycle hook behavior: exec_command ${command}`, (t) => {
    const root = fixture(t)
    const canonical = invoke(root, 'guard-bash', { tool_name: 'Bash', tool_input: { command } }, true)
    for (const payload of [{ tool_name: 'Bash', tool_input: { command } }, { tool_name: 'exec_command', tool_input: { cmd: command } }]) {
      const adapted = invoke(root, 'guard-bash', payload)
      assert.equal(adapted.status, expected, adapted.stderr)
      assert.equal(adapted.status, canonical.status)
      assert.equal(adapted.stderr, canonical.stderr)
    }
  })
}

for (const [stage, payload] of [['spec', {}], ['test', {}], ['implement', { stop_hook_active: true }]]) {
  test(`@RP-03 Preserve canonical shell and lifecycle hook behavior: stop skip ${stage}`, (t) => {
    const root = fixture(t, stage)
    const canonical = invoke(root, 'stop-check', payload, true)
    const adapted = invoke(root, 'stop-check', payload)
    assert.equal(canonical.status, 0, canonical.stderr)
    assert.deepEqual([adapted.status, adapted.stdout, adapted.stderr], [canonical.status, canonical.stdout, canonical.stderr])
  })
}

test('@RP-03 Preserve canonical shell and lifecycle hook behavior: same newest progress context', (t) => {
  const root = fixture(t, 'implement')
  put(root, 'docs/agents/progress.md', '# Progress\n\n## Latest\nNew evidence.\n\n## Older\nStale evidence.\n')
  const canonical = invoke(root, 'session-context', {}, true)
  const adapted = invoke(root, 'session-context', {})
  assert.equal(adapted.status, canonical.status)
  assert.equal(adapted.stdout, canonical.stdout)
  assert.match(adapted.stdout, /Stage: implement/)
  assert.match(adapted.stdout, /New evidence/)
  assert.doesNotMatch(adapted.stdout, /Stale evidence/)
})

test('@RP-03 Preserve canonical shell and lifecycle hook behavior: formatting forwards surviving patch paths only', (t) => {
  const root = fixture(t)
  for (const path of ['src/one.rs', 'src/two.rs', 'src/moved.rs']) put(root, path, 'fn main() {}\n')
  put(root, '.claude/hooks/format.mjs', `import { readFileSync, appendFileSync } from 'node:fs'\nimport { join } from 'node:path'\nconst p = JSON.parse(readFileSync(0, 'utf8'))\nappendFileSync(join(process.env.CLAUDE_PROJECT_DIR, 'format-calls.jsonl'), JSON.stringify(p) + '\\n')\n`)
  const command = '*** Begin Patch\n*** Add File: src/one.rs\n+new\n*** Update File: src/two.rs\n@@\n-old\n+new\n*** Delete File: src/deleted.rs\n*** Update File: src/source.rs\n*** Move to: src/moved.rs\n@@\n-old\n+new\n*** End Patch\n'
  const result = invoke(root, 'format', codexPatch(command))
  assert.equal(result.status, 0, result.stderr)
  const paths = readFileSync(join(root, 'format-calls.jsonl'), 'utf8').trim().split('\n').map((line) => JSON.parse(line).tool_input.file_path.replaceAll('\\', '/'))
  const suffix = (path) => path.replace(`${root.replaceAll('\\', '/')}/`, '')
  assert.deepEqual(paths.map(suffix).sort(), ['src/moved.rs', 'src/one.rs', 'src/two.rs'])
})

test('@RP-10 Preserve canonical shell and lifecycle hook behavior: supported Claude transcript context guard', (t) => {
  const root = fixture(t)
  const transcript = join(root, 'session.jsonl')
  put(root, 'session/subagents/agent-known.jsonl', JSON.stringify({ message: { usage: { input_tokens: 160_000 } } }) + '\n')
  const payload = { tool_name: 'SendMessage', transcript_path: transcript, tool_input: { to: 'known', message: 'continue' } }
  const canonical = invoke(root, 'guard-agents', payload, true)
  const adapted = invoke(root, 'guard-agents', payload)
  assert.equal(canonical.status, 2, canonical.stderr)
  assert.equal(adapted.status, canonical.status, adapted.stderr)
  assert.equal(adapted.stderr, canonical.stderr)
})

test('@RP-10 Report host-specific enforcement limits honestly: missing native transcript is identified', (t) => {
  const root = fixture(t)
  const result = invoke(root, 'guard-agents', { tool_name: 'send_input', tool_input: { id: 'codex-native-agent', message: 'continue' } })
  assert.equal(result.status, 0, result.stderr)
  assert.match(result.stdout + result.stderr, /unverified|unavailable|unsupported|not verified|cannot verify/i)
})
