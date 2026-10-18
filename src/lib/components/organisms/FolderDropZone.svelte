<script lang="ts">
  import { t } from '../../i18n'

  type Props = {
    active: boolean
    empty: boolean
  }

  let { active, empty }: Props = $props()
</script>

{#if empty || active}
  <div
    class="folder-drop-zone"
    class:active
    class:empty
    data-testid="folder-drop-zone"
    data-active={active ? 'true' : 'false'}
  >
    {#if empty && !active}
      <div class="hint-container">
        <p class="hint">{t('empty.hint')}</p>
      </div>
    {/if}
  </div>
{/if}

<style>
  .folder-drop-zone {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    box-sizing: border-box;
  }

  .folder-drop-zone.active {
    position: absolute;
    inset: 0;
    z-index: 50;
    background: rgba(232, 137, 43, 0.08);
    border: 2px dashed var(--color-accent);
    pointer-events: none;
  }

  .hint-container {
    text-align: center;
    color: var(--color-text-muted);
    font-size: var(--font-size);
  }

  .hint {
    margin: 0;
    font-size: 14px;
  }
</style>
