// Tests for the release script and its wiring (RNF-11, RNF-06, RNF-09).
// Scenarios: specs/features/release.feature. Contract: specs/contracts/release.md (8.1 for how
// these tests run). Every test names its scenario and tag. Everything happens in temporary
// folders under os.tmpdir(); repository files are only read, as text, by the "Wiring" scenarios.
// Only the nine scenarios listed in release.md 8.1 spawn the CLI; every other one calls
// run(argv, env) or a pure function in-process. Every spawn and every run call gets the
// isolated environment of 8.1 (no GITHUB_*, GH_*, GIT_*; no system or global git config).
import { test, before, after } from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import * as release from './release.mjs'

const here = path.dirname(fileURLToPath(import.meta.url))
const repo = path.join(here, '..')
const SCRIPT = path.join(here, 'release.mjs')

const NSIS = 'FakeFlac_0.1.0_x64-setup.exe'
const MSI = 'FakeFlac_0.1.0_x64_en-US.msi'
const SUMS =
  `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  ${NSIS}\n` +
  `88d4266fd4e6338d13b845fcf289579d209c897823b9217da3e161936f031589  ${MSI}\n`
const REPOSITORY = 'salvador-arance/FakeFlac'
const BOM = '﻿'
const MANIFESTS = ['package.json', 'package-lock.json', 'src-tauri/tauri.conf.json', 'Cargo.toml', 'Cargo.lock']
const SOURCES = [
  ['package.json', 'pkg'],
  ['package-lock.json', 'lock'],
  ['package-lock.json packages[""]', 'lockRoot'],
  ['src-tauri/tauri.conf.json', 'tauri'],
  ['Cargo.toml [workspace.package]', 'toml'],
  ['Cargo.lock fakeflac', 'lockFakeflac'],
  ['Cargo.lock fakeflac-core', 'lockCore'],
]
const PRE = (v) => `release: version "${v}" has a pre-release or build part; v1 ships no pre-releases (MAJOR.MINOR.PATCH only)`
const SHAPE = (v) => `release: version "${v}" is not MAJOR.MINOR.PATCH (numbers without leading zeros)`
const RANGE = (v) => `release: version "${v}" is outside the MSI range (MAJOR and MINOR at most 255, PATCH at most 65535)`

// --- Temporary folders, environment, invocation ------------------------------------------------

let base // one temporary folder per test file; every case gets its own sub-folder
let gitconfig // the empty GIT_CONFIG_GLOBAL file of 8.1
let savedCwd
let caseNo = 0

const tmp = () => {
  const dir = path.join(base, `c${++caseNo}`)
  fs.mkdirSync(dir)
  return dir
}

// The isolated environment of release.md 8.1, plus the variables a scenario names.
function isolatedEnv(extra = {}, { pathDir } = {}) {
  const env = {}
  for (const [k, v] of Object.entries(process.env)) {
    if (/^(GITHUB_|GH_|GIT_)/i.test(k)) continue
    if (pathDir !== undefined && k.toUpperCase() === 'PATH') continue
    env[k] = v
  }
  env.GIT_CONFIG_NOSYSTEM = '1'
  env.GIT_CONFIG_GLOBAL = gitconfig
  if (pathDir !== undefined) env.PATH = pathDir
  return Object.assign(env, extra)
}

// In-process: run(argv, env) of release.md 3.8.
async function cli(argv, extraEnv = {}) {
  const r = await release.run(argv, isolatedEnv(extraEnv))
  assert.ok(r && Array.isArray(r.stdout) && Array.isArray(r.stderr), 'run returns { code, stdout[], stderr[] }')
  return { code: r.code, stdout: [...r.stdout], stderr: [...r.stderr] }
}

const toLines = (s) => {
  if (!s) return []
  const a = s.split('\n')
  if (a[a.length - 1] === '') a.pop()
  return a
}

// Out of process: node scripts/release.mjs, with an argument array (never a shell).
function spawnCli(argv, env = isolatedEnv()) {
  const r = spawnSync(process.execPath, [SCRIPT, ...argv], { env, encoding: 'utf8', cwd: base, windowsHide: true })
  return { code: r.status, stdout: toLines(r.stdout), stderr: toLines(r.stderr), raw: r }
}

const show = (r) => JSON.stringify({ code: r.code, stdout: r.stdout, stderr: r.stderr }, null, 1)
const fwd = (p) => p.replace(/\\/g, '/')
const noStack = (r) => {
  for (const l of [...r.stdout, ...r.stderr]) assert.ok(!l.startsWith('    at '), `stack trace line: ${l}`)
}
const cmdArgs = (s) => (s === '(none)' ? [] : s.split(' ').map((a) => (a === "''" ? '' : a)))

// --- The test root (release.feature background) ------------------------------------------------

const versions = (v, over = {}) => ({
  pkg: v,
  lock: v,
  lockRoot: v,
  tauri: v,
  toml: v,
  lockFakeflac: v,
  lockCore: v,
  ...over,
})

function packageJson(V, o) {
  const val = o.pkgRaw ?? (o.pkgEscaped ? '"\\u0030.1.0"' : `"${V.pkg}"`)
  return [
    '{',
    '  "name": "fakeflac",',
    '  "config": {',
    '    "version": "0.1.0"',
    '  },',
    '  "description": "decoy \\"version\\": \\"0.1.0\\"",',
    `  "version": ${val}${o.pkgDuplicate ? ',' : ''}`,
    ...(o.pkgDuplicate ? ['  "version": "0.1.0"'] : []),
    '}',
    '',
  ].join('\n')
}

function packageLock(V, o) {
  const rootEntry = (last) => [
    '    "": {',
    o.lockRootNoVersion ? '      "name": "fakeflac"' : '      "name": "fakeflac",',
    ...(o.lockRootNoVersion ? [] : [`      "version": "${V.lockRoot}"`]),
    last ? '    }' : '    },',
  ]
  const decoy = (last) => ['    "node_modules/decoy": {', '      "version": "0.1.0"', last ? '    }' : '    },']
  if (o.lockReordered) {
    return [
      '{',
      '  "packages": {',
      ...decoy(false),
      ...rootEntry(true),
      '  },',
      '  "lockfileVersion": 3,',
      `  "version": "${V.lock}",`,
      '  "name": "fakeflac"',
      '}',
      '',
    ].join('\n')
  }
  return [
    '{',
    '  "name": "fakeflac",',
    `  "version": "${V.lock}",`,
    o.lockNoPackages ? '  "lockfileVersion": 3' : '  "lockfileVersion": 3,',
    ...(o.lockNoPackages ? [] : ['  "packages": {', ...rootEntry(false), ...decoy(true), '  }']),
    '}',
    '',
  ].join('\n')
}

function tauriConf(V, o) {
  const name = o.productNameRaw ?? '"FakeFlac"'
  return [
    '{',
    ...(o.noProductName ? [] : [`  "productName": ${name},`]),
    '  "bundle": {',
    '    "version": "0.1.0",',
    '    "targets": ["nsis", "msi"]',
    '  },',
    `  "version": "${V.tauri}"`,
    '}',
    '',
  ].join('\n')
}

function cargoToml(V, o) {
  const lines = ['[workspace.metadata.decoy]', 'version = "0.1.0"', '', '[workspace.dependencies]', 'foo = { version = "0.1.0" }', '']
  if (!o.tomlNoPackageTable) {
    lines.push('[workspace.package]', '# version = "0.1.0"')
    if (!o.tomlNoVersion) lines.push(o.tomlRaw !== undefined ? `version = ${o.tomlRaw}` : `version = "${V.toml}"`)
    if (!o.noRepository) lines.push('repository = "https://github.com/salvador-arance/FakeFlac"')
  }
  if (o.otherTable) lines.push('', '[other]', 'version = "9.9.9"')
  lines.push('')
  return lines.join('\n')
}

function cargoLock(V, o) {
  const registry = (name, v) => [
    '[[package]]',
    `name = "${name}"`,
    `version = "${v}"`,
    'source = "registry+https://github.com/rust-lang/crates.io-index"',
    `checksum = "${'0'.repeat(64)}"`,
    '',
  ]
  const workspace = (name, v, deps, e = {}) => {
    const nameLine = e.noName ? [] : [`name = "${name}"`]
    const versionLine = e.noVersion ? [] : [`version = "${v}"`]
    return [
      '[[package]]',
      ...(e.versionFirst ? [...versionLine, ...nameLine] : [...nameLine, ...versionLine]),
      'dependencies = [',
      ...deps.map((d) => ` "${d}",`),
      ']',
      '',
    ]
  }
  const lines = [
    '# This file is automatically @generated by Cargo.',
    '# It is not intended for manual editing.',
    'version = 4',
    '',
    ...registry('decoy', '0.1.0'),
  ]
  if (!o.lockOnlyRegistry) {
    lines.push(...workspace('fakeflac', V.lockFakeflac, ['fakeflac-core']))
    lines.push(
      ...workspace('fakeflac-core', V.lockCore, ['serde'], {
        versionFirst: o.coreVersionFirst,
        noVersion: o.coreNoVersion,
        noName: o.coreNoName,
      }),
    )
    if (o.coreDuplicate) lines.push(...workspace('fakeflac-core', '0.1.0', ['serde']))
  }
  lines.push(...registry('serde', '1.0.228'))
  return lines.join('\n')
}

// The five manifest files as text. Options: per-file BOM/CRLF/no final newline, damages, variants.
function manifestTexts(V, o = {}) {
  const texts = {
    'package.json': packageJson(V, o),
    'package-lock.json': packageLock(V, o),
    'src-tauri/tauri.conf.json': tauriConf(V, o),
    'Cargo.toml': cargoToml(V, o),
    'Cargo.lock': cargoLock(V, o),
  }
  for (const rel of MANIFESTS) {
    let t = texts[rel]
    if (o.noFinalNewline?.includes(rel)) t = t.replace(/\n$/, '')
    if (o.crlf?.includes(rel)) t = t.replace(/\n/g, '\r\n')
    if (o.bom?.includes(rel)) t = BOM + t
    if (o.content && rel in o.content) t = o.content[rel]
    texts[rel] = t
  }
  return texts
}

// Writes the test root. `version` is the version of every source; `o.over` overrides single
// sources; `o.omit` lists files not written. Returns the texts written.
function writeRoot(dir, version, o = {}) {
  const texts = manifestTexts(versions(version, o.over), o)
  fs.mkdirSync(path.join(dir, 'src-tauri'), { recursive: true })
  for (const rel of MANIFESTS) {
    if (o.omit?.includes(rel)) continue
    fs.writeFileSync(path.join(dir, rel), texts[rel], 'utf8')
  }
  return texts
}

const readText = (dir, rel) => fs.readFileSync(path.join(dir, rel), 'utf8')
function readManifests(dir) {
  const out = {}
  for (const rel of MANIFESTS) {
    const p = path.join(dir, rel)
    out[rel] = fs.existsSync(p) ? fs.readFileSync(p) : null
  }
  return out
}
function assertManifestsUnchanged(dir, before) {
  const now = readManifests(dir)
  for (const rel of MANIFESTS) {
    assert.deepEqual(now[rel], before[rel], `${rel} changed`)
  }
}

