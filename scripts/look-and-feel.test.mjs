// Repository checks of the look and feel (@RNF-10; specs/ui/look-and-feel.md elements L1 and L7,
// asset register 2.2, checks A1 and A2 of 2.3, acceptance K2). Scenarios of
// specs/features/look-and-feel.feature:
//   S4 "The application declares one window without native decorations"
//   S5 "Every shipped graphic file has a recorded origin" (check A1)
//   S6 "No shipped file copies a reference screenshot" (check A2)
// plus the register row `close` ("redrawn": the feature stage replaces the path in both branches of
// Icon.svelte) that K2 waits for. Every test names its scenario and its tag.
//
// Everything here is read-only: repository files are read as text or bytes, nothing is written.
// No audio, no corpus. The scanners are checked on inline synthetic texts first, so a scan that
// passes on the repository is not vacuous.
//
// Readings chosen where the spec is open (reported in the test-stage report):
// - "Listing the files" walks the file system (not `git ls-files`), case-insensitive extensions;
//   paths are repository-relative with '/'. `public/` is walked when it exists.
// - S4 scans the .ts, .js, .mjs, .svelte, .html and .rs files under src/ and src-tauri/src/ that
//   scripts/test-paths.mjs does not call tests (that excludes tests/ folders, *.test.*, *.spec.*
//   and stories). Comments are removed with a small lexer that keeps string literals, so a
//   forbidden word inside a string still counts. `WindowBuilder` is matched anywhere in a word, so
//   it also covers `WebviewWindowBuilder`.
// - S6 compares the bytes of every file (any extension) under the shipped folders with every file
//   of docs/reference/ other than app-icon-source.png. The path scan covers the text files under the
//   shipped folders plus index.html (the bundle's entry, look-and-feel.md 2.1); in Rust,
//   `include_bytes!` and `include_str!` count as imports. A path "names docs/reference/" when it
//   contains `docs/reference/` after turning '\' into '/'.
// - `close` redrawn: no run of SVG path data (from a moveto `M`/`m` to the first character that
//   cannot be path data) in a non-test .ts, .js, .mjs, .svelte, .html, .css or .svg file under src/
//   (nor index.html) draws the `x-lg` glyph of Bootstrap Icons, in its 1.10.5/1.11.0 form (the path in
//   Icon.svelte when the register was written) or its 1.11.3 form (the same path without the
//   redundant closing line-to, register note "1.11.3 only drops a redundant tail"). Path data is
//   compared in a canonical form (commands and numbers as tokens, numbers through Number()), so
//   whitespace or number formatting does not hide a copy. Only SHA-256 digests of the two
//   canonical forms are kept here, so this file does not carry the third-party glyph.
import { test } from 'node:test'
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { isTest } from './test-paths.mjs'

const here = path.dirname(fileURLToPath(import.meta.url))
const repo = path.join(here, '..')

const read = (rel) => fs.readFileSync(path.join(repo, rel), 'utf8')
const sha256 = (data) => createHash('sha256').update(data).digest('hex')

/** Every file under `rel` (repository-relative, '/'), skipping the folders in `skip`. */
function walk(rel, skip = []) {
  const out = []
  const abs = path.join(repo, rel)
  if (!fs.existsSync(abs)) return out
  for (const entry of fs.readdirSync(abs, { withFileTypes: true })) {
    const child = `${rel}/${entry.name}`
    if (entry.isDirectory()) {
      if (!skip.includes(child)) out.push(...walk(child, skip))
    } else if (entry.isFile()) {
      out.push(child)
    }
  }
  return out
}

/** The shipped folders of look-and-feel.md 2.3: src/, src-tauri/ (without target/ and gen/), public/. */
const shippedFiles = () => [
  ...walk('src'),
  ...walk('src-tauri', ['src-tauri/target', 'src-tauri/gen']),
  ...walk('public'),
]

const extOf = (p) => path.extname(p).slice(1).toLowerCase()

