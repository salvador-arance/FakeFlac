// Codex wire-format adapter. Policy is exclusively in .claude/hooks (ADR-0048).
import { spawnSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..')

function run(name, input) {
  const result = spawnSync(process.execPath, [join(root, '.claude/hooks', `${name}.mjs`)], {
    input: JSON.stringify(input), encoding: 'utf8', env: { ...process.env, CLAUDE_PROJECT_DIR: root },
  })
  if (result.stdout) process.stdout.write(result.stdout)
  if (result.stderr) process.stderr.write(result.stderr)
  if (result.error || result.status === null) throw new Error(`Cannot execute canonical hook ${name}`)
  if (result.status !== 0) process.exit(result.status)
}

function patchPaths(patch) {
  if (typeof patch !== 'string' || !patch.startsWith('*** Begin Patch') || !patch.trimEnd().endsWith('*** End Patch')) throw new Error('Invalid Codex patch payload')
  const edits = []
  let current
  for (const line of patch.split(/\r?\n/)) {
    const operation = line.match(/^\*\*\* (Add|Update|Delete) File: (.+)$/)
    if (operation) {
      current = { source: operation[2], destination: operation[2], deleted: operation[1] === 'Delete' }
      edits.push(current)
    } else if (line.startsWith('*** Move to: ')) {
      if (!current || current.deleted) throw new Error('Invalid Codex move payload')
      current.destination = line.slice('*** Move to: '.length)
    }
  }
  if (!edits.length) throw new Error('Codex patch contains no file operations')
  return edits
}

try {
  const name = (process.argv[2] ?? '').replace(/\.mjs$/, '')
  if (!/^[\w-]+$/.test(name) || !existsSync(join(root, '.claude/hooks', `${name}.mjs`))) throw new Error('Unknown canonical hook')
  const raw = readFileSync(0, 'utf8')
  const input = raw.trim() ? JSON.parse(raw) : {}
  const tool = input.tool_input ?? {}
  if (name === 'guard-bash' && typeof tool.cmd === 'string' && tool.command === undefined) {
    run(name, { ...input, tool_input: { ...tool, command: tool.cmd } })
  } else if ((name === 'guard-protected' || name === 'format') && input.tool_name === 'apply_patch') {
    const edits = patchPaths(typeof tool === 'string' ? tool : tool.command ?? tool.input ?? tool.patch)
    const cwd = input.cwd || process.cwd()
    // Validate all source and destination paths before a formatter can run.
    if (name === 'guard-protected') {
      const paths = new Set(edits.flatMap((edit) => [edit.source, edit.destination]))
      for (const file of paths) run(name, { ...input, tool_input: { file_path: resolve(cwd, file) } })
    } else {
      const paths = new Set(edits.filter((edit) => !edit.deleted).map((edit) => edit.destination))
      for (const file of paths) {
        const absolute = resolve(cwd, file)
        if (existsSync(absolute)) run(name, { ...input, tool_input: { file_path: absolute } })
      }
    }
  } else if (name === 'guard-agents' && input.tool_name !== 'SendMessage') {
    process.stderr.write('Codex compatibility: native transcript context-limit enforcement is unverified; use a fresh agent per iteration.\n')
    run(name, input)
  } else run(name, input)
} catch (error) {
  process.stderr.write(`Codex hook adapter: ${error.message}\n`)
  process.exitCode = 2
}
