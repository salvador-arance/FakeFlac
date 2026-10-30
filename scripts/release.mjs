// Release script (RNF-11, RNF-06; specs/contracts/release.md, ADR-0045).
// Checks and bumps the version held by the manifests, checks the release tag and the commit it
// names, collects the installers behind the size gate, writes SHA256SUMS.txt and the release notes,
// and creates the draft release. Node built-ins only. It never changes git state and never builds a
// shell command line: child processes get argument arrays and the `env` that `run` was given.
// Usage: node scripts/release.mjs <command> [arguments]   (commands: see COMMANDS below)
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const PREFIX = 'release: '
const DEFAULT_ROOT = path.join(path.dirname(fileURLToPath(import.meta.url)), '..')
const SUMS_FILE = 'SHA256SUMS.txt'
const BOM = '﻿'
const MAX_SAFE = BigInt(Number.MAX_SAFE_INTEGER)

// --- Arguments (release.md 3.0) -----------------------------------------------------------------

const COMMAND_NAMES = ['version', 'release-check', 'assets', 'draft', 'set-version']
const COMMANDS = {
  version: { options: [], flags: [], required: [], positionals: 0 },
  'release-check': { options: [], flags: [], required: [], positionals: 1 },
  assets: {
    options: ['--bundle', '--out', '--notes', '--max-bytes'],
    flags: [],
    required: ['--bundle', '--out', '--notes', '--max-bytes'],
    positionals: 0,
  },
  draft: { options: ['--assets', '--notes'], flags: ['--print'], required: ['--assets', '--notes'], positionals: 0 },
  'set-version': { options: [], flags: [], required: [], positionals: 1 },
}
const COMMAND_LIST = COMMAND_NAMES.join(', ')

const msg = (s) => PREFIX + s

/** Parses a command line (release.md 3.0). No environment, no I/O. */
export function parseArgs(argv) {
  const result = { command: undefined, positionals: [], options: {}, errors: [] }
  if (argv.length === 0) {
    result.errors.push(msg(`usage: node scripts/release.mjs <command> [arguments]; commands: ${COMMAND_LIST}`))
    return result
  }
  const command = argv[0]
  if (!Object.hasOwn(COMMANDS, command)) {
    result.errors.push(msg(`unknown command "${command}"; commands: ${COMMAND_LIST}`))
    return result
  }
  result.command = command
  const spec = COMMANDS[command]
  const valued = ['--root', ...spec.options]
  const seen = new Set()
  const errors = result.errors

  let i = 1
  while (i < argv.length) {
    const a = argv[i]
    if (a.startsWith('--')) {
      if (spec.flags.includes(a)) {
        if (seen.has(a)) errors.push(msg(`option ${a} is given more than once`))
        else result.options[a] = true
        seen.add(a)
        i += 1
        continue
      }
      if (!valued.includes(a)) {
        // An unknown option takes no value: the next argument is read on its own.
        errors.push(msg(`unknown option ${a} for ${command}`))
        i += 1
        continue
      }
      const next = argv[i + 1]
      const hasValue = next !== undefined && !next.startsWith('--')
      if (seen.has(a)) {
        errors.push(msg(`option ${a} is given more than once`))
        i += hasValue ? 2 : 1
        continue
      }
      seen.add(a)
      if (!hasValue) {
        errors.push(msg(`option ${a} needs a value`))
        i += 1
        continue
      }
      i += 2
      if (next === '') {
        errors.push(msg(`option ${a} has an empty value`))
        continue
      }
      if (a === '--max-bytes' && !validMaxBytes(next)) {
        errors.push(msg(`--max-bytes must be a whole number from 1 to 9007199254740991, not "${next}"`))
        continue
      }
      result.options[a] = next
      continue
    }
    if (result.positionals.length < spec.positionals) result.positionals.push(a)
    else errors.push(msg(`unexpected argument "${a}" for ${command}`))
    i += 1
  }

  const { '--notes': notes, '--out': out } = result.options
  if (command === 'assets' && notes !== undefined && out !== undefined && isInside(notes, out)) {
    errors.push(msg('--notes must not be inside --out'))
  }
  for (const r of spec.required) {
    if (!seen.has(r)) errors.push(msg(`${command} needs ${r}`))
  }
  if (command === 'set-version' && result.positionals.length === 0) {
    errors.push(msg('set-version needs a version such as 1.2.3'))
  }
  return result
}

function validMaxBytes(v) {
  return /^[1-9][0-9]*$/.test(v) && BigInt(v) <= MAX_SAFE
}

// True when `inner` is `outer` or inside it, compared as path.resolve results (case-insensitive on
// Windows, which path.relative does there).
function isInside(inner, outer) {
  const rel = path.relative(path.resolve(outer), path.resolve(inner))
  if (rel === '') return true
  return !(rel === '..' || rel.startsWith(`..${path.sep}`) || path.isAbsolute(rel))
}