// A recursive snapshot of a path: null (absent), the bytes of a file, or a sorted folder map.
function snap(p) {
  let st
  try {
    st = fs.lstatSync(p)
  } catch {
    return null
  }
  if (st.isFile()) return { file: fs.readFileSync(p).toString('base64') }
  if (st.isDirectory()) {
    const entries = {}
    for (const e of fs.readdirSync(p).sort()) entries[e] = snap(path.join(p, e))
    return { dir: entries }
  }
  return { other: true }
}

const sorted = (a) => [...a].sort()
const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex')

// --- The bundle and the assets ----------------------------------------------------------------

// files: { 'nsis/<name>': content (string, Buffer or byte count) }, dirs: ['nsis/<name>' …]
function makeBundle(dir, { files = {}, dirs = [], mkSubfolders = true } = {}) {
  if (mkSubfolders) {
    fs.mkdirSync(path.join(dir, 'nsis'), { recursive: true })
    fs.mkdirSync(path.join(dir, 'msi'), { recursive: true })
  }
  for (const d of dirs) fs.mkdirSync(path.join(dir, d), { recursive: true })
  for (const [rel, content] of Object.entries(files)) {
    fs.mkdirSync(path.dirname(path.join(dir, rel)), { recursive: true })
    fs.writeFileSync(path.join(dir, rel), typeof content === 'number' ? Buffer.alloc(content, 0x41) : content)
  }
}
const releaseBundle = { files: { [`nsis/${NSIS}`]: 'abc', [`msi/${MSI}`]: 'abcd' } }

// One case folder: <t>/root, <t>/bundle, <t>/out, <t>/notes.md.
function assetsCase({ version = '0.1.0', root = {}, bundle = releaseBundle, at } = {}) {
  const t = at ?? tmp()
  const c = {
    t,
    root: path.join(t, 'root'),
    bundle: path.join(t, 'bundle'),
    out: path.join(t, 'out'),
    notes: path.join(t, 'notes.md'),
  }
  fs.mkdirSync(c.root, { recursive: true })
  writeRoot(c.root, version, root)
  if (bundle) makeBundle(c.bundle, bundle)
  return c
}
const assetsArgs = (c, { maxBytes = 1000, notes = c.notes } = {}) => [
  'assets',
  '--root',
  c.root,
  '--bundle',
  c.bundle,
  '--out',
  c.out,
  '--notes',
  notes,
  '--max-bytes',
  String(maxBytes),
]

// "The assets of a release have been written".
async function writtenAssets() {
  const c = assetsCase()
  const r = await cli(assetsArgs(c))
  assert.equal(r.code, 0, show(r))
  return c
}

// The text of release.md 3.5.
function expectedNotes({ productName: p, version: v, nsis, msi, sums, repository }) {
  const parts = [
    `${p} ${v} for Windows 10 and 11 (64-bit).`,
    '',
    '## Files',
    '',
    `- \`${nsis}\`: the installer (recommended). It installs ${p} for the current user.`,
    `- \`${msi}\`: the same app as a Windows Installer package, for managed installs.`,
    '- `SHA256SUMS.txt`: the SHA-256 checksum of each installer.',
    '',
    '## Windows SmartScreen',
    '',
    `The installers are not code-signed. ${p} is free, open-source software and does not pay for a signing certificate. The first time you run a new download, Windows SmartScreen may show "Windows protected your PC". Check the download first (below), then click **More info** and **Run anyway**.`,
    '',
    '## Check the download',
    '',
    'In PowerShell, in the folder that holds the download:',
    '',
    '```powershell',
    `(Get-FileHash .\\${nsis} -Algorithm SHA256).Hash.ToLower()`,
    '```',
    '',
    "The result must equal the hash on that file's line of `SHA256SUMS.txt`:",
    '',
    '```text',
    `${sums}\`\`\``,
    '',
  ]
  if (repository !== null) {
    parts.push(
      `To check that GitHub Actions built the installers from this repository: \`gh attestation verify ${nsis} --repo ${repository}\` and \`gh attestation verify ${msi} --repo ${repository}\`.`,
      '',
    )
  }
  parts.push(
    '## Requirements',
    '',
    `Windows 10 or 11, 64-bit. If Microsoft Edge WebView2 is missing, the installer downloads it, which needs an internet connection once. ${p} itself makes no network connection.`,
    '',
  )
  return parts.join('\n')
}

// --- The git fixture (release.md 8.1) ---------------------------------------------------------

let gitFixture
const sha = {}
const GIT_ID = ['-c', 'user.name=FakeFlac Test', '-c', 'user.email=test@example.invalid', '-c', 'commit.gpgsign=false']

function git(cwd, args, input) {
  const r = spawnSync('git', args, { cwd, env: isolatedEnv(), encoding: 'utf8', input, windowsHide: true })
  assert.equal(r.status, 0, `git ${args.join(' ')}: ${r.stderr}`)
  return r.stdout.trim()
}

// A copy of the fixture with HEAD (detached) and refs/remotes/origin/main set as the scenario says.
function gitCase(head, main) {
  const dir = path.join(tmp(), 'repo')
  fs.cpSync(gitFixture, dir, { recursive: true })
  const cmds = ['option no-deref', `update HEAD ${sha[head]}`]
  if (main) cmds.push(`update refs/remotes/origin/main ${sha[main]}`)
  git(dir, ['update-ref', '--stdin'], cmds.join('\n') + '\n')
  return dir
}

before(() => {
  base = fs.mkdtempSync(path.join(os.tmpdir(), 'ff-release-'))
  gitconfig = path.join(base, 'empty-gitconfig')
  fs.writeFileSync(gitconfig, '')
  // Relative paths of the usage cases resolve against the cwd: keep it out of the repository.
  savedCwd = process.cwd()
  process.chdir(base)

  gitFixture = path.join(base, 'git-fixture')
  fs.mkdirSync(gitFixture)
  writeRoot(gitFixture, '0.1.0')
  git(gitFixture, ['init', '-q', '--template=', '-b', 'work'])
  git(gitFixture, ['add', '-A'])
  git(gitFixture, [...GIT_ID, 'commit', '-q', '-m', 'A'])
  git(gitFixture, [...GIT_ID, 'commit', '-q', '--allow-empty', '-m', 'B'])
  sha.B = git(gitFixture, ['rev-parse', 'HEAD'])
  sha.A = git(gitFixture, ['rev-parse', 'HEAD~1'])
})

after(() => {
  if (savedCwd) process.chdir(savedCwd)
  if (base) fs.rmSync(base, { recursive: true, force: true })
})

// =============================================================================================
// Versions (release.md 2 and 3.1)
// =============================================================================================

test('@RNF-11 Scenario: Manifests that agree on a releasable version pass (CLI)', () => {
  const root = tmp()
  writeRoot(root, '0.1.0')
  const r = spawnCli(['version', '--root', root])
  assert.equal(r.code, 0, show(r))
  assert.deepEqual(r.stdout, ['release: version 0.1.0 (7 sources agree)'])
  assert.deepEqual(r.stderr, [])
  assert.ok(r.raw.stdout.endsWith('\n'), 'lines end with LF')
})

for (const [source, key] of SOURCES) {
  test(`@RNF-11 Scenario Outline: A manifest that disagrees is named [${source}]`, async () => {
    const root = tmp()
    writeRoot(root, '0.1.0', { over: { [key]: '0.2.0' } })
    const r = await cli(['version', '--root', root])
    assert.equal(r.code, 1, show(r))
    assert.deepEqual(r.stderr, [
      'release: the manifests do not agree on the version',
      ...SOURCES.map(([name, k]) => `release:   ${name}: ${k === key ? '0.2.0' : '0.1.0'}`),
    ])
  })
}

test('@RNF-11 Scenario: The decoys are never read as the app\'s version', async () => {
  const root = tmp()
  writeRoot(root, '0.2.0', { otherTable: true })
  const r = await cli(['version', '--root', root])
  assert.equal(r.code, 0, show(r))
  assert.deepEqual(r.stdout, ['release: version 0.2.0 (7 sources agree)'])
  assert.deepEqual(r.stderr, [])
})

test('@RNF-11 Scenario: The version is found by key path, whatever the key order', async () => {
  const root = tmp()
  writeRoot(root, '0.2.0', { lockReordered: true })
  let r = await cli(['version', '--root', root])
  assert.equal(r.code, 0, show(r))
  assert.deepEqual(r.stdout, ['release: version 0.2.0 (7 sources agree)'])

  r = await cli(['set-version', '0.3.0', '--root', root])
  assert.equal(r.code, 0, show(r))
  const lock = readText(root, 'package-lock.json')
  assert.equal(lock, manifestTexts(versions('0.3.0'), { lockReordered: true })['package-lock.json'])
  assert.equal(JSON.parse(lock).packages['node_modules/decoy'].version, '0.1.0')
})

test('@RNF-11 Scenario: A Cargo.lock entry whose name line comes after its version line', async () => {
  const root = tmp()
  writeRoot(root, '0.1.0', { over: { lockCore: '0.2.0' }, coreVersionFirst: true })
  let r = await cli(['version', '--root', root])
  assert.equal(r.code, 1, show(r))
  assert.ok(r.stderr.includes('release:   Cargo.lock fakeflac: 0.1.0'), show(r))
  assert.ok(r.stderr.includes('release:   Cargo.lock fakeflac-core: 0.2.0'), show(r))

  r = await cli(['set-version', '0.2.0', '--root', root])
  assert.equal(r.code, 0, show(r))
  assert.deepEqual(r.stdout, [
    'release: set version 0.2.0 in package.json, package-lock.json, src-tauri/tauri.conf.json, Cargo.toml, Cargo.lock',
  ])
  const lock = readText(root, 'Cargo.lock')
  assert.equal(lock, manifestTexts(versions('0.2.0'), { coreVersionFirst: true })['Cargo.lock'])
  assert.ok(lock.includes('[[package]]\nversion = "0.2.0"\nname = "fakeflac-core"\n'), 'version line still before name line')

  r = await cli(['version', '--root', root])
  assert.deepEqual(r.stdout, ['release: version 0.2.0 (7 sources agree)'], show(r))
})

