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
 * summary when the run ends (finished or cancelled) and only after the terminal event has been
 * delivered: channel messages and the command reply travel separately in Tauri, so no event of
 * this run arrives after the promise settles. Rejects with `ApiError` on a contract error.
 */
export async function startAnalysis(
  root: string,
  onEvent: (event: ProgressEvent) => void,
): Promise<AnalysisSummary> {
  let terminalDelivered = () => {}
  const terminal = new Promise<void>((resolve) => {
    terminalDelivered = resolve
  })
  const channel = new Channel<ProgressEvent>()
  channel.onmessage = (event) => {
    onEvent(event)
    if (event.kind === 'finished' || event.kind === 'cancelled') terminalDelivered()
  }
  const result = await commands.startAnalysis(root, channel)
  if (result.status === 'error') throw new ApiError(toAppError(result.error))
  await terminal
  return result.data
}

/** Tauri rejects with a plain string for IPC-level failures; those become `INTERNAL`. */
function toAppError(error: unknown): AppError {
  if (typeof error === 'object' && error !== null && 'code' in error && 'message' in error) {
    return error as AppError
  }
  return { code: 'INTERNAL', message: String(error) }
}

/**
 * RF-01 drag and drop: analyses the first dropped path that is a folder. Paths are tried in order;
 * one that answers `NOT_A_FOLDER` is skipped. Resolves with `null` when every path is a file
 * (dropped files are ignored, no error). Any other error rejects as in `startAnalysis`.
 */
export async function startAnalysisOfDropped(
  paths: string[],
  onEvent: (event: ProgressEvent) => void,
): Promise<AnalysisSummary | null> {
  void paths
  void onEvent
  throw new Error('not implemented: startAnalysisOfDropped')
}

/** RF-05: cancels the running analysis; no-op if none is running. */
export async function cancelAnalysis(): Promise<void> {
  await commands.cancelAnalysis()
}
