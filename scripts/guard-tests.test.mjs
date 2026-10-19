// Tests for the CI test guard (scripts/guard-tests.mjs): plain commits in, violations out.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { violations } from './guard-tests.mjs'
import { isTest } from './test-paths.mjs'

const commit = (sha, subject, files) => ({ sha: sha.padEnd(8, '0'), subject, files })

test('isTest: test paths and production paths', () => {
  for (const p of [
    'crates/fakeflac-audio/tests/decode_lossy.rs',
    'crates/fakeflac-audio/tests/common/lossy.rs',
    'src/lib/components/organisms/DetailPanel.lossy.test.ts',
    'scripts/guard-tests.test.mjs',
    'src/lib/components/atoms/Button.stories.svelte',
    'specs/features/decode-lossy.feature',
  ]) {
    assert.ok(isTest(p), p)
  }
  for (const p of ['crates/fakeflac-core/src/domain.rs', 'Cargo.lock', 'crates/fakeflac-audio/Cargo.toml', 'src/lib/testing/fixtures.ts']) {
    assert.ok(!isTest(p), p)
  }
})

test('a later commit that edits a test of a test( commit is a violation', () => {
  const found = violations([
    commit('aaa', 'test(RF-14): decoding', ['crates/fakeflac-audio/tests/decode_lossy.rs']),
    commit('bbb', 'feat(RF-14): decoding', ['crates/fakeflac-audio/tests/decode_lossy.rs', 'crates/fakeflac-audio/src/mp3.rs']),
  ])
  assert.deepEqual(found, ['bbb00000 "feat(RF-14): decoding" modifies protected test file crates/fakeflac-audio/tests/decode_lossy.rs'])
})

test('production stubs in a test( commit stay editable', () => {
  const found = violations([
    commit('aaa', 'test(RF-14): decoding', ['crates/fakeflac-audio/tests/decode_lossy.rs', 'crates/fakeflac-core/src/domain.rs', 'Cargo.lock']),
    commit('bbb', 'feat(RF-14): decoding', ['crates/fakeflac-core/src/domain.rs', 'Cargo.lock']),
  ])
  assert.deepEqual(found, [])
})

test('a later test( commit may edit earlier tests', () => {
  const found = violations([
    commit('aaa', 'test(RF-14): decoding', ['crates/fakeflac-audio/tests/common/lossy.rs']),
    commit('bbb', 'test(RF-14): move tables', ['crates/fakeflac-audio/tests/common/lossy.rs']),
  ])
  assert.deepEqual(found, [])
})

test('commits before the first test( commit are not checked', () => {
  const found = violations([
    commit('aaa', 'build(RF-14): stubs', ['crates/fakeflac-audio/tests/decode_lossy.rs']),
    commit('bbb', 'test(RF-14): decoding', ['crates/fakeflac-audio/tests/decode_lossy.rs']),
  ])
  assert.deepEqual(found, [])
})