test('@RNF-11 Scenario: A JSON version written with escapes is compared decoded and replaced whole', async () => {
  const root = tmp()
  const before = writeRoot(root, '0.1.0', { pkgEscaped: true })
  assert.ok(before['package.json'].includes('"version": "\\u0030.1.0"'))

  let r = await cli(['version', '--root', root])
  assert.equal(r.code, 0, show(r))
  assert.deepEqual(r.stdout, ['release: version 0.1.0 (7 sources agree)'])

  const bytesBefore = fs.readFileSync(path.join(root, 'package.json'))
  r = await cli(['set-version', '0.1.0', '--root', root])
  assert.equal(r.code, 0, show(r))
  assert.deepEqual(r.stdout, ['release: nothing to change, already 0.1.0'])
  assert.deepEqual(fs.readFileSync(path.join(root, 'package.json')), bytesBefore)

  r = await cli(['set-version', '0.2.0', '--root', root])
  assert.equal(r.code, 0, show(r))
  const expected = before['package.json'].replace('\\u0030.1.0', '0.2.0')
  assert.equal(readText(root, 'package.json'), expected)
  assert.equal(expected, manifestTexts(versions('0.2.0'))['package.json'])

  // replaceSpans (release.md 3.7 step 3): the whole raw span, escapes included, is replaced.
  const text = '{"version": "\\u0030.1.0", "b": "1.2.3"}'
  const start = text.indexOf('\\u0030')
  assert.equal(release.replaceSpans(text, [{ start, end: start + 10 }], '0.2.0'), '{"version": "0.2.0", "b": "1.2.3"}')
  const s2 = text.indexOf('1.2.3')
  assert.equal(
    release.replaceSpans(text, [{ start, end: start + 10 }, { start: s2, end: s2 + 5 }], '0.2.0'),
    '{"version": "0.2.0", "b": "0.2.0"}',
  )
})

test('@RNF-11 Scenario: CRLF line endings and a UTF-8 BOM are read', async () => {
  const root = tmp()
  const texts = writeRoot(root, '0.1.0', { crlf: ['Cargo.toml', 'Cargo.lock'], bom: ['package.json'] })
  assert.ok(texts['Cargo.toml'].includes('\r\n') && texts['package.json'].startsWith(BOM))
  const r = await cli(['version', '--root', root])
  assert.equal(r.code, 0, show(r))
  assert.deepEqual(r.stdout, ['release: version 0.1.0 (7 sources agree)'])
})

const RELEASABLE = [
  ['0.0.1', 0, 'stdout', 'release: version 0.0.1 (7 sources agree)'],
  ['1.0.0', 0, 'stdout', 'release: version 1.0.0 (7 sources agree)'],
  ['255.255.65535', 0, 'stdout', 'release: version 255.255.65535 (7 sources agree)'],
  ['1.0.0-beta.1', 1, 'stderr', PRE('1.0.0-beta.1')],
  ['1.0.0+5', 1, 'stderr', PRE('1.0.0+5')],
  ['01.0.0', 1, 'stderr', SHAPE('01.0.0')],
  ['1.0', 1, 'stderr', SHAPE('1.0')],
  ['1.0.0.0', 1, 'stderr', SHAPE('1.0.0.0')],
  ['256.0.0', 1, 'stderr', RANGE('256.0.0')],
  ['0.256.0', 1, 'stderr', RANGE('0.256.0')],
  ['0.0.65536', 1, 'stderr', RANGE('0.0.65536')],
]
for (const [version, code, stream, message] of RELEASABLE) {
  test(`@RNF-11 Scenario Outline: Only MAJOR.MINOR.PATCH within the MSI range is a release version [${version}]`, async () => {
    assert.equal(release.versionProblem(version), code === 0 ? null : message)
    const root = tmp()
    writeRoot(root, version)
    const r = await cli(['version', '--root', root])
    assert.equal(r.code, code, show(r))
    assert.deepEqual(r[stream], [message])
  })
}

const READ_ERRORS = [
  ['package.json holds "{not json"', { content: { 'package.json': '{not json' } }, 'release: package.json is not valid JSON'],
  ['src-tauri/tauri.conf.json does not exist', { omit: ['src-tauri/tauri.conf.json'] }, 'release: missing src-tauri/tauri.conf.json'],
  ['package-lock.json does not exist', { omit: ['package-lock.json'] }, 'release: missing package-lock.json'],
  ['package-lock.json has no packages[""].version', { lockRootNoVersion: true }, 'release: package-lock.json packages[""] has no version'],
  ['package-lock.json has no "packages" key', { lockNoPackages: true }, 'release: package-lock.json packages[""] has no version'],
  ['package.json has "version": 1', { pkgRaw: '1' }, 'release: package.json version is not a string'],
  ['src-tauri/tauri.conf.json has "version": ""', { over: { tauri: '' } }, 'release: src-tauri/tauri.conf.json version is empty'],
  ['package.json has a second top-level "version" key holding "0.1.0"', { pkgDuplicate: true }, 'release: package.json has a duplicate "version" key'],
  ['Cargo.toml has no version line in [workspace.package]', { tomlNoVersion: true }, 'release: Cargo.toml [workspace.package] has no version'],
  ['Cargo.toml has no [workspace.package] table', { tomlNoPackageTable: true }, 'release: Cargo.toml [workspace.package] has no version'],
  ['the version line of Cargo.toml [workspace.package] is version = 1', { tomlRaw: '1' }, 'release: Cargo.toml [workspace.package] version is not a string'],
  ['Cargo.lock holds only the registry entries', { lockOnlyRegistry: true }, 'release: Cargo.lock lists no workspace package'],
  ['the Cargo.lock entry "fakeflac-core" has no version line', { coreNoVersion: true }, 'release: Cargo.lock fakeflac-core has no version'],
  ['the Cargo.lock entry "fakeflac-core" has no name line', { coreNoName: true }, 'release: Cargo.lock has a workspace package without a name'],
  [
    'Cargo.lock has a second source-less entry "fakeflac-core"',
    { coreDuplicate: true },
    'release: Cargo.lock lists the workspace package fakeflac-core more than once',
  ],
  ['Cargo.lock does not exist', { omit: ['Cargo.lock'] }, 'release: missing Cargo.lock'],
]
for (const [damage, o, message] of READ_ERRORS) {
  test(`@RNF-11 Scenario Outline: A broken or missing manifest is a clear error, never a crash [${damage}]`, async () => {
    const root = tmp()
    writeRoot(root, '0.1.0', o)
    const r = await cli(['version', '--root', root])
    assert.equal(r.code, 1, show(r))
    assert.deepEqual(r.stderr, [message])
    noStack(r)
  })
}

test('@RNF-11 Scenario: Every problem is reported, not only the first (CLI)', () => {
  const root = tmp()
  writeRoot(root, '0.1.0', { content: { 'package.json': '{not json' }, omit: ['Cargo.lock'] })
  const r = spawnCli(['version', '--root', root])
  assert.equal(r.code, 1, show(r))
  assert.deepEqual(r.stderr, ['release: package.json is not valid JSON', 'release: missing Cargo.lock'])
  noStack(r)
})

// =============================================================================================
// Release tag (release.md 3.2)
// =============================================================================================

const short = (k) => sha[k].slice(0, 12)

test('@RNF-11 Scenario: A tag of the manifests\' version on main passes (CLI)', () => {
  const dir = gitCase('B', 'B')
  const r = spawnCli(['release-check', 'v0.1.0', '--root', dir])
  assert.equal(r.code, 0, show(r))
  assert.deepEqual(r.stdout, [`release: tag v0.1.0 releases version 0.1.0 from commit ${short('B')} on origin/main`])
})

const TAG_ERRORS = [
  ['0.1.0', 'release: tag "0.1.0" is not v<MAJOR>.<MINOR>.<PATCH>'],
  ['V0.1.0', 'release: tag "V0.1.0" is not v<MAJOR>.<MINOR>.<PATCH>'],
  ['v0.1', 'release: tag "v0.1" is not v<MAJOR>.<MINOR>.<PATCH>'],
  ['v00.1.0', 'release: tag "v00.1.0" is not v<MAJOR>.<MINOR>.<PATCH>'],
  ['v256.0.0', 'release: tag "v256.0.0" is not v<MAJOR>.<MINOR>.<PATCH>'],
  ['v0.1.0-beta.1', 'release: tag "v0.1.0-beta.1" has a pre-release or build part; v1 ships no pre-releases (vMAJOR.MINOR.PATCH only)'],
  ['v0.1.0+5', 'release: tag "v0.1.0+5" has a pre-release or build part; v1 ships no pre-releases (vMAJOR.MINOR.PATCH only)'],
  ['v0.2.0', 'release: tag "v0.2.0" does not match the manifests\' version 0.1.0'],
]
for (const [tag, message] of TAG_ERRORS) {
  test(`@RNF-11 Scenario Outline: The release tag must name the manifests' version [${tag}]`, async () => {
    // tagProblem covers release.md 3.2 step 2 only: a well-formed tag of another version is null.
    assert.equal(release.tagProblem(tag), tag === 'v0.2.0' ? null : message)
    const dir = gitCase('B', 'B')
    const r = await cli(['release-check', tag, '--root', dir])
    assert.equal(r.code, 1, show(r))
    assert.deepEqual(r.stderr, [message])
  })
}

const MAIN_CASES = [
  ['HEAD at A and origin/main pointing at B', 'A', 'B', 0, 'stdout', () => `release: tag v0.1.0 releases version 0.1.0 from commit ${short('A')} on origin/main`],
  ['HEAD at B and origin/main pointing at A', 'B', 'A', 1, 'stderr', () => `release: commit ${short('B')} is not on origin/main; tag a commit of main`],
  [
    'HEAD at B and no origin/main',
    'B',
    null,
    1,
    'stderr',
    () => 'release: origin/main not found (fetch it, for example actions/checkout with fetch-depth: 0)',
  ],
]
for (const [refs, head, main, code, stream, message] of MAIN_CASES) {
  test(`@RNF-11 Scenario Outline: Only a commit of main can be released [${refs}]`, async () => {
    const dir = gitCase(head, main)
    const r = await cli(['release-check', 'v0.1.0', '--root', dir])
    assert.equal(r.code, code, show(r))
    assert.deepEqual(r[stream], [message()])
  })
}

test('@RNF-11 Scenario: A folder that is not a git repository is a git error', async () => {
  const t = tmp()
  const root = path.join(t, 'root')
  fs.mkdirSync(root)
  writeRoot(root, '0.1.0')
  const r = await cli(['release-check', 'v0.1.0', '--root', root], { GIT_CEILING_DIRECTORIES: t })
  assert.equal(r.code, 1, show(r))
  assert.equal(r.stderr.length, 1, show(r))
  assert.ok(r.stderr[0].startsWith('release: git rev-parse failed (exit 128): '), show(r))
})

test('@RNF-11 Scenario: In the workflow the tag comes from the environment (first step: CLI)', async () => {
  const dir = gitCase('B', 'B')
  const spawned = spawnCli(['release-check', '--root', dir], isolatedEnv({ GITHUB_REF_TYPE: 'tag', GITHUB_REF_NAME: 'v0.1.0' }))
  assert.equal(spawned.code, 0, show(spawned))

  let r = await cli(['release-check', '--root', dir], { GITHUB_REF_TYPE: 'tag', GITHUB_REF_NAME: 'v0.1.0-rc.1' })
  assert.equal(r.code, 1, show(r))
  assert.deepEqual(r.stderr, [
    'release: tag "v0.1.0-rc.1" has a pre-release or build part; v1 ships no pre-releases (vMAJOR.MINOR.PATCH only)',
  ])

  r = await cli(['release-check', '--root', dir], { GITHUB_REF_TYPE: 'branch', GITHUB_REF_NAME: 'main' })
  assert.equal(r.code, 2, show(r))
  assert.deepEqual(r.stderr, ['release: release-check needs a tag such as v1.2.3'])
})

