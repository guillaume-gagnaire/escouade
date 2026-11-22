<script lang="ts">
  import Icon from '../Icon.svelte';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { ceilingLine, parallelMax, parallelWarning, withinBound } from '../../lib/parallel';
  import type { ParallelCapacity } from '../../lib/types';
  import Row from './Row.svelte';

  // « En parallèle » of a Kanban (ESC-620): from 1 to its bound (16 on a desktop, 64 on a server, lower under a ceiling
  // imposed), the bound asked of the core that runs the Kanban (the server's, for a server's space). Under it, the
  // ceiling and who imposes it, and beyond 6 what the machine holds and that the quota goes faster: an indication,
  // never a refusal. `projectId` null: a project to come (the assistant), under the device's ceiling alone.
  let { value = $bindable(), projectId }: { value: number; projectId: string | null } = $props();

  let cap = $state<ParallelCapacity | null>(null);
  $effect(() => {
    const id = projectId;
    let gone = false;
    api
      .boardCapacity(id)
      .then((c) => {
        if (!gone) cap = c;
      })
      .catch(() => {
        // An older core, or a space that does not answer: a desktop's bound, nothing said.
      });
    return () => {
      gone = true;
    };
  });

  const max = $derived(parallelMax(cap));
  const ceiling = $derived(ceilingLine(cap));
  const warning = $derived(parallelWarning(value, cap));
</script>

<Row label={t('boardSettings.tab.parallel')} desc={t('boardSettings.tab.parallelDesc')}>
  {#snippet extra()}
    {#if ceiling}<p class="note" data-parallel-ceiling><Icon name="lock" size={13} />{ceiling}</p>{/if}
    {#if warning}<p class="note warn" data-parallel-warning>{warning}</p>{/if}
  {/snippet}
  <input
    class="field input num mono"
    type="number"
    min="1"
    {max}
    aria-label={t('boardSettings.tab.parallel')}
    value={Math.min(value, max)}
    oninput={(e) => {
      // Taken as it is typed when it is a number within the bound; the field made right when it is left.
      const typed = e.currentTarget.value.trim();
      const n = Number(typed);
      if (typed && n === withinBound(n, cap)) value = n;
    }}
    onchange={(e) => {
      const n = withinBound(Number(e.currentTarget.value.trim() || NaN), cap);
      value = n;
      e.currentTarget.value = String(n);
    }}
  />
  <span class="hint">{t('boardSettings.tab.parallelRange', { max })}</span>
</Row>

<style>
  .num {
    width: 64px;
    max-width: 100%;
    flex: 0 1 auto;
    text-align: right;
  }
  .hint {
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
  }
  .note {
    display: flex;
    gap: 6px;
    align-items: flex-start;
    margin: 2px 0 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.45;
  }
  .note.warn {
    color: var(--warn-ink);
  }
</style>
