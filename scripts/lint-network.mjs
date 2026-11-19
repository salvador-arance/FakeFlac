// Privacy lint (RNF-03, RNF-04): the app has no path to the Internet.
// Contract: specs/contracts/privacy.md section 3 (rules T1-T3, R1, R2, R4, F1, F2). R3 lives in
// deny.toml ([bans] deny) and is checked by `cargo deny check`.
// Node built-ins only: this module is copied alone next to lint-arch.mjs by its tests, and the CI
// `specs` job has no `npm ci`.
//
// The checkers are pure: they take a path relative to the repository root ('/' separators) and the
// file's text (null when the file is missing), never touch the file system and never throw. Each
// returns lines of the form `<relPath>: RNF-03 <rule>: <detail>` (lint-arch adds the ' - ').
// lintNetwork(root) reads the repository read-only and runs every checker on the files of its rule.
import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'

// privacy.md 3.2, normative. deny.toml [bans] deny holds the same names (P3).
export const NET = [
  'reqwest', 'hyper', 'hyper-util', 'h2', 'h3', 'quinn', 'ureq', 'curl', 'curl-sys', 'isahc',
  'attohttpc', 'surf', 'awc', 'tungstenite', 'tokio-tungstenite', 'async-tungstenite', 'rustls',
  'native-tls', 'openssl', 'openssl-sys', 'hyper-rustls', 'hyper-tls', 'tokio-rustls',
  'tokio-native-tls', 'tauri-plugin-updater', 'tauri-plugin-http', 'tauri-plugin-websocket',
  'tauri-plugin-upload', 'tauri-plugin-shell', 'tauri-plugin-opener',
]

// R2: tauri's mobile-only dependencies, present in today's lock (privacy.md 2).
const LOCK_TOLERATED = ['reqwest', 'hyper', 'hyper-util']

// T1
const CSP_SOURCES = ["'self'", "'unsafe-inline'", 'data:', 'ipc:', 'http://ipc.localhost']
const CSP_ONLY_IN = { 'ipc:': 'connect-src', 'http://ipc.localhost': 'connect-src', 'data:': 'img-src' }

// T3
const PERMISSIONS = [
  'core:default',
  'core:window:allow-minimize',
  'core:window:allow-toggle-maximize',
  'core:window:allow-close',
  'core:window:allow-start-dragging',
]

// R1
const TAURI_PLUGINS = ['tauri-plugin-dialog']
const TAURI_TLS_FEATURES = ['native-tls', 'native-tls-vendored', 'rustls-tls']
const WINDOWS_FEATURE_PREFIXES = ['Networking', 'Web', 'Win32_Network']
const DEP_TABLES = ['dependencies', 'dev-dependencies', 'build-dependencies', 'dev_dependencies', 'build_dependencies']

// R4
const RUST_TOKENS = ['std::net', 'TcpStream', 'TcpListener', 'UdpSocket', 'ToSocketAddrs']