test('@RNF-11 Scenario: A tag argument wins over the environment', async () => {
  const dir = gitCase('B', 'B')
  const r = await cli(['release-check', 'v0.1.0', '--root', dir], { GITHUB_REF_TYPE: 'tag', GITHUB_REF_NAME: 'v0.2.0' })
  assert.equal(r.code, 0, show(r))
  assert.deepEqual(r.stdout, [`release: tag v0.1.0 releases version 0.1.0 from commit ${short('B')} on origin/main`])
})

// =============================================================================================
// Installers, checksums and notes (release.md 3.3 to 3.5)
// =============================================================================================

test('@RNF-11 Scenario: The assets of a release (CLI)', () => {
  const c = assetsCase()
  const bundleBefore = snap(c.bundle)
  const r = spawnCli(assetsArgs(c))
  assert.equal(r.code, 0, show(r))
  assert.deepEqual(sorted(fs.readdirSync(c.out)), sorted([NSIS, MSI, 'SHA256SUMS.txt']))
  assert.equal(fs.readFileSync(path.join(c.out, NSIS), 'utf8'), 'abc')
  assert.equal(fs.readFileSync(path.join(c.out, MSI), 'utf8'), 'abcd')
  assert.deepEqual(snap(c.bundle), bundleBefore)
  assert.equal(fs.readFileSync(path.join(c.out, 'SHA256SUMS.txt'), 'utf8'), SUMS)
  const t = fwd(c.t)
  assert.deepEqual(r.stdout, [
    `release: ${NSIS} 3 bytes (0.00 MB), limit 1000 bytes`,
    `release: ${MSI} 4 bytes (0.00 MB), limit 1000 bytes`,
    `release: wrote ${t}/out (2 installers, SHA256SUMS.txt) and ${t}/notes.md`,
  ])
  assert.deepEqual(r.stderr, [])
})

test('@RNF-11 Scenario: The checksums file has the sha256sum text format', async () => {
  // sha256Sums (release.md 3.4): sorted by name, lowercase hex, two spaces, LF after every line.
  assert.equal(
    release.sha256Sums([
      { name: MSI, bytes: new Uint8Array(Buffer.from('abcd')) },
      { name: NSIS, bytes: new Uint8Array(Buffer.from('abc')) },
    ]),
    SUMS,
  )
  const c = await writtenAssets()
  const bytes = fs.readFileSync(path.join(c.out, 'SHA256SUMS.txt'))
  assert.notDeepEqual([...bytes.subarray(0, 3)], [0xef, 0xbb, 0xbf], 'no BOM')
  assert.ok(!bytes.includes(0x0d), 'no CR byte')
  const text = bytes.toString('utf8')
  assert.ok(text.endsWith('\n') && !text.endsWith('\n\n'), 'ends with exactly one LF')
  const lines = text.slice(0, -1).split('\n')
  for (const l of lines) assert.match(l, /^[0-9a-f]{64} {2}[^/\\]+$/)
  const names = lines.map((l) => l.slice(66))
  assert.deepEqual(names, [...names].sort())
  for (const f of fs.readdirSync(c.out).filter((f) => f !== 'SHA256SUMS.txt')) {
    assert.ok(lines.includes(`${sha256(fs.readFileSync(path.join(c.out, f)))}  ${f}`), `line of ${f}`)
  }
})

test('@RNF-11 Scenario: Running twice gives identical files', async () => {
  const c = assetsCase()
  const first = await cli(assetsArgs(c))
  assert.equal(first.code, 0, show(first))
  const sums1 = fs.readFileSync(path.join(c.out, 'SHA256SUMS.txt'))
  const notes1 = fs.readFileSync(c.notes)
  const second = await cli(assetsArgs(c))
  assert.equal(second.code, 0, show(second))
  assert.deepEqual(fs.readFileSync(path.join(c.out, 'SHA256SUMS.txt')), sums1)
  assert.deepEqual(fs.readFileSync(c.notes), notes1)
})

test('@RNF-11 Scenario: The release notes', async () => {
  const input = { productName: 'FakeFlac', version: '0.1.0', nsis: NSIS, msi: MSI, sums: SUMS, repository: REPOSITORY }
  const expected = expectedNotes(input)
  assert.equal(release.releaseNotes(input), expected)

  const c = await writtenAssets()
  const bytes = fs.readFileSync(c.notes)
  const notes = bytes.toString('utf8')
  assert.equal(notes, expected)
  assert.notDeepEqual([...bytes.subarray(0, 3)], [0xef, 0xbb, 0xbf], 'no BOM')
  assert.ok(!notes.includes('\r') && notes.endsWith('\n') && !notes.endsWith('\n\n'))
  const lines = notes.split('\n')
  assert.equal(lines[0], 'FakeFlac 0.1.0 for Windows 10 and 11 (64-bit).')
  for (const l of toLines(SUMS)) assert.ok(lines.includes(l), `sums line ${l}`)
  for (const s of ['The installers are not code-signed', '**More info**', '**Run anyway**', `Get-FileHash .\\${NSIS} -Algorithm SHA256`]) {
    assert.ok(notes.includes(s), s)
  }
  assert.ok(
    lines.includes(
      `To check that GitHub Actions built the installers from this repository: \`gh attestation verify ${NSIS} --repo ${REPOSITORY}\` and \`gh attestation verify ${MSI} --repo ${REPOSITORY}\`.`,
    ),
  )
  assert.ok(notes.includes('If Microsoft Edge WebView2 is missing, the installer downloads it'))
})

test('@RNF-11 Scenario: The notes take the product name from tauri.conf.json', async () => {
  const c = assetsCase({
    root: { productNameRaw: '"TestApp"' },
    bundle: { files: { 'nsis/TestApp_0.1.0_x64-setup.exe': 'abc', 'msi/TestApp_0.1.0_x64_en-US.msi': 'abcd' } },
  })
  const r = await cli(assetsArgs(c))
  assert.equal(r.code, 0, show(r))
  const notes = fs.readFileSync(c.notes, 'utf8')
  assert.equal(notes.split('\n')[0], 'TestApp 0.1.0 for Windows 10 and 11 (64-bit).')
  for (const s of ['It installs TestApp for the current user.', 'TestApp is free, open-source software', 'TestApp itself makes no network connection.']) {
    assert.ok(notes.includes(s), s)
  }
  const withFakeFlac = notes.split('\n').filter((l) => l.includes('FakeFlac'))
  assert.equal(withFakeFlac.length, 1, withFakeFlac.join('\n'))
  assert.ok(withFakeFlac[0].includes('gh attestation verify'))
})

test('@RNF-11 Scenario: No provenance line without a GitHub repository URL', async () => {
  const input = { productName: 'FakeFlac', version: '0.1.0', nsis: NSIS, msi: MSI, sums: SUMS, repository: null }
  assert.equal(release.releaseNotes(input), expectedNotes(input))

  const c = assetsCase({ root: { noRepository: true } })
  const r = await cli(assetsArgs(c))
  assert.equal(r.code, 0, show(r))
  const notes = fs.readFileSync(c.notes, 'utf8')
  assert.ok(!notes.includes('gh attestation verify'))
  assert.ok(!notes.includes('To check that GitHub Actions'))
})

const BAD_PRODUCT_NAMES = [
  ['a tauri.conf.json without "productName"', { noProductName: true }, 'release: src-tauri/tauri.conf.json has no productName'],
  ['productName ""', { productNameRaw: '""' }, 'release: src-tauri/tauri.conf.json productName is empty'],
  ['productName 1 (a number)', { productNameRaw: '1' }, 'release: src-tauri/tauri.conf.json productName is not a string'],
  ...[
    ['Fake/Flac', 'productName "Fake/Flac"'],
    ['Fake\\Flac', 'productName "Fake\\\\Flac" (one backslash)'],
    ['Fake:Flac', 'productName "Fake:Flac"'],
    ['Fake*Flac', 'productName "Fake*Flac"'],
    ['Fake?Flac', 'productName "Fake?Flac"'],
    ['Fake|Flac', 'productName "Fake|Flac"'],
    ['Fake\tFlac', 'productName holding a tab (U+0009) between Fake and Flac'],
  ].map(([name, label]) => [
    label,
    { productNameRaw: JSON.stringify(name) },
    `release: src-tauri/tauri.conf.json productName ${JSON.stringify(name)} has a character not allowed in a file name`,
  ]),
]
// The literal messages of the feature table, so the computed ones above cannot drift from it.
assert.equal(BAD_PRODUCT_NAMES[4][2], 'release: src-tauri/tauri.conf.json productName "Fake\\\\Flac" has a character not allowed in a file name')
assert.equal(BAD_PRODUCT_NAMES[9][2], 'release: src-tauri/tauri.conf.json productName "Fake\\tFlac" has a character not allowed in a file name')
for (const [label, o, message] of BAD_PRODUCT_NAMES) {
  test(`@RNF-11 Scenario Outline: A product name that cannot be part of a file name is a clear error [${label}]`, async () => {
    const c = assetsCase({ root: o })
    const r = await cli(assetsArgs(c))
    assert.equal(r.code, 1, show(r))
    assert.deepEqual(r.stderr, [message])
    assert.ok(!fs.existsSync(c.out) && !fs.existsSync(c.notes))
  })
}

test('@RNF-11 Scenario: Installers of other versions are ignored', async () => {
  const c = assetsCase({
    bundle: {
      files: {
        ...releaseBundle.files,
        'msi/FakeFlac_0.0.1_x64_en-US.msi': 'old',
        'nsis/FakeFlac_0.0.1_x64-setup.exe': 'old',
        'msi/0-old.msi': 'old',
      },
    },
  })
  const r = await cli(assetsArgs(c))
  assert.equal(r.code, 0, show(r))
  assert.deepEqual(sorted(fs.readdirSync(c.out)), sorted([NSIS, MSI, 'SHA256SUMS.txt']))
  assert.equal(toLines(fs.readFileSync(path.join(c.out, 'SHA256SUMS.txt'), 'utf8')).length, 2)
  const b = fwd(c.bundle)
  const t = fwd(c.t)
  assert.deepEqual(r.stdout, [
    `release: ignored ${b}/msi/0-old.msi (not an installer of 0.1.0)`,
    `release: ignored ${b}/nsis/FakeFlac_0.0.1_x64-setup.exe (not an installer of 0.1.0)`,
    `release: ignored ${b}/msi/FakeFlac_0.0.1_x64_en-US.msi (not an installer of 0.1.0)`,
    `release: ${NSIS} 3 bytes (0.00 MB), limit 1000 bytes`,
    `release: ${MSI} 4 bytes (0.00 MB), limit 1000 bytes`,
    `release: wrote ${t}/out (2 installers, SHA256SUMS.txt) and ${t}/notes.md`,
  ])
})

