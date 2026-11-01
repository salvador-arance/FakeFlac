// SessionStart: inject branch, stage, git status and the latest progress block.
import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { readInput, projectDir, currentStage } from './_lib.mjs'

const input = readInput()
const dir = projectDir(input)
const git = (...a) => {
  try {
    return execFileSync('git', a, { cwd: dir, encoding: 'utf8' }).trim()
  } catch {
    return ''
  }
}

const out = []
out.push(`Branch: ${git('rev-parse', '--abbrev-ref', 'HEAD') || '(no git)'}`)
out.push(`Stage: ${currentStage(dir) || '(free)'}`)
const status = git('status', '--short')
out.push(`Git status:\n${status || '(clean)'}`)

const progress = join(dir, 'docs', 'agents', 'progress.md')
if (existsSync(progress)) {
  const blocks = readFileSync(progress, 'utf8').split(/^## /m)
  const last = blocks.length > 1 ? '## ' + blocks[1] : blocks[0]
  out.push(`Latest progress block (docs/agents/progress.md):\n${last.trim()}`)
}
process.stdout.write(out.join('\n\n') + '\n')
