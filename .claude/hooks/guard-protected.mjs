// PreToolUse (Edit|Write|NotebookEdit): containment and stage-based protection (RP-04, RP-06).
import { readInput, projectDir, currentStage, relToProject, block } from './_lib.mjs'

const input = readInput()
const file = input.tool_input?.file_path ?? input.tool_input?.notebook_path
if (!file) process.exit(0)

const dir = projectDir(input)
const rel = relToProject(dir, file)

if (rel === null) {
  block(`Blocked: ${file} is outside the repository. Agents never write outside the repo (RP-06).`)
}

const isTest = (p) =>
  /(^|\/)tests\//.test(p) ||
  /\.(test|spec)\.(ts|js|mjs|svelte)$/.test(p) ||
  /\.stories\.(ts|svelte)$/.test(p) ||
  p.endsWith('.feature')
const isSpec = (p) => p.startsWith('specs/')

const stage = currentStage(dir)
if (stage === 'implement' && (isSpec(rel) || isTest(rel))) {
  block(
    `Blocked: stage is 'implement' and ${rel} is a spec or a protected test (RP-04). ` +
      `If it looks wrong, stop and report it: go back to the 'spec' or 'test' stage through /feature and gate G1. Never edit it in place.`,
  )
}
if (stage === 'review' && !rel.startsWith('docs/')) {
  block(`Blocked: stage is 'review' (read-only except docs/). ${rel} must not change during review.`)
}
process.exit(0)
