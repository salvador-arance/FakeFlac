// Shared helpers for the harness hooks (ADR-020). Hooks receive a JSON payload on stdin.
// Exit code 2 blocks the action and sends stderr back to the agent.
import { existsSync, readFileSync } from 'node:fs'
import { join, resolve, relative, isAbsolute, sep } from 'node:path'

export function readInput() {
  try {
    const raw = readFileSync(0, 'utf8')
    return raw.trim() ? JSON.parse(raw) : {}
  } catch {
    return {}
  }
}

export function projectDir(input = {}) {
  return resolve(process.env.CLAUDE_PROJECT_DIR || input.cwd || process.cwd())
}

/** Current workflow stage: spec | test | implement | review (set by /feature), or '' when free. */
export function currentStage(dir) {
  const f = join(dir, '.claude', 'stage')
  return existsSync(f) ? readFileSync(f, 'utf8').trim() : ''
}

/** Path relative to the project, with forward slashes; null when outside of it. */
export function relToProject(dir, file) {
  const abs = isAbsolute(file) ? resolve(file) : resolve(dir, file)
  const rel = relative(dir, abs)
  if (rel === '' || rel.startsWith('..') || isAbsolute(rel)) return null
  return rel.split(sep).join('/')
}

export function block(message) {
  process.stderr.write(message + '\n')
  process.exit(2)
}