// F1
const ALLOWED_URLS = ['http://www.w3.org/2000/svg']
const FRONTEND_APIS = [
  ['fetch', /\bfetch\s*\(/],
  ['XMLHttpRequest', /\bXMLHttpRequest\b/],
  ['WebSocket', /\bWebSocket\b/],
  ['EventSource', /\bEventSource\b/],
  ['sendBeacon', /\bsendBeacon\b/],
  ['RTCPeerConnection', /\bRTCPeerConnection\b/],
  ['window.open', /\bwindow\s*\.\s*open\s*\(/],
]
const FRONTEND_EXTENSIONS = ['.ts', '.js', '.svelte', '.css', '.html']

// ---- Helpers --------------------------------------------------------------------------------------

const isObj = (x) => x !== null && typeof x === 'object' && !Array.isArray(x)
const has = (o, k) => isObj(o) && Object.prototype.hasOwnProperty.call(o, k)
const fmt = (relPath, rule) => (detail) => `${relPath}: RNF-03 ${rule}: ${detail}`
const unique = (lines) => [...new Set(lines)]

// JSON.parse that reports failure instead of throwing.
const parseJson = (text) => {
  try {
    return { ok: true, value: JSON.parse(text.replace(/^﻿/, '')) }
  } catch {
    return { ok: false, value: undefined }
  }
}

// ---- T1, T2: tauri.conf.json ------------------------------------------------------------------------

export function checkTauriConf(relPath, text) {
  const t1 = fmt(relPath, 'T1')
  const t2 = fmt(relPath, 'T2')
  if (text === null || text === undefined) return [t1('missing')]
  const parsed = parseJson(String(text))
  if (!parsed.ok) return [t1('invalid JSON')]
  const conf = parsed.value
  const out = []

  // T1 CSP
  const app = has(conf, 'app') ? conf.app : undefined
  const security = has(app, 'security') ? app.security : undefined
  const csp = has(security, 'csp') ? security.csp : undefined
  if (typeof csp !== 'string') {
    out.push(t1('CSP missing'))
  } else {
    const directives = csp
      .split(';')
      .map((d) => d.trim().split(/\s+/).filter((tok) => tok !== ''))
      .filter((d) => d.length > 0)
    const defaultSrc = directives.find((d) => d[0].toLowerCase() === 'default-src')
    if (!defaultSrc || defaultSrc.length !== 2 || defaultSrc[1] !== "'self'") {
      out.push(t1("default-src must be 'self'"))
    }
    for (const [name, ...sources] of directives) {
      const directive = name.toLowerCase()
      if (directive === 'default-src') continue
      for (const source of sources) {
        const onlyIn = CSP_ONLY_IN[source]
        if (!CSP_SOURCES.includes(source) || (onlyIn !== undefined && onlyIn !== directive)) {
          out.push(t1(`CSP source '${source}' not allowed in ${name}`))
        }
      }
    }
  }

  // T2 configuration
  if (has(conf, 'plugins')) out.push(t2("'plugins' not allowed"))
  const bundle = has(conf, 'bundle') ? conf.bundle : undefined
  if (has(bundle, 'createUpdaterArtifacts')) out.push(t2("'bundle.createUpdaterArtifacts' not allowed"))
  const windows = has(app, 'windows') ? app.windows : undefined
  if (Array.isArray(windows)) {
    windows.forEach((w, i) => {
      if (has(w, 'url') && typeof w.url === 'string' && /^[A-Za-z][A-Za-z0-9+.-]*:/.test(w.url)) {
        out.push(t2(`'app.windows[${i}].url' not allowed`))
      }
    })
  }
  if (has(security, 'capabilities')) out.push(t2("'app.security.capabilities' not allowed"))

  return unique(out)
}

// ---- T3: one capability file ------------------------------------------------------------------------

export function checkCapability(relPath, text) {
  const t3 = fmt(relPath, 'T3')
  if (text === null || text === undefined) return [t3('missing')]
  const parsed = parseJson(String(text))
  if (!parsed.ok) return [t3('invalid JSON')]
  const doc = parsed.value
  // A capability file holds one capability, a list of them, or { "capabilities": [...] }.
  let capabilities = []
  if (Array.isArray(doc)) capabilities = doc.filter(isObj)
  else if (has(doc, 'capabilities') && Array.isArray(doc.capabilities)) capabilities = [doc, ...doc.capabilities.filter(isObj)]
  else if (isObj(doc)) capabilities = [doc]

  const out = []
  for (const cap of capabilities) {
    if (has(cap, 'remote')) out.push(t3('remote key not allowed'))
    const permissions = has(cap, 'permissions') && Array.isArray(cap.permissions) ? cap.permissions : []
    for (const p of permissions) {
      const id = typeof p === 'string' ? p : has(p, 'identifier') && typeof p.identifier === 'string' ? p.identifier : null
      if (id !== null && !PERMISSIONS.includes(id)) out.push(t3(`permission '${id}' not in the allow-list`))
    }
  }
  return unique(out)
}

// ---- Minimal TOML reader (enough for Cargo.toml) ------------------------------------------------------
// Returns the key/value entries with the table they belong to. On a syntax error it stops and
// returns the entries read so far; it never throws.

function parseToml(input) {
  const s = String(input).replace(/^﻿/, '')
  const entries = []
  let i = 0
  let table = []
  let arrayTable = false

  const fail = () => {
    throw new SyntaxError(`TOML syntax error at ${i}`)
  }
  const ws = () => {
    while (i < s.length && (s[i] === ' ' || s[i] === '\t')) i++
  }
  const wsnl = () => {
    for (;;) {
      ws()
      if (s[i] === '#') while (i < s.length && s[i] !== '\n') i++
      if (s[i] === '\n' || s[i] === '\r') {
        i++
        continue
      }
      return
    }
  }
  const basicString = () => {
    if (s.startsWith('"""', i)) {
      i += 3
      let out = ''
      while (i < s.length && !s.startsWith('"""', i)) {
        if (s[i] === '\\') i++
        out += s[i] ?? ''
        i++
      }
      if (i >= s.length) fail()
      i += 3
      return out
    }
    i++
    let out = ''
    while (i < s.length && s[i] !== '"') {
      if (s[i] === '\n') fail()
      if (s[i] === '\\') {
        i++
        const c = s[i]
        if (c === 'u' || c === 'U') {
          const n = c === 'u' ? 4 : 8
          out += String.fromCodePoint(parseInt(s.slice(i + 1, i + 1 + n), 16) || 0)
          i += n + 1
          continue
        }
        out += { n: '\n', t: '\t', r: '\r', b: '\b', f: '\f' }[c] ?? c ?? ''
        i++
        continue
      }
      out += s[i]
      i++
    }
    if (i >= s.length) fail()
    i++
    return out
  }
  const literalString = () => {
    const quote = s.startsWith("'''", i) ? "'''" : "'"
    i += quote.length
    const end = s.indexOf(quote, i)
    if (end < 0) fail()
    const out = s.slice(i, end)
    i = end + quote.length
    return out
  }
  const key = () => {
    const parts = []
    for (;;) {
      ws()
      if (s[i] === '"') parts.push(basicString())
      else if (s[i] === "'") parts.push(literalString())
      else {
        const m = /^[A-Za-z0-9_-]+/.exec(s.slice(i, i + 256))
        if (!m) fail()
        parts.push(m[0])
        i += m[0].length
      }
      ws()
      if (s[i] !== '.') return parts
      i++
    }
  }
  const setPath = (obj, parts, v) => {
    let o = obj
    for (const p of parts.slice(0, -1)) {
      if (!isObj(o[p])) o[p] = {}
      o = o[p]
    }
    o[parts[parts.length - 1]] = v
  }
  const value = () => {
    ws()
    const c = s[i]
    if (c === '"') return basicString()
    if (c === "'") return literalString()
    if (c === '[') {
      i++
      const arr = []
      for (;;) {
        wsnl()
        if (s[i] === ']') {
          i++
          return arr
        }
        arr.push(value())
        wsnl()
        if (s[i] === ',') {
          i++
          continue
        }
        if (s[i] === ']') {
          i++
          return arr
        }
        fail()
      }
    }
    if (c === '{') {
      i++
      const obj = {}
      for (;;) {
        wsnl()
        if (s[i] === '}') {
          i++
          return obj
        }
        const k = key()
        if (s[i] !== '=') fail()
        i++
        setPath(obj, k, value())
        wsnl()
        if (s[i] === ',') {
          i++
          continue
        }
        if (s[i] === '}') {
          i++
          return obj
        }
        fail()
      }
    }
    const m = /^[^,\]}\r\n#]+/.exec(s.slice(i, i + 256))
    if (!m) fail()
    i += m[0].length
    return m[0].trim()
  }

  try {
    for (;;) {
      wsnl()
      if (i >= s.length) break
      if (s[i] === '[') {
        arrayTable = s[i + 1] === '['
        i += arrayTable ? 2 : 1
        table = key()
        if (s[i] !== ']') fail()
        i++
        if (arrayTable) {
          if (s[i] !== ']') fail()
          i++
        }
      } else {
        const k = key()
        if (s[i] !== '=') fail()
        i++
        entries.push({ table, arrayTable, key: k, value: value() })
      }
      ws()
      if (i < s.length && s[i] !== '#' && s[i] !== '\n' && s[i] !== '\r') fail()
    }
  } catch {
    // Stop at the first syntax error; what was read is still checked.
  }
  return entries
}

// Length of the table-name prefix that designates a dependency table, or -1.
const depTableBase = (table) => {
  if (DEP_TABLES.includes(table[0])) return 1
  if (table[0] === 'target' && table.length >= 3 && DEP_TABLES.includes(table[2])) return 3
  if (table[0] === 'workspace' && table[1] === 'dependencies') return 2
  return -1
}

// ---- R1: one Cargo.toml -------------------------------------------------------------------------------

export function checkCargoManifest(relPath, text) {
  const r1 = fmt(relPath, 'R1')
  if (text === null || text === undefined) return []
  const deps = new Map()
  for (const entry of parseToml(text)) {
    if (entry.arrayTable) continue
    const base = depTableBase(entry.table)
    if (base < 0) continue
    const full = [...entry.table.slice(base), ...entry.key]
    const id = `${entry.table.slice(0, base).join('\u0000')}\u0000${full[0]}`
    if (!deps.has(id)) deps.set(id, { name: full[0], package: undefined, features: [] })
    const dep = deps.get(id)
    const attr = full.slice(1)
    const v = entry.value
    if (attr.length === 0) {
      if (isObj(v)) {
        if (typeof v.package === 'string') dep.package = v.package
        if (Array.isArray(v.features)) dep.features.push(...v.features.filter((f) => typeof f === 'string'))
      }
    } else if (attr.length === 1 && attr[0] === 'package' && typeof v === 'string') {
      dep.package = v
    } else if (attr.length === 1 && attr[0] === 'features' && Array.isArray(v)) {
      dep.features.push(...v.filter((f) => typeof f === 'string'))
    }
  }

  const out = []
  for (const dep of deps.values()) {
    const name = dep.package ?? dep.name
    if (NET.includes(name)) out.push(r1(`dependency '${name}' can reach the network`))
    else if (name.startsWith('tauri-plugin-') && !TAURI_PLUGINS.includes(name)) {
      out.push(r1(`tauri plugin '${name}' not in the allow-list`))
    }
    for (const f of dep.features) {
      const refused =
        (name === 'tauri' && TAURI_TLS_FEATURES.includes(f)) ||
        (name === 'windows' && WINDOWS_FEATURE_PREFIXES.some((p) => f.startsWith(p)))
      if (refused) out.push(r1(`feature '${f}' of '${name}' not allowed`))
    }
  }
  return unique(out)
}

// ---- R2: Cargo.lock -----------------------------------------------------------------------------------

export function checkCargoLock(relPath, text) {
  const r2 = fmt(relPath, 'R2')
  if (text === null || text === undefined) return [r2('missing')]
  const out = []
  let inPackage = false
  for (const raw of String(text).split(/\r?\n/)) {
    const l = raw.trim()
    if (l.startsWith('[')) {
      inPackage = l === '[[package]]'
      continue
    }
    if (!inPackage) continue
    const m = /^name\s*=\s*"([^"]*)"$/.exec(l)
    if (m && NET.includes(m[1]) && !LOCK_TOLERATED.includes(m[1])) out.push(r2(`Cargo.lock contains '${m[1]}'`))
  }
  return unique(out)
}

