// Release script (RNF-11, RNF-06; specs/contracts/release.md, ADR-0045).
// Compile-only stub for the test stage: every export throws until the implement stage.
import { pathToFileURL } from 'node:url'

const notImplemented = () => {
  throw new Error('not implemented')
}

export function run(_argv, _env) {
  return notImplemented()
}

export function parseArgs(_argv) {
  return notImplemented()
}

export function versionProblem(_v) {
  return notImplemented()
}

export function tagProblem(_tag) {
  return notImplemented()
}

export function locateJson(_text, _paths) {
  return notImplemented()
}

export function locateCargoToml(_text) {
  return notImplemented()
}

export function locateCargoLock(_text) {
  return notImplemented()
}

export function replaceSpans(_text, _spans, _value) {
  return notImplemented()
}

export function sha256Sums(_files) {
  return notImplemented()
}

export function releaseNotes(_input) {
  return notImplemented()
}

export function draftArgs(_input) {
  return notImplemented()
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.stderr.write('release: unexpected error: not implemented\n')
  process.exit(1)
}
