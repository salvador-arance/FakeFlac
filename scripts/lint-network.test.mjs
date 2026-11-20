// Tests for the privacy rules (RNF-03, RNF-04): scripts/lint-network.mjs, called by
// scripts/lint-arch.mjs. Contract: specs/contracts/privacy.md (rules T1-T3, R1-R4, F1-F2,
// sections 3-5). Scenarios: specs/features/privacy.feature (@RNF-03; @RNF-04 for unreadable files).
// Every test names its scenario (or the contract section it covers) and its tag.
//
// MODULE API (binding for the implementer; Node built-ins only, no other import, so the module
// works when copied next to lint-arch.mjs):
//
//   export const NET: string[]
//     The NET list of privacy.md 3.2, in any order. Equal to the deny.toml [bans] deny names (P3).
//
//   Per-rule checkers. Pure: no file system access. `relPath` is the file's path relative to the
//   repository root with '/' separators; it is copied verbatim into every line. `text` is the
//   file's content, or null when the file is missing. Each returns an array of violation strings
//   of the exact form `<relPath>: RNF-03 <rule>: <detail>` (WITHOUT the leading ' - ', which
//   lint-arch adds). An empty array means no violation. They never throw.
//     checkTauriConf(relPath, text)      T1 and T2 on tauri.conf.json. null -> 'T1: missing';
//                                        invalid JSON -> 'T1: invalid JSON' only (T2 skipped).
//     checkCapability(relPath, text)     T3 on the content of one capability JSON file.
//                                        invalid JSON -> 'T3: invalid JSON'.
//     checkCargoManifest(relPath, text)  R1 on one Cargo.toml (member or workspace root).
//     checkCargoLock(relPath, text)      R2. null -> 'R2: missing'.
//     checkRustSource(relPath, text)     R4 on one .rs text. It does not filter by path.
//     checkFrontendSource(relPath, text) F1 on one frontend text. It does not filter by path.
//     checkPackageJson(relPath, text)    F2. null -> 'F2: missing'; invalid JSON -> 'F2: invalid JSON'.
//
//   export function lintNetwork(root: string): string[]
//     Reads the repository at `root` (read-only) and returns every RNF-03 violation string, same
//     form as above, paths relative to `root` with '/'. It chooses the files of each rule
//     (privacy.md 3): src-tauri/tauri.conf.json; every file in src-tauri/capabilities/ (a non-.json
//     file gets 'T3: only JSON capability files are allowed'; a missing or empty folder passes);
//     the root Cargo.toml and the Cargo.toml of crates/*, src-tauri and tools/*; Cargo.lock;
//     crates/*/src/**/*.rs and src-tauri/src/**/*.rs for R4; index.html and the .ts, .js, .svelte,
//     .css and .html files under src/ for F1, except names containing '.test.' or '.stories.' and
//     files under a folder named 'tests' or 'testing'; package.json. Missing folders (crates/,
//     tools/, src/, ...) are tolerated. A missing or invalid file never stops the other rules and
//     never throws.
//
// LINT-ARCH INTEGRATION (binding): lint-arch.mjs keeps computing `root` from its own location
// (import.meta.url) and adds the result of `lintNetwork(root)` to its `violations` array, importing
// the module as './lint-network.mjs'. No root override, no environment variable, no other change.
// To test it without touching the real repository, the test copies scripts/lint-arch.mjs and
// scripts/lint-network.mjs into <tmp>/scripts/ next to a fixture tree written in <tmp>, and runs
// the copy with node. <tmp> is a fresh folder under the repository's git-ignored target/ folder
// (agents never write outside the repository), removed when the tests end.
//
// Fixture texts are written inline in each test, never committed as fixture files (privacy.md 4).
// The fixture trees are written at run time under target/ and removed afterwards.
import { test, after } from 'node:test'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const here = path.dirname(fileURLToPath(import.meta.url))
const repo = path.join(here, '..')

// Load the module lazily so that a missing module fails every test with the import error, and the
// deny.toml test can fail on its own reason.
let loadError = null
let mod = null
try {
  mod = await import('./lint-network.mjs')
} catch (e) {
  loadError = e
}
const net = mod ?? new Proxy({}, { get: () => { throw loadError } })

// --- Helpers -------------------------------------------------------------------------------------

const uniq = (lines) => [...new Set(lines)].sort()
const line = (file, rule, detail) => `${file}: RNF-03 ${rule}: ${detail}`

let base = null
let caseNo = 0
const tmp = () => {
  if (!base) {
    fs.mkdirSync(path.join(repo, 'target'), { recursive: true })
    base = fs.mkdtempSync(path.join(repo, 'target', 'lint-network-test-'))
  }
  const dir = path.join(base, `c${++caseNo}`)
  fs.mkdirSync(dir)
  return dir
}
after(() => {
  if (base) fs.rmSync(base, { recursive: true, force: true })
})

// files: { 'rel/path': text } ; a value of null with a trailing '/' creates an empty folder.
const writeTree = (dir, files) => {
  for (const [rel, text] of Object.entries(files)) {
    const p = path.join(dir, ...rel.split('/').filter(Boolean))
    if (text === null) {
      fs.mkdirSync(p, { recursive: true })
      continue
    }
    fs.mkdirSync(path.dirname(p), { recursive: true })
    fs.writeFileSync(p, text)
  }
  return dir
}

// --- Fixture texts -------------------------------------------------------------------------------

const CSP = "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; connect-src ipc: http://ipc.localhost"
const CONF = 'src-tauri/tauri.conf.json'
const CAP = 'src-tauri/capabilities/default.json'
const PERMS = [
  'core:default',
  'core:window:allow-minimize',
  'core:window:allow-toggle-maximize',
  'core:window:allow-close',
  'core:window:allow-start-dragging',
]

// Today's tauri.conf.json shape; `mutate` edits a fresh copy.
const confText = (mutate = () => {}) => {
  const conf = {
    $schema: 'https://schema.tauri.app/config/2',
    productName: 'FakeFlac',
    version: '0.3.0',
    identifier: 'io.github.fakeflac.app',
    build: {
      beforeDevCommand: 'npm run dev',
      devUrl: 'http://localhost:1420',
      beforeBuildCommand: 'npm run build',
      frontendDist: '../dist',
    },
    app: {
      windows: [{ title: 'FakeFlac', width: 1280, height: 720, decorations: false }],
      security: { csp: CSP },
    },
    bundle: {
      active: true,
      targets: ['nsis', 'msi'],
      windows: { webviewInstallMode: { type: 'downloadBootstrapper' } },
    },
  }
  mutate(conf)
  return JSON.stringify(conf, null, 2)
}
const cspConf = (csp) => confText((c) => { c.app.security.csp = csp })