// ---- comment stripping -------------------------------------------------------------------------

/**
 * Removes comments and keeps everything else (string literals included). `kind` is the file
 * extension: `rs` (// and /* *\/, "strings", char literals), `css` (/* *\/ only), `ts`/`js`/`mjs`
 * (// and /* *\/, ' " ` strings), `svelte`/`html` (the same plus <!-- -->).
 */
function stripComments(text, kind) {
  const line = kind !== 'css'
  const html = kind === 'svelte' || kind === 'html'
  const quotes = kind === 'rs' ? ['"'] : kind === 'css' ? ['"', "'"] : ['"', "'", '`']
  const rest = text.startsWith.bind(text)
  let out = ''
  let i = 0
  while (i < text.length) {
    if (line && rest('//', i)) {
      const end = text.indexOf('\n', i)
      i = end === -1 ? text.length : end
    } else if (rest('/*', i)) {
      const end = text.indexOf('*/', i + 2)
      i = end === -1 ? text.length : end + 2
      out += ' '
    } else if (html && rest('<!--', i)) {
      const end = text.indexOf('-->', i + 4)
      i = end === -1 ? text.length : end + 3
      out += ' '
    } else if (kind === 'rs' && text[i] === "'") {
      // a char literal ('a', '\n', '\u{1F3B5}'); otherwise a lifetime
      const m = /^'(?:\\[^']*|[^\\'])'/.exec(text.slice(i, i + 12))
      const len = m ? m[0].length : 1
      out += text.slice(i, i + len)
      i += len
    } else if (quotes.includes(text[i])) {
      const q = text[i]
      let j = i + 1
      while (j < text.length && text[j] !== q) j += text[j] === '\\' ? 2 : 1
      out += text.slice(i, j + 1)
      i = j + 1
    } else {
      out += text[i]
      i += 1
    }
  }
  return out
}

// ---- S4: window builders -----------------------------------------------------------------------

const WINDOW_BUILDERS = [/WindowBuilder/, /\bnew\s+WebviewWindow\b/, /\bnew\s+Window\b/]

/** The forbidden window constructions in one source text, comments removed. */
function windowBuilders(text, kind) {
  const code = stripComments(text, kind)
  return WINDOW_BUILDERS.filter((re) => re.test(code)).map(String)
}

const S4_KINDS = ['ts', 'js', 'mjs', 'svelte', 'html', 'rs']

// ---- S6: paths under docs/reference/ -----------------------------------------------------------

const REFERENCE = /docs\/reference\//i

