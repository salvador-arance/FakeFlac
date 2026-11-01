---
id: SPEC-UI-TRANSLATIONS
requirements: [RF-13, RNF-08, RNF-09, RNF-10]
status: approved
---

# Complete English and Spanish catalogue (RF-13)

Approved at G1 (#107); the behavior and acceptance checks are in `language-settings.md`. Each row is
an explicit catalogue entry in both languages, including identical technical text. Braces
identify interpolation parameters, substituted literally once per occurrence (never parsed
as markup). Names and paths supplied as parameters are displayed as text with their original
Unicode characters. Escaping in rendered HTML follows the framework's normal text handling.

## 1. Application, actions and initial state

| Key | English | Spanish |
|---|---|---|
| `app.title` | FakeFlac | FakeFlac |
| `toolbar.label` | Main toolbar | Barra de herramientas principal |
| `toolbar.open` | Open folder | Abrir carpeta |
| `toolbar.cancel` | Cancel | Cancelar |
| `toolbar.export` | Export CSV | Exportar CSV |
| `toolbar.clearCache` | Clear results cache | Borrar caché de resultados |
| `toolbar.settings` | Settings | Ajustes |
| `empty.hint` | Drop a folder here or click Open folder | Arrastra una carpeta aquí o pulsa Abrir carpeta |
| `window.minimize` | Minimize | Minimizar |
| `window.maximize` | Maximize | Maximizar |
| `window.close` | Close | Cerrar |
| `settings.title` | Settings | Ajustes |
| `settings.language` | Language | Idioma |
| `settings.english` | English | English |
| `settings.spanish` | Español | Español |
| `settings.save` | Save | Guardar |
| `settings.cancel` | Cancel | Cancelar |
| `settings.close` | Close | Cerrar |
| `settings.saving` | Saving… | Guardando… |
| `settings.invalid` | Saved settings could not be read. English is in use. Save to replace them. | No se han podido interpretar los ajustes guardados. Se está usando el inglés. Guarda para sustituirlos. |
| `settings.unavailable` | Settings are unavailable. English is in use. You can try saving again. | Los ajustes no están disponibles. Se está usando el inglés. Puedes volver a intentar guardarlos. |
| `settings.newerVersion` | These settings were saved by a newer app version. English is in use; these settings will be kept. | Estos ajustes se guardaron con una versión más reciente de la aplicación. Se está usando el inglés y se conservarán los ajustes. |
| `settings.saveFailed` | Cannot save the language preference. Try again. | No se puede guardar la preferencia de idioma. Vuelve a intentarlo. |

The maximize button retains its existing toggle action and name; RF-13 translates that name,
without introducing a window-state feature. The startup warning sentences truthfully mention
English because the contract only issues these load statuses with the fallback language.

## 2. Verdicts, reasons and errors

| Key | English | Spanish |
|---|---|---|
| `verdict.real` | Genuine | Real |
| `verdict.suspicious` | Suspicious | Sospechoso |
| `verdict.fake` | Fake | Falso |
| `verdict.error` | Error | Error |
| `reason.lowHighFrequencyContent` | Little high-frequency content: cannot confirm | Poco contenido de alta frecuencia: no se puede confirmar |
| `reason.gradualRolloff` | High frequencies fade out gradually | Las frecuencias altas se atenúan gradualmente |
| `reason.ambiguousCutoff` | Sharp cutoff at a high frequency | Corte abrupto a una frecuencia alta |
| `reason.highBandGaps` | Frequent gaps above 16 kHz, as in an MP3 | Huecos frecuentes por encima de 16 kHz, como en un MP3 |
| `reason.cutoffBelowDeclared` | Cutoff lower than expected for its bitrate | Corte inferior al esperado para su tasa de bits |
| `reason.streamingSampleRate` | Full band at 48 kHz: possibly a streaming rip; genuine 48 kHz files look the same. | Banda completa a 48 kHz: posible ripeo de streaming; los archivos auténticos a 48 kHz se ven igual. |
| `error.tooShort` | Too short to analyse | Demasiado corto para analizar |
| `error.corruptStream` | Corrupt stream | Flujo de audio dañado |
| `error.unreadable` | Cannot read the file | No se puede leer el archivo |
| `error.unsupportedFormat` | Unsupported audio format | Formato de audio no compatible |
| `error.NO_AUDIO_FOUND` | No supported audio files in this folder | No hay archivos de audio compatibles en esta carpeta |
| `error.FOLDER_UNREADABLE` | Cannot open this folder | No se puede abrir esta carpeta |
| `error.ANALYSIS_RUNNING` | An analysis is already running | Ya hay un análisis en curso |
| `error.EXPORT_FAILED` | Cannot save the CSV file | No se puede guardar el archivo CSV |
| `error.NOT_ANALYZED` | There are no results to export | No hay resultados para exportar |
| `error.INTERNAL` | The analysis stopped because of an unexpected error | El análisis se ha detenido debido a un error inesperado |
| `error.CACHE_FAILED` | Cannot clear the results cache | No se puede borrar la caché de resultados |
| `error.SETTINGS_WRITE_FAILED` | Cannot save the language preference. Try again. | No se puede guardar la preferencia de idioma. Vuelve a intentarlo. |
| `error.SETTINGS_READ_ONLY` | These settings belong to a newer app version and cannot be replaced. | Estos ajustes pertenecen a una versión más reciente de la aplicación y no se pueden sustituir. |

`NOT_A_FOLDER` remains silent for dropped files. An unexpected settings command failure uses
`settings.saveFailed` or the startup unavailable warning, never the analysis-specific
`error.INTERNAL` sentence. Track diagnostic details and paths remain untranslated where an
existing surface exposes them; the friendly TrackError labels are always translated.

## 3. Table and detail

| Key | English | Spanish |
|---|---|---|
| `column.state` | State | Estado |
| `column.file` | File | Archivo |
| `column.folder` | Folder | Carpeta |
| `column.format` | Format | Formato |
| `column.sampleRate` | Samplerate | Frecuencia de muestreo |
| `column.bits` | Bits | Bits |
| `column.duration` | Duration | Duración |
| `column.cutoff` | Cutoff | Corte |
| `column.bitrate` | Bitrate | Tasa de bits |
| `column.realBitrate` | Real bitrate | Tasa de bits real |
| `cutoff.fullBand` | Full band | Banda completa |
| `detail.cutoff` | Cutoff {khz} kHz | Corte {khz} kHz |
| `detail.declared` | {kbps} kbps | {kbps} kbps |
| `detail.source` | ≈{kbps} kbps source | Fuente de ≈{kbps} kbps |
| `bitrate.kbps` | {kbps} kbps | {kbps} kbps |
| `bitrate.vbr` | VBR | VBR |
| `bitrate.abr` | ABR | ABR |
| `bitrate.approx` | ≈{kbps} kbps | ≈{kbps} kbps |
| `bitrate.none` | — | — |
| `bitrate.noneLabel` | No bitrate | Sin tasa de bits |
| `format.m4aAlac` | M4A (ALAC) | M4A (ALAC) |
| `help.realBitrate` | The real bitrate is an estimate from the cutoff, in terms of the file's own codec. Files from sources with little treble, from encoders without a low-pass, or re-encoded from a close bitrate may not be detected. | La tasa de bits real se estima a partir del corte, según el códec del propio archivo. Es posible que no se detecten los archivos procedentes de fuentes con pocos agudos, de codificadores sin filtro de paso bajo o recodificados desde una tasa de bits cercana. |
| `spectrogram.label` | Spectrogram | Espectrograma |
| `spectrogram.loading` | Loading spectrogram… | Cargando espectrograma… |
| `spectrogram.unavailable` | Cannot show the spectrogram of this file | No se puede mostrar el espectrograma de este archivo |

Numeric presentation keeps existing rules: plain digits without grouping, the decimal point
in cutoff values, duration `m:ss`, unchanged Hz/kHz/kbps units and spectrogram ticks. Format
identifiers (WAV/FLAC/MP3/M4A/Opus and codec labels), approximation signs and empty-cell glyphs
are technical text. Localization changes labels, not stored values or computations.

## 4. Filters and albums

| Key | English | Spanish |
|---|---|---|
| `filter.label` | Filter by verdict | Filtrar por veredicto |
| `filter.fake` | Show fake tracks | Mostrar pistas falsas |
| `filter.suspicious` | Show suspicious tracks | Mostrar pistas sospechosas |
| `filter.real` | Show genuine tracks | Mostrar pistas reales |
| `filter.error` | Show tracks with errors | Mostrar pistas con errores |
| `search.label` | Search by name | Buscar por nombre |
| `search.placeholder` | Search (Ctrl+F) | Buscar (Ctrl+F) |
| `search.clear` | Clear search | Borrar búsqueda |
| `table.noMatches` | No tracks match the filters | Ninguna pista coincide con los filtros |
| `table.clearFilters` | Clear filters | Quitar filtros |
| `albums.title` | Albums | Álbumes |
| `albums.titleCount` | Albums ({count}) | Álbumes ({count}) |
| `albums.label` | Album summaries | Resúmenes de álbumes |
| `albums.tracks` | {n} tracks | {n} pistas |
| `albums.tracksOne` | 1 track | 1 pista |
| `albums.partial` | {done} of {files} tracks | {done} de {files} pistas |
| `albums.real` | {n} real | {n} reales |
| `albums.realOne` | 1 real | 1 real |
| `albums.suspicious` | {n} suspicious | {n} sospechosas |
| `albums.suspiciousOne` | 1 suspicious | 1 sospechosa |
| `albums.fake` | {n} fake | {n} falsas |
| `albums.fakeOne` | 1 fake | 1 falsa |
| `albums.errors` | {n} errors | {n} errores |
| `albums.errorsOne` | 1 error | 1 error |

Album names are user data. Singular keys apply exactly when their count equals 1; zero and
other integers use the general key. Album composition still includes only nonzero verdict
counts, in its existing order, separated by ` · `. A partial album uses `albums.partial`.

## 5. Status bar

| Key | English | Spanish |
|---|---|---|
| `status.ready` | Ready | Listo |
| `status.scanning` | Scanning… | Buscando archivos… |
| `status.cancelled` | Cancelled · {done} of {total} | Cancelado · {done} de {total} |
| `status.tracks` | {n} tracks | {n} pistas |
| `status.tracksOne` | 1 track | 1 pista |
| `status.real` | {n} real | {n} reales |
| `status.realOne` | 1 real | 1 real |
| `status.suspicious` | {n} suspicious | {n} sospechosas |
| `status.suspiciousOne` | 1 suspicious | 1 sospechosa |
| `status.fake` | {n} fake | {n} falsas |
| `status.fakeOne` | 1 fake | 1 falsa |
| `status.errors` | {n} errors | {n} errores |
| `status.errorsOne` | 1 error | 1 error |
| `status.shown` | {n} shown | {n} mostradas |
| `status.shownOne` | 1 shown | 1 mostrada |
| `status.fromCache` | {n} from cache | {n} de la caché |
| `status.cacheDegraded` | Results cache error: some results were not saved | Error en la caché de resultados: algunos resultados no se han guardado |
| `status.cacheUnavailable` | Results cache unavailable | Caché de resultados no disponible |
| `status.cacheCleared` | Results cache cleared | Caché de resultados borrada |
| `status.exported` | Saved {rows} rows to {file} | Se han guardado {rows} filas en {file} |
| `status.exportedOne` | Saved 1 row to {file} | Se ha guardado 1 fila en {file} |

Use singular keys exactly at count 1, general keys otherwise. During analysis compose
`done/total`, suspicious count and fake count with ` · `, preserving both zero counts. Finished
composition is tracks, real, suspicious, fake and errors, including zeros. Cancelled uses
`status.cancelled`. Append cache fields, shown count and existing notices in their already
specified order/conditions. Examples:

- English, finished: `2 tracks · 1 real · 0 suspicious · 0 fake · 1 error`.
- Spanish, finished: `2 pistas · 1 real · 0 sospechosas · 0 falsas · 1 error`.
- Spanish, running: `1/3 · 1 sospechosa · 0 falsas`.
- Spanish, cancelled: `Cancelado · 1 de 3`.

## 6. Boundaries

The catalogue covers the shipped UI plus RF-13. No untranslated app-owned English sentence
may remain in Spanish for a supported state. Do not translate user data, API enum identifiers,
raw diagnostic details, CSV headers/values, the app name or scientific units. The native folder
picker/save dialog/overwrite prompt follow Windows language; `CSV (*.csv)` stays the filter
name. Installer language and release notes are covered by the release spec. Fallback to the
English key, then raw key, may remain a defensive development behavior, but it does not satisfy
RF-13 coverage for any key listed here.