// A path as given on the command line, for messages (D9): `\` to `/`, trailing separators removed.
function disp(p) {
  const s = p.replace(/\\/g, '/')
  const trimmed = s.replace(/\/+$/, '')
  return trimmed === '' && s !== '' ? '/' : trimmed
}

// --- Versions (release.md 2.2) ------------------------------------------------------------------

const VERSION_RE = /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/
const PRE_RE = /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)[-+]/

/** null when releasable, else 'pre', 'shape' or 'range' (in that order of classification). */
function classify(v) {
  if (PRE_RE.test(v)) return 'pre'
  const m = VERSION_RE.exec(v)
  if (!m) return 'shape'
  if (BigInt(m[1]) > 255n || BigInt(m[2]) > 255n || BigInt(m[3]) > 65535n) return 'range'
  return null
}

/** The message of release.md 3.1 step 3, or null when `v` is releasable. */
export function versionProblem(v) {
  switch (classify(v)) {
    case 'pre':
      return msg(`version "${v}" has a pre-release or build part; v1 ships no pre-releases (MAJOR.MINOR.PATCH only)`)
    case 'shape':
      return msg(`version "${v}" is not MAJOR.MINOR.PATCH (numbers without leading zeros)`)
    case 'range':
      return msg(`version "${v}" is outside the MSI range (MAJOR and MINOR at most 255, PATCH at most 65535)`)
    default:
      return null
  }
}

/** The message of release.md 3.2 step 2, or null when the tag is `v` and a releasable version. */
export function tagProblem(tag) {
  const rest = tag.startsWith('v') ? tag.slice(1) : null
  const kind = rest === null ? 'shape' : classify(rest)
  if (kind === null) return null
  if (kind === 'pre') {
    return msg(`tag "${tag}" has a pre-release or build part; v1 ships no pre-releases (vMAJOR.MINOR.PATCH only)`)
  }
  return msg(`tag "${tag}" is not v<MAJOR>.<MINOR>.<PATCH>`)
}

// --- Locating the version (release.md 2.1) ------------------------------------------------------

const samePath = (a, b) => a.length === b.length && a.every((x, i) => x === b[i])

/**
 * The key-path scanner of release.md 2.1. `text` must be valid JSON without its BOM. For each target
 * path: { status: 'ok', value, start, end } (the raw span between the quotes), or
 * { status: 'absent' | 'duplicate' | 'not-string' }.
 */
export function locateJson(text, paths) {
  const found = paths.map(() => [])
  let i = 0
  const ws = () => {
    while (i < text.length && (text[i] === ' ' || text[i] === '\t' || text[i] === '\n' || text[i] === '\r')) i++
  }
  const readString = () => {
    const start = i + 1
    i++
    while (i < text.length && text[i] !== '"') i += text[i] === '\\' ? 2 : 1
    const end = i
    i++
    return { start, end, value: JSON.parse(text.slice(start - 1, end + 1)) }
  }
  const record = (at, entry) => {
    for (let k = 0; k < paths.length; k++) if (samePath(paths[k], at)) found[k].push(entry)
  }
  const value = (at) => {
    ws()
    const ch = text[i]
    if (ch === '{') {
      record(at, { string: false })
      i++
      ws()
      if (text[i] === '}') {
        i++
        return
      }
      for (;;) {
        ws()
        const key = readString().value
        ws()
        i++ // ':'
        value([...at, key])
        ws()
        if (text[i] === ',') {
          i++
          continue
        }
        i++ // '}'
        return
      }
    }
    if (ch === '[') {
      record(at, { string: false })
      i++
      ws()
      if (text[i] === ']') {
        i++
        return
      }
      for (let n = 0; ; n++) {
        value([...at, n])
        ws()
        if (text[i] === ',') {
          i++
          continue
        }
        i++ // ']'
        return
      }
    }
    if (ch === '"') {
      const s = readString()
      record(at, { string: true, ...s })
      return
    }
    while (i < text.length && !',}] \t\n\r'.includes(text[i])) i++
    record(at, { string: false })
  }
  value([])
  return found.map((entries) => {
    if (entries.length === 0) return { status: 'absent' }
    if (entries.length > 1) return { status: 'duplicate' }
    const e = entries[0]
    if (!e.string) return { status: 'not-string' }
    return { status: 'ok', value: e.value, start: e.start, end: e.end }
  })
}

// The lines of a text: { content (one trailing CR removed), start (offset of the line in text) }.
function linesOf(text) {
  const out = []
  let s = 0
  for (;;) {
    const e = text.indexOf('\n', s)
    const raw = e === -1 ? text.slice(s) : text.slice(s, e)
    out.push({ content: raw.endsWith('\r') ? raw.slice(0, -1) : raw, start: s })
    if (e === -1) return out
    s = e + 1
  }
}

