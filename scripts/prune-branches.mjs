// Deletes branches whose work is already on main, and their clean agent worktrees (ADR-0021).
// A branch goes only if main contains its tip, or a merged PR has exactly its tip as head (squash
// merges). Never main, never the current branch, never commits that no merged PR contains: a
// branch with commits pushed after its PR was merged is kept and reported, so they can be recovered.
// Usage: node scripts/prune-branches.mjs [--dry-run]
import { execFileSync } from 'node:child_process'
import { pathToFileURL } from 'node:url'

/**
 * Pure decision. Inputs are plain data so the rules can be tested without git.
 * locals: [{ name, sha }] · remotes: [{ name, sha }] (without the `origin/` prefix)
 * mergedIntoMain: names of local branches whose tip main contains
 * prs: merged PRs [{ number, head, oid }] · worktrees: [{ path, branch, clean }] (agent worktrees)
 */
export function plan({ main, current, locals, remotes, mergedIntoMain, prs, worktrees }) {
  const out = { worktrees: [], locals: [], remotes: [], kept: [] }
  const prsFor = (name) => prs.filter((p) => p.head === name)
  const mergedAt = (name, sha) => prsFor(name).find((p) => p.oid === sha)
  const isAgentWorktree = (path) => path.replace(/\\/g, '/').includes('/.claude/worktrees/')

  for (const { name, sha } of locals) {
    if (name === main || name === current) continue
    const pr = mergedAt(name, sha)
    if (!mergedIntoMain.includes(name) && !pr) {
      const late = prsFor(name)[0]
      if (late) out.kept.push(`${name}: has commits after PR #${late.number} was merged`)
      continue
    }
    const wt = worktrees.find((w) => w.branch === name)
    if (wt && !(isAgentWorktree(wt.path) && wt.clean)) {
      out.kept.push(`${name}: checked out in ${wt.path} (${wt.clean ? 'not an agent worktree' : 'uncommitted changes'})`)
      continue
    }
    if (wt) out.worktrees.push(wt.path)
    out.locals.push(name)
  }

  for (const { name, sha } of remotes) {
    if (name === main) continue
    if (mergedAt(name, sha)) {
      out.remotes.push(name)
      continue
    }
    const late = prsFor(name)[0]
    if (late) out.kept.push(`origin/${name}: commits pushed after PR #${late.number} was merged; recover them`)
  }
  return out
}

const run = (cmd, args) => execFileSync(cmd, args, { encoding: 'utf8' }).trim()
const lines = (s) => s.split('\n').map((l) => l.trim()).filter(Boolean)

function collect() {
  run('git', ['fetch', '--prune', 'origin'])
  const head = run('git', ['symbolic-ref', '--short', 'refs/remotes/origin/HEAD'])
  const main = head.replace(/^origin\//, '')
  const refs = (pattern) =>
    lines(run('git', ['for-each-ref', '--format=%(refname:short)\t%(objectname)', pattern])).map((l) => {
      const [name, sha] = l.split('\t')
      return { name, sha }
    })
  const remotes = refs('refs/remotes/origin')
    .filter((r) => r.name.startsWith('origin/') && r.name !== 'origin/HEAD')
    .map((r) => ({ name: r.name.slice('origin/'.length), sha: r.sha }))
  const worktrees = []
  let path = null
  for (const l of lines(run('git', ['worktree', 'list', '--porcelain']))) {
    if (l.startsWith('worktree ')) path = l.slice('worktree '.length)
    else if (l.startsWith('branch refs/heads/') && path) {
      const clean = run('git', ['-C', path, 'status', '--porcelain']) === ''
      worktrees.push({ path, branch: l.slice('branch refs/heads/'.length), clean })
    }
  }
  const prs = JSON.parse(
    run('gh', ['pr', 'list', '--state', 'merged', '--limit', '1000', '--json', 'number,headRefName,headRefOid']),
  ).map((p) => ({ number: p.number, head: p.headRefName, oid: p.headRefOid }))
  return {
    main,
    current: run('git', ['branch', '--show-current']),
    locals: refs('refs/heads'),
    remotes,
    mergedIntoMain: lines(run('git', ['branch', '--format=%(refname:short)', '--merged', `origin/${main}`])),
    prs,
    worktrees,
  }
}

function main() {
  const dry = process.argv.includes('--dry-run')
  const p = plan(collect())
  // One failure (e.g. a worktree folder another program holds open on Windows) must not stop the rest.
  let failed = 0
  const act = (label, fn) => {
    console.log(`${dry ? 'would delete' : 'deleting'} ${label}`)
    if (dry) return
    try {
      fn()
    } catch (e) {
      failed++
      console.error(`  failed: ${String(e.stderr || e.message).trim()}`)
    }
  }
  for (const path of p.worktrees) act(`worktree ${path}`, () => run('git', ['worktree', 'remove', path]))
  for (const name of p.locals) act(`branch ${name}`, () => run('git', ['branch', '-D', name]))
  for (const name of p.remotes)
    act(`origin/${name}`, () =>
      run('gh', ['api', '-X', 'DELETE', `repos/{owner}/{repo}/git/refs/heads/${name}`]),
    )
  if (!dry && p.remotes.length) run('git', ['fetch', '--prune', 'origin'])
  for (const k of p.kept) console.log(`kept ${k}`)
  if (!p.worktrees.length && !p.locals.length && !p.remotes.length) console.log('prune-branches: nothing to delete')
  if (failed) process.exitCode = 1
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) main()
