// AnalysisPage: the spectrogram of the selected row (inspect-track.feature, @RF-07).
// The API client is mocked: the page is the only component that talks to it.
//
// Readings chosen where the specs are open (see SpectrogramView.test.ts and DetailPanel.test.ts
// for the markup contract): the page calls `getSpectrogram(path)` from `../api` with the selected
// track's `info.path`, once per selection of a track that is not in error, and renders the
// `DetailPanel` in the template's `detail` slot.
import { render, screen, within, fireEvent } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { AnalysisSummary, ProgressEvent } from "../api";
import * as api from "../api";
import type { Spectrogram } from "../model";
import { makeAlbum, makeSummary, makeTrack } from "../testing/fixtures";
import { mockSettings, waitForMainScreen } from "./tests/settingsHarness";
import type { TrackOptions } from "../testing/fixtures";
import { installCanvasStub, makeSpectrogram } from "../testing/spectrogram";
import type { CanvasStub } from "../testing/spectrogram";
import AnalysisPage from "./AnalysisPage.svelte";

vi.mock("../api", () => {
  class ApiError extends Error {
    code: string;
    constructor(error: { code: string; message: string }) {
      super(error.message);
      this.name = "ApiError";
      this.code = error.code;
    }
  }
  return {
    ApiError,
    pickFolder: vi.fn(),
    startAnalysis: vi.fn(),
    startAnalysisOfDropped: vi.fn(),
    cancelAnalysis: vi.fn(),
    getSpectrogram: vi.fn(),
    onDragDrop: vi.fn(),
    minimizeWindow: vi.fn(),
    toggleMaximizeWindow: vi.fn(),
    closeWindow: vi.fn(),
    getSettings: vi.fn(),
    setSettings: vi.fn(),
  };
});

let stub: CanvasStub;

/** A controllable `getSpectrogram`: one deferred answer per requested path. */
function fakeSpectrograms() {
  const pending = new Map<
    string,
    { resolve: (s: Spectrogram) => void; reject: (e: unknown) => void }[]
  >();
  vi.mocked(api.getSpectrogram).mockImplementation((path: string) => {
    return new Promise<Spectrogram>((resolve, reject) => {
      const list = pending.get(path) ?? [];
      list.push({ resolve, reject });
      pending.set(path, list);
    });
  });
  const last = (path: string) => {
    const list = pending.get(path);
    if (!list || list.length === 0) throw new Error(`no request for ${path}`);
    return list[list.length - 1];
  };
  return {
    resolve: (path: string, s: Spectrogram) => last(path).resolve(s),
    reject: (path: string, e: unknown) => last(path).reject(e),
  };
}

/** A finished run with the given tracks (index = position in the list). */
async function mountWithFinishedRun(tracks: TrackOptions[]) {
  let onEvent: (event: ProgressEvent) => void = () => {};
  let resolveRun: (s: AnalysisSummary) => void = () => {};
  vi.mocked(api.onDragDrop).mockImplementation(async () => () => {});
  vi.mocked(api.pickFolder).mockResolvedValue("C:/Music/Album A");
  vi.mocked(api.startAnalysis).mockImplementation((_root, cb) => {
    onEvent = cb;
    return new Promise<AnalysisSummary>((res) => {
      resolveRun = res;
    });
  });
  render(AnalysisPage);
  await vi.waitFor(() => expect(api.onDragDrop).toHaveBeenCalled());
  await waitForMainScreen();
  await userEvent.click(screen.getByRole("button", { name: "Open folder" }));
  await vi.waitFor(() => expect(api.startAnalysis).toHaveBeenCalled());
  onEvent({ kind: "scanned", total: tracks.length });
  tracks.forEach((o, index) =>
    onEvent({ kind: "trackDone", index, analysis: makeTrack(o), album: makeAlbum() }),
  );
  const summary = makeSummary({ total: tracks.length, done: tracks.length });
  onEvent({ kind: "finished", summary });
  resolveRun(summary);
  await vi.waitFor(() => expect(dataRows()).toHaveLength(tracks.length));
}