const MISSING = [
  ['the bundle folder does not exist', null, ['nsis', 'msi']],
  ['the bundle has empty nsis and msi folders', {}, ['nsis', 'msi']],
  [
    'the bundle holds only the installers of 0.0.1',
    { files: { 'nsis/FakeFlac_0.0.1_x64-setup.exe': 'old', 'msi/FakeFlac_0.0.1_x64_en-US.msi': 'old' } },
    ['nsis', 'msi'],
  ],
  ['the bundle holds a folder named "nsis/<nsis>" and a file "msi/<msi>"', { dirs: [`nsis/${NSIS}`], files: { [`msi/${MSI}`]: 'abcd' } }, ['nsis']],
  ['the bundle holds only "nsis/<nsis>" of 10 bytes', { files: { [`nsis/${NSIS}`]: 10 } }, ['msi']],
  ['the bundle holds only "msi/<msi>" of 10 bytes', { files: { [`msi/${MSI}`]: 10 } }, ['nsis']],
]
for (const [state, bundle, missing] of MISSING) {
  test(`@RNF-11 Scenario Outline: Missing installers (the empty state) [${state}]`, async () => {
    const c = assetsCase({ bundle })
    assert.ok(!fs.existsSync(c.out))
    const r = await cli(assetsArgs(c))
    assert.equal(r.code, 1, show(r))
    const b = fwd(c.bundle)
    const line = { nsis: `release: missing installer ${b}/nsis/${NSIS}`, msi: `release: missing installer ${b}/msi/${MSI}` }
    assert.deepEqual(r.stderr, missing.map((k) => line[k]))
    assert.ok(!fs.existsSync(c.out) && !fs.existsSync(c.notes))
  })
}

test('@RNF-11 Scenario: Every failure of steps 1 to 4 is reported, in order', async () => {
  const c = assetsCase({ bundle: { files: { [`nsis/${NSIS}`]: 1001 } } })
  fs.mkdirSync(c.out)
  fs.writeFileSync(path.join(c.out, 'notes.txt'), 'keep me')
  const outBefore = snap(c.out)
  const r = await cli(assetsArgs(c, { maxBytes: 1000 }))
  assert.equal(r.code, 1, show(r))
  assert.ok(r.stdout.includes(`release: ${NSIS} 1001 bytes (0.00 MB), limit 1000 bytes`), show(r))
  const b = fwd(c.bundle)
  const t = fwd(c.t)
  assert.deepEqual(r.stderr, [
    `release: missing installer ${b}/msi/${MSI}`,
    `release: ${NSIS} is 1001 bytes, not under the limit of 1000 bytes (RNF-06)`,
    `release: ${t}/out holds notes.txt, which assets did not write; remove it by hand`,
  ])
  assert.deepEqual(snap(c.out), outBefore)
  assert.ok(!fs.existsSync(c.notes))
})

test('@RNF-11 Scenario: An earlier output folder is replaced', async () => {
  const c = assetsCase()
  fs.mkdirSync(c.out)
  fs.writeFileSync(path.join(c.out, 'FakeFlac_0.0.1_x64-setup.exe'), 'old')
  fs.writeFileSync(path.join(c.out, 'FakeFlac_0.0.1_x64_en-US.msi'), 'old')
  fs.writeFileSync(path.join(c.out, 'SHA256SUMS.txt'), 'older sums\n')
  const r = await cli(assetsArgs(c))
  assert.equal(r.code, 0, show(r))
  assert.deepEqual(sorted(fs.readdirSync(c.out)), sorted([NSIS, MSI, 'SHA256SUMS.txt']))
  assert.equal(fs.readFileSync(path.join(c.out, 'SHA256SUMS.txt'), 'utf8'), SUMS)
})

const OUT_STATES = [
  ['<tmp>/out holds a file "notes.txt"', (out) => (fs.mkdirSync(out), fs.writeFileSync(path.join(out, 'notes.txt'), 'n')), (t) => `release: ${t}/out holds notes.txt, which assets did not write; remove it by hand`],
  ['<tmp>/out holds a folder "old"', (out) => fs.mkdirSync(path.join(out, 'old'), { recursive: true }), (t) => `release: ${t}/out holds old/, which assets did not write; remove it by hand`],
  ['<tmp>/out holds a file "FakeFlac.exe"', (out) => (fs.mkdirSync(out), fs.writeFileSync(path.join(out, 'FakeFlac.exe'), 'x')), (t) => `release: ${t}/out holds FakeFlac.exe, which assets did not write; remove it by hand`],
  ['<tmp>/out is a file', (out) => fs.writeFileSync(out, 'a file'), (t) => `release: ${t}/out is not a folder`],
]
for (const [state, prepare, message] of OUT_STATES) {
  test(`@RNF-11 Scenario Outline: An output folder holding anything else is left alone [${state}]`, async () => {
    const c = assetsCase()
    prepare(c.out)
    const outBefore = snap(c.out)
    const r = await cli(assetsArgs(c))
    assert.equal(r.code, 1, show(r))
    assert.deepEqual(r.stderr, [message(fwd(c.t))])
    assert.deepEqual(snap(c.out), outBefore)
    assert.ok(!fs.existsSync(c.notes))
  })
}

test('@RNF-11 Scenario: A write failure after the checks leaves nothing of the run behind', async () => {
  const c = assetsCase()
  const afile = path.join(c.t, 'afile')
  fs.writeFileSync(afile, 'x')
  const bundleBefore = snap(c.bundle)
  const r = await cli(assetsArgs(c, { notes: path.join(afile, 'notes.md') }))
  assert.equal(r.code, 1, show(r))
  assert.equal(r.stderr.length, 1, show(r))
  assert.ok(r.stderr[0].startsWith(`release: cannot write ${fwd(c.t)}/afile`), show(r))
  assert.ok(!r.stdout.some((l) => l.startsWith('release: wrote')), show(r))
  assert.ok(!fs.existsSync(c.out), 'the output folder this run created is removed')
  assert.ok(fs.statSync(afile).isFile())
  assert.equal(fs.readFileSync(afile, 'utf8'), 'x')
  assert.deepEqual(snap(c.bundle), bundleBefore)
})

const DISAGREE_TOML = [
  'release: the manifests do not agree on the version',
  'release:   package.json: 0.1.0',
  'release:   package-lock.json: 0.1.0',
  'release:   package-lock.json packages[""]: 0.1.0',
  'release:   src-tauri/tauri.conf.json: 0.1.0',
  'release:   Cargo.toml [workspace.package]: 0.2.0',
  'release:   Cargo.lock fakeflac: 0.1.0',
  'release:   Cargo.lock fakeflac-core: 0.1.0',
]

test('@RNF-11 Scenario: assets with manifests that disagree writes nothing', async () => {
  const c = assetsCase({ root: { over: { toml: '0.2.0' } } })
  const r = await cli(assetsArgs(c))
  assert.equal(r.code, 1, show(r))
  assert.deepEqual(r.stderr, DISAGREE_TOML)
  assert.deepEqual(r.stdout, [])
  assert.ok(!fs.existsSync(c.out) && !fs.existsSync(c.notes))
})

test('@RNF-11 @RNF-09 Scenario: Paths with spaces and Unicode characters (CLI)', () => {
  const dir = path.join(tmp(), 'Rélease ″x″ 🎵 dir')
  fs.mkdirSync(dir)
  const c = assetsCase({ at: dir })
  const r = spawnCli(assetsArgs(c))
  assert.equal(r.code, 0, show(r))
  assert.equal(fs.readFileSync(path.join(c.out, 'SHA256SUMS.txt'), 'utf8'), SUMS)
  const t = fwd(dir)
  assert.ok(t.includes('Rélease ″x″ 🎵 dir'))
  assert.ok(r.stdout.includes(`release: wrote ${t}/out (2 installers, SHA256SUMS.txt) and ${t}/notes.md`), show(r))
})

// =============================================================================================
// Arguments (release.md 3.0)
// =============================================================================================

const MAX = (v) => `release: --max-bytes must be a whole number from 1 to 9007199254740991, not "${v}"`
// The feature's table has two rows on one line (the "--max-bytes ''" row and the "--bundle --out"
// row); both are listed here as separate cases.
const USAGE = [
  ['(none)', 'release: usage: node scripts/release.mjs <command> [arguments]; commands: version, release-check, assets, draft, set-version'],
  ['publish', 'release: unknown command "publish"; commands: version, release-check, assets, draft, set-version'],
  ['version --verbose', 'release: unknown option --verbose for version'],
  ['version --root=x', 'release: unknown option --root=x for version'],
  ['version --print', 'release: unknown option --print for version'],
  ['version extra', 'release: unexpected argument "extra" for version'],
  ['version --root a --root b', 'release: option --root is given more than once'],
  ['version --root', 'release: option --root needs a value'],
  ["version --root ''", 'release: option --root has an empty value'],
  ["assets --bundle b --out '' --notes n.md --max-bytes 1000", 'release: option --out has an empty value'],
  ["assets --bundle b --out o --notes n.md --max-bytes ''", 'release: option --max-bytes has an empty value'],
  ['assets --bundle --out o --notes n.md --max-bytes 1000', 'release: option --bundle needs a value'],
  ['assets --bundle b --out o --notes n.md', 'release: assets needs --max-bytes'],
  ['assets --bundle b --out o --notes o/n.md --max-bytes 1000', 'release: --notes must not be inside --out'],
  ['assets --bundle b --out o --notes o --max-bytes 1000', 'release: --notes must not be inside --out'],
  ['assets --bundle b --out o --notes n.md --max-bytes 0', MAX('0')],
  ['assets --bundle b --out o --notes n.md --max-bytes 20MB', MAX('20MB')],
  ['assets --bundle b --out o --notes n.md --max-bytes 01000', MAX('01000')],
  ['assets --bundle b --out o --notes n.md --max-bytes 1.5', MAX('1.5')],
  ['assets --bundle b --out o --notes n.md --max-bytes -5', MAX('-5')],
  ['assets --bundle b --out o --notes n.md --max-bytes 9007199254740992', MAX('9007199254740992')],
  ['draft --assets a', 'release: draft needs --notes'],
  ['draft --assets a --notes n.md --print --print', 'release: option --print is given more than once'],
  ['set-version', 'release: set-version needs a version such as 1.2.3'],
  ['set-version 1.2.3 4.5.6', 'release: unexpected argument "4.5.6" for set-version'],
  ['release-check v0.1.0 v0.2.0', 'release: unexpected argument "v0.2.0" for release-check'],
  ['release-check', 'release: release-check needs a tag such as v1.2.3'],
]
for (const [command, message] of USAGE) {
  test(`@RNF-11 Scenario Outline: Usage errors [${command}]`, async () => {
    const r = await cli(cmdArgs(command))
    assert.equal(r.code, 2, show(r))
    assert.deepEqual(r.stderr, [message])
  })
}

