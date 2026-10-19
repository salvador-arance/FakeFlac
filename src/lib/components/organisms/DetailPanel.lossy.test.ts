// DetailPanel for a lossy file (specs/features/decode-lossy.feature, @RF-07 @RF-14).
// Scenario: An unsupported lossy file shows its error text. The analysis is stubbed (no audio): a
// `TrackAnalysis` with format "m4a", verdict "error" and error "unsupportedFormat". Text from
// `specs/ui/lossy-formats.md`: `error.unsupportedFormat` = "Unsupported audio format".
import { render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { makeTrack } from "../../testing/fixtures";
import { installCanvasStub } from "../../testing/spectrogram";
import type { CanvasStub } from "../../testing/spectrogram";
import DetailPanel from "./DetailPanel.svelte";

let stub: CanvasStub;
beforeEach(() => {
  stub = installCanvasStub();
});
afterEach(() => stub.restore());

const text = (container: HTMLElement) =>
  (container.textContent ?? "").replace(/\s+/g, " ").trim();

describe("DetailPanel: an unsupported lossy file", () => {
  // @RF-07 @RF-14 An unsupported lossy file shows its error text
  it('shows "Unsupported audio format" instead of a spectrogram', () => {
    const { container } = render(DetailPanel, {
      props: {
        track: makeTrack({
          path: "C:/Mixed/he-aac.m4a",
          format: "m4a",
          verdict: "error",
          error: { kind: "unsupportedFormat", detail: "HE-AAC is not supported" },
        }),
        spectrogram: null,
        loading: false,
      },
    });
    expect(text(container)).toContain("Unsupported audio format");
    expect(screen.queryByRole("img", { name: /spectrogram/i })).toBeNull();
    expect(screen.queryByTestId("spectrogram-loading")).toBeNull();
    expect(text(container)).not.toMatch(/Cutoff/);
  });
});
