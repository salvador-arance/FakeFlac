import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vitest/config'

// Tauri expects a fixed dev-server port (src-tauri/tauri.conf.json -> build.devUrl).
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: 'es2022', outDir: 'dist' },
  // Frontend tests only; the harness hooks use node:test (see `just check-fast`).
  test: { include: ['src/**/*.{test,spec}.ts'], passWithNoTests: true },
})