const dataRows = () =>
  screen
    .queryAllByRole("row")
    .filter((r) => within(r).queryAllByRole("gridcell").length > 0);
const rowOf = (file: string) =>
  dataRows().find(
    (r) => within(r).getAllByRole("gridcell")[1].textContent?.trim() === file,
  )!;
const selectRow = (file: string) =>
  userEvent.click(within(rowOf(file)).getAllByRole("gridcell")[1]);
const image = () =>
  screen.queryByRole("img", {
    name: /spectrogram/i,
  }) as HTMLCanvasElement | null;
const loading = () => screen.queryByTestId("spectrogram-loading");

const A = "C:/Music/Album A/a.flac";
const B = "C:/Music/Album A/b.flac";
const LP = "C:/Music/Album A/lp16k.flac";
const CORRUPT = "C:/Music/Album A/corrupt.flac";

beforeEach(() => {
  // RF-13: English default settings, locale reset (tests/settingsHarness.ts)
  mockSettings();
  vi.mocked(api.getSpectrogram).mockReset();
  vi.mocked(api.cancelAnalysis).mockReset().mockResolvedValue(undefined);
  stub = installCanvasStub();
});

afterEach(() => {
  stub.restore();
  vi.useRealTimers();
});

describe("@RF-07 Scenario: Selecting a row shows its spectrogram", () => {
  it("requests the spectrogram of the clicked row, shows loading, then the image and the line", async () => {
    const spectrograms = fakeSpectrograms();
    await mountWithFinishedRun([
      { path: A, verdict: "real" },
      { path: LP, verdict: "fake", cutoffHz: 16000 },
    ]);
    // nothing selected: the first row is not selected automatically, nothing is requested
    expect(api.getSpectrogram).not.toHaveBeenCalled();
    expect(image()).toBeNull();

    await selectRow("lp16k.flac");
    await vi.waitFor(() => expect(api.getSpectrogram).toHaveBeenCalledWith(LP));
    expect(api.getSpectrogram).toHaveBeenCalledTimes(1);
    await vi.waitFor(() => expect(loading()).not.toBeNull());
    expect(image()).toBeNull();

    spectrograms.resolve(LP, makeSpectrogram({ width: 8, height: 4 }));
    await vi.waitFor(() => expect(image()).not.toBeNull());
    expect(image()!.width).toBe(8);
    expect(loading()).toBeNull();
    expect(
      screen.getByTestId("cutoff-line").getAttribute("data-cutoff-hz"),
    ).toBe("16000");
    const panelText = (document.body.textContent ?? "").replace(/\s+/g, " ");
    expect(panelText).toContain("Cutoff 16.0 kHz · Fake");
  });
});