const capText = (permissions = PERMS, extra = {}) =>
  JSON.stringify(
    {
      $schema: '../gen/schemas/desktop-schema.json',
      identifier: 'default',
      description: 'Minimal capabilities (RNF-03, RP-06).',
      windows: ['main'],
      permissions,
      ...extra,
    },
    null,
    2,
  )

const manifest = (table, depLine) =>
  `[package]\nname = "x"\nversion = "0.1.0"\nedition = "2021"\n\n${table}\n${depLine}\n\n[lints]\nworkspace = true\n`

const SRC_TAURI_TOML = `[package]
name = "fakeflac"
version.workspace = true

[lib]
name = "fakeflac_lib"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
serde.workspace = true
serde_json.workspace = true
fakeflac-core = { path = "../crates/fakeflac-core" }
tauri-specta = { version = "=2.0.0-rc.25", features = ["derive", "typescript"] }
tauri-plugin-dialog = "2.7.3"

# The Windows user's language list. Only these features are enabled.
[target.'cfg(windows)'.dependencies]
windows = { version = "0.62", features = ["System_UserProfile", "Foundation_Collections"] }

[dev-dependencies]
hound = "3.5.1"

[lints]
workspace = true
`

const ROOT_TOML = `[workspace]
resolver = "2"
members = ["crates/fakeflac-core", "tools/fakeflac-calibrate", "src-tauri"]

[workspace.package]
version = "0.3.0"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[profile.dev.package."*"]
opt-level = 3
`

const lockPkg = (name, version = '1.0.0', deps = []) =>
  `[[package]]\nname = "${name}"\nversion = "${version}"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\n` +
  (deps.length ? `dependencies = [\n${deps.map((d) => ` "${d}",`).join('\n')}\n]\n` : '')
const lockText = (...pkgs) =>
  '# This file is automatically @generated by Cargo.\n# It is not intended for manual editing.\nversion = 4\n\n' +
  pkgs.join('\n')
// Today's lock holds these three as tauri's mobile-only dependencies (privacy.md 2).
const TODAY_LOCK = lockText(
  lockPkg('hyper', '1.12.0'),
  lockPkg('hyper-util', '0.1.21', ['hyper']),
  lockPkg('reqwest', '0.13.5', ['hyper', 'hyper-util']),
  lockPkg('tauri', '2.12.6', ['reqwest', 'serde', 'tokio']),
  lockPkg('serde', '1.0.0'),
  lockPkg('tokio', '1.53.3', ['mio', 'socket2']),
)

const PACKAGE_JSON = (deps = { '@tauri-apps/api': '^2' }, devDeps = { '@tauri-apps/cli': '^2', svelte: '^5' }) =>
  JSON.stringify({ name: 'fakeflac', private: true, type: 'module', dependencies: deps, devDependencies: devDeps }, null, 2)

const INDEX_HTML = `<!doctype html>
<html lang="en">
  <head><meta charset="UTF-8" /><title>FakeFlac</title></head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
`

const ICON_SVELTE = `<script lang="ts">
  let { name }: { name: string } = $props()
</script>

<svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
></svg>
`

// A minimal repository that meets every rule (and lint-arch's other rules).
const validTree = () => ({
  'Cargo.toml': ROOT_TOML,
  'Cargo.lock': TODAY_LOCK,
  'package.json': PACKAGE_JSON(),
  'index.html': INDEX_HTML,
  'src-tauri/Cargo.toml': SRC_TAURI_TOML,
  'src-tauri/tauri.conf.json': confText(),
  'src-tauri/capabilities/default.json': capText(),
  'src-tauri/src/lib.rs': 'pub fn run() {}\n',
  'crates/fakeflac-core/Cargo.toml': manifest('[dependencies]', 'thiserror = "2"'),
  'crates/fakeflac-core/src/lib.rs': '//! Domain.\npub struct Verdict;\n',
  'tools/fakeflac-calibrate/Cargo.toml': manifest('[dependencies]', 'fakeflac-core = { path = "../../crates/fakeflac-core" }'),
  'tools/fakeflac-calibrate/src/main.rs': 'fn main() {}\n',
  'src/main.ts': "import App from './App.svelte'\nexport default App\n",
  'src/lib/components/atoms/Icon.svelte': ICON_SVELTE,
})

// Copies lint-arch.mjs and lint-network.mjs next to a fixture tree and runs the copy.
const runLintArch = (files) => {
  if (loadError) throw loadError
  const dir = writeTree(tmp(), files)
  fs.mkdirSync(path.join(dir, 'scripts'))
  for (const f of ['lint-arch.mjs', 'lint-network.mjs']) {
    fs.copyFileSync(path.join(here, f), path.join(dir, 'scripts', f))
  }
  const r = spawnSync(process.execPath, [path.join(dir, 'scripts', 'lint-arch.mjs')], { cwd: dir, encoding: 'utf8' })
  return { status: r.status, stdout: r.stdout, stderr: r.stderr }
}
// Every stderr line is the header or a violation line: no stack trace, nothing else.
const assertCleanFailure = (r) => {
  assert.equal(r.status, 1, `exit code\n${r.stderr}`)
  const lines = r.stderr.split(/\r?\n/).filter((l) => l !== '')
  assert.equal(lines[0], 'lint-arch: violations found')
  for (const l of lines.slice(1)) assert.match(l, /^ - /, `unexpected stderr line: ${l}`)
  return lines
}

// --- NET and deny.toml (P3) ----------------------------------------------------------------------

const SPEC_NET = [
  'reqwest', 'hyper', 'hyper-util', 'h2', 'h3', 'quinn', 'ureq', 'curl', 'curl-sys', 'isahc',
  'attohttpc', 'surf', 'awc', 'tungstenite', 'tokio-tungstenite', 'async-tungstenite', 'rustls',
  'native-tls', 'openssl', 'openssl-sys', 'hyper-rustls', 'hyper-tls', 'tokio-rustls',
  'tokio-native-tls', 'tauri-plugin-updater', 'tauri-plugin-http', 'tauri-plugin-websocket',
  'tauri-plugin-upload', 'tauri-plugin-shell', 'tauri-plugin-opener',
]

