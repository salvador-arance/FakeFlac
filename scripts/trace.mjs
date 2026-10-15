// Traceability check (S1, S2): requirement ids <-> scenario tags.
//  - every @RF-xx / @RNF-xx tag in specs/features must exist in specs/requirements
//  - every high-priority RF whose milestone is already specified must have >= 1 scenario
// Requirements without a scenario yet are reported as warnings while their spec is pending.
import { readFileSync, readdirSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(fileURLToPath(new URL('.', import.meta.url)), '..')
const reqDir = join(root, 'specs', 'requirements')
const featDir = join(root, 'specs', 'features')
const errors = []
const warnings = []

const reqs = new Map()
if (existsSync(reqDir)) {
  for (const f of readdirSync(reqDir).filter((f) => /^(RF|RNF|RP)-\d+\.md$/.test(f))) {
    const txt = readFileSync(join(reqDir, f), 'utf8')
    const get = (k) => txt.match(new RegExp(`^${k}:\\s*(.+)$`, 'm'))?.[1]?.trim()
    reqs.set(f.replace('.md', ''), { priority: get('priority'), status: get('status') })
  }
}

const tagged = new Set()
if (existsSync(featDir)) {
  for (const f of readdirSync(featDir).filter((f) => f.endsWith('.feature'))) {
    for (const m of readFileSync(join(featDir, f), 'utf8').matchAll(/@((?:RF|RNF|RP)-\d+)/g)) {
      tagged.add(m[1])
      if (!reqs.has(m[1])) errors.push(`${f}: tag @${m[1]} does not match any requirement`)
    }
  }
}

for (const [id, r] of reqs) {
  if (id.startsWith('RF-') && /^high/i.test(r.priority ?? '') && r.status === 'specified' && !tagged.has(id)) {
    errors.push(`${id}: high-priority requirement marked 'specified' has no scenario`)
  } else if (id.startsWith('RF-') && !tagged.has(id)) {
    warnings.push(`${id}: no scenario yet (status: ${r.status ?? 'unknown'})`)
  }
}

if (warnings.length) console.log(`trace: ${warnings.length} requirement(s) pending a scenario`)
if (errors.length) {
  console.error('trace: errors\n' + errors.map((e) => ' - ' + e).join('\n'))
  process.exit(1)
}
console.log(`trace: ok (${reqs.size} requirements, ${tagged.size} tagged)`)