const firstNonBlank = (line) => line.replace(/^[ \t]+/, '')[0]

/**
 * The line rule for the root Cargo.toml (release.md 2.1): the version of [workspace.package]
 * ({ status, value, start, end }) and its `repository` string (or null).
 */
export function locateCargoToml(text) {
  const lines = linesOf(text)
  const head = lines.findIndex((l) => l.content.replace(/#.*$/, '').replace(/^[ \t]+|[ \t]+$/g, '') === '[workspace.package]')
  const result = { version: { status: 'absent' }, repository: null }
  if (head === -1) return result
  let versionDone = false
  let repositoryDone = false
  for (const l of lines.slice(head + 1)) {
    if (firstNonBlank(l.content) === '[') break
    if (firstNonBlank(l.content) === '#') continue
    if (!versionDone && /^[ \t]*version[ \t]*=/.test(l.content)) {
      versionDone = true
      const m = /^([ \t]*version[ \t]*=[ \t]*")([^"\\]*)"[ \t]*(#.*)?$/.exec(l.content)
      result.version = m
        ? { status: 'ok', value: m[2], start: l.start + m[1].length, end: l.start + m[1].length + m[2].length }
        : { status: 'not-string' }
    }
    if (!repositoryDone) {
      const m = /^[ \t]*repository[ \t]*=[ \t]*"([^"\\]*)"[ \t]*(#.*)?$/.exec(l.content)
      if (m) {
        repositoryDone = true
        result.repository = m[1]
      }
    }
  }
  return result
}

/**
 * The line rule for Cargo.lock (release.md 2.1): one result per workspace package (an entry
 * without a `source` line), in file order: { name (or null), version: { status, value, start, end } }.
 */