/** The import, url(), src or href values (and Rust include_*! paths) of one text, comments removed. */
function referencedPaths(text, kind) {
  const code = stripComments(text, kind)
  const found = []
  const patterns = [
    /\bimport\s+(?:[^'"`;]*?\s+from\s+)?(['"`])([^'"`]*)\1/g, // import x from '...' / import '...'
    /\bexport\s+[^'"`;]*?\s+from\s+(['"`])([^'"`]*)\1/g, // export ... from '...'
    /\bimport\s*\(\s*(['"`])([^'"`]*)\1/g, // import('...')
    /\brequire\s*\(\s*(['"`])([^'"`]*)\1/g, // require('...')
    /\burl\(\s*(['"]?)([^'")]*)\1\s*\)/g, // CSS url(...)
    /@import\s+(['"])([^'"]*)\1/g, // CSS @import '...'
    /\b(?:src|href|srcset)\s*=\s*(['"])([^'"]*)\1/g, // HTML src= / href=
    /\binclude_(?:bytes|str)!\s*\(\s*(")([^"]*)\1/g, // Rust include_bytes!("...")
  ]
  for (const re of patterns) for (const m of code.matchAll(re)) found.push(m[2])
  return found
}

const namesReference = (p) => REFERENCE.test(p.replace(/\\/g, '/'))

/** Every string value of a JSON document, depth first. */
function jsonStrings(value) {
  if (typeof value === 'string') return [value]
  if (Array.isArray(value)) return value.flatMap(jsonStrings)
  if (value && typeof value === 'object') return Object.values(value).flatMap(jsonStrings)
  return []
}

const TEXT_KINDS = ['ts', 'js', 'mjs', 'cjs', 'svelte', 'html', 'css', 'rs', 'json', 'toml', 'xml']

// ---- S5: the asset register --------------------------------------------------------------------

const VOCABULARY = ['own drawing', 'derived from own drawing', 'redrawn', 'third party', 'unrecorded']
const GRAPHIC = ['png', 'ico', 'icns', 'svg', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'woff', 'woff2', 'ttf', 'otf']

const cellsOf = (row) =>
  row
    .trim()
    .replace(/^\|/, '')
    .replace(/\|$/, '')
    .split('|')
    .map((c) => c.trim())

/**
 * The tables under `### 2.2 Asset register` of look-and-feel.md: `[{ header, rows }]`, each row
 * `{ name, origin, line }`. `name` is null when the first cell is not one backticked name.
 */
function registerTables(spec) {
  const lines = spec.split(/\r?\n/)
  const start = lines.findIndex((l) => l.trim() === '### 2.2 Asset register')
  assert.notEqual(start, -1, 'look-and-feel.md has no "### 2.2 Asset register" heading')
  let end = lines.findIndex((l, i) => i > start && /^#{2,3} /.test(l))
  if (end === -1) end = lines.length
  const tables = []
  let current = null
  for (const l of lines.slice(start + 1, end)) {
    if (!l.trim().startsWith('|')) {
      current = null
      continue
    }
    if (current === null) {
      current = { header: cellsOf(l), rows: [], separatorSeen: false }
      tables.push(current)
    } else if (!current.separatorSeen && /^\|[\s:|-]+\|$/.test(l.trim())) {
      current.separatorSeen = true
    } else {
      const cells = cellsOf(l)
      const originAt = current.header.indexOf('Origin')
      const first = /^`([^`]+)`$/.exec(cells[0] ?? '')
      current.rows.push({ name: first ? first[1] : null, origin: cells[originAt] ?? '', line: l })
    }
  }
  return tables.map(({ header, rows }) => ({ header, rows }))
}

/** The word of the vocabulary an origin starts with, or null. */
function originWord(origin) {
  // longest first: "derived from own drawing" before "own drawing"
  const words = [...VOCABULARY].sort((a, b) => b.length - a.length)
  return words.find((w) => origin.startsWith(w) && !/[A-Za-z0-9]/.test(origin.charAt(w.length))) ?? null
}

/** The string literals of the IconName union in icons.ts. */
function iconNames(source) {
  const m = /export\s+type\s+IconName\s*=([^;]*?)(?:\n\s*\n|;|$)/.exec(source)
  assert.ok(m, 'icons.ts has no `export type IconName = ...`')
  return [...m[1].matchAll(/'([^']+)'|"([^"]+)"/g)].map((x) => x[1] ?? x[2])
}

// ---- close redrawn -----------------------------------------------------------------------------

/** SVG path data as tokens: commands kept, numbers through Number() (".5" and "0.5" agree). */
function canonicalPath(d) {
  const tokens = d.match(/[A-Za-z]|[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/g) ?? []
  return tokens.map((t) => (/^[A-Za-z]$/.test(t) ? t : String(Number(t)))).join(' ')
}

/** SHA-256 of the canonical form of Bootstrap Icons `x-lg` (MIT): 1.10.5/1.11.0, then 1.11.3. */
const X_LG = new Set([
  '7c7f4f5703eb03d4a1ba17486a83c9a44c84f20a8e0a798049c4bf489b3808cd',
  'ddb69d185984d66e71693d493d0aaa38738388627a10f368116238cc86613075',
])

/** Every run of SVG path data in a text (from a moveto to the first character that cannot be path data). */
function pathRuns(text) {
  return text.match(/[Mm][\s,]*[-+.\d][MmZzLlHhVvCcSsQqTtAaEe\d\s,.+-]*/g) ?? []
}

const drawsXlg = (s) => /\d/.test(s) && X_LG.has(sha256(canonicalPath(s)))

// =================================================================================================
// The scanners on synthetic texts (not vacuous)
// =================================================================================================

test('@RNF-10 scanners: comments are removed, strings and code are kept', () => {
  assert.deepEqual(windowBuilders('// WebviewWindowBuilder::new(app)\nfn main() {}', 'rs'), [])
  assert.deepEqual(windowBuilders('/* new WebviewWindow(x) */ let a = 1', 'ts'), [])
  assert.deepEqual(windowBuilders('<!-- new Window("x") -->\n<div></div>', 'svelte'), [])
  assert.equal(windowBuilders("fn f<'a>(x: &'a str) { WebviewWindowBuilder::new(app, \"w\", url) }", 'rs').length, 1)
  assert.equal(windowBuilders('let b = tauri::window::WindowBuilder::new(&app, "x")', 'rs').length, 1)
  assert.equal(windowBuilders("const w = new WebviewWindow('second', {})", 'ts').length, 1)
  assert.equal(windowBuilders("const w = new  Window('second')", 'ts').length, 1)
  assert.equal(windowBuilders('const url = "http://x"; new WebviewWindow("a")', 'ts').length, 1)
  assert.deepEqual(windowBuilders("const s = 'a // b'; const n = 1", 'ts'), [])
})

test('@RNF-10 scanners: paths named by import, url(), src, href and include_bytes!', () => {
  assert.deepEqual(referencedPaths("import shot from '../../docs/reference/a.png'", 'ts'), ['../../docs/reference/a.png'])
  assert.deepEqual(referencedPaths("import '../docs/reference/x.css'", 'ts'), ['../docs/reference/x.css'])
  assert.deepEqual(referencedPaths("const m = await import('/docs/reference/b.png')", 'ts'), ['/docs/reference/b.png'])
  assert.deepEqual(referencedPaths('.a { background: url("../../docs/reference/c.png") }', 'css'), ['../../docs/reference/c.png'])
  assert.deepEqual(referencedPaths('.a { background: url(../docs/reference/c.png) }', 'css'), ['../docs/reference/c.png'])
  assert.deepEqual(referencedPaths('<img src="/docs/reference/d.png" alt="">', 'svelte'), ['/docs/reference/d.png'])
  assert.deepEqual(referencedPaths('<link rel="icon" href="docs/reference/e.png">', 'html'), ['docs/reference/e.png'])
  assert.deepEqual(referencedPaths('static B: &[u8] = include_bytes!("../../docs/reference/f.png");', 'rs'), ['../../docs/reference/f.png'])
  assert.deepEqual(referencedPaths('/* the reference UI (docs/reference/), url(docs/reference/g.png) */', 'css'), [])
  assert.deepEqual(referencedPaths('<!-- <img src="docs/reference/h.png"> -->', 'html'), [])
  assert.ok(namesReference('..\\..\\docs\\reference\\i.png'))
  assert.ok(!namesReference('../docs/references.md'))
})

test('@RNF-10 scanners: the register parser, the vocabulary and the IconName union', () => {
  const spec = [
    '### 2.2 Asset register',
    '',
    '| File | Kind | Origin | Shipped as |',
    '|---|---|---|---|',
    '| `a/b.png` | App icon | own drawing | x |',
    '| a/c.png | App icon | own drawing | x |',
    '',
    '| `IconName` (inline SVG) | Origin |',
    '|---|---|',
    '| `close` | redrawn (note) |',
    '',
    '### 2.3 Next',
    '| `ignored` | own drawing |',
  ].join('\n')
  const tables = registerTables(spec)
  assert.equal(tables.length, 2)
  assert.deepEqual(tables[0].rows.map((r) => [r.name, r.origin]), [['a/b.png', 'own drawing'], [null, 'own drawing']])
  assert.deepEqual(tables[1].rows.map((r) => [r.name, r.origin]), [['close', 'redrawn (note)']])
  assert.equal(originWord('derived from own drawing (same)'), 'derived from own drawing')
  assert.equal(originWord('own drawing'), 'own drawing')
  assert.equal(originWord('third party: Name, MIT'), 'third party')
  assert.equal(originWord('unrecorded'), 'unrecorded')
  assert.equal(originWord('ownership unclear'), null)
  assert.equal(originWord('redrawnish'), null)
  assert.equal(originWord('copied'), null)
  assert.deepEqual(iconNames("// x\nexport type IconName =\n  | 'a'\n  | 'b-c'\n"), ['a', 'b-c'])
})

test('@RNF-10 scanners: path data is compared in canonical form', () => {
  assert.equal(canonicalPath('M2 8.5a.5.5 0 0 1 .5-.5z'), canonicalPath('M 2 8.50 a 0.5 0.5 0 0 1 0.5 -0.5 z'))
  assert.notEqual(canonicalPath('M2 8.5h11'), canonicalPath('M2 8.5h12'))
  assert.equal(drawsXlg('M2 8.5h11'), false)
  assert.deepEqual(pathRuns('<path d="M2 8.5a.5.5 0 0 1 .5-.5z" /> <path d="m1-2L3 4Z"/>'), [
    'M2 8.5a.5.5 0 0 1 .5-.5z',
    'm1-2L3 4Z',
  ])
})

// =================================================================================================
// S4
// =================================================================================================

test('@RNF-10 S4 The application declares one window without native decorations: app.windows', () => {
  const conf = JSON.parse(read('src-tauri/tauri.conf.json'))
  const windows = conf?.app?.windows
  assert.ok(Array.isArray(windows), 'app.windows is not an array')
  assert.equal(windows.length, 1, 'app.windows does not hold exactly one window')
  const [w] = windows
  assert.equal(w.title, 'FakeFlac')
  assert.equal(w.decorations, false)
  assert.equal(w.minWidth, 900)
  assert.equal(w.minHeight, 500)
})

test('@RNF-10 S4 The application declares one window without native decorations: no code builds another window', () => {
  const files = [...walk('src'), ...walk('src-tauri/src')].filter(
    (p) => S4_KINDS.includes(extOf(p)) && !isTest(p),
  )
  assert.ok(files.some((p) => p.startsWith('src-tauri/src/')), 'no Rust source scanned')
  assert.ok(files.some((p) => p.endsWith('.svelte')), 'no Svelte source scanned')
  const found = files.flatMap((p) => windowBuilders(read(p), extOf(p)).map((w) => `${p}: ${w}`))
  assert.deepEqual(found, [])
})

// =================================================================================================
// S5 (check A1)
// =================================================================================================

const SPEC = 'specs/ui/look-and-feel.md'

function register() {
  const tables = registerTables(read(SPEC))
  assert.equal(tables.length, 2, 'section 2.2 must hold two tables (files, then IconName values)')
  const [files, icons] = tables
  assert.equal(files.header[0], 'File')
  assert.ok(icons.header[0].startsWith('`IconName`'), `unexpected header ${icons.header[0]}`)
  for (const t of tables) assert.ok(t.header.includes('Origin'), `no Origin column in ${t.header.join(' | ')}`)
  return { files: files.rows, icons: icons.rows }
}

test('@RNF-10 S5 Every shipped graphic file has a recorded origin: every row names one backticked file or icon', () => {
  const { files, icons } = register()
  assert.ok(files.length > 0 && icons.length > 0, 'the register is empty')
  for (const row of [...files, ...icons]) assert.notEqual(row.name, null, `first cell is not one backticked name: ${row.line}`)
  const names = (rows) => rows.map((r) => r.name)
  assert.equal(new Set(names(files)).size, files.length, 'a file row is repeated')
  assert.equal(new Set(names(icons)).size, icons.length, 'an icon row is repeated')
})

test('@RNF-10 S5 Every shipped graphic file has a recorded origin: the graphic and font files equal the file rows', () => {
  const listed = shippedFiles().filter((p) => GRAPHIC.includes(extOf(p))).sort()
  const rows = register().files.map((r) => r.name).sort()
  assert.deepEqual(listed, rows)
})

test('@RNF-10 S5 Every shipped graphic file has a recorded origin: the IconName values equal the icon rows', () => {
  const values = iconNames(read('src/lib/components/atoms/icons.ts')).sort()
  assert.ok(values.length > 0)
  const rows = register().icons.map((r) => r.name).sort()
  assert.deepEqual(values, rows)
})

test('@RNF-10 S5 Every shipped graphic file has a recorded origin: every origin starts with a word of the vocabulary', () => {
  const { files, icons } = register()
  const rows = [...files, ...icons]
  const bad = rows.filter((r) => originWord(r.origin) === null).map((r) => `${r.name}: "${r.origin}"`)
  assert.deepEqual(bad, [])
  // K2: printed, never a failure
  const unrecorded = rows.filter((r) => originWord(r.origin) === 'unrecorded').length
  console.log(`look-and-feel asset register: ${unrecorded} of ${rows.length} rows unrecorded (K2)`)
})

test('@RNF-10 S5 register row `close` (redrawn, K2): no shipped source draws the x-lg glyph of Bootstrap Icons', () => {
  const files = [...walk('src').filter((p) => !isTest(p)), 'index.html'].filter((p) =>
    ['ts', 'js', 'mjs', 'svelte', 'html', 'css', 'svg'].includes(extOf(p)),
  )
  assert.ok(files.includes('src/lib/components/atoms/Icon.svelte'), 'Icon.svelte not scanned')
  const found = files.flatMap((p) => pathRuns(read(p)).filter(drawsXlg).map(() => p))
  assert.deepEqual(found, [], 'the close icon still draws the x-lg glyph (register 2.2: it is redrawn)')
})

// =================================================================================================
// S6 (check A2)
// =================================================================================================

test('@RNF-10 S6 No shipped file copies a reference screenshot: no byte-identical file', () => {
  const references = walk('docs/reference').filter((p) => path.basename(p) !== 'app-icon-source.png')
  assert.ok(references.length > 0, 'docs/reference/ has no screenshot')
  const digests = new Map(references.map((p) => [sha256(fs.readFileSync(path.join(repo, p))), p]))
  const shipped = shippedFiles()
  assert.ok(shipped.includes('src-tauri/icons/icon.png'), 'the shipped folders were not listed')
  const copies = shipped
    .map((p) => [p, digests.get(sha256(fs.readFileSync(path.join(repo, p))))])
    .filter(([, ref]) => ref !== undefined)
    .map(([p, ref]) => `${p} = ${ref}`)
  assert.deepEqual(copies, [])
})

test('@RNF-10 S6 No shipped file copies a reference screenshot: no import, url(), src, href or tauri.conf.json value names docs/reference/', () => {
  const files = [...shippedFiles(), 'index.html'].filter((p) => TEXT_KINDS.includes(extOf(p)))
  const named = files.flatMap((p) =>
    ['json', 'toml', 'xml'].includes(extOf(p))
      ? []
      : referencedPaths(read(p), extOf(p) === 'cjs' ? 'js' : extOf(p))
          .filter(namesReference)
          .map((ref) => `${p}: ${ref}`),
  )
  const conf = jsonStrings(JSON.parse(read('src-tauri/tauri.conf.json')))
  assert.ok(conf.includes('FakeFlac'), 'tauri.conf.json values were not read')
  const confNamed = conf.filter(namesReference).map((v) => `src-tauri/tauri.conf.json: ${v}`)
  assert.deepEqual([...named, ...confNamed], [])
})