// parseArgs's `options` shape is not fixed by release.md 3.8: accept the option under its
// flag name, its bare name or its camelCase name, as a plain object or a Map.
function option(parsed, flag) {
  const o = parsed.options ?? {}
  const bare = flag.replace(/^--/, '')
  const camel = bare.replace(/-([a-z])/g, (_, ch) => ch.toUpperCase())
  for (const k of [flag, bare, camel]) {
    const v = o instanceof Map ? o.get(k) : o[k]
    if (v !== undefined) return v
  }
  return undefined
}

for (const value of ['1', '1000', '20000000', '9007199254740991']) {
  test(`@RNF-11 Scenario Outline: Accepted --max-bytes values [${value}]`, () => {
    const parsed = release.parseArgs(['assets', '--bundle', 'b', '--out', 'o', '--notes', 'n.md', '--max-bytes', value])
    assert.deepEqual(parsed.errors, [])
    assert.equal(parsed.command, 'assets')
    assert.equal(String(option(parsed, '--max-bytes')), value)
  })
}

const UNKNOWN_TAKES_NO_VALUE = [
  ['version --verbose x', 'release: unknown option --verbose for version', 'release: unexpected argument "x" for version'],
  [
    'assets --bundle b --out o --notes n.md --max-bytes 1000 --print x',
    'release: unknown option --print for assets',
    'release: unexpected argument "x" for assets',
  ],
  ['set-version 1.2.3 --root=r x', 'release: unknown option --root=r for set-version', 'release: unexpected argument "x" for set-version'],
]
for (const [command, first, second] of UNKNOWN_TAKES_NO_VALUE) {
  test(`@RNF-11 Scenario Outline: An unknown option takes no value [${command}]`, async () => {
    const r = await cli(cmdArgs(command))
    assert.equal(r.code, 2, show(r))
    assert.deepEqual(r.stderr, [first, second])
  })
}

test('@RNF-11 Scenario: --print of draft is a flag', () => {
  const parsed = release.parseArgs(['draft', '--print', '--assets', 'a', '--notes', 'n.md'])
  assert.deepEqual(parsed.errors, [])
  assert.equal(parsed.command, 'draft')
  assert.ok(option(parsed, '--print'), '--print is set')
  assert.equal(option(parsed, '--assets'), 'a')
  assert.equal(option(parsed, '--notes'), 'n.md')
  assert.deepEqual(parsed.positionals, [])
})

test('@RNF-11 Scenario: Every usage error is reported', async () => {
  const r = await cli(['assets', '--out', 'o', '--print'])
  assert.equal(r.code, 2, show(r))
  assert.deepEqual(r.stderr, [
    'release: unknown option --print for assets',
    'release: assets needs --bundle',
    'release: assets needs --notes',
    'release: assets needs --max-bytes',
  ])
})

test('@RNF-11 Scenario: Usage errors decided before anything is read (CLI)', () => {
  const empty = tmp()
  const r = spawnCli([
    'assets',
    '--root',
    empty,
    '--bundle',
    path.join(empty, 'missing'),
    '--out',
    path.join(empty, 'out'),
    '--notes',
    path.join(empty, 'n.md'),
    '--max-bytes',
    '0',
  ])
  assert.equal(r.code, 2, show(r))
  assert.deepEqual(r.stderr, [MAX('0')])
  assert.deepEqual(fs.readdirSync(empty), [])
})

// =============================================================================================
// Installer size (RNF-06, release.md 3.3 step 3)
// =============================================================================================

const SIZES = [
  [999, 999, 0, []],
  [1000, 999, 1, ['nsis']],
  [999, 1001, 1, ['msi']],
  [1000, 1000, 1, ['nsis', 'msi']],
]
for (const [nsisBytes, msiBytes, code, failing] of SIZES) {
  test(`@RNF-11 @RNF-06 Scenario Outline: Each installer must be strictly under the limit [${nsisBytes}, ${msiBytes}]`, async () => {
    const c = assetsCase({ bundle: { files: { [`nsis/${NSIS}`]: nsisBytes, [`msi/${MSI}`]: msiBytes } } })
    const r = await cli(assetsArgs(c, { maxBytes: 1000 }))
    assert.equal(r.code, code, show(r))
    assert.ok(r.stdout.includes(`release: ${NSIS} ${nsisBytes} bytes (0.00 MB), limit 1000 bytes`), show(r))
    assert.ok(r.stdout.includes(`release: ${MSI} ${msiBytes} bytes (0.00 MB), limit 1000 bytes`), show(r))
    const line = {
      nsis: `release: ${NSIS} is ${nsisBytes} bytes, not under the limit of 1000 bytes (RNF-06)`,
      msi: `release: ${MSI} is ${msiBytes} bytes, not under the limit of 1000 bytes (RNF-06)`,
    }
    assert.deepEqual(r.stderr, failing.map((k) => line[k]))
  })
}

for (const bytes of [0, 999]) {
  test(`@RNF-11 @RNF-06 Scenario Outline: A lone MSI is still measured, and the NSIS installer is reported missing [${bytes}]`, async () => {
    const c = assetsCase({ bundle: { files: { [`msi/${MSI}`]: bytes } } })
    const r = await cli(assetsArgs(c, { maxBytes: 1000 }))
    assert.equal(r.code, 1, show(r))
    assert.deepEqual(r.stdout, [`release: ${MSI} ${bytes} bytes (0.00 MB), limit 1000 bytes`])
    assert.deepEqual(r.stderr, [`release: missing installer ${fwd(c.bundle)}/nsis/${NSIS}`])
  })
}

test('@RNF-11 @RNF-06 Scenario: An installer over the limit writes nothing', async () => {
  const c = assetsCase()
  const first = await cli(assetsArgs(c))
  assert.equal(first.code, 0, show(first))
  const outBefore = snap(c.out)
  const notesBefore = snap(c.notes)
  fs.writeFileSync(path.join(c.bundle, 'msi', MSI), Buffer.alloc(1001, 0x41))
  const r = await cli(assetsArgs(c, { maxBytes: 1000 }))
  assert.equal(r.code, 1, show(r))
  assert.deepEqual(r.stderr, [`release: ${MSI} is 1001 bytes, not under the limit of 1000 bytes (RNF-06)`])
  assert.deepEqual(snap(c.out), outBefore)
  assert.deepEqual(snap(c.notes), notesBefore)
})

for (const [bytes, code, mb] of [
  [19_999_999, 0, '20.00'],
  [20_000_000, 1, '20.00'],
]) {
  test(`@RNF-11 @RNF-06 Scenario Outline: The 20 MB limit of the recipe [${bytes}]`, async () => {
    const c = assetsCase({ bundle: { files: { [`nsis/${NSIS}`]: Buffer.alloc(bytes), [`msi/${MSI}`]: 1000 } } })
    const r = await cli(assetsArgs(c, { maxBytes: 20000000 }))
    assert.equal(r.code, code, show(r))
    assert.ok(r.stdout.includes(`release: ${NSIS} ${bytes} bytes (${mb} MB), limit 20000000 bytes`), show(r))
  })
}

// =============================================================================================
// Draft release (release.md 3.6)
// =============================================================================================

test('@RNF-11 Scenario: The draft command', async () => {
  const c = await writtenAssets()
  const expected = [
    'release',
    'create',
    'v0.1.0',
    '--draft',
    '--verify-tag',
    '--title',
    'FakeFlac 0.1.0',
    '--generate-notes',
    '--notes-file',
    c.notes,
    path.join(c.out, NSIS),
    path.join(c.out, MSI),
    path.join(c.out, 'SHA256SUMS.txt'),
  ]
  assert.deepEqual(release.draftArgs({ productName: 'FakeFlac', version: '0.1.0', assets: c.out, notes: c.notes }), expected)
  const r = await cli(['draft', '--root', c.root, '--assets', c.out, '--notes', c.notes, '--print'])
  assert.equal(r.code, 0, show(r))
  assert.deepEqual(r.stdout, [JSON.stringify(expected)])
})

test('@RNF-11 Scenario: The --notes item of the draft command is never rewritten', async () => {
  const c = await writtenAssets()
  const itemAfterNotesFile = (r) => {
    assert.equal(r.code, 0, show(r))
    assert.equal(r.stdout.length, 1, show(r))
    const args = JSON.parse(r.stdout[0])
    return args[args.indexOf('--notes-file') + 1]
  }
  const slashDot = `${c.t}/./notes.md`
  let r = await cli(['draft', '--root', c.root, '--assets', c.out, '--notes', slashDot, '--print'])
  assert.equal(itemAfterNotesFile(r), slashDot)
  if (process.platform === 'win32') {
    const backslashDot = `${c.t}\\.\\notes.md`
    r = await cli(['draft', '--root', c.root, '--assets', c.out, '--notes', backslashDot, '--print'])
    assert.equal(itemAfterNotesFile(r), backslashDot)
  }
})

test('@RNF-11 Scenario: The draft never runs outside GitHub Actions (CLI)', async () => {
  const c = await writtenAssets()
  const emptyPath = path.join(c.t, 'empty-path')
  fs.mkdirSync(emptyPath)
  const env = isolatedEnv({}, { pathDir: emptyPath })
  assert.ok(!Object.keys(env).some((k) => /^GITHUB_ACTIONS$/i.test(k)))
  assert.deepEqual(
    Object.keys(env).filter((k) => k.toUpperCase() === 'PATH'),
    ['PATH'],
  )
  const r = spawnCli(['draft', '--root', c.root, '--assets', c.out, '--notes', c.notes], env)
  assert.equal(r.code, 1, show(r))
  assert.deepEqual(r.stderr, ['release: draft runs only in GitHub Actions (use --print to see the command)'])
  assert.deepEqual(r.stdout, [])
})

