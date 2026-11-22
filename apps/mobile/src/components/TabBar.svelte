<script lang="ts">
  // The three tabs of direction B (MO-1), at the bottom, above the system's gesture bar: À toi (what waits, its count
  // in amber), Agents, Kanban.
  import Icon from '$shared/components/Icon.svelte';
  import { t } from '$shared/lib/i18n';
  import type { IconName } from '$shared/lib/icons';
  import { goTab, nav, TABS, type Tab } from '../lib/nav.svelte';
  import { phone, waitingIn } from '../lib/phone-spaces.svelte';

  const ICON: Record<Tab, IconName> = { inbox: 'inbox', agents: 'agents', kanban: 'kanban' };
  const waiting = $derived(phone.spaces.reduce((n, s) => n + waitingIn(s), 0));

  function label(tab: Tab): string {
    if (tab === 'inbox' && waiting) return t('mobile.tabs.inboxWaiting', { count: waiting });
    return t(`mobile.tabs.${tab}`);
  }

  /** The arrows move between the tabs, as a tab list does. */
  function onKey(e: KeyboardEvent, i: number) {
    const to = e.key === 'ArrowRight' ? i + 1 : e.key === 'ArrowLeft' ? i - 1 : null;
    if (to === null) return;
    e.preventDefault();
    const tab = TABS[(to + TABS.length) % TABS.length];
    goTab(tab);
    document.getElementById(`tab-${tab}`)?.focus();
  }
</script>

<nav class="bar">
  <div class="tabs" role="tablist" aria-label={t('mobile.tabs.label')}>
    {#each TABS as tab, i (tab)}
      <button
        type="button"
        role="tab"
        id="tab-{tab}"
        aria-selected={nav.tab === tab}
        aria-controls="panel"
        aria-label={label(tab)}
        tabindex={nav.tab === tab ? 0 : -1}
        onclick={() => goTab(tab)}
        onkeydown={(e) => onKey(e, i)}
      >
        <span class="ic">
          <Icon name={ICON[tab]} size={22} />
          {#if tab === 'inbox' && waiting}<span class="badge" aria-hidden="true">{waiting}</span>{/if}
        </span>
        <span class="lb" aria-hidden="true">{t(`mobile.tabs.${tab}`)}</span>
      </button>
    {/each}
  </div>
</nav>

<style>
  .bar {
    flex: none;
    background: var(--frame);
    box-shadow: inset 0 1px 0 var(--line);
    padding: 4px calc(6px + var(--safe-right)) var(--safe-bottom) calc(6px + var(--safe-left));
  }
  .tabs {
    display: flex;
    gap: var(--gap-target);
  }
  button {
    flex: 1;
    min-width: var(--target);
    min-height: max(var(--target), var(--tabbar) - 4px);
    border: 0;
    border-radius: var(--r-m);
    background: transparent;
    display: grid;
    justify-items: center;
    align-content: center;
    gap: 3px;
    color: var(--muted);
    cursor: pointer;
  }
  button[aria-selected='true'] {
    color: var(--text);
  }
  button[aria-selected='true'] .ic {
    color: var(--p-ink);
  }
  .ic {
    position: relative;
  }
  .lb {
    font-size: 12px;
    font-weight: 600;
  }
  .badge {
    position: absolute;
    top: -6px;
    left: calc(100% - 4px);
    min-width: 19px;
    height: 19px;
    padding: 0 5px;
    border-radius: 10px;
    background: var(--wait);
    color: var(--on-wait);
    font-size: 11.5px;
    font-weight: 700;
    display: inline-grid;
    place-items: center;
    font-variant-numeric: tabular-nums;
  }
</style>
