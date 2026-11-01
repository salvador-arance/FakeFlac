// Stop / SubagentStop: run `just check-fast`; if it fails, return the error so the agent keeps working.
// In the 'test' stage tests are expected to be red, so the check is skipped (3.11.6).
import { spawnSync } from 'node:child_process'
import { readInput, projectDir, currentStage, block } from './_lib.mjs'

const input = readInput()
if (input.stop_hook_active) process.exit(0) // avoid infinite loops

const dir = projectDir(input)
const stage = currentStage(dir)
if (stage === 'test' || stage === 'spec') process.exit(0)

const res = spawnSync('just', ['check-fast'], { cwd: dir, encoding: 'utf8', shell: process.platform === 'win32' })
if (res.error || res.status === null) {
  process.stderr.write('stop-check: `just` is not available, skipping check-fast.\n')
  process.exit(0)
}
if (res.status !== 0) {
  const tail = (res.stdout + res.stderr).split('\n').slice(-40).join('\n')
  block(`just check-fast failed. Fix it before finishing:\n${tail}`)
}
process.exit(0)