// Small TOML reader for deny.toml: the names in [bans] deny (strings, or inline tables with
// `crate` or `name`; a '@version' suffix is dropped). Returns null when there is no deny list.
function denyNames(toml) {
  // Drop comments outside strings.
  const stripped = toml
    .split(/\r?\n/)
    .map((l) => {
      let inStr = false
      for (let i = 0; i < l.length; i++) {
        if (l[i] === '"' && l[i - 1] !== '\\') inStr = !inStr
        else if (l[i] === '#' && !inStr) return l.slice(0, i)
      }
      return l
    })
  const header = /^\s*\[{1,2}[^\[\]]+\]{1,2}\s*$/
  const start = stripped.findIndex((l) => /^\s*\[bans\]\s*$/.test(l))
  if (start < 0) return null
  let end = stripped.findIndex((l, i) => i > start && header.test(l))
  if (end < 0) end = stripped.length
  const section = stripped.slice(start + 1, end).join('\n')
  const m = /^\s*deny\s*=\s*\[/m.exec(section)
  if (!m) return null
  // Balanced scan of the array, splitting top-level elements.
  const elems = []
  let depth = 0
  let inStr = false
  let cur = ''
  for (let i = m.index + m[0].length; i < section.length; i++) {
    const ch = section[i]
    if (inStr) {
      cur += ch
      if (ch === '"' && section[i - 1] !== '\\') inStr = false
      continue
    }
    if (ch === '"') { inStr = true; cur += ch; continue }
    if (ch === '[' || ch === '{') depth++
    if (ch === ']' || ch === '}') {
      if (depth === 0) { elems.push(cur); break }
      depth--
    }
    if (ch === ',' && depth === 0) { elems.push(cur); cur = ''; continue }
    cur += ch
  }
  return elems
    .map((e) => e.trim())
    .filter((e) => e !== '')
    .map((e) => {
      if (e.startsWith('"')) return JSON.parse(e)
      const k = /\b(?:crate|name)\s*=\s*"([^"]*)"/.exec(e)
      return k ? k[1] : e
    })
    .map((n) => n.replace(/@.*$/, ''))
}

test('@RNF-03 Scenario: The banned crates of deny.toml are the NET list (P3)', () => {
  const names = denyNames(fs.readFileSync(path.join(repo, 'deny.toml'), 'utf8'))
  assert.ok(names, 'deny.toml has no [bans] deny list')
  assert.deepEqual([...names].sort(), [...SPEC_NET].sort(), 'deny.toml [bans] deny must list exactly the NET names')
  assert.deepEqual([...net.NET].sort(), [...names].sort(), 'NET in lint-network.mjs must equal deny.toml [bans] deny')
})

test('@RNF-03 privacy.md 3.2: NET in the module is exactly the normative list', () => {
  assert.ok(Array.isArray(net.NET))
  assert.deepEqual([...net.NET].sort(), [...SPEC_NET].sort())
})

test('@RNF-03 test helper: the deny.toml reader reads strings, inline tables and versions', () => {
  const toml = `[graph]\ntargets = ["x"]\n\n[bans]\nmultiple-versions = "warn"\ndeny = [\n  "reqwest", # comment, with "quotes"\n  { crate = "hyper@1", reason = "net, [x]" },\n  { name = "h2", wrappers = ["a", "b"] },\n]\n\n[sources]\nunknown-git = "deny"\n`
  assert.deepEqual(denyNames(toml), ['reqwest', 'hyper', 'h2'])
  assert.equal(denyNames('[bans]\nmultiple-versions = "warn"\n'), null)
})

// --- The real repository -------------------------------------------------------------------------

test('@RNF-03 Scenario: The repository as it is passes the privacy rules (lintNetwork)', () => {
  assert.deepEqual(net.lintNetwork(repo), [])
})

test('@RNF-03 Scenario: The repository as it is passes the privacy rules (lint-arch)', () => {
  if (loadError) throw loadError
  const r = spawnSync(process.execPath, [path.join(here, 'lint-arch.mjs')], { cwd: repo, encoding: 'utf8' })
  assert.equal(r.status, 0, r.stderr)
  assert.match(r.stdout, /^lint-arch: ok\r?\n?$/)
})

// --- T1 CSP ----------------------------------------------------------------------------------------

const T1_ROWS = [
  ["default-src 'self'; connect-src ipc: http://ipc.localhost https://example.com", "CSP source 'https://example.com' not allowed in connect-src"],
  ["default-src 'self'; img-src 'self' https:", "CSP source 'https:' not allowed in img-src"],
  ['default-src *', "default-src must be 'self'"],
  ["default-src 'self'; script-src 'self' https://cdn.example.com", "CSP source 'https://cdn.example.com' not allowed in script-src"],
  ["default-src 'self'; style-src 'self' http://ipc.localhost", "CSP source 'http://ipc.localhost' not allowed in style-src"],
]
for (const [csp, detail] of T1_ROWS) {
  test(`@RNF-03 Scenario Outline: A CSP source that can reach a remote origin is refused [${csp}]`, () => {
    assert.deepEqual(uniq(net.checkTauriConf(CONF, cspConf(csp))), [line(CONF, 'T1', detail)])
  })
}

test('@RNF-03 T1: data: outside img-src and ipc: outside connect-src are refused', () => {
  assert.deepEqual(uniq(net.checkTauriConf(CONF, cspConf("default-src 'self'; style-src 'self' data:"))), [
    line(CONF, 'T1', "CSP source 'data:' not allowed in style-src"),
  ])
  assert.deepEqual(uniq(net.checkTauriConf(CONF, cspConf("default-src 'self'; img-src 'self' ipc:"))), [
    line(CONF, 'T1', "CSP source 'ipc:' not allowed in img-src"),
  ])
})

test('@RNF-03 T1: a default-src that is not exactly self gives only that line, and a missing one is refused', () => {
  assert.deepEqual(uniq(net.checkTauriConf(CONF, cspConf("default-src 'self' https://example.com; img-src 'self'"))), [
    line(CONF, 'T1', "default-src must be 'self'"),
  ])
  assert.deepEqual(uniq(net.checkTauriConf(CONF, cspConf("img-src 'self'; connect-src ipc: http://ipc.localhost"))), [
    line(CONF, 'T1', "default-src must be 'self'"),
  ])
})

test('@RNF-03 Scenario: A missing CSP is refused (no csp key, or not a string)', () => {
  const missing = [line(CONF, 'T1', 'CSP missing')]
  assert.deepEqual(uniq(net.checkTauriConf(CONF, confText((c) => { delete c.app.security.csp }))), missing)
  assert.deepEqual(uniq(net.checkTauriConf(CONF, confText((c) => { delete c.app.security }))), missing)
  assert.deepEqual(uniq(net.checkTauriConf(CONF, confText((c) => { c.app.security.csp = { 'default-src': "'self'" } }))), missing)
  assert.deepEqual(uniq(net.checkTauriConf(CONF, confText((c) => { c.app.security.csp = null }))), missing)
  for (const odd of ['null', '[]', '42', '"x"']) {
    assert.deepEqual(uniq(net.checkTauriConf(CONF, odd)), missing, `tauri.conf.json = ${odd}`)
  }
})