const DRAFT_CHANGES = [
  ['one byte of <tmp>/out/<msi> is changed', (c) => fs.writeFileSync(path.join(c.out, MSI), 'abce'), () => `release: SHA256SUMS.txt does not match ${MSI}`],
  [
    'an LF is appended to <tmp>/out/SHA256SUMS.txt',
    (c) => fs.appendFileSync(path.join(c.out, 'SHA256SUMS.txt'), '\n'),
    () => 'release: SHA256SUMS.txt is not exactly the checksums of the two installers',
  ],
  ['<tmp>/out/<nsis> is deleted', (c) => fs.rmSync(path.join(c.out, NSIS)), (t) => `release: ${t}/out has no ${NSIS}`],
  ['<tmp>/out also holds "extra.txt"', (c) => fs.writeFileSync(path.join(c.out, 'extra.txt'), 'x'), (t) => `release: ${t}/out holds an unexpected extra.txt`],
  ['<tmp>/out also holds an empty folder "sub"', (c) => fs.mkdirSync(path.join(c.out, 'sub')), (t) => `release: ${t}/out holds an unexpected sub/`],
  ['<tmp>/notes.md is deleted', (c) => fs.rmSync(c.notes), (t) => `release: missing ${t}/notes.md`],
  [
    'notes.md starts with "FakeFlac 0.0.9 "',
    (c) => fs.writeFileSync(c.notes, fs.readFileSync(c.notes, 'utf8').replace(/^FakeFlac 0\.1\.0 /, 'FakeFlac 0.0.9 ')),
    (t) => `release: ${t}/notes.md is not the release notes of 0.1.0`,
  ],
  [
    'the environment has GITHUB_REF_TYPE "tag" and GITHUB_REF_NAME "v0.1.1"',
    () => ({ GITHUB_REF_TYPE: 'tag', GITHUB_REF_NAME: 'v0.1.1' }),
    () => 'release: the workflow\'s tag "v0.1.1" does not match the manifests\' version 0.1.0',
  ],
]
for (const [change, apply, message] of DRAFT_CHANGES) {
  test(`@RNF-11 Scenario Outline: The draft checks what it uploads [${change}]`, async () => {
    const c = await writtenAssets()
    const env = apply(c) ?? {}
    const r = await cli(['draft', '--root', c.root, '--assets', c.out, '--notes', c.notes, '--print'], typeof env === 'object' ? env : {})
    assert.equal(r.code, 1, show(r))
    assert.deepEqual(r.stderr, [message(fwd(c.t))])
    assert.deepEqual(r.stdout, [])
  })
}

test('@RNF-11 Scenario: draft with manifests that disagree runs nothing', async () => {
  const c = await writtenAssets()
  fs.writeFileSync(path.join(c.root, 'package.json'), manifestTexts(versions('0.1.0', { pkg: '0.2.0' }))['package.json'])
  const outBefore = snap(c.out)
  const notesBefore = snap(c.notes)
  const r = await cli(['draft', '--root', c.root, '--assets', c.out, '--notes', c.notes, '--print'])
  assert.equal(r.code, 1, show(r))
  assert.deepEqual(r.stderr, [
    'release: the manifests do not agree on the version',
    'release:   package.json: 0.2.0',
    'release:   package-lock.json: 0.1.0',
    'release:   package-lock.json packages[""]: 0.1.0',
    'release:   src-tauri/tauri.conf.json: 0.1.0',
    'release:   Cargo.toml [workspace.package]: 0.1.0',
    'release:   Cargo.lock fakeflac: 0.1.0',
    'release:   Cargo.lock fakeflac-core: 0.1.0',
  ])
  assert.deepEqual(r.stdout, [])
  assert.deepEqual(snap(c.out), outBefore)
  assert.deepEqual(snap(c.notes), notesBefore)
})

test('@RNF-11 Scenario: A draft for another version lists every problem', async () => {
  const c = await writtenAssets()
  const bumped = await cli(['set-version', '0.2.0', '--root', c.root])
  assert.equal(bumped.code, 0, show(bumped))
  const r = await cli(['draft', '--root', c.root, '--assets', c.out, '--notes', c.notes, '--print'])
  assert.equal(r.code, 1, show(r))
  const t = fwd(c.t)
  assert.deepEqual(r.stderr, [
    `release: ${t}/out has no FakeFlac_0.2.0_x64-setup.exe`,
    `release: ${t}/out has no FakeFlac_0.2.0_x64_en-US.msi`,
    `release: ${t}/out holds an unexpected ${NSIS}`,
    `release: ${t}/out holds an unexpected ${MSI}`,
    `release: ${t}/notes.md is not the release notes of 0.2.0`,
  ])
})

// =============================================================================================
// Version bump (release.md 3.7)
// =============================================================================================

test('@RNF-11 Scenario: set-version rewrites only the version strings (CLI)', async () => {
  const root = tmp()
  const opts = { crlf: ['Cargo.toml'], bom: ['src-tauri/tauri.conf.json'], noFinalNewline: ['package.json'] }
  const before = writeRoot(root, '0.1.0', opts)
  assert.ok(!before['package.json'].endsWith('\n'))
  const r = spawnCli(['set-version', '0.2.0', '--root', root])
  assert.equal(r.code, 0, show(r))
  assert.deepEqual(r.stdout, [
    'release: set version 0.2.0 in package.json, package-lock.json, src-tauri/tauri.conf.json, Cargo.toml, Cargo.lock',
  ])
  const check = await cli(['version', '--root', root])
  assert.deepEqual(check.stdout, ['release: version 0.2.0 (7 sources agree)'], show(check))

  // Each file is its text before with only the located spans changed: the same builder at the new
  // version (decoys are fixed at 0.1.0 by the builder, whatever the version).
  const expected = manifestTexts(versions('0.2.0'), opts)
  const spans = { 'package.json': 1, 'package-lock.json': 2, 'src-tauri/tauri.conf.json': 1, 'Cargo.toml': 1, 'Cargo.lock': 2 }
  for (const rel of MANIFESTS) {
    const now = readText(root, rel)
    assert.equal(now, expected[rel], `${rel} differs beyond its version spans`)
    assert.equal(now.split('0.2.0').length - 1, spans[rel], `${rel}: ${spans[rel]} spans`)
    assert.equal(now.length, before[rel].length, `${rel}: same length (0.1.0 and 0.2.0 have five characters)`)
  }
  const tauri = readText(root, 'src-tauri/tauri.conf.json')
  assert.ok(tauri.startsWith(BOM))
  assert.ok(tauri.includes('"targets": ["nsis", "msi"]'))

  // The eight decoys.
  const pkg = JSON.parse(readText(root, 'package.json'))
  assert.equal(pkg.config.version, '0.1.0')
  assert.equal(pkg.description, 'decoy "version": "0.1.0"')
  assert.equal(JSON.parse(readText(root, 'package-lock.json')).packages['node_modules/decoy'].version, '0.1.0')
  assert.equal(JSON.parse(tauri.slice(1)).bundle.version, '0.1.0')
  const toml = readText(root, 'Cargo.toml')
  assert.ok(toml.includes('[workspace.metadata.decoy]\r\nversion = "0.1.0"\r\n'))
  assert.ok(toml.includes('foo = { version = "0.1.0" }\r\n'))
  assert.ok(toml.includes('# version = "0.1.0"\r\n'))
  assert.ok(toml.includes('\r\nversion = "0.2.0"\r\n'))
  const lock = readText(root, 'Cargo.lock')
  assert.ok(lock.includes('name = "decoy"\nversion = "0.1.0"\n'))
  assert.ok(lock.includes('\nversion = 4\n'))
  assert.ok(lock.includes('name = "serde"\nversion = "1.0.228"\n'))
})

test('@RNF-11 Scenario: set-version also aligns manifests that disagreed, and lists only the files it changed', async () => {
  const root = tmp()
  writeRoot(root, '0.2.0', { over: { toml: '0.0.9' } })
  const before = readManifests(root)
  const r = await cli(['set-version', '0.2.0', '--root', root])
  assert.equal(r.code, 0, show(r))
  assert.deepEqual(r.stdout, ['release: set version 0.2.0 in Cargo.toml'])
  const now = readManifests(root)
  for (const rel of MANIFESTS.filter((f) => f !== 'Cargo.toml')) assert.deepEqual(now[rel], before[rel], rel)
  assert.equal(readText(root, 'Cargo.toml'), manifestTexts(versions('0.2.0'))['Cargo.toml'])
  const check = await cli(['version', '--root', root])
  assert.equal(check.code, 0, show(check))
  assert.deepEqual(check.stdout, ['release: version 0.2.0 (7 sources agree)'])
})

test('@RNF-11 Scenario: set-version with nothing to change writes nothing', async () => {
  const root = tmp()
  writeRoot(root, '0.2.0')
  // An old modification time, so that any rewrite would show.
  const old = new Date('2020-01-01T00:00:00Z')
  for (const rel of MANIFESTS) fs.utimesSync(path.join(root, rel), old, old)
  const before = readManifests(root)
  const mtimes = MANIFESTS.map((rel) => fs.statSync(path.join(root, rel)).mtimeMs)
  const r = await cli(['set-version', '0.2.0', '--root', root])
  assert.equal(r.code, 0, show(r))
  assert.deepEqual(r.stdout, ['release: nothing to change, already 0.2.0'])
  assertManifestsUnchanged(root, before)
  assert.deepEqual(
    MANIFESTS.map((rel) => fs.statSync(path.join(root, rel)).mtimeMs),
    mtimes,
  )
})

for (const [version, message] of [
  ['1.0.0-beta.1', PRE('1.0.0-beta.1')],
  ['1.0', SHAPE('1.0')],
  ['256.0.0', RANGE('256.0.0')],
]) {
  test(`@RNF-11 Scenario Outline: set-version refuses a version that cannot be released [${version}]`, async () => {
    const root = tmp()
    writeRoot(root, '0.1.0')
    const before = readManifests(root)
    const r = await cli(['set-version', version, '--root', root])
    assert.equal(r.code, 1, show(r))
    assert.deepEqual(r.stderr, [message])
    assertManifestsUnchanged(root, before)
  })
}

for (const [damage, o, message] of [
  ['package.json holds "{not json"', { content: { 'package.json': '{not json' } }, 'release: package.json is not valid JSON'],
  ['package.json has a second top-level "version" key holding "0.1.0"', { pkgDuplicate: true }, 'release: package.json has a duplicate "version" key'],
]) {
  test(`@RNF-11 Scenario Outline: set-version writes nothing when a manifest is broken [${damage}]`, async () => {
    const root = tmp()
    writeRoot(root, '0.1.0', o)
    const before = readManifests(root)
    const r = await cli(['set-version', '0.2.0', '--root', root])
    assert.equal(r.code, 1, show(r))
    assert.deepEqual(r.stderr, [message])
    assertManifestsUnchanged(root, before)
  })
}

// =============================================================================================
// Wiring (static checks of repository files, release.md 5.3)
// =============================================================================================

const readRepo = (rel) => fs.readFileSync(path.join(repo, rel), 'utf8').replace(/\r\n/g, '\n')

// A recipe's body: the indented, non-empty, non-comment lines after its header line.
function recipeBody(justfile, header) {
  const lines = justfile.split('\n')
  const i = lines.indexOf(header)
  assert.notEqual(i, -1, `justfile has no recipe line "${header}"`)
  const body = []
  for (const l of lines.slice(i + 1)) {
    if (!/^[ \t]/.test(l) || l.trim() === '') break
    if (!l.trim().startsWith('#')) body.push(l.trim())
  }
  return body
}

const RECIPES = [
  ['version-check:', 'node scripts/release.mjs version'],
  ['release-check *args:', 'node scripts/release.mjs release-check {{args}}'],
  [
    'release-assets:',
    'node scripts/release.mjs assets --bundle target/release/bundle --out target/release-staging/assets --notes target/release-staging/release-notes.md --max-bytes 20000000',
  ],
  ['release-draft:', 'node scripts/release.mjs draft --assets target/release-staging/assets --notes target/release-staging/release-notes.md'],
  ['set-version version:', 'node scripts/release.mjs set-version {{version}}'],
]

