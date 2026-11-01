// PreToolUse (SendMessage): never resume a subagent whose context is already large. Each /feature
// iteration (back to test or implement, each review round) gets a fresh agent with a summary brief;
// a long context costs more and degrades the agent's work (docs/agents/lessons.md).
import { existsSync, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { readInput, block } from './_lib.mjs'

const DEFAULT_LIMIT = 150_000

// Context of the agent's last model call: every input token it sent, cached or not.
function contextTokens(jsonl) {
  let tokens = 0
  for (const line of jsonl.split('\n')) {
    try {
      const u = JSON.parse(line).message?.usage
      if (u) tokens = (u.input_tokens ?? 0) + (u.cache_read_input_tokens ?? 0) + (u.cache_creation_input_tokens ?? 0)
    } catch {
      // partial or non-JSON line
    }
  }
  return tokens
}

const input = readInput()
const to = input.tool_input?.to
const transcript = input.transcript_path
if (typeof to !== 'string' || !/^[\w-]+$/.test(to) || !transcript) process.exit(0)

// Subagents of this session keep their transcript next to the session's own one. Anything else
// (a teammate, another session) has no file here and is not checked.
const file = join(transcript.replace(/\.jsonl$/, ''), 'subagents', `agent-${to}.jsonl`)
if (!existsSync(file)) process.exit(0)

const limit = Number(process.env.FAKEFLAC_AGENT_CONTEXT_LIMIT) || DEFAULT_LIMIT
const tokens = contextTokens(readFileSync(file, 'utf8'))
if (tokens > limit) {
  block(
    `Blocked: agent ${to} already has ${Math.round(tokens / 1000)}k tokens of context (limit ${Math.round(limit / 1000)}k). ` +
      `Do not resume it: launch a fresh agent of the same type with a short brief (scope, what was decided, what changed, what to do now).`,
  )
}
process.exit(0)
