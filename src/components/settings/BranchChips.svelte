<script lang="ts">
  import { tick } from 'svelte';
  import { api } from '../../lib/ipc';

  // One of the project's local branches, each a chip, in a list a few rows high that scrolls past them: a repository may
  // have hundreds. `first`: a choice ahead of them that is no branch (« Branche courante »), chosen while `value` is
  // empty. The branch chosen is always among the chips: before the branches are read, and once it is gone.
  let {
    projectId,
    value,
    label,
    first,
    onpick,
  }: { projectId: string; value: string; label: string; first?: string; onpick: (branch: string) => void } = $props();

  /** The local branches, by project. */
  let read = $state<Record<string, string[]>>({});
  let list = $state<HTMLDivElement>();

  $effect(() => {
    const id = projectId;
    if (read[id]) return;
    api
      .gitBranches(id)
      .then((branches) => {
        read[id] = branches ?? [];
      })
      .catch(() => {});
  });

  const branches = $derived.by(() => {
    const known = read[projectId] ?? [];
    return !value || known.includes(value) ? known : [value, ...known];
  });

  // The chip chosen is in sight once the branches are read: it may be rows below the first ones. Only the list is
  // scrolled, never the settings around it.
  $effect(() => {
    void [branches, value];
    void tick().then(() => {
      const on = list?.querySelector<HTMLElement>('.chip.on');
      if (!list || !on) return;
      const top = on.offsetTop - list.offsetTop;
      if (top < list.scrollTop || top + on.offsetHeight > list.scrollTop + list.clientHeight) list.scrollTop = top;
    });
  });
</script>

<div class="branches" role="group" aria-label={label} bind:this={list}>
  {#if first}
    <button class="chip" class:on={!value} aria-pressed={!value} onclick={() => onpick('')}>{first}</button>
  {/if}
  {#each branches as br (br)}
    <button class="chip mono" class:on={value === br} aria-pressed={value === br} onclick={() => onpick(br)}>⎇ {br}</button>
  {/each}
</div>

<style>
  .branches {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    align-content: flex-start;
    gap: 6px;
    /* Three rows and the top of a fourth, which says there is more to scroll to. */
    max-height: 114px;
    overflow-y: auto;
    /* Room for the focus ring of the chips at its edges. */
    padding: 2px;
  }
  .chip {
    height: 28px;
    max-width: 100%;
    padding: 0 11px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: pointer;
  }
  .chip:hover:not(.on) {
    color: var(--text);
  }
  .chip.mono {
    font-family: var(--mono);
    font-size: 11.5px;
  }
  .chip.on {
    border-color: var(--accent);
    background: var(--elev2);
    color: var(--text);
  }
</style>
