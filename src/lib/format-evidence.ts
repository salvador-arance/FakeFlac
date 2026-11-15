import { t } from './i18n'
import type { FakeEvidence } from './model'

// Both catalogues use a decimal point and plain digits, including for very large values.
const decimal = new Intl.NumberFormat('en-US', {
  useGrouping: false,
  minimumFractionDigits: 1,
  maximumFractionDigits: 1,
})

export function formatEvidence({ kind, value, threshold, margin }: FakeEvidence): string {
  const divisor = kind === 'shortfall' ? 1000 : 1
  return t(`evidence.${kind}`, {
    value: decimal.format(value / divisor),
    threshold: decimal.format(threshold / divisor),
    margin: decimal.format(margin / divisor),
  })
}