test("@RNF-03 T1 passing: today's CSP, unsafe-inline anywhere, empty directives and devCsp are accepted", () => {
  assert.deepEqual(net.checkTauriConf(CONF, confText()), [])
  assert.deepEqual(net.checkTauriConf(CONF, cspConf("default-src 'self'; script-src 'self' 'unsafe-inline'; img-src 'self'")), [])
  assert.deepEqual(net.checkTauriConf(CONF, cspConf(`${CSP}; upgrade-insecure-requests;`)), [])
  // D3: devUrl and devCsp are dev-only and ignored.
  assert.deepEqual(net.checkTauriConf(CONF, confText((c) => { c.app.security.devCsp = 'default-src *' })), [])
})

test('@RNF-03 Scenario: Tightening the surface is not a violation (CSP without data:)', () => {
  const tightened = "default-src 'self'; img-src 'self'; style-src 'self' 'unsafe-inline'; connect-src ipc: http://ipc.localhost"
  assert.deepEqual(net.checkTauriConf(CONF, cspConf(tightened)), [])
  assert.deepEqual(net.checkTauriConf(CONF, cspConf("default-src 'self'")), [])
})

// --- T2 Configuration ------------------------------------------------------------------------------

const T2_ROWS = [
  ['plugins', (c) => { c.plugins = { updater: { endpoints: ['https://example.com/latest.json'] } } }],
  ['bundle.createUpdaterArtifacts', (c) => { c.bundle.createUpdaterArtifacts = true }],
  ['app.windows[0].url', (c) => { c.app.windows[0].url = 'https://example.com' }],
  ['app.security.capabilities', (c) => { c.app.security.capabilities = [{ identifier: 'inline', windows: ['main'], permissions: ['core:default'] }] }],
]
for (const [jsonPath, mutate] of T2_ROWS) {
  test(`@RNF-03 Scenario Outline: Tauri configuration that could reach the network is refused [${jsonPath}]`, () => {
    assert.deepEqual(uniq(net.checkTauriConf(CONF, confText(mutate))), [line(CONF, 'T2', `'${jsonPath}' not allowed`)])
  })
}

test('@RNF-03 T2: the window index is written in the path and any scheme counts', () => {
  const text = confText((c) => {
    c.app.windows.push({ title: 'Second', url: 'tauri://localhost/other' })
  })
  assert.deepEqual(uniq(net.checkTauriConf(CONF, text)), [line(CONF, 'T2', "'app.windows[1].url' not allowed")])
})

test('@RNF-03 T2 passing: a bare relative window url is allowed', () => {
  assert.deepEqual(net.checkTauriConf(CONF, confText((c) => { c.app.windows[0].url = 'index.html' })), [])
})

// --- T3 Capabilities -------------------------------------------------------------------------------

for (const permission of ['http:default', 'shell:allow-open', 'opener:default', 'core:webview:allow-create-webview-window']) {
  test(`@RNF-03 Scenario Outline: A capability outside the allow-list is refused [${permission}]`, () => {
    assert.deepEqual(uniq(net.checkCapability(CAP, capText([...PERMS, permission]))), [
      line(CAP, 'T3', `permission '${permission}' not in the allow-list`),
    ])
  })
}

test('@RNF-03 T3: a permission object is checked by its identifier', () => {
  const perms = [...PERMS, { identifier: 'fs:allow-read', allow: [{ path: '$HOME/**' }] }]
  assert.deepEqual(uniq(net.checkCapability(CAP, capText(perms))), [line(CAP, 'T3', "permission 'fs:allow-read' not in the allow-list")])
  const allowed = ['core:default', { identifier: 'core:window:allow-close' }]
  assert.deepEqual(net.checkCapability(CAP, capText(allowed)), [])
})

test('@RNF-03 Scenario: A capability open to remote origins is refused', () => {
  const text = capText(PERMS, { remote: { urls: ['https://example.com'] } })
  assert.deepEqual(uniq(net.checkCapability(CAP, text)), [line(CAP, 'T3', 'remote key not allowed')])
})

test("@RNF-03 T3 passing: today's five permissions", () => {
  assert.deepEqual(net.checkCapability(CAP, capText()), [])
})

test('@RNF-03 Scenario: Tightening the surface is not a violation (empty permissions list)', () => {
  assert.deepEqual(net.checkCapability(CAP, capText([])), [])
})

test('@RNF-03 Scenario: Tightening the surface is not a violation (capabilities folder empty or missing)', () => {
  const empty = validTree()
  delete empty['src-tauri/capabilities/default.json']
  empty['src-tauri/capabilities/'] = null
  assert.deepEqual(net.lintNetwork(writeTree(tmp(), empty)), [])
  const missing = validTree()
  delete missing['src-tauri/capabilities/default.json']
  assert.deepEqual(net.lintNetwork(writeTree(tmp(), missing)), [])
})

test('@RNF-03 T3: every capability file is checked and a non-JSON file is refused', () => {
  const files = validTree()
  files['src-tauri/capabilities/extra.json'] = capText(['core:default', 'http:default'], { identifier: 'extra' })
  files['src-tauri/capabilities/extra.toml'] = 'identifier = "extra"\npermissions = ["core:default"]\n'
  assert.deepEqual(uniq(net.lintNetwork(writeTree(tmp(), files))), [
    line('src-tauri/capabilities/extra.json', 'T3', "permission 'http:default' not in the allow-list"),
    line('src-tauri/capabilities/extra.toml', 'T3', 'only JSON capability files are allowed'),
  ])
})

// --- R1 Direct dependencies ------------------------------------------------------------------------

const R1_ROWS = [
  ['crates/fakeflac-fs/Cargo.toml', '[dependencies]', 'reqwest = "0.13"', "dependency 'reqwest' can reach the network"],
  ['crates/fakeflac-store/Cargo.toml', '[dependencies]', 'net = { package = "ureq", version = "3" }', "dependency 'ureq' can reach the network"],
  ['tools/fakeflac-calibrate/Cargo.toml', '[dev-dependencies]', 'hyper = "1"', "dependency 'hyper' can reach the network"],
  ['src-tauri/Cargo.toml', '[dependencies]', 'tauri-plugin-clipboard-manager = "2"', "tauri plugin 'tauri-plugin-clipboard-manager' not in the allow-list"],
  ['src-tauri/Cargo.toml', '[dependencies]', 'tauri = { version = "2", features = ["rustls-tls"] }', "feature 'rustls-tls' of 'tauri' not allowed"],
  ['src-tauri/Cargo.toml', "[target.'cfg(windows)'.dependencies]", 'windows = { version = "0.62", features = ["Win32_Networking_WinHttp"] }', "feature 'Win32_Networking_WinHttp' of 'windows' not allowed"],
]
for (const [file, table, depLine, detail] of R1_ROWS) {
  test(`@RNF-03 Scenario Outline: A direct Rust dependency that can reach the network is refused [${file} ${table} ${depLine}]`, () => {
    assert.deepEqual(uniq(net.checkCargoManifest(file, manifest(table, depLine))), [line(file, 'R1', detail)])
  })
}