// Workflow level (before `jobs:`) and each job's text (from `  <id>:` to the next such line).
function workflow(rel) {
  const lines = readRepo(rel).split('\n')
  const j = lines.findIndex((l) => /^jobs:\s*$/.test(l))
  assert.notEqual(j, -1, `${rel} has no "jobs:" line`)
  const jobs = new Map()
  let id = null
  for (const l of lines.slice(j + 1)) {
    const m = /^ {2}([A-Za-z0-9_-]+):\s*(#.*)?$/.exec(l)
    if (m) {
      id = m[1]
      jobs.set(id, [l])
    } else if (id) jobs.get(id).push(l)
  }
  const text = new Map([...jobs].map(([k, v]) => [k, v.join('\n')]))
  return { all: lines.join('\n'), level: lines.slice(0, j).join('\n'), jobs: text }
}
const job = (wf, id) => {
  assert.ok(wf.jobs.has(id), `no job "${id}"`)
  return wf.jobs.get(id)
}
const escapeRe = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
const runsStep = (text, cmd) => new RegExp(`^\\s*(- )?run: ${escapeRe(cmd)}\\s*$`, 'm').test(text)

test('@RNF-11 @RNF-06 Scenario: The size gate runs on every PR and every release with 20 MB', () => {
  const just = readRepo('justfile')
  assert.deepEqual(recipeBody(just, 'release-assets:'), [RECIPES[2][1]])
  assert.equal(just.split('20000000').length - 1, 1, '"20000000" appears exactly once in the justfile')

  const ci = workflow('.github/workflows/ci.yml')
  const bw = job(ci, 'build-windows')
  for (const cmd of ['just build', 'just release-assets']) assert.ok(runsStep(bw, cmd), `build-windows runs ${cmd}`)
  assert.ok(bw.includes('target/release-staging/'), 'build-windows uploads target/release-staging/')

  const rel = workflow('.github/workflows/release.yml')
  const pkg = job(rel, 'package')
  for (const cmd of ['just build', 'just release-assets']) assert.ok(pkg.includes(cmd), `package runs ${cmd}`)

  for (const wf of [ci, rel]) {
    for (const s of ['Get-FileHash', '20MB', 'npm run tauri build']) assert.ok(!wf.all.includes(s), `a workflow contains ${s}`)
  }
})

test('@RNF-11 @RNF-06 Scenario: The bundle keeps the small WebView2 bootstrapper', () => {
  const conf = JSON.parse(readRepo('src-tauri/tauri.conf.json'))
  assert.deepEqual(conf.bundle.targets, ['nsis', 'msi'])
  assert.equal(conf.bundle.windows.webviewInstallMode.type, 'downloadBootstrapper')
})

test('@RNF-11 Scenario: The recipes and the CI version check', () => {
  const just = readRepo('justfile')
  for (const [header, command] of RECIPES) assert.deepEqual(recipeBody(just, header), [command], header)
  assert.ok(just.split('\n').includes('ci: check-fast test trace guard-tests version-check check-app'), 'the ci line')
  const ci = workflow('.github/workflows/ci.yml')
  assert.ok(runsStep(job(ci, 'specs'), 'just version-check'), 'specs runs just version-check')
  const bw = job(ci, 'build-windows')
  assert.ok(runsStep(bw, 'just check-app'), 'build-windows runs just check-app')
})

test('@RNF-11 Scenario: The release workflow verifies, packages and only publishes from a tag', () => {
  const wf = workflow('.github/workflows/release.yml')
  assert.ok(wf.level.includes('tags: ["v*.*.*"]'), 'triggered by tags v*.*.*')
  assert.ok(/^\s*workflow_dispatch:/m.test(wf.level), 'triggered by workflow_dispatch')
  assert.ok(wf.level.includes('contents: read'), 'workflow level contents: read')
  assert.ok(!wf.level.includes(': write'), 'no write permission at workflow level')

  const verify = job(wf, 'verify')
  for (const s of ['fetch-depth: 0', 'components: "rustfmt, clippy"', 'just release-check', 'just version-check', 'just ci']) {
    assert.ok(verify.includes(s), `verify contains ${s}`)
  }
  const pkg = job(wf, 'package')
  for (const s of ['needs: verify', 'just build', 'just release-assets', 'target/release-staging/']) assert.ok(pkg.includes(s), `package contains ${s}`)
  assert.ok(!pkg.includes(': write'), 'package has no write permission')

  const publish = job(wf, 'publish')
  for (const s of [
    'needs: package',
    'contents: write',
    'id-token: write',
    'attestations: write',
    'actions/attest-build-provenance',
    'target/release-staging',
    'just release-draft',
  ]) {
    assert.ok(publish.includes(s), `publish contains ${s}`)
  }
  for (const s of ['tauri build', 'just build']) assert.ok(!publish.includes(s), `publish builds nothing (${s})`)

  for (const [id, text] of wf.jobs) {
    if (id === 'publish') continue
    for (const s of ['contents: write', 'id-token: write', 'attestations: write']) assert.ok(!text.includes(s), `${id} has ${s}`)
  }
  for (const l of wf.all.split('\n').filter((l) => l.includes('run:'))) {
    assert.match(l, /^\s*(- )?run: (just [a-z-]+|npm ci)\s*$/)
  }
  for (const s of ['${{ github.ref_name }}', 'Get-FileHash', 'ConvertFrom-Json', '20MB', 'npm run tauri build', 'gh release']) {
    assert.ok(!wf.all.includes(s), `release.yml contains ${s}`)
  }
})

test('@RNF-11 Scenario: The dry run never publishes', () => {
  const wf = workflow('.github/workflows/release.yml')
  const publish = job(wf, 'publish').split('\n')
  assert.ok(publish.includes("    if: github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v')"), 'the exact if: of publish')
  for (const [id, text] of wf.jobs) {
    if (id === 'publish') continue
    assert.ok(!text.split('\n').some((l) => /^ {4}if:/.test(l)), `job ${id} has a job-level if:`)
  }
})

// =============================================================================================
// The given env reaches git and gh (release.md 3 "The environment", 3.2 step 4, 3.6 step 8, 8.1)
// =============================================================================================

test('@RNF-11 release.md 3.2 step 4 and 8.1: release-check runs git with the env given to run, not process.env', async () => {
  // --root is a repository whose HEAD is on origin/main, so git run there passes. The given env
  // names, through GIT_DIR, a second repository whose HEAD is not on origin/main. Only a git that
  // received the given env answers "not on origin/main". process.env has no GIT_* (8.1 isolation).
  assert.ok(!Object.keys(process.env).some((k) => /^GIT_DIR$/i.test(k)), 'process.env has no GIT_DIR')
  const root = gitCase('B', 'B')
  const other = gitCase('B', 'A')
  const control = await cli(['release-check', 'v0.1.0', '--root', root])
  assert.equal(control.code, 0, `control without GIT_DIR passes: ${show(control)}`)

  const r = await cli(['release-check', 'v0.1.0', '--root', root], { GIT_DIR: path.join(other, '.git') })
  assert.equal(r.code, 1, show(r))
  assert.deepEqual(r.stderr, [`release: commit ${short('B')} is not on origin/main; tag a commit of main`])
  assert.deepEqual(r.stdout, [])
})

test('@RNF-11 release.md 3.6 step 8 and 8.1: draft in GitHub Actions starts gh with the env given to run, not process.env', async () => {
  const c = await writtenAssets()
  const emptyPath = path.join(c.t, 'empty-path')
  const fakeBin = path.join(c.t, 'fake-bin')
  fs.mkdirSync(emptyPath)
  fs.mkdirSync(fakeBin)
  const record = path.join(c.t, 'gh-record.json')
  const recorder = path.join(c.t, 'gh-recorder.cjs')
  // The fake gh is node itself (an .exe on Windows, so spawn finds it without a shell). The
  // recorder is preloaded only through NODE_OPTIONS of the given env; it writes the sentinel it
  // sees and its arguments, then exits 0 before node would look for a script named "release".
  fs.writeFileSync(
    recorder,
    [
      "const fs = require('node:fs')",
      `fs.writeFileSync(${JSON.stringify(record)}, JSON.stringify({ sentinel: process.env.FAKEFLAC_GH_SENTINEL ?? null, argv: process.argv.slice(1) }))`,
      'process.exit(0)',
      '',
    ].join('\n'),
  )
  if (process.platform === 'win32') fs.copyFileSync(process.execPath, path.join(fakeBin, 'gh.exe'))
  else fs.symlinkSync(process.execPath, path.join(fakeBin, 'gh'))

  const sentinel = `sentinel-${process.pid}-${Date.now()}`
  const env = isolatedEnv(
    { GITHUB_ACTIONS: 'true', FAKEFLAC_GH_SENTINEL: sentinel, NODE_OPTIONS: `--require "${fwd(recorder)}"` },
    { pathDir: fakeBin },
  )
  assert.deepEqual(
    Object.keys(env).filter((k) => k.toUpperCase() === 'PATH'),
    ['PATH'],
  )

  // SAFETY: while run() executes, the in-process environment can start no gh at all, real or
  // fake: every PATH variant points to an empty folder and no token is set. A script that spawns
  // gh with process.env (or with no env) therefore fails to start it.
  const saved = Object.entries(process.env).filter(([k]) => /^(PATH|GH_TOKEN|GITHUB_TOKEN)$/i.test(k))
  let r
  try {
    for (const [k] of saved) {
      if (/^PATH$/i.test(k)) process.env[k] = emptyPath
      else delete process.env[k]
    }
    process.env.PATH = emptyPath
    r = await release.run(['draft', '--root', c.root, '--assets', c.out, '--notes', c.notes], env)
  } finally {
    for (const k of Object.keys(process.env)) if (/^(PATH|GH_TOKEN|GITHUB_TOKEN)$/i.test(k)) delete process.env[k]
    for (const [k, v] of saved) process.env[k] = v
  }

  assert.ok(fs.existsSync(record), `the fake gh did not run: ${JSON.stringify(r)}`)
  const seen = JSON.parse(fs.readFileSync(record, 'utf8'))
  assert.equal(seen.sentinel, sentinel, 'the given env reached gh')
  assert.equal(path.basename(seen.argv[0]), 'release')
  assert.deepEqual(seen.argv.slice(1), [
    'create',
    'v0.1.0',
    '--draft',
    '--verify-tag',
    '--title',
    'FakeFlac 0.1.0',
    '--generate-notes',
    '--notes-file',
    c.notes,
    path.join(c.out, NSIS),
    path.join(c.out, MSI),
    path.join(c.out, 'SHA256SUMS.txt'),
  ])
  assert.equal(r.code, 0, JSON.stringify(r))
  assert.deepEqual(r.stderr, [])
})
