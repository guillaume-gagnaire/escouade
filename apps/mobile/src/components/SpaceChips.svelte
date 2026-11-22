<script lang="ts">
  // The chips of the spaces (MO-1, direction B): « Partout » and each space, its plate and what waits there; a space
  // that is cut says it (an icon and a word for a screen reader, never the colour alone). One chip at a time: the
  // thread and the agents show its spaces.
  import Icon from '$shared/components/Icon.svelte';
  import SpacePlate from '$shared/components/servers/SpacePlate.svelte';
  import { t } from '$shared/lib/i18n';
  import { ALL, phone, stateOf, waitingIn } from '../lib/phone-spaces.svelte';

  const total = $derived(phone.spaces.reduce((n, s) => n + waitingIn(s), 0));

  function labelOf(s: (typeof phone.spaces)[number]): string {
    const state = stateOf(s);
    if (state === 'cut' || state === 'refused') return t('mobile.spaces.chipCut', { name: s.name });
    if (state === 'connecting') return t('mobile.spaces.chipConnecting', { name: s.name });
    return t('mobile.spaces.chip', { name: s.name, count: waitingIn(s) });
  }
</script>

<div class="chips" role="radiogroup" aria-label={t('mobile.spaces.label')}>
  <button
    type="button"
    role="radio"
    class="chip"
    aria-checked={phone.filter === ALL}
    aria-label={t('mobile.spaces.everywhereLabel', { count: total })}
    onclick={() => (phone.filter = ALL)}
  >
    {t('mobile.spaces.everywhere')} <span class="c">{total}</span>
  </button>
  {#each phone.spaces as s (s.id)}
    {@const state = stateOf(s)}
    {@const cut = state === 'cut' || state === 'refused'}
    <button
      type="button"
      role="radio"
      class="chip"
      class:cut
      class:connecting={state === 'connecting'}
      data-space={s.id}
      data-state={state}
      aria-checked={phone.filter === s.id}
      aria-label={labelOf(s)}
      onclick={() => (phone.filter = s.id)}
    >
      <SpacePlate name={s.name} here={s.here} {cut} size={22} />
      <span class="name">{s.name}</span>
      {#if cut}<Icon name="unlink" size={14} />{:else if state === 'open'}<span class="c">{waitingIn(s)}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .chips {
    display: flex;
    gap: 6px;
    overflow-x: auto;
    margin: 0 calc(-1 * var(--gut));
    padding: 2px var(--gut) 4px;
    scrollbar-width: none;
    scroll-padding-inline: var(--gut);
  }
  .chips::-webkit-scrollbar {
    display: none;
  }
  .chip {
    flex: none;
    min-height: var(--target);
    border: 0;
    border-radius: var(--r-m);
    padding: 0 12px;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
    font-size: 14.5px;
    font-weight: 600;
    white-space: nowrap;
    cursor: pointer;
  }
  .chip[aria-checked='true'] {
    background: var(--primary);
    color: var(--on-primary);
    box-shadow: none;
  }
  .c {
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .chip[aria-checked='true'] .c {
    color: inherit;
    opacity: 0.7;
  }
  .chip.cut {
    color: var(--wait);
  }
  .chip.connecting .name {
    opacity: 0.6;
  }
</style>