export function locateCargoLock(text) {
  const lines = linesOf(text)
  const entries = []
  let current = null
  for (const l of lines) {
    if (l.content === '[[package]]') {
      current = []
      entries.push(current)
      continue
    }
    if (l.content.startsWith('[')) {
      current = null
      continue
    }
    if (current) current.push(l)
  }
  const out = []
  for (const entry of entries) {
    if (entry.some((l) => l.content.startsWith('source = '))) continue
    let name = null
    let version = { status: 'absent' }
    for (const l of entry) {
      const n = name === null ? /^name = "(.*)"$/.exec(l.content) : null
      if (n) name = n[1]
      if (version.status === 'absent') {
        const v = /^(version = ")(.*)"$/.exec(l.content)
        if (v) version = { status: 'ok', value: v[2], start: l.start + v[1].length, end: l.start + v[1].length + v[2].length }
      }
    }
    out.push({ name, version })
  }
  return out
}

/** Replaces each span of `text` with `value` (release.md 3.7 step 3). */
export function replaceSpans(text, spans, value) {
  let out = text
  for (const s of [...spans].sort((a, b) => b.start - a.start)) out = out.slice(0, s.start) + value + out.slice(s.end)
  return out
}

// --- Reading the manifests ----------------------------------------------------------------------

const JSON_FILES = [
  { rel: 'package.json', sources: [['package.json', ['version']]] },
  {
    rel: 'package-lock.json',
    sources: [
      ['package-lock.json', ['version']],
      ['package-lock.json packages[""]', ['packages', '', 'version']],
    ],
  },
  { rel: 'src-tauri/tauri.conf.json', sources: [['src-tauri/tauri.conf.json', ['version']]] },
]
const TAURI = 'src-tauri/tauri.conf.json'
const BAD_NAME_CHAR = /[/\\:*?"<>|\u0000-\u001f\u007f-\u009f]/

// The text of a manifest, or a read error line.
function readFile(root, rel) {
  const file = path.join(root, ...rel.split('/'))
  let st
  try {
    st = fs.statSync(file)
  } catch (e) {
    if (e.code === 'ENOENT' || e.code === 'ENOTDIR') return { error: msg(`missing ${rel}`) }
    return { error: msg(`cannot read ${rel} (${e.code ?? 'EIO'})`) }
  }
  if (!st.isFile()) return { error: msg(`missing ${rel}`) }
  try {
    return { text: fs.readFileSync(file).toString('utf8'), file }
  } catch (e) {
    return { error: msg(`cannot read ${rel} (${e.code ?? 'EIO'})`) }
  }
}

function sourceError(name, loc, key = 'version') {
  switch (loc.status) {
    case 'absent':
      return msg(`${name} has no ${key}`)
    case 'duplicate':
      return msg(`${name} has a duplicate "${key}" key`)
    case 'not-string':
      return msg(`${name} ${key} is not a string`)
    default:
      return loc.value === '' ? msg(`${name} ${key} is empty`) : null
  }
}

/**
 * Reads every source of release.md 2.1 under `root`. Returns { errors, sources: [{ name, rel,
 * value, start, end }], files: { rel: { file, bom, text } }, productName, repository }.
 */
function readManifests(root, { productName: wantName = false } = {}) {
  const errors = []
  const sources = []
  const files = {}
  let productLoc = null
  let repository = null

  for (const { rel, sources: defs } of JSON_FILES) {
    const r = readFile(root, rel)
    if (r.error) {
      errors.push(r.error)
      continue
    }
    const bom = r.text.startsWith(BOM) ? BOM : ''
    const text = r.text.slice(bom.length)
    try {
      JSON.parse(text)
    } catch {
      errors.push(msg(`${rel} is not valid JSON`))
      continue
    }
    files[rel] = { file: r.file, bom, text }
    const paths = defs.map(([, p]) => p)
    if (rel === TAURI) paths.push(['productName'])
    const locs = locateJson(text, paths)
    defs.forEach(([name], k) => {
      const e = sourceError(name, locs[k])
      if (e) errors.push(e)
      else sources.push({ name, rel, value: locs[k].value, start: locs[k].start, end: locs[k].end })
    })
    if (rel === TAURI) productLoc = locs[defs.length]
  }

  const toml = readFile(root, 'Cargo.toml')
  if (toml.error) errors.push(toml.error)
  else {
    files['Cargo.toml'] = { file: toml.file, bom: '', text: toml.text }
    const loc = locateCargoToml(toml.text)
    repository = loc.repository
    const name = 'Cargo.toml [workspace.package]'
    const e = sourceError(name, loc.version)
    if (e) errors.push(e)
    else sources.push({ name, rel: 'Cargo.toml', value: loc.version.value, start: loc.version.start, end: loc.version.end })
  }

  const lock = readFile(root, 'Cargo.lock')
  if (lock.error) errors.push(lock.error)
  else {
    files['Cargo.lock'] = { file: lock.file, bom: '', text: lock.text }
    const packages = locateCargoLock(lock.text)
    const fileErrors = []
    if (packages.length === 0) fileErrors.push(msg('Cargo.lock lists no workspace package'))
    if (packages.some((p) => p.name === null)) fileErrors.push(msg('Cargo.lock has a workspace package without a name'))
    const counts = new Map()
    for (const p of packages) if (p.name !== null) counts.set(p.name, (counts.get(p.name) ?? 0) + 1)
    for (const n of [...counts.keys()].sort()) {
      if (counts.get(n) > 1) fileErrors.push(msg(`Cargo.lock lists the workspace package ${n} more than once`))
    }
    if (fileErrors.length > 0) errors.push(...fileErrors)
    else {
      for (const p of [...packages].sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0))) {
        const name = `Cargo.lock ${p.name}`
        const e = sourceError(name, p.version)
        if (e) errors.push(e)
        else sources.push({ name, rel: 'Cargo.lock', value: p.version.value, start: p.version.start, end: p.version.end })
      }
    }
  }

  let productName = null
  if (wantName && productLoc) {
    const e = sourceError(TAURI, productLoc, 'productName')
    if (e) errors.push(e)
    else if (BAD_NAME_CHAR.test(productLoc.value)) {
      errors.push(msg(`${TAURI} productName ${JSON.stringify(productLoc.value)} has a character not allowed in a file name`))
    } else productName = productLoc.value
  }
  return { errors, sources, files, productName, repository }
}

/**
 * release.md 3.1 steps 1 to 3 on the result of readManifests: { errors, version } where `version`
 * is set only when there is no error.
 */
function checkVersion(m) {
  if (m.errors.length > 0) return { errors: m.errors, version: null }
  const distinct = [...new Set(m.sources.map((s) => s.value))]
  if (distinct.length > 1) {
    return {
      errors: [msg('the manifests do not agree on the version'), ...m.sources.map((s) => msg(`  ${s.name}: ${s.value}`))],
      version: null,
    }
  }
  const problem = versionProblem(distinct[0])
  if (problem) return { errors: [problem], version: null }
  return { errors: [], version: distinct[0] }
}

// --- Checksums, notes, draft arguments (release.md 3.4 to 3.6) ----------------------------------

const byCodeUnits = (a, b) => (a < b ? -1 : a > b ? 1 : 0)

/** SHA256SUMS.txt in sha256sum text format (release.md 3.4). */
export function sha256Sums(files) {
  return [...files]
    .sort((a, b) => byCodeUnits(a.name, b.name))
    .map((f) => `${createHash('sha256').update(f.bytes).digest('hex')}  ${f.name}\n`)
    .join('')
}

