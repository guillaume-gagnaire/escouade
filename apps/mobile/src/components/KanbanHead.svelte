<script lang="ts">
  // The head of « Kanban » (MO-7, direction B): the project shown with its space (its plate, its dot, its name; touched,
  // the sheet of the projects), and the column chosen in segments, each with its count.
  import Icon from '$shared/components/Icon.svelte';
  import SpacePlate from '$shared/components/servers/SpacePlate.svelte';
  import { t } from '$shared/lib/i18n';
  import type { Column } from '$shared/lib/types';
  import { kanban, segmentsOf, shownColumn, showColumn } from '../lib/board.svelte';
  import { openSheet } from '../lib/nav.svelte';
  import { phone } from '../lib/phone-spaces.svelte';

  const space = $derived(phone.spaces.find((s) => s.id === kanban.choice?.space) ?? null);
  const project = $derived(kanban.lists[kanban.choice?.space ?? '']?.find((p) => p.id === kanban.choice?.project) ?? null);
  const counts = $derived(kanban.board?.counts ?? null);
  const shown = $derived(counts ? shownColumn(counts, kanban.column) : null);

  // The arrows move between the segments, as in a radio group.
  function onKey(e: KeyboardEvent, columns: Column[]) {
    const i = shown ? columns.indexOf(shown) : -1;
    const step = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
    if (!step || i < 0) return;
    e.preventDefault();
    const next = columns[(i + step + columns.length) % columns.length];
    showColumn(next);
    (e.currentTarget as HTMLElement).querySelector<HTMLElement>(`[data-column="${next}"]`)?.focus();
  }
</script>

{#if space && project}
  <button
    type="button"
    class="pick"
    aria-haspopup="dialog"
    aria-label={t('mobile.kanban.picker', { project: project.name, space: space.name })}
    onclick={() => openSheet('project')}
  >
    <SpacePlate name={space.name} here={space.here} size={22} />
    <i class="dot" style:background={project.color} aria-hidden="true"></i>
    <span class="nm">{project.name}</span>
    <span class="sp">{space.name}</span>
    <Icon name="down" size={16} />
  </button>
{/if}
{#if counts}
  {@const columns = segmentsOf(counts)}
  <!-- svelte-ignore a11y_interactive_supports_focus -->
  <div class="seg" role="radiogroup" aria-label={t('mobile.kanban.columns')} onkeydown={(e) => onKey(e, columns)}>
    {#each columns as c (c)}
      <button
        type="button"
        role="radio"
        data-column={c}
        aria-checked={shown === c}
        tabindex={shown === c ? 0 : -1}
        aria-label={t('mobile.kanban.segment', { column: t(`board.columns.${c}`), count: counts[c] })}
        onclick={() => showColumn(c)}
      >
        <span class="cl">{t(`board.columns.${c}`)}</span>
        <span class="n">{counts[c]}</span>
      </button>
    {/each}
  </div>
{/if}

<style>
  .pick {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: var(--target);
    min-width: 0;
    padding: 0 12px;
    border: 0;
    border-radius: var(--r-m);
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font-size: var(--body);
    font-weight: 620;
    cursor: pointer;
    text-align: left;
  }
  .pick:active {
    background: var(--hover);
  }
  .dot {
    flex: none;
    width: 9px;
    height: 9px;
    border-radius: 3px;
  }
  .nm {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sp {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
    font-weight: 500;
    font-size: var(--small);
  }
  .pick :global(svg) {
    flex: none;
    color: var(--muted);
  }
  .seg {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: var(--r-m);
    box-shadow: inset 0 0 0 1px var(--line-2);
    background: rgba(var(--fg-rgb), 0.03);
  }
  .seg button {
    flex: 1 1 0;
    min-width: 0;
    min-height: calc(var(--target) - 6px);
    border: 0;
    border-radius: calc(var(--r-m) - 3px);
    padding: 0 2px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    background: transparent;
    color: var(--muted);
    font-size: 13.5px;
    letter-spacing: -0.01em;
    font-weight: 600;
    cursor: pointer;
  }
  .seg button[aria-checked='true'] {
    background: var(--raise-2);
    box-shadow: inset 0 0 0 1px var(--line-3);
    color: var(--text);
  }
  .cl {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .n {
    flex: none;
    font-variant-numeric: tabular-nums;
    opacity: 0.75;
    font-weight: 500;
  }
</style>
