// ADR-0048 / specs/contracts/agent-harness.md. Claude sources -> Codex discovery files.
// No provider policy lives here: only format conversion, ownership and drift checks.
import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const manifestPath = '.codex/generated-files.json'
const notice = (source) => `Generated from ${source}; run just agents-sync. Do not edit.`

function fail(message) { throw new Error(message) }

// Preflight every ancestor, including existing junctions, before reading or writing a path.
function contained(root, relative) {
  if (!relative || relative.includes('\\') || path.posix.isAbsolute(relative) || /^[A-Za-z]:/.test(relative) || relative.split('/').some((p) => !p || p === '.' || p === '..')) {
    fail(`Invalid repository-relative path: ${relative}`)
  }
  let current = root
  for (const part of relative.split('/')) {
    current = path.join(current, part)
    let entry
    try { entry = fs.lstatSync(current) } catch (error) { if (error.code !== 'ENOENT') throw error }
    if (entry?.isSymbolicLink()) fail(`Symlink/junction is not supported: ${relative}`)
  }
  return current
}

function isManaged(relative) {
  return relative === 'AGENTS.md' || /\/AGENTS\.md$/.test(relative) ||
    relative.startsWith('.agents/skills/') || /^\.codex\/agents\/[^/]+\.toml$/.test(relative) ||
    relative === '.codex/hooks.json' || /^\.codex\/hooks\/(?!dispatch\.mjs$)[^/]+\.mjs$/.test(relative)
}

function collect(root, directory) {
  const absolute = contained(root, directory)
  if (!fs.existsSync(absolute)) return []
  const files = []
  for (const entry of fs.readdirSync(absolute, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name, 'en'))) {
    const relative = `${directory}/${entry.name}`
    contained(root, relative)
    if (entry.isDirectory()) files.push(...collect(root, relative))
    else if (entry.isFile()) files.push(relative)
    else fail(`Unsupported source file: ${relative}`)
  }
  return files
}

function metadata(text, source) {
  const match = text.match(/^\uFEFF?---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)([\s\S]*)$/)
  if (!match) fail(`Invalid front matter: ${source}`)
  const fields = {}
  for (const line of match[1].split(/\r?\n/)) {
    if (!line.trim() || line.trimStart().startsWith('#')) continue
    const field = line.match(/^([\w-]+):\s*(.*)$/)
    if (!field || Object.hasOwn(fields, field[1])) fail(`Invalid or duplicate metadata in ${source}`)
    let value = field[2].trim()
    if (value.startsWith('"')) {
      try { value = JSON.parse(value) } catch { fail(`Invalid quoted metadata in ${source}`) }
    } else if (value.startsWith("'")) {
      if (!value.endsWith("'")) fail(`Invalid quoted metadata in ${source}`)
      value = value.slice(1, -1).replaceAll("''", "'")
    }
    fields[field[1]] = value
  }
  if (typeof fields.name !== 'string' || !/^[a-zA-Z0-9][\w-]*$/.test(fields.name) || typeof fields.description !== 'string' || !fields.description.trim()) {
    fail(`Invalid name/description metadata: ${source}`)
  }
  return { fields, body: match[2] }
}

function modelAssignments(root, names) {
  const source = '.claude/codex-models.json'
  const file = contained(root, source)
  if (!fs.existsSync(file)) return null
  const invalid = (detail) => fail(`Invalid model configuration in ${source}: ${detail}`)
  const object = (value) => value !== null && typeof value === 'object' && !Array.isArray(value)
  const exactKeys = (value, keys) => object(value) && Object.keys(value).length === keys.length && keys.every((key) => Object.hasOwn(value, key))
  let config
  try { config = JSON.parse(fs.readFileSync(file, 'utf8')) } catch { invalid('malformed JSON') }
  if (!exactKeys(config, ['version', 'agents']) || config.version !== 1 || !object(config.agents)) invalid('expected version 1 and an agents object')
  if (!exactKeys(config.agents, names)) invalid('assign exactly the canonical role names')
  const efforts = new Set(['low', 'medium', 'high', 'xhigh', 'max'])
  for (const name of names) {
    const entry = config.agents[name]
    if (!exactKeys(entry, ['model', 'model_reasoning_effort']) ||
      typeof entry.model !== 'string' || !/^gpt-[a-z0-9]+(?:[.-][a-z0-9]+)*$/.test(entry.model) || !efforts.has(entry.model_reasoning_effort)) {
      invalid(`invalid model assignment for ${name}`)
    }
  }
  return config.agents
}

