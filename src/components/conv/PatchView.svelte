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
  .patch {
    font-family: var(--mono);
    font-size: 12px;
    line-height: 1.5;
    overflow: auto;
    max-height: 420px;
    background: var(--term);
  }
  .l {
    display: flex;
    white-space: pre;
    min-width: max-content;
  }
  .no {
    width: 44px;
    flex: none;
    padding-right: 8px;
    text-align: right;
    color: var(--dim);
    user-select: none;
  }
  .sign {
    width: 16px;
    flex: none;
    color: var(--dim);
    user-select: none;
  }
  .add {
    background: color-mix(in oklch, var(--add) 14%, transparent);
  }
  .add .sign {
    color: var(--add);
  }
  .del {
    background: color-mix(in oklch, var(--del) 14%, transparent);
  }
  .del .sign {
    color: var(--del);
  }
  .meta {
    color: var(--dim);
  }
  .more {
    padding: 6px 12px;
    color: var(--dim);
    font-size: 11px;
  }
</style>