test('@RNF-03 R1: every dependency table kind is read', () => {
  const file = 'crates/fakeflac-audio/Cargo.toml'
  for (const table of ['[dependencies]', '[dev-dependencies]', '[build-dependencies]', "[target.'cfg(windows)'.dependencies]", "[target.'cfg(windows)'.dev-dependencies]", '[target.x86_64-pc-windows-msvc.build-dependencies]']) {
    assert.deepEqual(uniq(net.checkCargoManifest(file, manifest(table, 'h2 = "0.4"'))), [line(file, 'R1', "dependency 'h2' can reach the network")], table)
  }
})

test('@RNF-03 R1: the root [workspace.dependencies] table is read', () => {
  const text = ROOT_TOML.replace('serde_json = "1"', 'serde_json = "1"\nnative-tls = "0.2"')
  assert.deepEqual(uniq(net.checkCargoManifest('Cargo.toml', text)), [line('Cargo.toml', 'R1', "dependency 'native-tls' can reach the network")])
})

test('@RNF-03 R1: the dotted form names the dependency', () => {
  const file = 'crates/fakeflac-export/Cargo.toml'
  assert.deepEqual(uniq(net.checkCargoManifest(file, manifest('[dependencies]', 'reqwest.workspace = true'))), [
    line(file, 'R1', "dependency 'reqwest' can reach the network"),
  ])
  assert.deepEqual(uniq(net.checkCargoManifest(file, manifest('[dependencies]', 'openssl.version = "0.10"'))), [
    line(file, 'R1', "dependency 'openssl' can reach the network"),
  ])
})

test('@RNF-03 R1: [….dependencies.<name>] headers, with package and multi-line features', () => {
  const file = 'src-tauri/Cargo.toml'
  const text = `[package]
name = "fakeflac"

[dependencies.net]
package = "tokio-rustls"
version = "0.26"

[dependencies.tauri]
version = "2"
features = [
    "image-png",
    "native-tls-vendored",
]

[target.'cfg(windows)'.dependencies.windows]
version = "0.62"
features = ["System_UserProfile", "Web_Http"]

[dev-dependencies.isahc]
version = "1"
`
  assert.deepEqual(uniq(net.checkCargoManifest(file, text)), uniq([
    line(file, 'R1', "dependency 'tokio-rustls' can reach the network"),
    line(file, 'R1', "feature 'native-tls-vendored' of 'tauri' not allowed"),
    line(file, 'R1', "feature 'Web_Http' of 'windows' not allowed"),
    line(file, 'R1', "dependency 'isahc' can reach the network"),
  ]))
})

test('@RNF-03 R1: renamed packages are judged by their real name', () => {
  const file = 'src-tauri/Cargo.toml'
  // A harmless crate under a network name passes; a refused crate under a harmless name does not.
  assert.deepEqual(net.checkCargoManifest(file, manifest('[dependencies]', 'reqwest = { package = "serde_json", version = "1" }')), [])
  assert.deepEqual(uniq(net.checkCargoManifest(file, manifest('[dependencies]', 'dialog = { package = "tauri-plugin-fs", version = "2" }'))), [
    line(file, 'R1', "tauri plugin 'tauri-plugin-fs' not in the allow-list"),
  ])
  assert.deepEqual(uniq(net.checkCargoManifest(file, manifest('[dependencies]', 't = { package = "tauri", version = "2", features = ["native-tls"] }'))), [
    line(file, 'R1', "feature 'native-tls' of 'tauri' not allowed"),
  ])
})

test('@RNF-03 R1: a NET Tauri plugin is refused as a network dependency', () => {
  const file = 'src-tauri/Cargo.toml'
  for (const name of ['tauri-plugin-updater', 'tauri-plugin-http', 'tauri-plugin-shell', 'tauri-plugin-opener']) {
    const out = net.checkCargoManifest(file, manifest('[dependencies]', `${name} = "2"`))
    assert.ok(out.includes(line(file, 'R1', `dependency '${name}' can reach the network`)), `${name}: ${JSON.stringify(out)}`)
  }
})

test('@RNF-03 R1: the windows feature prefixes Networking, Web and Win32_Network are refused', () => {
  const file = 'src-tauri/Cargo.toml'
  const text = manifest("[target.'cfg(windows)'.dependencies]", 'windows = { version = "0.62", features = ["Networking_Sockets", "Win32_NetworkManagement_IpHelper", "Win32_Foundation"] }')
  assert.deepEqual(uniq(net.checkCargoManifest(file, text)), uniq([
    line(file, 'R1', "feature 'Networking_Sockets' of 'windows' not allowed"),
    line(file, 'R1', "feature 'Win32_NetworkManagement_IpHelper' of 'windows' not allowed"),
  ]))
})

test("@RNF-03 R1 passing: today's manifests, comments, features tables and profile tables", () => {
  assert.deepEqual(net.checkCargoManifest('src-tauri/Cargo.toml', SRC_TAURI_TOML), [])
  assert.deepEqual(net.checkCargoManifest('Cargo.toml', ROOT_TOML), [])
  const file = 'crates/fakeflac-audio/Cargo.toml'
  const text = `[package]
name = "fakeflac-audio"

[features]
rustls = []
hyper = ["rustls"]

[dependencies]
# reqwest = "0.12"  (commented out)
symphonia = { version = "0.5", features = ["mp3"] } # not hyper
windows = { version = "0.62", features = ["System_UserProfile", "Win32_Foundation"] }

[profile.dev.package.reqwest]
opt-level = 3
`
  assert.deepEqual(net.checkCargoManifest(file, text), [])
})