function build(root) {
  const outputs = new Map()
  const read = (relative) => fs.readFileSync(contained(root, relative))
  const emit = (relative, content) => {
    if (outputs.has(relative)) fail(`Duplicate output: ${relative}`)
    outputs.set(relative, Buffer.isBuffer(content) ? content : Buffer.from(content))
  }
  const tracked = spawnSync('git', ['-c', `safe.directory=${root.replaceAll('\\', '/')}`, 'ls-files', '-z', '--', 'CLAUDE.md', '**/CLAUDE.md'], { cwd: root, encoding: 'utf8' })
  if (tracked.status !== 0) fail('Cannot discover tracked CLAUDE.md instructions with git ls-files')
  for (const source of tracked.stdout.split('\0').filter(Boolean).sort()) {
    if (!fs.existsSync(contained(root, source))) continue // tracked deletion
    emit(source.replace(/CLAUDE\.md$/, 'AGENTS.md'), `<!-- ${notice(source)} -->\n\n${read(source).toString('utf8')}`)
  }

  for (const source of collect(root, '.claude/skills')) {
    const output = source.replace('.claude/skills/', '.agents/skills/')
    const content = read(source)
    if (source.endsWith('/SKILL.md')) {
      metadata(content.toString('utf8'), source)
      emit(output, Buffer.concat([content, Buffer.from(`\n<!-- ${notice(source)} -->\n`)]))
    } else emit(output, content)
  }
  const agentSources = collect(root, '.claude/agents').filter((f) => f.endsWith('.md'))
  const models = modelAssignments(root, agentSources.map((source) => path.posix.basename(source, '.md')))
  for (const source of agentSources) {
    const { fields, body } = metadata(read(source).toString('utf8'), source)
    const filename = path.posix.basename(source, '.md')
    if (fields.name !== filename) fail(`Agent name must match its filename: ${source}`)
    const annotations = Object.entries(fields).filter(([key]) => key !== 'name' && key !== 'description')
      .map(([key, value]) => `# Claude ${key} (metadata only, not Codex enforcement): ${JSON.stringify(value)}\n`).join('')
    const assignment = models ? `# Codex model source: .claude/codex-models.json\nmodel = ${JSON.stringify(models[filename].model)}\nmodel_reasoning_effort = ${JSON.stringify(models[filename].model_reasoning_effort)}\n` : ''
    emit(`.codex/agents/${filename}.toml`, `# ${notice(source)}\n${annotations}${assignment}name = ${JSON.stringify(fields.name)}\ndescription = ${JSON.stringify(fields.description)}\ndeveloper_instructions = ${JSON.stringify(body)}\n`)
  }

  const hookSources = collect(root, '.claude/hooks').filter((f) => f.endsWith('.mjs'))
  for (const source of hookSources) {
    const name = path.posix.basename(source)
    if (name === 'dispatch.mjs') continue // reserved handwritten adapter, never a generated wrapper
    const statement = name === '_lib.mjs' ? `export * from '../../${source}'` : `import '../../${source}'`
    emit(`.codex/hooks/${name}`, `// ${notice(source)}\n${statement}\n`)
  }
  let settings
  try { settings = JSON.parse(read('.claude/settings.json').toString('utf8')) } catch { fail('Invalid .claude/settings.json') }
  if (!settings.hooks || typeof settings.hooks !== 'object' || Array.isArray(settings.hooks)) fail('Invalid canonical hook registrations')
  const hooks = structuredClone(settings.hooks)
  for (const [event, groups] of Object.entries(hooks)) {
    if (!Array.isArray(groups)) fail(`Invalid hook event: ${event}`)
    for (const group of groups) {
      if (!Array.isArray(group.hooks)) fail(`Invalid hook group: ${event}`)
      // Codex native message tools have no Claude matcher alias.
      if (group.matcher?.split('|').includes('SendMessage')) group.matcher += '|send_input|resume_agent|followup_task'
      for (const hook of group.hooks) {
        const match = typeof hook.command === 'string' && hook.command.match(/^node "\$CLAUDE_PROJECT_DIR\/\.claude\/hooks\/([\w-]+\.mjs)"$/)
        if (hook.type !== 'command' || !match || !hookSources.includes(`.claude/hooks/${match[1]}`)) fail(`Unsupported canonical hook command in ${event}`)
        hook.command = 'node "$(git rev-parse --show-toplevel)/.codex/hooks/dispatch.mjs" ' + match[1]
      }
    }
  }
  emit('.codex/hooks.json', JSON.stringify({ hooks }, null, 2) + '\n')
  return outputs
}

