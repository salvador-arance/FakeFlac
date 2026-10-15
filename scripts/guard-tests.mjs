// S7 (CI side): once a `test(RF-xx)` commit exists on a branch, later commits on that branch
// must not modify the files that commit touched. Compares against the merge base with main.
import { execFileSync } from 'node:child_process'

const git = (...args) => execFileSync('git', args, { encoding: 'utf8' }).trim()

let base
try {
  base = git('merge-base', 'HEAD', 'origin/main')
} catch {
  try {
    base = git('merge-base', 'HEAD', 'main')
  } catch {
    console.log('guard-tests: no base branch found, skipping')
    process.exit(0)
  }
}

const commits = git('rev-list', '--reverse', `${base}..HEAD`).split('\n').filter(Boolean)
const protectedFiles = new Set()
const violations = []
for (const sha of commits) {
  const subject = git('log', '-1', '--format=%s', sha)
  const files = git('diff-tree', '--no-commit-id', '--name-only', '-r', sha).split('\n').filter(Boolean)
  if (/^test\(/.test(subject)) {
    files.forEach((f) => protectedFiles.add(f))
    continue
  }
  for (const f of files) {
    if (protectedFiles.has(f)) violations.push(`${sha.slice(0, 8)} "${subject}" modifies protected test file ${f}`)
  }
}

if (violations.length) {
  console.error('guard-tests: protected tests were modified after their test(...) commit\n' + violations.map((v) => ' - ' + v).join('\n'))
  process.exit(1)
}
console.log('guard-tests: ok')