test('@RNF-03 R1: lintNetwork reads every workspace member and the root manifest', () => {
  const files = validTree()
  files['Cargo.toml'] = ROOT_TOML.replace('serde_json = "1"', 'serde_json = "1"\nreqwest = "0.12"')
  files['tools/fakeflac-calibrate/Cargo.toml'] = manifest('[dev-dependencies]', 'hyper = "1"')
  files['crates/fakeflac-fs/Cargo.toml'] = manifest('[dependencies]', 'curl = "0.4"')
  files['src-tauri/Cargo.toml'] = SRC_TAURI_TOML.replace('tauri-plugin-dialog = "2.7.3"', 'tauri-plugin-dialog = "2.7.3"\ntauri-plugin-store = "2"')
  assert.deepEqual(uniq(net.lintNetwork(writeTree(tmp(), files))), uniq([
    line('Cargo.toml', 'R1', "dependency 'reqwest' can reach the network"),
    line('tools/fakeflac-calibrate/Cargo.toml', 'R1', "dependency 'hyper' can reach the network"),
    line('crates/fakeflac-fs/Cargo.toml', 'R1', "dependency 'curl' can reach the network"),
    line('src-tauri/Cargo.toml', 'R1', "tauri plugin 'tauri-plugin-store' not in the allow-list"),
  ]))
})

// --- R2 Lockfile -----------------------------------------------------------------------------------

test('@RNF-03 Scenario: A network crate in Cargo.lock is refused, and tauri\'s mobile-only crates are not', () => {
  const text = lockText(lockPkg('rustls', '0.23.0', ['ring']), lockPkg('ring', '0.17.0'))
  assert.deepEqual(uniq(net.checkCargoLock('Cargo.lock', text)), [line('Cargo.lock', 'R2', "Cargo.lock contains 'rustls'")])
  assert.deepEqual(net.checkCargoLock('Cargo.lock', TODAY_LOCK), [])
})

test('@RNF-03 R2: only [[package]] names count, and every NET name but the three is refused', () => {
  // A dependency entry that names h2 is not a package entry.
  assert.deepEqual(net.checkCargoLock('Cargo.lock', lockText(lockPkg('foo', '1.0.0', ['h2 0.4.0']))), [])
  const refused = SPEC_NET.filter((n) => !['reqwest', 'hyper', 'hyper-util'].includes(n))
  const text = lockText(...refused.map((n) => lockPkg(n)))
  assert.deepEqual(uniq(net.checkCargoLock('Cargo.lock', text)), uniq(refused.map((n) => line('Cargo.lock', 'R2', `Cargo.lock contains '${n}'`))))
})

test('@RNF-03 R2: CRLF line endings are read', () => {
  const text = lockText(lockPkg('openssl-sys', '0.9.0')).replace(/\n/g, '\r\n')
  assert.deepEqual(uniq(net.checkCargoLock('Cargo.lock', text)), [line('Cargo.lock', 'R2', "Cargo.lock contains 'openssl-sys'")])
})

test('@RNF-04 R2: a missing Cargo.lock is one violation', () => {
  assert.deepEqual(net.checkCargoLock('Cargo.lock', null), [line('Cargo.lock', 'R2', 'missing')])
  const files = validTree()
  delete files['Cargo.lock']
  assert.deepEqual(net.lintNetwork(writeTree(tmp(), files)), [line('Cargo.lock', 'R2', 'missing')])
})

// --- R4 Sources ------------------------------------------------------------------------------------

const R4_ROWS = [
  ['crates/fakeflac-store/src/lib.rs', 'std::net', 'use std::net;\n'],
  ['src-tauri/src/run.rs', 'UdpSocket', 'fn f() { let _s = UdpSocket::bind; }\n'],
  ['crates/fakeflac-fs/src/lib.rs', 'TcpStream', 'pub type S = TcpStream;\n'],
]
for (const [file, token, text] of R4_ROWS) {
  test(`@RNF-03 Scenario Outline: A socket API in shipped Rust source is refused [${file} ${token}]`, () => {
    assert.deepEqual(uniq(net.checkRustSource(file, text)), [line(file, 'R4', `'${token}' in shipped source`)])
  })
}

test('@RNF-03 R4: TcpListener and ToSocketAddrs are refused, and comments count', () => {
  const file = 'crates/fakeflac-core/src/lib.rs'
  assert.deepEqual(uniq(net.checkRustSource(file, '// a TcpListener would go here\n/* ToSocketAddrs */\n')), uniq([
    line(file, 'R4', "'TcpListener' in shipped source"),
    line(file, 'R4', "'ToSocketAddrs' in shipped source"),
  ]))
})

test('@RNF-03 R4 passing: tokens match as whole words only', () => {
  const file = 'crates/fakeflac-core/src/lib.rs'
  const text = 'use std::fs;\nuse std::network_free;\nstruct MyTcpStreamLike;\nfn udp_socket_count() {}\nconst URL: &str = "https://example.com"; // not R4\n'
  assert.deepEqual(net.checkRustSource(file, text), [])
})

test('@RNF-03 Scenario: Rust test files are not scanned for socket APIs', () => {
  const files = validTree()
  files['crates/fakeflac-audio/Cargo.toml'] = manifest('[dependencies]', 'symphonia = "0.5"')
  files['crates/fakeflac-audio/src/lib.rs'] = 'pub fn decode() {}\n'
  files['crates/fakeflac-audio/tests/common/aac_tables.rs'] =
    '// Tables from https://example.com/aac and http://example.org/spec\n// TcpStream std::net\npub const T: [u8; 1] = [0];\n'
  files['src-tauri/tests/perf.rs'] = 'use std::net::TcpListener;\n'
  files['tools/fakeflac-calibrate/src/main.rs'] = 'use std::net::UdpSocket;\nfn main() {}\n'
  files['crates/fakeflac-core/src/nested/deep.rs'] = 'use std::net::TcpStream;\n'
  const out = net.lintNetwork(writeTree(tmp(), files))
  assert.deepEqual(uniq(out), uniq([
    line('crates/fakeflac-core/src/nested/deep.rs', 'R4', "'std::net' in shipped source"),
    line('crates/fakeflac-core/src/nested/deep.rs', 'R4', "'TcpStream' in shipped source"),
  ]))
})

// --- F1 Frontend sources ---------------------------------------------------------------------------

test('@RNF-03 Scenario Outline: A network call or remote URL in the frontend is refused [src/lib/api/extra.ts fetch URL]', () => {
  const file = 'src/lib/api/extra.ts'
  const out = uniq(net.checkFrontendSource(file, 'export const a = () => fetch("https://example.com/a")\n'))
  const url = line(file, 'F1', "URL 'https://example.com/a'")
  assert.ok(out.includes(url), JSON.stringify(out))
  // The only other line allowed is the fetch API line (privacy.md 3.3 (c)).
  for (const l of out) {
    if (l !== url) assert.match(l, /^src\/lib\/api\/extra\.ts: RNF-03 F1: 'fetch\(?' not allowed$/)
  }
})