describe("@RF-07 Scenario: Changing the selection", () => {
  it("replaces the spectrogram of a.flac by the one of b.flac", async () => {
    const spectrograms = fakeSpectrograms();
    await mountWithFinishedRun([
      { path: A, verdict: "fake", cutoffHz: 16000 },
      { path: B, verdict: "fake", cutoffHz: 19000 },
    ]);
    await selectRow("a.flac");
    await vi.waitFor(() => expect(api.getSpectrogram).toHaveBeenCalledWith(A));
    spectrograms.resolve(A, makeSpectrogram({ width: 2, height: 4 }));
    await vi.waitFor(() => expect(image()?.width).toBe(2));

    await selectRow("b.flac");
    await vi.waitFor(() => expect(api.getSpectrogram).toHaveBeenCalledWith(B));
    // the previous image does not stay under the new selection
    await vi.waitFor(() => expect(image()).toBeNull());
    expect(loading()).not.toBeNull();

    spectrograms.resolve(B, makeSpectrogram({ width: 3, height: 4 }));
    await vi.waitFor(() => expect(image()?.width).toBe(3));
    expect(
      screen.getByTestId("cutoff-line").getAttribute("data-cutoff-hz"),
    ).toBe("19000");
    expect((document.body.textContent ?? "").replace(/\s+/g, " ")).toContain(
      "Cutoff 19.0 kHz",
    );
  });

  it("discards a late response for a.flac", async () => {
    const spectrograms = fakeSpectrograms();
    await mountWithFinishedRun([
      { path: A, verdict: "fake", cutoffHz: 16000 },
      { path: B, verdict: "fake", cutoffHz: 19000 },
    ]);
    await selectRow("a.flac");
    await vi.waitFor(() => expect(api.getSpectrogram).toHaveBeenCalledWith(A));
    await selectRow("b.flac");
    await vi.waitFor(() => expect(api.getSpectrogram).toHaveBeenCalledWith(B));

    spectrograms.resolve(B, makeSpectrogram({ width: 3, height: 4 }));
    await vi.waitFor(() => expect(image()?.width).toBe(3));

    // a.flac answers after b.flac: it must not replace b's image
    spectrograms.resolve(A, makeSpectrogram({ width: 2, height: 4 }));
    await new Promise((r) => setTimeout(r, 30));
    expect(image()?.width).toBe(3);
    expect(
      screen.getByTestId("cutoff-line").getAttribute("data-cutoff-hz"),
    ).toBe("19000");
  });

  it("discards a late response for a.flac while b.flac is still loading", async () => {
    const spectrograms = fakeSpectrograms();
    await mountWithFinishedRun([
      { path: A, verdict: "fake", cutoffHz: 16000 },
      { path: B, verdict: "fake", cutoffHz: 19000 },
    ]);
    await selectRow("a.flac");
    await vi.waitFor(() => expect(api.getSpectrogram).toHaveBeenCalledWith(A));
    await selectRow("b.flac");
    await vi.waitFor(() => expect(api.getSpectrogram).toHaveBeenCalledWith(B));

    spectrograms.resolve(A, makeSpectrogram({ width: 2, height: 4 }));
    await new Promise((r) => setTimeout(r, 30));
    expect(image()).toBeNull();
    expect(loading()).not.toBeNull();
  });

  it("ignores a late failure for a.flac", async () => {
    const spectrograms = fakeSpectrograms();
    await mountWithFinishedRun([
      { path: A, verdict: "fake", cutoffHz: 16000 },
      { path: B, verdict: "fake", cutoffHz: 19000 },
    ]);
    await selectRow("a.flac");
    await vi.waitFor(() => expect(api.getSpectrogram).toHaveBeenCalledWith(A));
    await selectRow("b.flac");
    await vi.waitFor(() => expect(api.getSpectrogram).toHaveBeenCalledWith(B));
    spectrograms.resolve(B, makeSpectrogram({ width: 3, height: 4 }));
    await vi.waitFor(() => expect(image()?.width).toBe(3));

    spectrograms.reject(
      A,
      new api.ApiError({ code: "INTERNAL", message: "late" } as never),
    );
    await new Promise((r) => setTimeout(r, 30));
    expect(image()?.width).toBe(3);
    expect(screen.queryByText(/unexpected error/)).toBeNull();
    expect(screen.queryByText(/Cannot show the spectrogram/)).toBeNull();
  });
});

describe("@RF-07 Scenario: Track with an error", () => {
  it("shows the error reason instead of a spectrogram and requests nothing", async () => {
    fakeSpectrograms();
    await mountWithFinishedRun([
      {
        path: CORRUPT,
        verdict: "error",
        error: { kind: "corruptStream", detail: "x" },
      },
      { path: A, verdict: "real" },
    ]);
    await selectRow("corrupt.flac");
    await vi.waitFor(() =>
      expect(screen.getAllByText(/Corrupt stream/).length).toBeGreaterThan(0),
    );
    expect(api.getSpectrogram).not.toHaveBeenCalled();
    expect(image()).toBeNull();
    expect(loading()).toBeNull();
  });

  it("drops the previous spectrogram when an error row is selected", async () => {
    const spectrograms = fakeSpectrograms();
    await mountWithFinishedRun([
      { path: A, verdict: "real" },
      { path: CORRUPT, verdict: "error", error: { kind: "tooShort" } },
    ]);
    await selectRow("a.flac");
    await vi.waitFor(() => expect(api.getSpectrogram).toHaveBeenCalledWith(A));
    spectrograms.resolve(A, makeSpectrogram({ width: 2, height: 4 }));
    await vi.waitFor(() => expect(image()).not.toBeNull());

    await selectRow("corrupt.flac");
    await vi.waitFor(() => expect(image()).toBeNull());
    expect(screen.getAllByText(/Too short to analyse/).length).toBeGreaterThan(
      0,
    );
    expect(api.getSpectrogram).toHaveBeenCalledTimes(1);
  });
});

