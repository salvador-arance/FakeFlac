// App shell: the window shows the main screen (specs/ui/main-screen.md), not the H0 placeholder.
// The API client is mocked as in AnalysisPage.test.ts.
import { render, screen } from '@testing-library/svelte'
import { describe, expect, it, vi } from 'vitest'
import App from './App.svelte'

vi.mock('./lib/api', () => {
  class ApiError extends Error {
    code: string
    constructor(error: { code: string; message: string }) {
      super(error.message)
      this.name = 'ApiError'
      this.code = error.code
    }
  }
  return {
    ApiError,
    pickFolder: vi.fn(),
    startAnalysis: vi.fn(),
    startAnalysisOfDropped: vi.fn(),
    cancelAnalysis: vi.fn(),
    onDragDrop: vi.fn(() => Promise.resolve(() => {})),
    minimizeWindow: vi.fn(),
    toggleMaximizeWindow: vi.fn(),
    closeWindow: vi.fn(),
  }
})

describe('App', () => {
  it('shows the main screen: Open folder, the drop hint and Ready', () => {
    render(App)
    expect(screen.getByRole('button', { name: 'Open folder' })).toBeTruthy()
    expect(screen.getByText('Drop a folder here or click Open folder')).toBeTruthy()
    expect(screen.getByRole('status').textContent).toContain('Ready')
  })

  it('does not show the H0 placeholder', () => {
    render(App)
    expect(screen.queryByText(/Scaffold ready/)).toBeNull()
  })
})
