import { svelte } from '@sveltejs/vite-plugin-svelte'
import { svelteTesting } from '@testing-library/svelte/vite'
import { defineConfig } from 'vitest/config'

// Tauri expects a fixed dev-server port (src-tauri/tauri.conf.json -> build.devUrl).
export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: 'es2022', outDir: 'dist' },
  // Frontend tests only; the harness hooks use node:test (see `just check-fast`). Component tests
  // render in jsdom (ADR-0037); svelteTesting() adds browser conditions and auto-cleanup.
  test: {
    include: ['src/**/*.{test,spec}.ts'],
    environment: 'jsdom',
    passWithNoTests: true,
    // tests read tokens.css as text (?raw); vitest returns an empty string for CSS unless processed
    css: { include: [/tokens.css/] },
  },
})
