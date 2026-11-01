// PreToolUse (Bash/PowerShell): block destructive commands and the human-only gates (RP-02, RP-06).
import { readInput, block } from './_lib.mjs'

const input = readInput()
const cmd = String(input.tool_input?.command ?? '')
if (!cmd) process.exit(0)

const rules = [
  [/\bgit\s+push\b[^\n;|&]*(--force\b|--force-with-lease\b|\s-f\b)/i, 'force-push is not allowed'],
  [/\bgit\s+push\b[^\n;|&]*\b(origin\s+)?(HEAD:)?(refs\/heads\/)?main\b/i, 'pushing to main is not allowed (gate G2 is human)'],
  [/\bgh\s+pr\s+merge\b/i, 'merging is a human gate (G2, RP-02)'],
  [/\bgh\s+(repo\s+delete|release\s+delete)\b/i, 'destructive gh operation'],
  [/\brm\s+(-[a-z]*r[a-z]*f|-[a-z]*f[a-z]*r|-r\s+-f|-f\s+-r)\b/i, 'recursive force delete is not allowed'],
  [/\bRemove-Item\b[^\n]*-Recurse/i, 'recursive delete is not allowed'],
  [/\b(rmdir|rd)\s+\/s\b/i, 'recursive delete is not allowed'],
  [/\bgit\s+reset\s+--hard\b/i, 'git reset --hard discards work; ask the human'],
  [/\bgit\s+clean\s+-[a-z]*f/i, 'git clean -f discards work; ask the human'],
  [/--no-verify\b/i, 'skipping hooks is not allowed'],
  [/\bgit\s+push\b[^\n;|&]*(\s--tags\b|\s--follow-tags\b|[\s:+](refs\/tags\/|v\d+\.\d+\.\d+))/i, 'pushing a tag is a human act (releases, ADR-0045)'],
  [/\bgh\s+release\s+(create|edit|upload)\b/i, 'creating or changing a release is a human act (ADR-0045)'],
]
for (const [re, why] of rules) {
  if (re.test(cmd)) block(`Blocked by guard-bash: ${why}.\nCommand: ${cmd}`)
}

// The audio corpus is read-only (ADR-022, RNF-05).
const corpus = process.env.FAKEFLAC_CORPUS
if (corpus) {
  const norm = (s) => s.replace(/\\/g, '/').toLowerCase()
  const writes = /\b(rm|mv|cp|del|move|copy|Remove-Item|Move-Item|Set-Content|Out-File|Copy-Item)\b|[^>]>[^>]|>>/i
  if (writes.test(cmd) && norm(cmd).includes(norm(corpus))) {
    block('Blocked by guard-bash: the audio corpus is read-only (ADR-022).')
  }
}
process.exit(0)
