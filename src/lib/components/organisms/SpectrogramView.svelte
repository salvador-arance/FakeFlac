<script lang="ts" module>
  import { SPECTROGRAM_PALETTE } from '../../tokens/spectrogram'

  /** 256 RGB triplets: byte value -> colour, interpolated between the palette stops. */
  function buildLookup(): Uint8ClampedArray {
    const stops = SPECTROGRAM_PALETTE.map(([at, hex]) => {
      const n = parseInt(hex.slice(1), 16)
      return { at, rgb: [(n >> 16) & 0xff, (n >> 8) & 0xff, n & 0xff] }
    })
    const lut = new Uint8ClampedArray(256 * 3)
    for (let v = 0; v < 256; v++) {
      const x = v / 255
      let i = 0
      while (i < stops.length - 2 && x > stops[i + 1].at) i++
      const a = stops[i]
      const b = stops[i + 1]
      const k = b.at > a.at ? (x - a.at) / (b.at - a.at) : 0
      for (let c = 0; c < 3; c++) lut[v * 3 + c] = Math.round(a.rgb[c] + (b.rgb[c] - a.rgb[c]) * k)
    }
    return lut
  }

  const LOOKUP = buildLookup()

  /** Frequency axis ticks (Hz), every 5 kHz (10 kHz above 50 kHz), excluding 0 and the very top. */
  function ticksFor(maxHz: number): number[] {
    const step = maxHz > 50000 ? 10000 : 5000
    const ticks: number[] = []
    for (let hz = step; hz < maxHz * 0.97; hz += step) ticks.push(hz)
    return ticks
  }
</script>

<script lang="ts">
  import { t } from '../../i18n'
  import type { Spectrogram } from '../../model'

  type Props = {
    spectrogram: Spectrogram | null
    cutoffHz: number | null
    loading: boolean
  }

  let { spectrogram, cutoffHz, loading }: Props = $props()

  let canvas: HTMLCanvasElement | undefined = $state()

  let ticks = $derived(spectrogram ? ticksFor(spectrogram.maxFrequencyHz) : [])

  let cutoffPercent = $derived(
    spectrogram && cutoffHz !== null && spectrogram.maxFrequencyHz > 0
      ? Math.min(100, Math.max(0, (cutoffHz / spectrogram.maxFrequencyHz) * 100))
      : null,
  )

  const percentOf = (hz: number, maxHz: number) => (hz / maxHz) * 100

  // Paint the bytes: data[t * height + f] -> pixel (x = t, y = height - 1 - f), 0 Hz at the bottom.
  $effect(() => {
    if (!canvas || !spectrogram) return
    const ctx = canvas.getContext('2d')
    if (!ctx) return
    const { width, height, data } = spectrogram
    const image = ctx.createImageData(width, height)
    const pixels = image.data
    for (let x = 0; x < width; x++) {
      for (let f = 0; f < height; f++) {
        const v = data[x * height + f] ?? 0
        const p = ((height - 1 - f) * width + x) * 4
        pixels[p] = LOOKUP[v * 3]
        pixels[p + 1] = LOOKUP[v * 3 + 1]
        pixels[p + 2] = LOOKUP[v * 3 + 2]
        pixels[p + 3] = 255
      }
    }
    ctx.putImageData(image, 0, 0)
  })
</script>

{#if loading}
  <div class="spectrogram-view" aria-busy="true">
    <div class="loading" data-testid="spectrogram-loading">
      <span class="spinner" aria-hidden="true"></span>
      <span class="visually-hidden">{t('spectrogram.loading')}</span>
    </div>
  </div>
{:else if spectrogram}
  <div class="spectrogram-view">
    <div class="axis">
      {#each ticks as hz (hz)}
        <span class="tick" style="bottom: {percentOf(hz, spectrogram.maxFrequencyHz)}%">
          {hz / 1000} kHz
        </span>
      {/each}
    </div>
    <div class="plot">
      <!-- The canvas is a static picture (no pointer or keyboard handling): role img + label. -->
      <!-- svelte-ignore a11y_no_interactive_element_to_noninteractive_role -->
      <canvas
        bind:this={canvas}
        role="img"
        aria-label={t('spectrogram.label')}
        width={spectrogram.width}
        height={spectrogram.height}
      ></canvas>
      {#if cutoffPercent !== null}
        <div
          class="cutoff-line"
          data-testid="cutoff-line"
          data-cutoff-hz={cutoffHz}
          style="bottom: {cutoffPercent}%"
        ></div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .spectrogram-view {
    display: flex;
    width: 100%;
    height: 100%;
    min-height: 0;
    box-sizing: border-box;
  }

  .loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--color-border);
    border-top-color: var(--color-accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }

  .axis {
    position: relative;
    flex-shrink: 0;
    width: 48px;
    color: var(--color-text-muted);
    font-size: var(--font-size);
  }

  .tick {
    position: absolute;
    right: var(--space-1);
    transform: translateY(50%);
    white-space: nowrap;
  }

  .plot {
    position: relative;
    flex: 1;
    min-width: 0;
    background: var(--color-bg);
  }

  /* absolute so the bitmap size (e.g. 1200 x 512) never drives the layout; it is stretched */
  canvas {
    position: absolute;
    inset: 0;
    display: block;
    width: 100%;
    height: 100%;
  }

  .cutoff-line {
    position: absolute;
    left: 0;
    right: 0;
    height: 0;
    border-top: 1px dashed var(--color-accent);
    pointer-events: none;
  }
</style>