const F1_ROWS = [
  ['src/lib/pages/Extra.svelte', '<script lang="ts">\n  const s = new WebSocket(url)\n</script>\n', "'WebSocket' not allowed"],
  ['src/lib/pages/Extra.svelte', '<script lang="ts">\n  const e = new EventSource("/events")\n</script>\n', "'EventSource' not allowed"],
  ['src/lib/pages/Extra.svelte', '<script lang="ts">\n  navigator.sendBeacon(path, body)\n</script>\n', "'sendBeacon' not allowed"],
  ['index.html', '<!doctype html>\n<html><head><link rel="stylesheet" href="https://fonts.googleapis.com/css2"></head></html>\n', "URL 'https://fonts.googleapis.com/css2'"],
  ['src/lib/tokens/tokens.css', '@import url(//cdn.example.com/a.css);\n:root { --x: 1px; }\n', "URL '//cdn.example.com/a.css'"],
]
for (const [file, text, detail] of F1_ROWS) {
  test(`@RNF-03 Scenario Outline: A network call or remote URL in the frontend is refused [${file} ${detail}]`, () => {
    assert.deepEqual(uniq(net.checkFrontendSource(file, text)), [line(file, 'F1', detail)])
  })
}

test('@RNF-03 F1: every URL scheme of 3.3 (a) is refused', () => {
  const file = 'src/lib/model.ts'
  const text = [
    'const a = "http://localhost:1420/x"',
    "const b = 'wss://example.com/socket'",
    'const c = `ws://10.0.0.1:80/`',
    'const d = "ftp://files.example.com/a.txt"',
    "const e = 'http://www.w3.org/1999/xlink'",
  ].join('\n')
  assert.deepEqual(uniq(net.checkFrontendSource(file, text)), uniq([
    line(file, 'F1', "URL 'http://localhost:1420/x'"),
    line(file, 'F1', "URL 'wss://example.com/socket'"),
    line(file, 'F1', "URL 'ws://10.0.0.1:80/'"),
    line(file, 'F1', "URL 'ftp://files.example.com/a.txt'"),
    line(file, 'F1', "URL 'http://www.w3.org/1999/xlink'"),
  ]))
})

test('@RNF-03 F1: protocol-relative URLs in src=, href=, action=, url( and @import are refused', () => {
  const file = 'src/lib/pages/Page.svelte'
  const text = [
    '<img src="//cdn.example.com/a.png" alt="" />',
    "<a href='//example.com/b'>b</a>",
    '<form action="//example.com/c"></form>',
    "<style>\n  .x { background: url('//cdn.example.com/d.png'); }\n  @import \"//cdn.example.com/e.css\";\n</style>",
  ].join('\n')
  assert.deepEqual(uniq(net.checkFrontendSource(file, text)), uniq([
    line(file, 'F1', "URL '//cdn.example.com/a.png'"),
    line(file, 'F1', "URL '//example.com/b'"),
    line(file, 'F1', "URL '//example.com/c'"),
    line(file, 'F1', "URL '//cdn.example.com/d.png'"),
    line(file, 'F1', "URL '//cdn.example.com/e.css'"),
  ]))
})

test('@RNF-03 F1: XMLHttpRequest, RTCPeerConnection and window.open( are refused', () => {
  const file = 'src/lib/stores/extra.ts'
  assert.deepEqual(uniq(net.checkFrontendSource(file, 'const x = new XMLHttpRequest()\n')), [line(file, 'F1', "'XMLHttpRequest' not allowed")])
  assert.deepEqual(uniq(net.checkFrontendSource(file, 'const p = new RTCPeerConnection()\n')), [line(file, 'F1', "'RTCPeerConnection' not allowed")])
  const open = uniq(net.checkFrontendSource(file, "window.open('/help')\n"))
  assert.equal(open.length, 1, JSON.stringify(open))
  assert.match(open[0], /^src\/lib\/stores\/extra\.ts: RNF-03 F1: 'window\.open\(?' not allowed$/)
  const fetchOnly = uniq(net.checkFrontendSource(file, 'await fetch(path)\n'))
  assert.equal(fetchOnly.length, 1, JSON.stringify(fetchOnly))
  assert.match(fetchOnly[0], /^src\/lib\/stores\/extra\.ts: RNF-03 F1: 'fetch\(?' not allowed$/)
})

test('@RNF-03 F1 passing: whole words only, comments and local paths', () => {
  const file = 'src/lib/stores/extra.ts'
  const text = [
    'prefetch(rows) // see the notes',
    'const sockets = mySendBeaconLike + WebSocketish',
    "import x from '/src/lib/model'",
    'const half = a // b',
    '<img src="/icons/a.png" />',
    'refetch(x); popup.open(y)',
  ].join('\n')
  assert.deepEqual(net.checkFrontendSource(file, text), [])
})

test('@RNF-03 Scenario: The SVG namespace, tests and stories are not violations (namespace)', () => {
  assert.deepEqual(net.checkFrontendSource('src/lib/components/atoms/Icon.svelte', ICON_SVELTE), [])
  const ns = "const el = document.createElementNS('http://www.w3.org/2000/svg', 'svg')\n"
  assert.deepEqual(net.checkFrontendSource('src/lib/format-evidence.ts', ns), [])
})

test('@RNF-03 Scenario: The SVG namespace, tests and stories are not violations (scan scope)', () => {
  const files = validTree()
  const bad = 'export const r = fetch("https://example.com")\n'
  files['src/lib/api/extra.test.ts'] = bad
  files['src/App.test.ts'] = bad
  files['src/lib/components/atoms/Icon.stories.svelte'] = `<a href="https://example.com">x</a>\n`
  files['src/lib/components/atoms/Icon.stories.ts'] = bad
  files['src/lib/testing/helpers.ts'] = bad
  files['src/lib/tests/setup.ts'] = bad
  files['src/lib/data.json'] = '{ "u": "https://example.com" }\n'
  files['src/CLAUDE.md'] = 'See https://example.com\n'
  files['docs/notes.html'] = '<a href="https://example.com">x</a>\n'
  assert.deepEqual(net.lintNetwork(writeTree(tmp(), files)), [])
})