describe("@RF-07 Scenario: Keyboard navigation", () => {
  it("selects the next row with the down arrow and shows its spectrogram", async () => {
    const spectrograms = fakeSpectrograms();
    await mountWithFinishedRun([
      { path: A, verdict: "fake", cutoffHz: 16000 },
      { path: B, verdict: "fake", cutoffHz: 19000 },
    ]);
    await selectRow("a.flac");
    await vi.waitFor(() => expect(api.getSpectrogram).toHaveBeenCalledWith(A));
    spectrograms.resolve(A, makeSpectrogram({ width: 2, height: 4 }));
    await vi.waitFor(() => expect(image()?.width).toBe(2));

    await fireEvent.keyDown(screen.getByRole("grid"), { key: "ArrowDown" });
    await vi.waitFor(() =>
      expect(rowOf("b.flac").getAttribute("aria-selected")).toBe("true"),
    );
    expect(rowOf("a.flac").getAttribute("aria-selected")).toBe("false");
    await vi.waitFor(() => expect(api.getSpectrogram).toHaveBeenCalledWith(B));
    spectrograms.resolve(B, makeSpectrogram({ width: 3, height: 4 }));
    await vi.waitFor(() => expect(image()?.width).toBe(3));
    expect(
      screen.getByTestId("cutoff-line").getAttribute("data-cutoff-hz"),
    ).toBe("19000");
  });
});

describe("@RF-07 Scenario: A Suspicious track shows its reason in plain words", () => {
  it("shows the cutoff, the verdict and the reason of the selected row", async () => {
    const spectrograms = fakeSpectrograms();
    await mountWithFinishedRun([
      {
        path: A,
        verdict: "suspicious",
        cutoffHz: 17000,
        reasons: ["highBandGaps"],
      },
    ]);
    await selectRow("a.flac");
    await vi.waitFor(() => expect(api.getSpectrogram).toHaveBeenCalledWith(A));
    spectrograms.resolve(A, makeSpectrogram({ width: 2, height: 4 }));
    await vi.waitFor(() => expect(image()).not.toBeNull());
    expect((document.body.textContent ?? "").replace(/\s+/g, " ")).toContain(
      "Cutoff 17.0 kHz · Suspicious · Frequent gaps above 16 kHz, as in an MP3",
    );
  });
});

describe("@RF-07 Scenario: The spectrogram cannot be read", () => {
  it("keeps the verdict line and shows the unavailable text instead of a spectrogram", async () => {
    const spectrograms = fakeSpectrograms();
    await mountWithFinishedRun([
      { path: LP, verdict: "fake", cutoffHz: 16000 },
    ]);
    await selectRow("lp16k.flac");
    await vi.waitFor(() => expect(api.getSpectrogram).toHaveBeenCalledWith(LP));
    spectrograms.reject(
      LP,
      new api.ApiError({ code: "INTERNAL", message: "gone" } as never),
    );
    await vi.waitFor(() =>
      expect(
        screen.queryByText("Cannot show the spectrogram of this file"),
      ).not.toBeNull(),
    );
    expect(image()).toBeNull();
    expect(loading()).toBeNull();
    expect((document.body.textContent ?? "").replace(/\s+/g, " ")).toContain(
      "Cutoff 16.0 kHz · Fake",
    );
  });
});
