// Tests for the branch clean-up rules (scripts/prune-branches.mjs): plain data in, decisions out.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { plan } from './prune-branches.mjs'

const base = {
  main: 'main',
  current: 'main',
  locals: [],
  remotes: [],
  mergedIntoMain: [],
  prs: [],
  worktrees: [],
}

test('a squash-merged branch at the PR head is deleted, locally and on origin', () => {
  const p = plan({
    ...base,
    locals: [{ name: 'feat/x', sha: 'a1' }],
    remotes: [{ name: 'feat/x', sha: 'a1' }],
    prs: [{ number: 5, head: 'feat/x', oid: 'a1' }],
  })
  assert.deepEqual(p.locals, ['feat/x'])
  assert.deepEqual(p.remotes, ['feat/x'])
})

test('a branch whose tip main already contains is deleted without a PR', () => {
  const p = plan({ ...base, locals: [{ name: 'old', sha: 'b1' }], mergedIntoMain: ['main', 'old'] })
  assert.deepEqual(p.locals, ['old'])
})

test('main and the current branch are never deleted', () => {
  const p = plan({
    ...base,
    current: 'feat/x',
    locals: [{ name: 'main', sha: 'm' }, { name: 'feat/x', sha: 'a1' }],
    remotes: [{ name: 'main', sha: 'm' }],
    mergedIntoMain: ['main', 'feat/x'],
    prs: [{ number: 5, head: 'feat/x', oid: 'a1' }, { number: 6, head: 'main', oid: 'm' }],
  })
  assert.deepEqual(p.locals, [])
  assert.deepEqual(p.remotes, [])
})

test('commits pushed after the merge keep the branch and are reported', () => {
  const p = plan({
    ...base,
    locals: [{ name: 'feat/x', sha: 'later' }],
    remotes: [{ name: 'feat/x', sha: 'later' }],
    prs: [{ number: 20, head: 'feat/x', oid: 'a1' }],
  })
  assert.deepEqual(p.locals, [])
  assert.deepEqual(p.remotes, [])
  assert.equal(p.kept.length, 2)
  assert.match(p.kept[1], /origin\/feat\/x: commits pushed after PR #20/)
})

test('a branch without a merged PR is left alone silently', () => {
  const p = plan({ ...base, locals: [{ name: 'wip', sha: 'c1' }], remotes: [{ name: 'wip', sha: 'c1' }] })
  assert.deepEqual(p, { worktrees: [], locals: [], remotes: [], kept: [] })
})

test('a clean agent worktree is removed before its branch', () => {
  const p = plan({
    ...base,
    locals: [{ name: 'spec/y', sha: 'd1' }],
    prs: [{ number: 21, head: 'spec/y', oid: 'd1' }],
    worktrees: [{ path: 'C:/repo/.claude/worktrees/spec-y', branch: 'spec/y', clean: true }],
  })
  assert.deepEqual(p.worktrees, ['C:/repo/.claude/worktrees/spec-y'])
  assert.deepEqual(p.locals, ['spec/y'])
})

test('a dirty worktree or one outside .claude/worktrees keeps its branch', () => {
  const p = plan({
    ...base,
    locals: [{ name: 'a', sha: '1' }, { name: 'b', sha: '2' }],
    prs: [{ number: 1, head: 'a', oid: '1' }, { number: 2, head: 'b', oid: '2' }],
    worktrees: [
      { path: 'C:/repo/.claude/worktrees/a', branch: 'a', clean: false },
      { path: 'C:/elsewhere/b', branch: 'b', clean: true },
    ],
  })
  assert.deepEqual(p.worktrees, [])
  assert.deepEqual(p.locals, [])
  assert.equal(p.kept.length, 2)
})