/** The release notes of release.md 3.5. `repository` null leaves out the provenance paragraph. */
export function releaseNotes({ productName: p, version: v, nsis, msi, sums, repository }) {
  const parts = [
    `${p} ${v} for Windows 10 and 11 (64-bit).`,
    '',
    '## Files',
    '',
    `- \`${nsis}\`: the installer (recommended). It installs ${p} for the current user.`,
    `- \`${msi}\`: the same app as a Windows Installer package, for managed installs.`,
    `- \`${SUMS_FILE}\`: the SHA-256 checksum of each installer.`,
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
    `The result must equal the hash on that file's line of \`${SUMS_FILE}\`:`,
    '',
    '```text',
    `${sums}\`\`\``,
    '',
  ]
  if (repository !== null && repository !== undefined) {
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

const installerNames = (p, v) => ({ nsis: `${p}_${v}_x64-setup.exe`, msi: `${p}_${v}_x64_en-US.msi` })

/** The `gh` arguments of release.md 3.6 step 6. */
export function draftArgs({ productName: p, version: v, assets, notes }) {
  const { nsis, msi } = installerNames(p, v)
  return [
    'release',
    'create',
    `v${v}`,
    '--draft',
    '--verify-tag',
    '--title',
    `${p} ${v}`,
    '--generate-notes',
    '--notes-file',
    notes,
    path.join(assets, nsis),
    path.join(assets, msi),
    path.join(assets, SUMS_FILE),
  ]
}

// `owner/name` of a GitHub repository URL, or null.
function githubRepository(url) {
  if (url === null) return null
  const m = /^https:\/\/github\.com\/([^/]+)\/([^/]+?)(\.git)?\/?$/.exec(url)
  return m ? `${m[1]}/${m[2]}` : null
}

// --- File system helpers ------------------------------------------------------------------------

// fs.statSync, or null when the path does not exist (ENOENT, or ENOTDIR for a parent that is a file).
function statOrNull(p) {
  try {
    return fs.statSync(p)
  } catch (e) {
    if (e.code === 'ENOENT' || e.code === 'ENOTDIR') return null
    throw e
  }
}

function lexists(p) {
  try {
    fs.lstatSync(p)
    return true
  } catch {
    return false
  }
}

function entriesOf(dir) {
  try {
    return fs.readdirSync(dir, { withFileTypes: true })
  } catch {
    return []
  }
}

const matchesLiteral = (name, prefix, suffix) =>
  name.length >= prefix.length + suffix.length && name.startsWith(prefix) && name.endsWith(suffix)

// --- Commands -----------------------------------------------------------------------------------

function cmdVersion(ctx) {
  const m = readManifests(ctx.root)
  const c = checkVersion(m)
  if (c.errors.length > 0) return ctx.fail(c.errors)
  ctx.out(msg(`version ${c.version} (${m.sources.length} sources agree)`))
  return 0
}

// Runs git with an argument array in `cwd`. { status, stdout, stderr } or { startError }.
function git(ctx, args) {
  const r = spawnSync('git', args, { cwd: ctx.root, env: ctx.env, encoding: 'utf8', windowsHide: true })
  if (r.error) return { startError: r.error.code ?? 'EIO' }
  return { status: r.status ?? 1, stdout: r.stdout ?? '', stderr: r.stderr ?? '' }
}

function gitFailure(sub, r) {
  if (r.startError) return msg(`git could not be started (${r.startError})`)
  return msg(`git ${sub} failed (exit ${r.status}): ${r.stderr.trim().replace(/\r?\n/g, ' ')}`)
}

// release.md 3.2 step 4: { error } or { short }.
function onMain(ctx) {
  let r = git(ctx, ['rev-parse', '--verify', '--quiet', 'refs/remotes/origin/main'])
  if (r.startError || (r.status !== 0 && r.status !== 1)) return { error: gitFailure('rev-parse', r) }
  if (r.status === 1) return { error: msg('origin/main not found (fetch it, for example actions/checkout with fetch-depth: 0)') }
  r = git(ctx, ['rev-parse', '--verify', '--quiet', 'HEAD^{commit}'])
  if (r.startError || r.status !== 0) return { error: gitFailure('rev-parse', r) }
  const short = r.stdout.trim().slice(0, 12)
  r = git(ctx, ['merge-base', '--is-ancestor', 'HEAD', 'refs/remotes/origin/main'])
  if (r.startError || (r.status !== 0 && r.status !== 1)) return { error: gitFailure('merge-base', r) }
  if (r.status === 1) return { error: msg(`commit ${short} is not on origin/main; tag a commit of main`) }
  return { short }
}

function cmdReleaseCheck(ctx, tag) {
  const errors = []
  const c = checkVersion(readManifests(ctx.root))
  errors.push(...c.errors)
  const tp = tagProblem(tag)
  if (tp) errors.push(tp)
  if (c.version !== null && !tp && tag !== `v${c.version}`) {
    errors.push(msg(`tag "${tag}" does not match the manifests' version ${c.version}`))
  }
  const main = onMain(ctx)
  if (main.error) errors.push(main.error)
  if (errors.length > 0) return ctx.fail(errors)
  ctx.out(msg(`tag ${tag} releases version ${c.version} from commit ${main.short} on origin/main`))
  return 0
}

function cmdAssets(ctx, opts) {
  const bundleArg = opts['--bundle']
  const outArg = opts['--out']
  const notesArg = opts['--notes']
  const limit = Number(opts['--max-bytes'])
  const m = readManifests(ctx.root, { productName: true })
  const c = checkVersion(m)
  if (c.errors.length > 0) return ctx.fail(c.errors)
  const p = m.productName
  const v = c.version
  const names = installerNames(p, v)
  const bundle = disp(bundleArg)
  const out = disp(outArg)
  const kinds = ['nsis', 'msi']
  const failures = { step1: [], step3: [], step4: [] }

  // Step 1: both installers exist as regular files.
  const installers = {}
  for (const k of kinds) {
    const file = path.join(bundleArg, k, names[k])
    const st = statOrNull(file)
    if (st && st.isFile()) installers[k] = { file, size: st.size, name: names[k] }
    else failures.step1.push(msg(`missing installer ${bundle}/${k}/${names[k]}`))
  }

  // Step 2: every other entry of the two folders is ignored and reported.
  const ignored = []
  kinds.forEach((k, order) => {
    for (const e of entriesOf(path.join(bundleArg, k))) {
      if (e.name === names[k]) continue
      ignored.push({ name: e.name, order, line: msg(`ignored ${bundle}/${k}/${e.name}${e.isDirectory() ? '/' : ''} (not an installer of ${v})`) })
    }
  })
  ignored.sort((a, b) => byCodeUnits(a.name, b.name) || a.order - b.order)
  for (const i of ignored) ctx.out(i.line)

  // Step 3: the size gate (RNF-06).
  for (const k of kinds) {
    const inst = installers[k]
    if (!inst) continue
    ctx.out(msg(`${inst.name} ${inst.size} bytes (${(inst.size / 1000000).toFixed(2)} MB), limit ${opts['--max-bytes']} bytes`))
    if (!(inst.size < limit)) {
      failures.step3.push(msg(`${inst.name} is ${inst.size} bytes, not under the limit of ${opts['--max-bytes']} bytes (RNF-06)`))
    }
  }

  // Step 4: the outputs may be replaced.
  const earlier = []
  const outStat = statOrNull(outArg)
  if (outStat && !outStat.isDirectory()) failures.step4.push(msg(`${out} is not a folder`))
  else if (outStat) {
    const entries = fs.readdirSync(outArg, { withFileTypes: true }).sort((a, b) => byCodeUnits(a.name, b.name))
    for (const e of entries) {
      const ours =
        e.isFile() &&
        (e.name === SUMS_FILE ||
          matchesLiteral(e.name, `${p}_`, '_x64-setup.exe') ||
          matchesLiteral(e.name, `${p}_`, '_x64_en-US.msi'))
      if (ours) earlier.push(e.name)
      else failures.step4.push(msg(`${out} holds ${e.name}${e.isDirectory() ? '/' : ''}, which assets did not write; remove it by hand`))
    }
  }
  const notesStat = statOrNull(notesArg)
  if (notesStat && !notesStat.isFile()) failures.step4.push(msg(`${disp(notesArg)} is not a file`))

  const all = [...failures.step1, ...failures.step3, ...failures.step4]
  if (all.length > 0) return ctx.fail(all)

  // Steps 5 to 7: write, and on the first failure remove everything this run created.
  const createdFiles = []
  const createdDirs = [] // batches, each deepest first
  const fail = (line) => {
    const errors = [line]
    for (const f of [...createdFiles].reverse()) {
      try {
        fs.rmSync(f.path, { force: true })
      } catch (e) {
        errors.push(msg(`cannot remove ${f.shown} (${e.code ?? 'EIO'})`))
      }
    }
    for (const batch of [...createdDirs].reverse()) {
      for (const d of batch) {
        if (!lexists(d)) continue // mkdirSync failed before creating it
        try {
          fs.rmdirSync(d)
        } catch (e) {
          errors.push(msg(`cannot remove ${disp(d)} (${e.code ?? 'EIO'})`))
        }
      }
    }
    return ctx.fail(errors)
  }
  // Creates `dir` (and its missing parents), recording what it created. Returns an error line or null.
  const makeDir = (dir) => {
    const missing = []
    for (let d = dir; !lexists(d); d = path.dirname(d)) {
      missing.push(d)
      if (path.dirname(d) === d) break
    }
    // Recorded before the call: a recursive mkdir can fail after creating some parents.
    createdDirs.push(missing)
    try {
      fs.mkdirSync(dir, { recursive: true })
    } catch (e) {
      return msg(`cannot write ${disp(dir)} (${e.code ?? 'EIO'})`)
    }
    return null
  }

  for (const name of earlier) {
    try {
      fs.rmSync(path.join(outArg, name))
    } catch (e) {
      return fail(msg(`cannot remove ${out}/${name} (${e.code ?? 'EIO'})`))
    }
  }
  if (!outStat) {
    const e = makeDir(outArg)
    if (e) return fail(e)
  }
  const copied = []
  for (const k of kinds) {
    const inst = installers[k]
    const dest = path.join(outArg, inst.name)
    const shown = `${out}/${inst.name}`
    createdFiles.push({ path: dest, shown })
    try {
      const bytes = fs.readFileSync(inst.file)
      fs.writeFileSync(dest, bytes)
      copied.push({ name: inst.name, bytes })
    } catch (e) {
      return fail(msg(`cannot write ${shown} (${e.code ?? 'EIO'})`))
    }
  }
  const sums = sha256Sums(copied)
  const sumsPath = path.join(outArg, SUMS_FILE)
  createdFiles.push({ path: sumsPath, shown: `${out}/${SUMS_FILE}` })
  try {
    fs.writeFileSync(sumsPath, sums)
  } catch (e) {
    return fail(msg(`cannot write ${out}/${SUMS_FILE} (${e.code ?? 'EIO'})`))
  }
  const notesParent = path.dirname(notesArg)
  const parentStat = statOrNull(notesParent)
  if (!parentStat || !parentStat.isDirectory()) {
    const e = makeDir(notesParent)
    if (e) return fail(e)
  }
  const notes = releaseNotes({
    productName: p,
    version: v,
    nsis: names.nsis,
    msi: names.msi,
    sums,
    repository: githubRepository(m.repository),
  })
  createdFiles.push({ path: notesArg, shown: disp(notesArg) })
  try {
    fs.writeFileSync(notesArg, notes)
  } catch (e) {
    return fail(msg(`cannot write ${disp(notesArg)} (${e.code ?? 'EIO'})`))
  }
  ctx.out(msg(`wrote ${out} (2 installers, ${SUMS_FILE}) and ${disp(notesArg)}`))
  return 0
}

function cmdDraft(ctx, opts) {
  const assetsArg = opts['--assets']
  const notesArg = opts['--notes']
  const m = readManifests(ctx.root, { productName: true })
  const c = checkVersion(m)
  if (c.errors.length > 0) return ctx.fail(c.errors)
  const p = m.productName
  const v = c.version
  const names = installerNames(p, v)
  const assets = disp(assetsArg)
  const errors = []

  // Step 2: the workflow's tag.
  if (ctx.env.GITHUB_REF_TYPE === 'tag' && ctx.env.GITHUB_REF_NAME !== `v${v}`) {
    errors.push(msg(`the workflow's tag "${ctx.env.GITHUB_REF_NAME ?? ''}" does not match the manifests' version ${v}`))
  }

  // Step 3: the assets folder holds exactly the two installers and SHA256SUMS.txt.
  const st = statOrNull(assetsArg)
  if (!st) errors.push(msg(`missing ${assets}`))
  else if (!st.isDirectory()) errors.push(msg(`${assets} is not a folder`))
  else {
    const expected = [names.nsis, names.msi, SUMS_FILE]
    const entries = fs.readdirSync(assetsArg, { withFileTypes: true })
    const files = new Set(entries.filter((e) => e.isFile()).map((e) => e.name))
    for (const n of expected) if (!files.has(n)) errors.push(msg(`${assets} has no ${n}`))
    const others = entries
      .filter((e) => !(e.isFile() && expected.includes(e.name)))
      .sort((a, b) => byCodeUnits(a.name, b.name))
    for (const e of others) errors.push(msg(`${assets} holds an unexpected ${e.name}${e.isDirectory() ? '/' : ''}`))
    if (expected.every((n) => files.has(n))) {
      const installers = [names.nsis, names.msi].map((name) => ({ name, bytes: fs.readFileSync(path.join(assetsArg, name)) }))
      const want = sha256Sums(installers)
      const have = fs.readFileSync(path.join(assetsArg, SUMS_FILE))
      if (!have.equals(Buffer.from(want, 'utf8'))) {
        const haveLines = have.toString('utf8').split('\n')
        const wantLines = want.slice(0, -1).split('\n')
        let named = false
        installers.forEach((inst, k) => {
          if (!haveLines.includes(wantLines[k])) {
            errors.push(msg(`${SUMS_FILE} does not match ${inst.name}`))
            named = true
          }
        })
        if (!named) errors.push(msg(`${SUMS_FILE} is not exactly the checksums of the two installers`))
      }
    }
  }

  // Step 4: the notes file.
  const notesStat = statOrNull(notesArg)
  if (!notesStat || !notesStat.isFile()) errors.push(msg(`missing ${disp(notesArg)}`))
  else if (!fs.readFileSync(notesArg, 'utf8').startsWith(`${p} ${v} `)) {
    errors.push(msg(`${disp(notesArg)} is not the release notes of ${v}`))
  }
  if (errors.length > 0) return ctx.fail(errors)

  const args = draftArgs({ productName: p, version: v, assets: assetsArg, notes: notesArg })
  if (opts['--print']) {
    ctx.out(JSON.stringify(args))
    return 0
  }
  if (ctx.env.GITHUB_ACTIONS !== 'true') {
    return ctx.fail([msg('draft runs only in GitHub Actions (use --print to see the command)')])
  }
  const r = spawnSync('gh', args, { stdio: 'inherit', env: ctx.env, windowsHide: true })
  if (r.error) return ctx.fail([msg(`gh could not be started (${r.error.code ?? 'EIO'})`)])
  return r.status ?? 1
}

function cmdSetVersion(ctx, version) {
  const problem = versionProblem(version)
  if (problem) return ctx.fail([problem])
  const m = readManifests(ctx.root)
  if (m.errors.length > 0) return ctx.fail(m.errors)

  const changed = []
  for (const rel of ['package.json', 'package-lock.json', TAURI, 'Cargo.toml', 'Cargo.lock']) {
    const spans = m.sources.filter((s) => s.rel === rel && s.value !== version)
    if (spans.length === 0) continue
    const f = m.files[rel]
    changed.push({ rel, file: f.file, text: f.bom + replaceSpans(f.text, spans, version) })
  }
  if (changed.length === 0) {
    ctx.out(msg(`nothing to change, already ${version}`))
    return 0
  }
  for (const c of changed) {
    try {
      fs.writeFileSync(c.file, c.text, 'utf8')
    } catch (e) {
      return ctx.fail([msg(`cannot write ${c.rel} (${e.code ?? 'EIO'})`)])
    }
  }
  const check = checkVersion(readManifests(ctx.root))
  if (check.errors.length > 0) return ctx.fail(check.errors)
  ctx.out(msg(`set version ${version} in ${changed.map((c) => c.rel).join(', ')}`))
  return 0
}

/**
 * The whole CLI (release.md 3.0 to 3.7) with `env` in place of process.env.
 * Returns { code, stdout: string[], stderr: string[] } (lines without their LF).
 */
export function run(argv, env = {}) {
  const stdout = []
  const stderr = []
  const ctx = {
    env: env ?? {},
    root: DEFAULT_ROOT,
    out: (line) => stdout.push(line),
    fail: (lines) => {
      stderr.push(...lines)
      return 1
    },
  }
  try {
    const parsed = parseArgs(argv)
    const errors = [...parsed.errors]
    let tag
    if (parsed.command === 'release-check') {
      tag = parsed.positionals[0]
      if (tag === undefined && ctx.env.GITHUB_REF_TYPE === 'tag' && ctx.env.GITHUB_REF_NAME) tag = ctx.env.GITHUB_REF_NAME
      if (tag === undefined) errors.push(msg('release-check needs a tag such as v1.2.3'))
    }
    if (errors.length > 0) return { code: 2, stdout, stderr: errors }
    if (parsed.options['--root'] !== undefined) ctx.root = parsed.options['--root']

    let code
    switch (parsed.command) {
      case 'version':
        code = cmdVersion(ctx)
        break
      case 'release-check':
        code = cmdReleaseCheck(ctx, tag)
        break
      case 'assets':
        code = cmdAssets(ctx, parsed.options)
        break
      case 'draft':
        code = cmdDraft(ctx, parsed.options)
        break
      default:
        code = cmdSetVersion(ctx, parsed.positionals[0])
    }
    return { code, stdout, stderr }
  } catch (e) {
    stderr.push(msg(`unexpected error: ${e instanceof Error ? e.message : String(e)}`))
    return { code: 1, stdout, stderr }
  }
}

function isMain() {
  if (!process.argv[1]) return false
  if (import.meta.url === pathToFileURL(process.argv[1]).href) return true
  try {
    return import.meta.url === pathToFileURL(fs.realpathSync(process.argv[1])).href
  } catch {
    return false
  }
}

if (isMain()) {
  const r = run(process.argv.slice(2), process.env)
  if (r.stdout.length > 0) process.stdout.write(r.stdout.map((l) => `${l}\n`).join(''))
  if (r.stderr.length > 0) process.stderr.write(r.stderr.map((l) => `${l}\n`).join(''))
  process.exitCode = r.code
}