function main() {
  let root = repo
  let check = false
  const args = process.argv.slice(2)
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--check') check = true
    else if (args[i] === '--root' && args[i + 1]) root = path.resolve(args[++i])
    else fail('Usage: node scripts/sync-agents.mjs [--check] [--root REPOSITORY]')
  }
  const realRoot = fs.realpathSync(root)
  const relativeRoot = path.relative(repo, realRoot)
  if (relativeRoot.startsWith('..') || path.isAbsolute(relativeRoot)) fail('Root must stay inside this repository')
  if (realRoot !== root && path.resolve(realRoot).toLowerCase() !== root.toLowerCase()) fail('Repository root must not be a symlink')
  const outputs = build(root)
  let previous = []
  const manifest = contained(root, manifestPath)
  if (fs.existsSync(manifest)) {
    const saved = JSON.parse(fs.readFileSync(manifest, 'utf8'))
    if (saved.version !== 1 || !Array.isArray(saved.files)) fail('Invalid generated-files manifest')
    previous = saved.files
    for (const relative of previous) {
      if (typeof relative !== 'string' || !isManaged(relative)) fail('Invalid managed manifest path')
      contained(root, relative)
    }
  }
  const files = [...outputs.keys()].sort()
  outputs.set(manifestPath, Buffer.from(JSON.stringify({ version: 1, files }, null, 2) + '\n'))
  const obsolete = previous.filter((relative) => !outputs.has(relative))
  const owned = new Set(previous)
  // All input validation and destination preflight happen before the first mutation.
  for (const relative of [...outputs.keys(), ...obsolete]) {
    const absolute = contained(root, relative)
    if (fs.existsSync(absolute) && !fs.statSync(absolute).isFile()) fail(`Output is not a file: ${relative}`)
    if (relative !== manifestPath && outputs.has(relative) && fs.existsSync(absolute) && !owned.has(relative)) {
      fail(`Refusing to overwrite unowned output: ${relative}`)
    }
  }
  const changed = [...outputs].filter(([relative, content]) => {
    const absolute = path.join(root, relative)
    return !fs.existsSync(absolute) || !fs.readFileSync(absolute).equals(content)
  })
  const removed = obsolete.filter((relative) => fs.existsSync(path.join(root, relative)))
  if (check && (changed.length || removed.length)) {
    fail(`Generated drift:\n${[...changed.map(([relative]) => relative), ...removed].map((p) => ` - ${p}`).join('\n')}\nRun just agents-sync.`)
  }
  if (!check) {
    for (const [relative, content] of changed) {
      const absolute = path.join(root, relative)
      fs.mkdirSync(path.dirname(absolute), { recursive: true })
      fs.writeFileSync(absolute, content)
    }
    for (const relative of removed) fs.unlinkSync(path.join(root, relative))
  }
  process.stdout.write(`agents-${check ? 'check' : 'sync'}: ${files.length} managed files; ${changed.length} updated, ${removed.length} removed\n`)
}

try { main() } catch (error) {
  process.stderr.write(`agents-sync: ${error.message}\n`)
  process.exitCode = 1
}
