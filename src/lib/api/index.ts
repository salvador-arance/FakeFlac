// API client: the only place the frontend talks to the backend (no `invoke()` elsewhere).
// Contract: specs/contracts/commands.md. Types come from the generated `bindings.ts` (S3).
import { Channel } from '@tauri-apps/api/core'
import { commands } from './bindings'
import type { AnalysisSummary, AppError, AppErrorCode, ProgressEvent } from './bindings'

export type {
  AnalysisSummary,
  AppError,
  AppErrorCode,
  AudioFormat,
  CutoffEstimate,
  ProgressEvent,
  SuspicionReason,
  TrackAnalysis,
  TrackError,
  TrackInfo,
  Verdict,
} from './bindings'

/** A contract error (`AppError`) thrown by the client. */
export class ApiError extends Error {
  readonly code: AppErrorCode

  constructor(error: AppError) {
    super(error.message)
    this.name = 'ApiError'
    this.code = error.code
  }
}

/** RF-01: native folder dialog; `null` if the user cancels it. */
export async function pickFolder(): Promise<string | null> {
  return commands.pickFolder()
}

/**
 * RF-01..06: analyses `root`. Every progress event is passed to `onEvent`; resolves with the
 * summary when the run ends (finished or cancelled). Rejects with `ApiError` on a contract error.
 */
export async function startAnalysis(
  root: string,
  onEvent: (event: ProgressEvent) => void,
): Promise<AnalysisSummary> {
  const channel = new Channel<ProgressEvent>()
  channel.onmessage = onEvent
  const result = await commands.startAnalysis(root, channel)
  if (result.status === 'error') throw new ApiError(result.error)
  return result.data
}

/** RF-05: cancels the running analysis; no-op if none is running. */
export async function cancelAnalysis(): Promise<void> {
  await commands.cancelAnalysis()
}
