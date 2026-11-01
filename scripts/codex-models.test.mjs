// @RP-01 @RP-03 @RP-06 @RP-10: specs/features/agent-harness.feature and its approved model amendment.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const generator = join(repo, 'scripts/sync-agents.mjs')
const source = '.claude/codex-models.json'
const scratch = join(repo, 'target/codex-model-tests')
const assignments = {
  'spec-writer': { model: 'gpt-6-astra', model_reasoning_effort: 'high' },
  'test-author': { model: 'gpt-6.1-sol', model_reasoning_effort: 'high' },
  'rust-implementer': { model: 'gpt-6.1-sol', model_reasoning_effort: 'high' },
  'svelte-implementer': { model: 'gpt-6.1-sol', model_reasoning_effort: 'medium' },
  'code-reviewer': { model: 'gpt-6-astra', model_reasoning_effort: 'high' },
}
const roleNames = Object.keys(assignments)
const body = 'Read `src/CLAUDE.md` and `.claude/stage`. Preserve """, \\ and 音.\n'
mkdirSync(scratch, { recursive: true })

function put(root, relative, content) {
  mkdirSync(dirname(join(root, relative)), { recursive: true })
  writeFileSync(join(root, relative), content)
}
function config(roles = roleNames) {
  return { version: 1, agents: Object.fromEntries(roles.map((name) => [name, { ...assignments[name] }])) }
}
function save(root, value) { put(root, source, JSON.stringify(value, null, 2) + '\n') }
function fixture(t, roles = roleNames) {
  const root = mkdtempSync(join(scratch, 'models-'))
  t.after(() => rmSync(root, { recursive: true, force: true }))
  put(root, 'CLAUDE.md', '# Canonical instructions\n')
  put(root, '.claude/settings.json', '{"hooks":{}}\n')
  for (const name of roles) {
    put(root, `.claude/agents/${name}.md`, `---\nname: ${name}\ndescription: Role ${name}\nmodel: ${name === 'code-reviewer' ? 'sonnet' : 'opus'}\n---\n${body}`)
  }
  for (const args of [['init', '-q'], ['add', 'CLAUDE.md', '.claude']]) {
    const result = spawnSync('git', ['-c', `safe.directory=${root}`, ...args], { cwd: root, encoding: 'utf8' })
    assert.equal(result.status, 0, result.stderr)
  }
  return root
}
function run(root, ...args) {
  return spawnSync(process.execPath, [generator, '--root', root, ...args], { cwd: root, encoding: 'utf8' })
}
function ok(root, ...args) {
  const result = run(root, ...args)
  assert.equal(result.status, 0, result.stdout + result.stderr)
}
function snapshot(root, relative = '') {
  const files = {}
  for (const entry of readdirSync(join(root, relative), { withFileTypes: true })) {
    if (entry.name === '.git') continue
    const path = relative ? `${relative}/${entry.name}` : entry.name
    if (entry.isDirectory()) Object.assign(files, snapshot(root, path))
    else files[path] = { content: readFileSync(join(root, path), 'utf8'), mtime: statSync(join(root, path)).mtimeMs }
  }
  return files
}
function fields(root, name) {
  const text = readFileSync(join(root, `.codex/agents/${name}.toml`), 'utf8')
  return Object.fromEntries(text.split(/\r?\n/).flatMap((line) => {
    const match = line.match(/^([a-z_]+)\s*=\s*("(?:[^"\\]|\\.)*")\s*$/)
    return match ? [[match[1], JSON.parse(match[2])]] : []
  }))
}

test('@RP-01 Generate the owner-approved Codex model assignments: five roles preserve Claude sources and bodies', (t) => {
  const root = fixture(t)
  save(root, config())
  const before = snapshot(root)
  ok(root)
  for (const [name, assignment] of Object.entries(assignments)) {
    const emitted = fields(root, name)
    assert.equal(emitted.model, assignment.model, name)
    assert.equal(emitted.model_reasoning_effort, assignment.model_reasoning_effort, name)
    assert.equal(emitted.developer_instructions, body)
  }
  const after = snapshot(root)
  for (const [path, value] of Object.entries(before)) assert.deepEqual(after[path], value, `canonical source unchanged: ${path}`)
  ok(root, '--check')
  ok(root)
  assert.deepEqual(snapshot(root), after, 'repeated generation does not rewrite any file')
})

test('@RP-01 Generate Codex configuration from Claude sources: absent optional model source inherits the host model', (t) => {
  const root = fixture(t)
  const before = snapshot(root)
  ok(root)
  for (const name of roleNames) {
    const emitted = fields(root, name)
    assert.equal(emitted.model, undefined)
    assert.equal(emitted.model_reasoning_effort, undefined)
    assert.equal(emitted.developer_instructions, body)
  }
  const after = snapshot(root)
  for (const [path, value] of Object.entries(before)) assert.deepEqual(after[path], value)
})