// ---- R4: one shipped .rs file -------------------------------------------------------------------------

export function checkRustSource(relPath, text) {
  const r4 = fmt(relPath, 'R4')
  if (text === null || text === undefined) return []
  const src = String(text)
  return RUST_TOKENS.filter((t) => new RegExp(`\\b${t.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\b`).test(src)).map(
    (t) => r4(`'${t}' in shipped source`),
  )
}

// ---- F1: one frontend file ----------------------------------------------------------------------------

export function checkFrontendSource(relPath, text) {
  const f1 = fmt(relPath, 'F1')
  if (text === null || text === undefined) return []
  const src = String(text)
  const out = []
  // (a) absolute URLs with a network scheme; the match stops at quotes, brackets and the
  // punctuation that ends a statement or a list (';', ',', '}').
  for (const m of src.matchAll(/(?<![A-Za-z0-9+.-])(?:https?|wss?|ftp):\/\/[^\s'"`<>()\\;,}]*/g)) {
    if (!ALLOWED_URLS.includes(m[0])) out.push(f1(`URL '${m[0]}'`))
  }
  // (b) protocol-relative URLs where the browser would load them.
  const relative = /(?:\b(?:src|href|action)\s*=\s*["']?|@import\s+(?:url\(\s*)?["']?|\burl\(\s*["']?)(\/\/[^\s'"`<>()\\;,}]+)/g
  for (const m of src.matchAll(relative)) out.push(f1(`URL '${m[1]}'`))
  // (c) network APIs, as whole words.
  for (const [api, re] of FRONTEND_APIS) if (re.test(src)) out.push(f1(`'${api}' not allowed`))
  return unique(out)
}

// ---- F2: package.json ---------------------------------------------------------------------------------

export function checkPackageJson(relPath, text) {
  const f2 = fmt(relPath, 'F2')
  if (text === null || text === undefined) return [f2('missing')]
  const parsed = parseJson(String(text))
  if (!parsed.ok) return [f2('invalid JSON')]
  const pkg = parsed.value
  const out = []
  for (const field of ['dependencies', 'devDependencies']) {
    const deps = has(pkg, field) ? pkg[field] : undefined
    if (!isObj(deps)) continue
    for (const name of Object.keys(deps)) {
      if (name.startsWith('@tauri-apps/plugin-')) out.push(f2(`package '${name}' not allowed`))
    }
  }
  return unique(out)
}

// ---- The repository -----------------------------------------------------------------------------------

const readText = (root, rel) => {
  try {
    return readFileSync(join(root, ...rel.split('/')), 'utf8')
  } catch {
    return null
  }
}

// Entries of a folder, sorted; [] when it is missing or unreadable.
const list = (root, rel) => {
  try {
    return readdirSync(join(root, ...rel.split('/')), { withFileTypes: true }).sort((a, b) =>
      a.name < b.name ? -1 : a.name > b.name ? 1 : 0,
    )
  } catch {
    return []
  }
}

// Every file under a folder, as '/'-separated paths relative to the root.
const walk = (root, rel) =>
  list(root, rel).flatMap((e) => {
    const p = `${rel}/${e.name}`
    if (e.isDirectory()) return walk(root, p)
    return e.isFile() ? [p] : []
  })

const subfolders = (root, rel) =>
  list(root, rel)
    .filter((e) => e.isDirectory())
    .map((e) => `${rel}/${e.name}`)

export function lintNetwork(root) {
  const out = []
  // The rule each checker answers for when it cannot run.
  const ruleOf = new Map([
    [checkTauriConf, 'T1'],
    [checkCapability, 'T3'],
    [checkCargoManifest, 'R1'],
    [checkCargoLock, 'R2'],
    [checkRustSource, 'R4'],
    [checkFrontendSource, 'F1'],
    [checkPackageJson, 'F2'],
  ])
  const run = (check, rel) => {
    try {
      out.push(...check(rel, readText(root, rel)))
    } catch {
      // The checkers never throw; if one does, the file fails closed instead of passing unchecked.
      out.push(`${rel}: RNF-03 ${ruleOf.get(check) ?? 'T1'}: could not be checked`)
    }
  }

  // T1, T2
  run(checkTauriConf, 'src-tauri/tauri.conf.json')

  // T3: a missing or empty folder passes.
  for (const rel of walk(root, 'src-tauri/capabilities')) {
    if (rel.toLowerCase().endsWith('.json')) run(checkCapability, rel)
    else out.push(`${rel}: RNF-03 T3: only JSON capability files are allowed`)
  }

  // R1: the root manifest and every workspace member; a missing manifest is skipped.
  const manifests = [
    'Cargo.toml',
    ...subfolders(root, 'crates').map((d) => `${d}/Cargo.toml`),
    'src-tauri/Cargo.toml',
    ...subfolders(root, 'tools').map((d) => `${d}/Cargo.toml`),
  ]
  for (const rel of manifests) run(checkCargoManifest, rel)

  // R2
  run(checkCargoLock, 'Cargo.lock')

  // R4: shipped crates only.
  const rustRoots = [...subfolders(root, 'crates').map((d) => `${d}/src`), 'src-tauri/src']
  for (const dir of rustRoots) {
    for (const rel of walk(root, dir)) if (rel.endsWith('.rs')) run(checkRustSource, rel)
  }

  // F1: index.html and the shipped sources of src/.
  run(checkFrontendSource, 'index.html')
  for (const rel of walk(root, 'src')) {
    const parts = rel.split('/')
    const name = parts[parts.length - 1]
    if (!FRONTEND_EXTENSIONS.some((ext) => name.endsWith(ext))) continue
    if (name.includes('.test.') || name.includes('.stories.')) continue
    if (parts.slice(0, -1).some((p) => p === 'tests' || p === 'testing')) continue
    run(checkFrontendSource, rel)
  }

  // F2
  run(checkPackageJson, 'package.json')

  return out
}
