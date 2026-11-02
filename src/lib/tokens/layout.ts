import type { SortColumn } from '../model'

export const COLUMN_MIN: Record<SortColumn, number> = {
  state: 120, file: 100, folder: 100, format: 70, sampleRate: 80,
  bitDepth: 50, duration: 70, cutoff: 90, bitrate: 110, realBitrate: 100,
}
export const COLUMN_MAX = 800
export const RESPONSIVE_MIN = 140
export const FILE_WEIGHT = 3 / 7
export const SIDEBAR_DEFAULT = 240
export const SIDEBAR_MIN = 160
export const SIDEBAR_MAX = 480
export const MAIN_MIN = 360
export const SEPARATOR_WIDTH = 4
export const RESIZE_STEP = 10
