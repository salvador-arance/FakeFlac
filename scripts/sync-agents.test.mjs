// @RP-01 @RP-03 @RP-06 @RP-10: specs/features/agent-harness.feature.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, symlinkSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const generator = join(repo, 'scripts', 'sync-agents.mjs')
const roles = ['spec-writer', 'test-author', 'rust-implementer', 'svelte-implementer', 'code-reviewer']
const skills = ['new-spec', 'feature', 'handoff', 'new-adr', 'calibrate']
const scratch = join(repo, 'target', 'agent-harness-tests')
mkdirSync(scratch, { recursive: true })

function put(root, path, text) {
  mkdirSync(dirname(join(root, path)), { recursive: true })
  writeFileSync(join(root, path), text)
}

function git(root, ...args) {
  const result = spawnSync('git', ['-c', `safe.directory=${root}`, ...args], { cwd: root, encoding: 'utf8' })
  assert.equal(result.status, 0, result.stderr)
}

function fixture(t) {
  const root = mkdtempSync(join(scratch, 'sync-'))
  t.after(() => rmSync(root, { recursive: true, force: true }))
  put(root, 'CLAUDE.md', '# FakeFlac\r\nRead `.claude/stage`; preserve café and Unicode 音.\r\n')
  put(root, 'src/CLAUDE.md', '# Frontend\nUse Svelte 5 runes.\n')
  for (const name of roles) {
    put(root, `.claude/agents/${name}.md`, `---\nname: ${name}\ndescription: Role ${name}\ntools: Read, Bash\nmodel: ${name === 'code-reviewer' ? 'sonnet' : 'opus'}\n---\nRead \`src/CLAUDE.md\`. Stage: \`.claude/stage\`.\nPreserve quotes \"\"\", backslash \\ and 音.\n`)
  }
  for (const name of skills) {
    put(root, `.claude/skills/${name}/SKILL.md`, `---\nname: ${name}\ndescription: Skill ${name}\n---\n# ${name}\nUse \`.claude/stage\` and \`src/CLAUDE.md\`.\n`)
  }
  put(root, '.claude/skills/feature/references/detail.md', 'Preserve this supporting file.\n')
  const hooks = {
    SessionStart: [{ hooks: [{ type: 'command', command: 'node "$CLAUDE_PROJECT_DIR/.claude/hooks/session-context.mjs"' }] }],
    PreToolUse: [{ matcher: 'Edit|Write|NotebookEdit', hooks: [{ type: 'command', command: 'node "$CLAUDE_PROJECT_DIR/.claude/hooks/guard-protected.mjs"' }] }],
    Stop: [{ hooks: [{ type: 'command', command: 'node "$CLAUDE_PROJECT_DIR/.claude/hooks/stop-check.mjs"', timeout: 300 }] }],
  }
  put(root, '.claude/settings.json', JSON.stringify({ permissions: { allow: ['Bash(just:*)'], ask: ['Bash(git push:*)'], deny: ['Read(./.env*)'] }, hooks }, null, 2))
  for (const name of ['session-context', 'guard-protected', 'stop-check']) put(root, `.claude/hooks/${name}.mjs`, '// canonical hook\n')
  put(root, '.claude/settings.local.json', '{"private":"personal-value-do-not-copy"}\n')
  git(root, 'init', '-q')
  git(root, 'add', 'CLAUDE.md', 'src/CLAUDE.md', '.claude/agents', '.claude/skills', '.claude/hooks', '.claude/settings.json')
  return root
}

function run(root, ...args) {
  assert.ok(existsSync(generator), 'Canonical-to-Codex generator is not implemented')
  return spawnSync(process.execPath, [generator, '--root', root, ...args], { cwd: root, encoding: 'utf8' })
}

function ok(root, ...args) {
  const result = run(root, ...args)
  assert.equal(result.status, 0, result.stdout + result.stderr)
  return result
}

function read(root, path) { return readFileSync(join(root, path), 'utf8') }
function files(root, path = '') {
  const out = {}
  for (const entry of readdirSync(join(root, path), { withFileTypes: true })) {
    if (entry.name === '.git') continue
    const relative = path ? `${path}/${entry.name}` : entry.name
    if (entry.isDirectory()) Object.assign(out, files(root, relative))
    else if (entry.isFile()) out[relative] = read(root, relative)
  }
  return out
}
function tomlBasicStrings(text) {
  // Independent decoder for the standard TOML basic strings emitted by the CLI.
  const fields = {}
  for (const line of text.split(/\r?\n/)) {
    if (!line.trim() || line.trimStart().startsWith('#')) continue
    const match = line.match(/^([A-Za-z_][\w]*)\s*=\s*("(?:[^"\\]|\\.)*")\s*$/)
    if (match) fields[match[1]] = JSON.parse(match[2])
  }
  return fields
}

