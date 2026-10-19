// S7 (CI side): once a `test(RF-xx)` commit exists on a branch, later commits on that branch
// must not modify the test files that commit touched. Compares against the merge base with main.
// Test files are the ones the guard-protected hook protects (scripts/test-paths.mjs); production
// stubs a test commit adds so the tests compile stay editable by the implementer.
import { execFileSync } from 'node:child_process'
import { pathToFileURL } from 'node:url'
import { isTest } from './test-paths.mjs'

// Commits oldest first, as { sha, subject, files }. Returns one message per violation.
export function violations(commits) {
  const protectedFiles = new Set()
  const found = []
  for (const { sha, subject, files } of commits) {
    if (/^test\(/.test(subject)) {
      files.filter(isTest).forEach((f) => protectedFiles.add(f))
      continue
    }
    for (const f of files) {
      if (protectedFiles.has(f)) found.push(`${sha.slice(0, 8)} "${subject}" modifies protected test file ${f}`)
    }
  }
  return found
}

function main() {
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

  const commits = git('rev-list', '--reverse', `${base}..HEAD`)
    .split('\n')
    .filter(Boolean)
    .map((sha) => ({
      sha,
      subject: git('log', '-1', '--format=%s', sha),
      files: git('diff-tree', '--no-commit-id', '--name-only', '-r', sha).split('\n').filter(Boolean),
    }))

  const found = violations(commits)
  if (found.length) {
    console.error('guard-tests: protected tests were modified after their test(...) commit\n' + found.map((v) => ' - ' + v).join('\n'))
    process.exit(1)
  }
  console.log('guard-tests: ok')
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) main()
