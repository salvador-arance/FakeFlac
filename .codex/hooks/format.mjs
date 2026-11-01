// PostToolUse (Edit|Write): format only the touched file (RP-08). Never blocks.
import { spawnSync } from 'node:child_process'
import { readInput, projectDir, relToProject } from './_lib.mjs'

const input = readInput()
const file = input.tool_input?.file_path
if (!file) process.exit(0)
const dir = projectDir(input)
const rel = relToProject(dir, file)
if (rel && rel.endsWith('.rs')) {
  spawnSync('rustfmt', ['--edition', '2021', file], { cwd: dir, stdio: 'ignore' })
}
process.exit(0)
