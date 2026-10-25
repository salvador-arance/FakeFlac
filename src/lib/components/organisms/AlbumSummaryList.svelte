<script lang="ts">
  import VerdictCell from '../molecules/VerdictCell.svelte'
  import { t } from '../../i18n'
  import type { AlbumUpdate } from '../../model'

  type Props = {
    /** Latest update of each album, sorted by `index`. */
    albums: AlbumUpdate[]
    /** Album index of the selected row, or null. */
    currentIndex: number | null
    onselect?: (albumIndex: number) => void
  }

  let { albums, currentIndex, onselect }: Props = $props()

  let listElement: HTMLElement | undefined = $state()

  let title = $derived(
    albums.length > 0 ? t('albums.titleCount', { count: albums.length }) : t('albums.title'),
  )

  /** "12 tracks · 11 real · 1 fake": the track part, then the non-zero counts (album-summary.md). */
  function countsText(album: AlbumUpdate): string {
    const s = album.summary
    const parts: string[] = []
    if (s.total < album.files) parts.push(t('albums.partial', { done: s.total, files: album.files }))
    else parts.push(s.total === 1 ? t('albums.tracksOne') : t('albums.tracks', { n: s.total }))
    if (s.real > 0) parts.push(t('albums.real', { n: s.real }))
    if (s.suspicious > 0) parts.push(t('albums.suspicious', { n: s.suspicious }))
    if (s.fake > 0) parts.push(t('albums.fake', { n: s.fake }))
    if (s.errors > 0) {
      parts.push(s.errors === 1 ? t('albums.errorsOne') : t('albums.errors', { n: s.errors }))
    }
    return parts.join(' · ')
  }

  function label(album: AlbumUpdate): string {
    return [album.name, t(`verdict.${album.summary.verdict}`), countsText(album)].join(', ')
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (albums.length === 0) return
    const position = albums.findIndex((a) => a.index === currentIndex)
    let target: number
    if (e.key === 'ArrowDown') target = position < 0 ? 0 : Math.min(albums.length - 1, position + 1)
    else if (e.key === 'ArrowUp') target = position < 0 ? albums.length - 1 : Math.max(0, position - 1)
    else if (e.key === 'Home') target = 0
    else if (e.key === 'End') target = albums.length - 1
    else return
    e.preventDefault()
    onselect?.(albums[target].index)
  }

  // keep the current item visible in the panel's own scroll
  $effect(() => {
    if (currentIndex === null || !listElement) return
    listElement
      .querySelector(`[data-album-index="${currentIndex}"]`)
      ?.scrollIntoView?.({ block: 'nearest' })
  })
</script>

<div class="album-panel">
  <div class="header">{title}</div>
  <!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
  <ul
    bind:this={listElement}
    class="list"
    role="listbox"
    tabindex="0"
    aria-label={t('albums.label')}
    aria-activedescendant={currentIndex !== null && albums.some((a) => a.index === currentIndex)
      ? `album-option-${currentIndex}`
      : undefined}
    onkeydown={handleKeyDown}
  >
    {#each albums as album (album.index)}
      {@const counts = countsText(album)}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <li
        id="album-option-{album.index}"
        class="item"
        class:current={album.index === currentIndex}
        role="option"
        aria-selected={album.index === currentIndex ? 'true' : 'false'}
        aria-label={label(album)}
        data-album-index={album.index}
        onclick={() => onselect?.(album.index)}
      >
        <div class="name" title={album.summary.folder}>{album.name}</div>
        <div class="line">
          <VerdictCell verdict={album.summary.verdict} />
          <span class="counts" title={counts}>{counts}</span>
        </div>
      </li>
    {/each}
  </ul>
</div>

<style>
  .album-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    font-family: var(--font-ui);
    font-size: var(--font-size);
    color: var(--color-text);
  }

  .header {
    flex-shrink: 0;
    padding: var(--space-1) var(--space-2);
    color: var(--color-text-muted);
    border-bottom: 1px solid var(--color-border);
  }

  .list {
    flex: 1;
    min-height: 0;
    margin: 0;
    padding: 0;
    list-style: none;
    overflow-y: auto;
    outline: none;
  }

  .list:focus-visible {
    outline: 1px solid var(--color-accent);
    outline-offset: -1px;
  }

  .item {
    height: calc(2 * var(--row-height));
    box-sizing: border-box;
    padding: 0 var(--space-2);
    display: flex;
    flex-direction: column;
    justify-content: center;
    cursor: pointer;
    overflow: hidden;
  }

  .item:hover {
    background: var(--color-surface-raised);
  }

  .item.current {
    background: var(--color-row-selected);
  }

  .list:focus-visible .item.current {
    outline: 1px solid var(--color-accent);
    outline-offset: -1px;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .line {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }

  .counts {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--color-text-muted);
  }
</style>
