<script lang="ts">
  // The spaces in the projects' rail (ESC-537, direction B « les piles », S1): once this computer knows a server, the
  // open space heads its projects, and each other one waits under « + », a click away (the way back to this computer).
  // One server at a time here; the piles of several spaces come with MS-5.
  import { connection } from '../../lib/connection.svelte';
  import { t } from '../../lib/i18n';
  import { goHome, goToServer, servers } from '../../lib/servers.svelte';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import SpacePlate from './SpacePlate.svelte';

  let { part }: { part: 'head' | 'others' } = $props();

  const here = $derived(connection.server);
  const cut = $derived(here !== null && connection.state !== 'open');
  /** The other spaces: this computer when on a server, else the servers known. */
  const others = $derived(
    here
      ? [{ id: null as string | null, name: t('servers.thisComputer'), short: t('servers.thisComputer') }]
      : (servers.view?.servers ?? []).map((s) => ({ id: s.id as string | null, name: s.name, short: s.name })),
  );
  let busy = $state(false);

  async function go(id: string | null) {
    if (busy) return;
    busy = true;
    try {
      if (id === null) await goHome();
      else if (await goToServer(id)) app.modal = { kind: 'settings', page: 'servers' };
    } finally {
      busy = false;
    }
  }
</script>

{#if part === 'head'}
  <div
    class="space head"
    role="status"
    aria-label={t('servers.rail.current', { name: here ? here.name : t('servers.thisComputer') })}
    data-testid="rail-space"
  >
    <SpacePlate name={here?.name ?? ''} here={!here} {cut} size={26} />
    <span class="nm" class:cut>{here ? here.name : t('servers.thisComputer')}</span>
  </div>
{:else}
  <div class="sep"></div>
  <div class="others" role="navigation" aria-label={t('servers.rail.spaces')}>
    {#each others as o (o.id ?? 'here')}
      <button
        class="space pile"
        disabled={busy}
        aria-label={o.id === null ? t('servers.backHome') : t('servers.goTo', { name: o.name })}
        use:tip={{ title: o.id === null ? t('servers.backHome') : t('servers.goTo', { name: o.name }), side: 'right', aria: false }}
        onclick={() => void go(o.id)}
        data-testid="rail-space-other"
      >
        <SpacePlate name={o.name} here={o.id === null} size={26} />
        <span class="nm">{o.short}</span>
      </button>
    {/each}
  </div>
{/if}

<style>
  .space {
    display: grid;
    justify-items: center;
    gap: 3px;
    width: 100%;
    padding: 4px 0;
    border: 0;
    border-radius: 8px;
    background: none;
    color: var(--text);
  }
  .pile {
    cursor: pointer;
  }
  .pile:hover {
    background: var(--hover);
  }
  .nm {
    max-width: calc(var(--rail, 52px) - 6px);
    overflow: hidden;
    font-size: 10.5px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .nm.cut {
    color: var(--wait);
  }
  .sep {
    width: 22px;
    height: 1px;
    margin: 6px auto;
    background: var(--line-2);
  }
  .others {
    display: grid;
    gap: 4px;
    width: 100%;
  }
</style>
