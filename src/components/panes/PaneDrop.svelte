<script lang="ts">
  // The drag from the sidebar under way (ESC-392, direction B): a card that follows the pointer, and on the pane under
  // it a compass of five cells (top, left, replace, right, bottom) whose cell under the pointer lights up, the place it
  // would take drawn behind. A side with no room left is crossed out. Nothing here takes the pointer.
  import Icon from '../Icon.svelte';
  import { t } from '../../lib/i18n';
  import type { IconName } from '../../lib/icons';
  import { paneDrag } from '../../lib/pane-drag.svelte';
  import type { PaneZone } from '../../lib/panes';

  const CELLS: { zone: PaneZone; icon: IconName; row: number; col: number }[] = [
    { zone: 'top', icon: 'pane-t', row: 1, col: 2 },
    { zone: 'left', icon: 'pane-l', row: 2, col: 1 },
    { zone: 'center', icon: 'pane-c', row: 2, col: 2 },
    { zone: 'right', icon: 'pane-r', row: 2, col: 3 },
    { zone: 'bottom', icon: 'pane-b', row: 3, col: 2 },
  ];

  const target = $derived(paneDrag.target);
  /** The place the content would take, in the pane's rectangle. */
  const preview = $derived.by(() => {
    if (!target?.zone) return null;
    const { left, top, width: w, height: h } = target.rect;
    switch (target.zone) {
      case 'left':
        return { left, top, w: w / 2, h };
      case 'right':
        return { left: left + w / 2, top, w: w / 2, h };
      case 'top':
        return { left, top, w, h: h / 2 };
      case 'bottom':
        return { left, top: top + h / 2, w, h: h / 2 };
      default:
        return { left, top, w, h };
    }
  });
  /** Near the window's right edge, the card goes to the left of the pointer, so that it stays in the window. */
  const flip = $derived(paneDrag.x > window.innerWidth - 300);
  const icon = $derived<IconName>(paneDrag.item?.kind === 'agent' ? 'agents' : paneDrag.item?.kind === 'editor' ? 'code' : 'term');
</script>

{#if paneDrag.on}
  <div class="layer" aria-hidden="true" data-testid="pane-drop">
    {#if preview && target}
      <div
        class="preview"
        style:left="{preview.left}px"
        style:top="{preview.top}px"
        style:width="{preview.w}px"
        style:height="{preview.h}px"
      >
        <span class="lbl">{paneDrag.label}<small> · {t(`shell.panes.zone.${target.zone!}`)}</small></span>
      </div>
    {/if}
    {#if target}
      <div
        class="compass"
        style:left="{target.rect.left + target.rect.width / 2}px"
        style:top="{target.rect.top + target.rect.height / 2}px"
      >
        {#each CELLS as c (c.zone)}
          <span
            class="cell"
            class:hot={target.aimed === c.zone && target.fits[c.zone]}
            class:off={!target.fits[c.zone]}
            data-zone={c.zone}
            style:grid-row={c.row}
            style:grid-column={c.col}><Icon name={c.icon} size={22} /></span
          >
        {/each}
      </div>
      <div class="hint" style:left="{target.rect.left + target.rect.width / 2}px" style:top="{target.rect.top + target.rect.height - 52}px">
        {#if !target.zone}
          <span class="full">{t('shell.panes.full')}</span>
        {:else}
          <span>{t('shell.panes.hintSides')}</span><span>{t('shell.panes.hintMiddle')}</span>
        {/if}
        <span>{t('shell.panes.hintCancel')}</span>
      </div>
    {/if}
    <div class="ghost" class:flip style:left="{paneDrag.x + (flip ? -14 : 14)}px" style:top="{paneDrag.y + 12}px">
      <Icon name={icon} size={14} /><b>{paneDrag.label}</b>{#if paneDrag.detail}<small>{paneDrag.detail}</small>{/if}
    </div>
  </div>
{/if}

<style>
  .layer {
    position: fixed;
    inset: 0;
    z-index: 900;
    pointer-events: none;
  }
  .preview {
    position: fixed;
    border-radius: 12px;
    background: color-mix(in oklch, var(--p) 12%, transparent);
    box-shadow: inset 0 0 0 1.5px color-mix(in oklch, var(--p-ink) 75%, transparent);
    transition:
      left 0.12s var(--ease),
      top 0.12s var(--ease),
      width 0.12s var(--ease),
      height 0.12s var(--ease);
  }
  .lbl {
    position: absolute;
    top: 14px;
    left: 50%;
    transform: translateX(-50%);
    max-width: calc(100% - 24px);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 5px 11px;
    border-radius: 8px;
    background: var(--drop-base);
    box-shadow: 0 0 0 1px var(--line);
    font-weight: 600;
    font-size: 12.5px;
    color: var(--text);
  }
  .lbl small {
    color: var(--muted);
    font-weight: 500;
    font-size: 12px;
  }
  .compass {
    position: fixed;
    transform: translate(-50%, -50%);
    display: grid;
    grid-template-columns: repeat(3, 44px);
    grid-template-rows: repeat(3, 44px);
    gap: 5px;
    padding: 8px;
    border-radius: 14px;
    background: var(--drop-base);
    box-shadow:
      0 0 0 1px var(--line),
      0 16px 40px -10px rgba(0, 0, 0, calc(0.7 * var(--shade)));
  }
  .cell {
    display: grid;
    place-items: center;
    border-radius: 8px;
    background: var(--raise);
    box-shadow: 0 0 0 1px var(--line);
    color: var(--muted);
  }
  .cell.hot {
    background: color-mix(in oklch, var(--p) 32%, var(--raise));
    box-shadow: 0 0 0 1px var(--p-ink);
    color: var(--text);
  }
  .cell.off {
    opacity: 0.35;
  }
  .hint {
    position: fixed;
    transform: translateX(-50%);
    display: flex;
    gap: 14px;
    white-space: nowrap;
    padding: 6px 12px;
    border-radius: 8px;
    background: var(--drop-base);
    box-shadow: 0 0 0 1px var(--line);
    font-size: 12px;
    color: var(--muted);
  }
  .hint .full {
    color: var(--warn-ink);
  }
  .ghost {
    position: fixed;
    display: flex;
    align-items: center;
    gap: 7px;
    max-width: 280px;
    padding: 6px 11px 6px 9px;
    border-radius: 9px;
    background: var(--raise);
    box-shadow:
      0 0 0 1px var(--line),
      0 14px 34px -8px rgba(0, 0, 0, calc(0.7 * var(--shade)));
    color: var(--soft);
    font-size: 13px;
    transform: rotate(-1.5deg);
  }
  .ghost.flip {
    transform: translateX(-100%) rotate(-1.5deg);
  }
  .ghost b {
    color: var(--text);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ghost small {
    color: var(--muted);
    font-size: 11.5px;
    white-space: nowrap;
  }
</style>