for (const [field, value] of [['model', 'gpt-6.1-sol'], ['model_reasoning_effort', 'xhigh']]) {
  test(`@RP-03 Generate the owner-approved Codex model assignments: ${field} change triggers read-only drift and regeneration`, (t) => {
    const root = fixture(t)
    const models = config()
    save(root, models)
    ok(root)
    models.agents['spec-writer'][field] = value
    save(root, models)
    const before = snapshot(root)
    const result = run(root, '--check')
    assert.notEqual(result.status, 0, 'changed assignment must trigger generated drift')
    assert.match(result.stdout + result.stderr, /\.codex\/agents\/spec-writer\.toml/)
    assert.match(result.stdout + result.stderr, /just agents-sync/)
    assert.deepEqual(snapshot(root), before, 'check mode writes no files')
    ok(root)
    assert.equal(fields(root, 'spec-writer')[field], value)
    const after = snapshot(root)
    for (const path of Object.keys(before).filter((p) => p !== '.codex/agents/spec-writer.toml')) assert.deepEqual(after[path], before[path], path)
    ok(root, '--check')
    ok(root)
    assert.deepEqual(snapshot(root), after)
  })
}

test('@RP-01 Generate the owner-approved Codex model assignments: all supported reasoning efforts are accepted', (t) => {
  const root = fixture(t)
  const models = config()
  const efforts = ['low', 'medium', 'high', 'xhigh', 'max']
  roleNames.forEach((name, index) => { models.agents[name].model_reasoning_effort = efforts[index] })
  save(root, models)
  ok(root)
  roleNames.forEach((name, index) => assert.equal(fields(root, name).model_reasoning_effort, efforts[index]))
})

test('@RP-06 Reject invalid Codex model assignments before changing outputs: invalid schema and assignments', async (t) => {
  const roles = roleNames.slice(0, 2)
  const root = fixture(t, roles)
  save(root, config(roles))
  ok(root)
  const cases = [
    ['null source', () => null],
    ['array source', () => []],
    ['missing version', (v) => { delete v.version }],
    ['unsupported version', (v) => { v.version = 2 }],
    ['non-numeric version', (v) => { v.version = '1' }],
    ['unknown source field', (v) => { v.extra = true }],
    ['missing agents', (v) => { delete v.agents }],
    ['null agents', (v) => { v.agents = null }],
    ['array agents', (v) => { v.agents = [] }],
    ['missing canonical role', (v) => { delete v.agents['test-author'] }],
    ['empty assignments', (v) => { v.agents = {} }],
    ['unknown role', (v) => { v.agents.unknown = { ...v.agents['test-author'] } }],
    ['null assignment', (v) => { v.agents['test-author'] = null }],
    ['array assignment', (v) => { v.agents['test-author'] = [] }],
    ['unknown assignment field', (v) => { v.agents['test-author'].extra = true }],
    ['missing model', (v) => { delete v.agents['test-author'].model }],
    ['missing effort', (v) => { delete v.agents['test-author'].model_reasoning_effort }],
    ...['opus', 'gpt-', 'gpt-6 Astra', 'gpt-6/sol', 'GPT-6-sol', '', 6, null].map((model) => [`invalid model ${JSON.stringify(model)}`, (v) => { v.agents['test-author'].model = model }]),
    ...['ultra', 'HIGH', '', 1, null].map((effort) => [`invalid effort ${JSON.stringify(effort)}`, (v) => { v.agents['test-author'].model_reasoning_effort = effort }]),
  ]
  for (const [name, invalidate] of cases) {
    await t.test(`@RP-06 Reject invalid Codex model assignments before changing outputs: ${name}`, () => {
      const models = config(roles)
      const replacement = invalidate(models)
      save(root, replacement === undefined ? models : replacement)
      put(root, 'CLAUDE.md', `# Valid pending change: ${name}\n`)
      const before = snapshot(root)
      const result = run(root)
      assert.notEqual(result.status, 0, `must reject ${name}`)
      assert.match(result.stdout + result.stderr, /codex-models\.json|model assignment|model configuration/i)
      assert.deepEqual(snapshot(root), before, 'invalid assignment must prevent all output mutations')
    })
  }
})

test('@RP-06 Reject invalid Codex model assignments before changing outputs: malformed JSON cannot create initial outputs', (t) => {
  const root = fixture(t)
  put(root, source, '{"version":1,"agents":')
  const before = snapshot(root)
  const result = run(root)
  assert.notEqual(result.status, 0)
  assert.match(result.stdout + result.stderr, /codex-models\.json|model assignment|model configuration/i)
  assert.deepEqual(snapshot(root), before)
})