test('@RP-01 Generate Codex configuration from Claude sources: scoped instructions, five roles and skills preserve content', (t) => {
  const root = fixture(t)
  const before = files(root)
  ok(root)
  for (const path of ['CLAUDE.md', 'src/CLAUDE.md']) {
    const output = read(root, path.replace('CLAUDE.md', 'AGENTS.md'))
    assert.match(output, /generated/i)
    assert.match(output, /just agents-sync/)
    assert.ok(output.includes(before[path]), `${path}: source instructions preserved`)
  }
  for (const name of roles) {
    const source = before[`.claude/agents/${name}.md`]
    const body = source.split('---\n').slice(2).join('---\n')
    const output = read(root, `.codex/agents/${name}.toml`)
    const fields = tomlBasicStrings(output)
    assert.equal(fields.name, name)
    assert.equal(fields.description, `Role ${name}`)
    assert.equal(fields.developer_instructions.trim(), body.trim())
    assert.equal(fields.model, undefined, 'Claude model aliases must not become Codex models')
    assert.match(output, /generated/i)
  }
  for (const name of skills) assert.ok(read(root, `.agents/skills/${name}/SKILL.md`).includes(before[`.claude/skills/${name}/SKILL.md`]))
  assert.equal(read(root, '.agents/skills/feature/references/detail.md'), before['.claude/skills/feature/references/detail.md'])
  for (const [path, source] of Object.entries(before)) assert.equal(read(root, path), source, `canonical/local source unchanged: ${path}`)
  const output = JSON.parse(read(root, '.codex/hooks.json'))
  assert.deepEqual(Object.keys(output.hooks), ['SessionStart', 'PreToolUse', 'Stop'])
  assert.equal(output.hooks.PreToolUse[0].matcher, 'Edit|Write|NotebookEdit')
  assert.equal(output.hooks.Stop[0].hooks[0].timeout, 300)
  assert.ok(!read(root, '.codex/hooks.json').includes('personal-value-do-not-copy'))
  assert.ok(!('permissions' in output))
  assert.ok(!read(root, '.codex/hooks.json').includes('CLAUDE_PROJECT_DIR'))
})

test('@RP-03 One canonical edit updates the derived configuration: read-only check and byte-stable generation', (t) => {
  const root = fixture(t)
  ok(root)
  const first = files(root)
  const timestamp = statSync(join(root, 'AGENTS.md')).mtimeMs
  ok(root)
  assert.deepEqual(files(root), first)
  assert.equal(statSync(join(root, 'AGENTS.md')).mtimeMs, timestamp, 'unchanged output is not rewritten')
  put(root, 'CLAUDE.md', '# Changed once\nNew canonical rule.\n')
  const beforeCheck = files(root)
  const result = run(root, '--check')
  assert.notEqual(result.status, 0)
  assert.match(result.stdout + result.stderr, /AGENTS\.md/)
  assert.match(result.stdout + result.stderr, /just agents-sync/)
  assert.deepEqual(files(root), beforeCheck)
  ok(root)
  assert.match(read(root, 'AGENTS.md'), /New canonical rule/)
  ok(root, '--check')
})

for (const [source, output, change] of [
  ['.claude/agents/test-author.md', '.codex/agents/test-author.toml', (s) => s.replace('Role test-author', 'Updated test author')],
  ['.claude/skills/feature/SKILL.md', '.agents/skills/feature/SKILL.md', (s) => `${s}\nFresh canonical skill rule.\n`],
  ['.claude/settings.json', '.codex/hooks.json', (s) => s.replace('"timeout": 300', '"timeout": 250')],
]) {
  test(`@RP-03 One canonical edit updates the derived configuration: ${source}`, (t) => {
    const root = fixture(t)
    ok(root)
    put(root, source, change(read(root, source)))
    const result = run(root, '--check')
    assert.notEqual(result.status, 0)
    assert.ok((result.stdout + result.stderr).includes(output))
    ok(root)
    ok(root, '--check')
  })
}

test('@RP-06 Manage source additions and removals safely: only obsolete managed outputs are removed', (t) => {
  const root = fixture(t)
  ok(root)
  put(root, '.agents/skills/personal/SKILL.md', 'unowned personal skill\n')
  put(root, '.codex/agents/personal.toml', '# unowned local agent\n')
  put(root, '.codex/config.toml', '# local host settings\n')
  put(root, 'crates/domain/CLAUDE.md', 'New scope rule.\n')
  put(root, '.claude/skills/new-skill/SKILL.md', '---\nname: new-skill\ndescription: New skill\n---\nNew skill body.\n')
  rmSync(join(root, '.claude/skills/calibrate'), { recursive: true })
  rmSync(join(root, '.claude/agents/code-reviewer.md'))
  rmSync(join(root, 'src/CLAUDE.md'))
  git(root, 'add', '-A')
  ok(root)
  assert.match(read(root, 'crates/domain/AGENTS.md'), /New scope rule/)
  assert.match(read(root, '.agents/skills/new-skill/SKILL.md'), /New skill body/)
  assert.equal(existsSync(join(root, '.agents/skills/calibrate/SKILL.md')), false)
  assert.equal(existsSync(join(root, '.codex/agents/code-reviewer.toml')), false)
  assert.equal(existsSync(join(root, 'src/AGENTS.md')), false)
  assert.equal(read(root, '.agents/skills/personal/SKILL.md'), 'unowned personal skill\n')
  assert.equal(read(root, '.codex/agents/personal.toml'), '# unowned local agent\n')
  assert.equal(read(root, '.codex/config.toml'), '# local host settings\n')
  assert.match(read(root, '.claude/settings.local.json'), /personal-value-do-not-copy/)
  ok(root, '--check')
})

