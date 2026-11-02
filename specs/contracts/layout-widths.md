---
id: SPEC-CONTRACT-LAYOUT-WIDTHS
requirements: [RF-06, RF-10]
status: proposed
---

# Local layout width preferences

Companion to `../ui/resizable-widths.md`; proposed, pending G1. Backend command
contract version 8, generated bindings and RF-13's language-only `Settings`
remain unchanged. This extends the existing frontend local-storage pattern
used for `fakeflac.layout.detailHeight`; it does not migrate or edit that key.

`AnalysisPage` owns the new in-memory preferences and uses a frontend helper to
read/write the WebView's `localStorage` key `fakeflac.layout.widths`:

```ts
type LayoutWidthsV1 = {
  version: 1
  sidebarWidth?: number
  columns?: Partial<Record<SortColumn, number>>
}
```

Absence means default mode. Column IDs are the ten stable `SortColumn` IDs,
never translated labels. Widths are integer CSS px within the UI spec's user
bounds. Store only explicit preferences, not measured responsive widths. The
sidebar value is its preferred width before automatic window clamping.

| Input / failure | Result |
|---|---|
| Missing key | Default sidebar and no column overrides |
| Invalid JSON, non-object root, array, or version other than numeric 1 | Ignore entire record; defaults |
| Missing `sidebarWidth` / `columns` | Default for that part |
| Sidebar value not a finite number, or `columns` not a non-array object | Ignore that part |
| Unknown property or column ID | Ignore it |
| Known width not a finite number | Ignore that width; use default mode |
| Finite width outside bounds or fractional | Round to nearest integer, then clamp to the user bounds |
| Reading unavailable/throws | Defaults; resizing still works in memory |
| Writing/removing unavailable/throws | Keep the current in-memory layout; no dialog, status notice or analysis failure |

Reading never repairs/writes storage. After an explicit committed resize or
reset, write a normalized version-1 snapshot of all current preferences; an
invalid/newer saved record may therefore be replaced by that user action.
Remove the key when both sidebar and every column are in default mode. A reset
of one target removes only that preference. Pointer motion does not write;
completion/cancellation/blur after a real change and keyboard steps commit.
A reset commits immediately. A no-motion click is not a change. Resizing the
window, changing language or receiving results does not write preferences.

Reload restores only successfully persisted changes. No cross-window live
synchronization is introduced; another process's later write may win. This
record contains no audio paths/names, results, detector settings or language.
Components and adapters make no new IPC call, and the audio corpus is untouched.
