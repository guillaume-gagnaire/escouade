<script lang="ts">
  import type { DiffLine } from '../../lib/diff';
  import { fInt } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import DiffView from '../DiffView.svelte';

  let { lines, max = 400 }: { lines: DiffLine[]; max?: number } = $props();
  const shown = $derived(lines.slice(0, max));
</script>

<div class="patch">
  {#if app.diffSplit}
    <DiffView lines={shown} split />
  {:else}
    {#each shown as l, i (i)}
      <div class="l {l.kind}">
        <span class="no">{l.kind === 'meta' ? '' : (l.newNo ?? l.oldNo ?? '')}</span>
        <span class="sign">{l.kind === 'add' ? '+' : l.kind === 'del' ? '−' : ' '}</span>
        <span class="t">{l.text}</span>
      </div>
    {/each}
  {/if}
  {#if lines.length > max}
    <div class="more">{t('conv.patch.more', { count: lines.length - max, n: fInt(lines.length - max) })}</div>
  {/if}
</div>

<style>
  /* A diff in its frame (the frame is its container's): line numbers, signs, added and removed lines tinted. */
  .patch {
    padding: 10px 0;
    font-family: var(--mono);
    font-size: 12px;
    line-height: 1.7;
    overflow: auto;
    max-height: 420px;
    background: var(--well);
    color: var(--code-fg);
  }
  .l {
    display: flex;
    white-space: pre;
    min-width: max-content;
    padding-right: 14px;
  }
  .no {
    width: 34px;
    flex: none;
    padding-right: 10px;
    text-align: right;
    color: var(--code-ln);
    user-select: none;
  }
  .sign {
    width: 16px;
    flex: none;
    color: var(--code-ln);
    user-select: none;
  }
  .add {
    background: oklch(from var(--add) l c h / 0.08);
  }
  .add .sign {
    color: var(--add);
  }
  .del {
    background: oklch(from var(--del) l c h / 0.09);
  }
  .del .sign {
    color: var(--del);
  }
  .meta {
    color: var(--code-dim);
  }
  .more {
    padding: 6px 14px 0;
    color: var(--muted);
    font-family: var(--font);
    font-size: 11.5px;
  }
</style>