test('@RP-06 Reject invalid input before changing outputs: malformed role metadata', (t) => {
  const root = fixture(t)
  ok(root)
  put(root, 'CLAUDE.md', 'A valid change must not be partly published.\n')
  put(root, '.claude/agents/test-author.md', '---\nname: [invalid\n---\nBad metadata.\n')
  const before = files(root)
  const result = run(root)
  assert.notEqual(result.status, 0)
  assert.match(result.stdout + result.stderr, /invalid|metadata|front.?matter|description|name/i)
  assert.deepEqual(files(root), before)
})

test('@RP-06 Reject invalid input before changing outputs: escaping managed manifest path', (t) => {
  const root = fixture(t)
  ok(root)
  put(root, '.codex/generated-files.json', JSON.stringify({ version: 1, files: ['../outside-sentinel.md'] }))
  const before = files(root)
  const result = run(root)
  assert.notEqual(result.status, 0)
  assert.deepEqual(files(root), before)
})

test('@RP-06 Reject invalid input before changing outputs: output directory junction escapes fixture', (t) => {
  const root = fixture(t)
  const outside = mkdtempSync(join(scratch, 'outside-'))
  t.after(() => rmSync(outside, { recursive: true, force: true }))
  put(outside, 'sentinel.md', 'untouched\n')
  mkdirSync(join(root, '.codex'), { recursive: true })
  symlinkSync(outside, join(root, '.codex/agents'), process.platform === 'win32' ? 'junction' : 'dir')
  const before = files(outside)
  const result = run(root)
  assert.notEqual(result.status, 0)
  assert.deepEqual(files(outside), before)
  assert.equal(existsSync(join(root, 'AGENTS.md')), false, 'all destinations are preflighted before any output')
})

test('@RP-03 Fail existing checks on generated drift: check-fast includes agents-check', () => {
  const justfile = read(repo, 'justfile')
  const fast = justfile.match(/^check-fast[^\n]*\n((?:[ \t].*\n|\n)*)/m)?.[0] ?? ''
  assert.match(fast, /agents-check/)
  assert.match(justfile, /^agents-check\s*:/m)
  assert.match(justfile, /^agents-sync\s*:/m)
})

test('@RP-06 Manage source additions and removals safely: canonical dispatch never owns the Codex adapter', (t) => {
  const root = fixture(t)
  const adapter = '// handwritten compatibility adapter; preserve exactly\n'
  put(root, '.codex/hooks/dispatch.mjs', adapter)
  ok(root)
  put(root, '.claude/hooks/dispatch.mjs', '// new canonical dispatch policy\n')
  put(root, '.claude/hooks/new-hook.mjs', '// another canonical hook\n')
  const settings = JSON.parse(read(root, '.claude/settings.json'))
  settings.hooks.SessionStart[0].hooks.push({ type: 'command', command: 'node "$CLAUDE_PROJECT_DIR/.claude/hooks/dispatch.mjs"' })
  put(root, '.claude/settings.json', JSON.stringify(settings))
  ok(root)
  assert.equal(read(root, '.codex/hooks/dispatch.mjs'), adapter)
  assert.ok(!JSON.parse(read(root, '.codex/generated-files.json')).files.includes('.codex/hooks/dispatch.mjs'))
  assert.match(read(root, '.codex/hooks/new-hook.mjs'), /\.claude\/hooks\/new-hook\.mjs/)
  const hooks = JSON.parse(read(root, '.codex/hooks.json')).hooks.SessionStart[0].hooks
  assert.match(hooks.at(-1).command, /dispatch\.mjs" dispatch\.mjs$/)
  ok(root, '--check')
})

test('@RP-06 Reject invalid input before changing outputs: unowned initial instruction collision', (t) => {
  const root = fixture(t)
  put(root, 'AGENTS.md', '# Personal instructions; not owned by the generator\n')
  const before = files(root)
  const result = run(root)
  assert.notEqual(result.status, 0, 'must reject an existing output without generated ownership')
  assert.deepEqual(files(root), before, 'collision is detected before any output mutation')
})

test('@RP-06 Reject invalid input before changing outputs: new canonical skill cannot overwrite an unowned skill', (t) => {
  const root = fixture(t)
  ok(root)
  put(root, '.agents/skills/personal/SKILL.md', '# Personal skill; not generated\n')
  put(root, '.claude/skills/personal/SKILL.md', '---\nname: personal\ndescription: New canonical skill\n---\nCanonical body.\n')
  put(root, 'CLAUDE.md', '# Another canonical change must remain unpublished on collision\n')
  const before = files(root)
  const result = run(root)
  assert.notEqual(result.status, 0, 'a new canonical source does not grant ownership of an existing local output')
  assert.deepEqual(files(root), before, 'all collision checks happen before any output mutation')
})