test('@RNF-03 F1: lintNetwork scans index.html and the .ts, .js, .svelte, .css and .html files of src/', () => {
  const files = validTree()
  files['index.html'] = INDEX_HTML.replace('</head>', '<script src="https://cdn.example.com/x.js"></script></head>')
  files['src/lib/a.ts'] = 'const u = "https://example.com/ts"\n'
  files['src/lib/b.js'] = 'const u = "https://example.com/js"\n'
  files['src/lib/pages/C.svelte'] = '<a href="https://example.com/svelte">c</a>\n'
  files['src/lib/tokens/d.css'] = '.d { background: url(https://example.com/css); }\n'
  files['src/lib/e.html'] = '<a href="https://example.com/html">e</a>\n'
  assert.deepEqual(uniq(net.lintNetwork(writeTree(tmp(), files))), uniq([
    line('index.html', 'F1', "URL 'https://cdn.example.com/x.js'"),
    line('src/lib/a.ts', 'F1', "URL 'https://example.com/ts'"),
    line('src/lib/b.js', 'F1', "URL 'https://example.com/js'"),
    line('src/lib/pages/C.svelte', 'F1', "URL 'https://example.com/svelte'"),
    line('src/lib/tokens/d.css', 'F1', "URL 'https://example.com/css'"),
    line('src/lib/e.html', 'F1', "URL 'https://example.com/html'"),
  ]))
})

// --- F2 Packages -----------------------------------------------------------------------------------

test('@RNF-03 Scenario: A frontend Tauri plugin package is refused', () => {
  const text = PACKAGE_JSON(undefined, { '@tauri-apps/cli': '^2', '@tauri-apps/plugin-http': '^2' })
  assert.deepEqual(uniq(net.checkPackageJson('package.json', text)), [line('package.json', 'F2', "package '@tauri-apps/plugin-http' not allowed")])
})

test('@RNF-03 F2: a plugin in dependencies is refused too, the dialog plugin included', () => {
  const text = PACKAGE_JSON({ '@tauri-apps/api': '^2', '@tauri-apps/plugin-dialog': '^2' })
  assert.deepEqual(uniq(net.checkPackageJson('package.json', text)), [line('package.json', 'F2', "package '@tauri-apps/plugin-dialog' not allowed")])
})

test("@RNF-03 F2 passing: today's packages, @tauri-apps/api and @tauri-apps/cli", () => {
  assert.deepEqual(net.checkPackageJson('package.json', PACKAGE_JSON()), [])
  assert.deepEqual(net.checkPackageJson('package.json', '{ "name": "x" }'), [])
})

// --- Unreadable files (@RNF-04) --------------------------------------------------------------------

test('@RNF-04 privacy.md 3: the checkers turn missing and invalid files into one violation', () => {
  assert.deepEqual(net.checkTauriConf(CONF, null), [line(CONF, 'T1', 'missing')])
  assert.deepEqual(net.checkTauriConf(CONF, '{ "app": { "security": { "csp": '), [line(CONF, 'T1', 'invalid JSON')])
  assert.deepEqual(net.checkCapability(CAP, '{ "permissions": [ "core:default", }'), [line(CAP, 'T3', 'invalid JSON')])
  assert.deepEqual(net.checkPackageJson('package.json', '{ "devDependencies": '), [line('package.json', 'F2', 'invalid JSON')])
  assert.deepEqual(net.checkPackageJson('package.json', null), [line('package.json', 'F2', 'missing')])
})

test('@RNF-04 privacy.md 3: odd but valid JSON shapes never throw', () => {
  for (const text of ['null', '[]', '1', '{ "app": "x" }', '{ "app": { "windows": "x", "security": [] } }', '{ "bundle": null }']) {
    assert.ok(Array.isArray(net.checkTauriConf(CONF, text)), text)
  }
  for (const text of ['null', '[]', '{ "permissions": "core:default" }', '{ "permissions": [1, null, {}] }']) {
    assert.ok(Array.isArray(net.checkCapability(CAP, text)), text)
  }
  for (const text of ['null', '[]', '{ "dependencies": [] }', '{ "devDependencies": "x" }']) {
    assert.ok(Array.isArray(net.checkPackageJson('package.json', text)), text)
  }
})

// Fixture extra violations that must still be reported when a configuration file is unreadable.
const OTHER_RULES = {
  'src/lib/pages/Extra.svelte': '<script lang="ts">\n  const s = new WebSocket(url)\n</script>\n',
  'crates/fakeflac-fs/Cargo.toml': manifest('[dependencies]', 'reqwest = "0.13"'),
}
const OTHER_LINES = [
  ' - ' + line('src/lib/pages/Extra.svelte', 'F1', "'WebSocket' not allowed"),
  ' - ' + line('crates/fakeflac-fs/Cargo.toml', 'R1', "dependency 'reqwest' can reach the network"),
]

const RNF04_ROWS = [
  ['src-tauri/tauri.conf.json', 'T1', 'invalid JSON', (f) => { f['src-tauri/tauri.conf.json'] = '{ "app": { "windows": [ }' }],
  ['src-tauri/tauri.conf.json', 'T1', 'missing', (f) => { delete f['src-tauri/tauri.conf.json'] }],
  ['src-tauri/capabilities/default.json', 'T3', 'invalid JSON', (f) => { f['src-tauri/capabilities/default.json'] = '{ "permissions": [ "core:default" ' }],
  ['package.json', 'F2', 'invalid JSON', (f) => { f['package.json'] = '{ "name": "fakeflac", ' }],
]
for (const [file, rule, state, mutate] of RNF04_ROWS) {
  test(`@RNF-04 Scenario Outline: An unreadable configuration file is a violation, not a crash [${file} ${state}]`, () => {
    const files = { ...validTree(), ...OTHER_RULES }
    mutate(files)
    const r = runLintArch(files)
    const lines = assertCleanFailure(r)
    assert.deepEqual(uniq(lines.slice(1)), uniq([' - ' + line(file, rule, state), ...OTHER_LINES]))
  })
}

// --- lint-arch integration (privacy.md 4) ----------------------------------------------------------

test('@RNF-03 privacy.md 4: lint-arch prints the RNF-03 lines under its header and exits 1', () => {
  const files = validTree()
  files['src-tauri/tauri.conf.json'] = cspConf("default-src 'self'; connect-src ipc: http://ipc.localhost https://example.com")
  files['package.json'] = PACKAGE_JSON(undefined, { '@tauri-apps/plugin-http': '^2' })
  const lines = assertCleanFailure(runLintArch(files))
  assert.deepEqual(uniq(lines.slice(1)), uniq([
    ' - ' + line(CONF, 'T1', "CSP source 'https://example.com' not allowed in connect-src"),
    ' - ' + line('package.json', 'F2', "package '@tauri-apps/plugin-http' not allowed"),
  ]))
})

test('@RNF-03 privacy.md 4: lint-arch passes a tree that meets every rule', () => {
  const r = runLintArch(validTree())
  assert.equal(r.status, 0, r.stderr)
  assert.match(r.stdout, /^lint-arch: ok\r?\n?$/)
})
