// Architecture lint (S8): enforces the dependency rules of docs/architecture/dependencies.md
// without compiling anything. Exits 1 with a list of violations.
import { readFileSync, readdirSync, statSync, existsSync } from 'node:fs'
import { join, relative, sep } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(fileURLToPath(new URL('.', import.meta.url)), '..')
const violations = []

// ---- 1. Cargo crate dependency rules ------------------------------------
const ADAPTERS = ['fakeflac-audio', 'fakeflac-fs', 'fakeflac-store', 'fakeflac-export']
const ALLOWED_TOOL_DEPS = ['fakeflac-core', 'fakeflac-audio', 'fakeflac-fs']
const crateDeps = (dir) => {
  const toml = readFileSync(join(dir, 'Cargo.toml'), 'utf8')
  const section = toml.split(/^\[dependencies\]/m)[1]?.split(/^\[/m)[0] ?? ''
  return [...section.matchAll(/^(fakeflac[\w-]*)\s*[=.]/gm)].map((m) => m[1])
}
const cratesDir = join(root, 'crates')
const toolsDir = join(root, 'tools')
const toolCrates = existsSync(toolsDir)
  ? readdirSync(toolsDir).filter((name) => existsSync(join(toolsDir, name, 'Cargo.toml')))
  : []

if (existsSync(cratesDir)) {
  for (const name of readdirSync(cratesDir)) {
    const dir = join(cratesDir, name)
    if (!existsSync(join(dir, 'Cargo.toml'))) continue
    const deps = crateDeps(dir)
    if (name === 'fakeflac-core' && deps.length > 0) {
      violations.push(`crates/${name}: core must not depend on workspace crates (found ${deps.join(', ')})`)
    }
    if (ADAPTERS.includes(name)) {
      for (const d of deps) {
        if (d !== 'fakeflac-core') {
          violations.push(`crates/${name}: adapters may depend only on fakeflac-core (found ${d})`)
        }
      }
    }
    for (const d of deps) {
      if (toolCrates.includes(d) || d === 'fakeflac-calibrate') {
        violations.push(`crates/${name}: product crate must not depend on dev tool '${d}'`)
      }
    }
  }
}

// src-tauri composition root must not depend on dev tools
const srcTauriDir = join(root, 'src-tauri')
if (existsSync(join(srcTauriDir, 'Cargo.toml'))) {
  const deps = crateDeps(srcTauriDir)
  for (const d of deps) {
    if (toolCrates.includes(d) || d === 'fakeflac-calibrate') {
      violations.push(`src-tauri: composition root must not depend on dev tool '${d}'`)
    }
  }
}

// Dev tools under tools/ (ADR-0036)
if (existsSync(toolsDir)) {
  for (const name of toolCrates) {
    const dir = join(toolsDir, name)
    const deps = crateDeps(dir)
    for (const d of deps) {
      if (!ALLOWED_TOOL_DEPS.includes(d)) {
        violations.push(`tools/${name}: dev tools may depend only on ${ALLOWED_TOOL_DEPS.join(', ')} (found ${d})`)
      }
    }
  }
}

// ---- 2. Atomic design import rules ---------------------------------------
const LEVELS = ['tokens', 'atoms', 'molecules', 'organisms', 'templates', 'pages']
const levelOf = (file) => {
  const parts = relative(join(root, 'src', 'lib'), file).split(sep)
  if (parts[0] === 'components') return LEVELS.indexOf(parts[1])
  if (parts[0] === 'pages') return LEVELS.indexOf('pages')
  if (parts[0] === 'tokens') return 0
  return -1
}
const walk = (dir) =>
  readdirSync(dir).flatMap((f) => {
    const p = join(dir, f)
    return statSync(p).isDirectory() ? walk(p) : [p]
  })
const libDir = join(root, 'src', 'lib')
if (existsSync(libDir)) {
  for (const file of walk(libDir).filter((f) => /\.(svelte|ts)$/.test(f))) {
    const level = levelOf(file)
    if (level < 1 || level >= LEVELS.indexOf('pages')) continue // only components/* are constrained
    const src = readFileSync(file, 'utf8')
    for (const m of src.matchAll(/from\s+['"]([^'"]+)['"]/g)) {
      const spec = m[1]
      if (/(^|\/)(stores|api)(\/|$)/.test(spec) || /@tauri-apps/.test(spec)) {
        violations.push(`${relative(root, file)}: presentational component imports '${spec}' (only pages may)`)
      }
      const target = LEVELS.findIndex((l) => new RegExp(`(^|/)${l}(/|$)`).test(spec))
      if (target > level) {
        violations.push(`${relative(root, file)}: ${LEVELS[level]} imports higher level '${LEVELS[target]}'`)
      }
    }
    if (/\bexport\s+let\b/.test(src) || /^\s*\$:/m.test(src)) {
      violations.push(`${relative(root, file)}: Svelte 4 syntax is not allowed (use runes)`)
    }
  }
}

// ---- 3. No direct invoke() outside src/lib/api ----------------------------
if (existsSync(libDir)) {
  for (const file of walk(join(root, 'src')).filter((f) => /\.(svelte|ts)$/.test(f))) {
    const rel = relative(root, file).split(sep).join('/')
    if (rel.startsWith('src/lib/api/')) continue
    if (/\binvoke\s*\(/.test(readFileSync(file, 'utf8'))) {
      violations.push(`${rel}: direct invoke() call; go through src/lib/api/`)
    }
  }
}

if (violations.length) {
  console.error('lint-arch: violations found\n' + violations.map((v) => ' - ' + v).join('\n'))
  process.exit(1)
}
console.log('lint-arch: ok')
