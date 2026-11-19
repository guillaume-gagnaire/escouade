<script lang="ts">
  // The spaces in the projects' rail (ESC-537, then ESC-539: direction B « les piles », S1). Once this computer knows a
  // server, the open space heads its projects (its plate and its name); under « + », each other space is a pile — its
  // projects as cards set off one behind the other, its plate in front, what waits there in a dot — a click away
  // (`Alt+n`, `⌃n` on macOS). Beyond three spaces, or at 1024 px, the piles pack into plain plates, so that the rail
  // never scrolls for them (safeguard 3). A right click (or the menu key) opens the space's menu: in its own window,
  // « Réessayer » while cut, its settings, removing it.
  import { connection } from '../../lib/connection.svelte';
  import { t } from '../../lib/i18n';
  import { menu, type MenuItem } from '../../lib/menu.svelte';
  import { IS_MAC } from '../../lib/platform';
  import {
    compactPiles,
    goToSpace,
    openWindow,
    retrySpace,
    spaceList,
    spacesState,
    stateOf,
    waitingIn,
    type Space,
  } from '../../lib/spaces.svelte';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import { windowSpace } from '../../lib/transport';
  import SpacePlate from './SpacePlate.svelte';

  let { part }: { part: 'head' | 'others' } = $props();

  let width = $state(typeof window === 'undefined' ? 1440 : window.innerWidth);
  const list = $derived(spaceList());
  const here = $derived(list.find((s) => s.current) ?? list[0]);
  const others = $derived(list.filter((s) => !s.current));
  const compact = $derived(compactPiles(list.length, width));
  const cut = $derived(connection.server !== null && connection.state !== 'open');
  let busy = $state(false);

  /** « Alt 2 », « ⌃2 ». */
  const keyOf = (s: Space) => (IS_MAC ? `⌃${s.index}` : `Alt ${s.index}`);

  async function go(s: Space) {
    if (busy) return;
    busy = true;
    try {
      if (await goToSpace(s.id)) app.modal = { kind: 'settings', page: 'servers' };
    } finally {
      busy = false;
    }
  }

  function items(s: Space): MenuItem[] {
    const out: MenuItem[] = [];
    if (!s.current) out.push({ label: t('spaces.menu.go', { name: s.name }), icon: 'right', hint: keyOf(s), onClick: () => void go(s) });
    out.push({ label: t('spaces.menu.window'), icon: 'ext', onClick: () => void openWindow(s.id) });
    if (!s.here && stateOf(s) !== 'open')
      out.push({ label: t('spaces.menu.retry'), icon: 'refresh', onClick: () => void retrySpace(s.id) });
    if (s.current && !s.here) {
      out.push({
        label: t('spaces.menu.settings', { name: s.name }),
        icon: 'sliders',
        onClick: () => (app.modal = { kind: 'settings', page: 'devices' }),
      });
    }
    if (!s.here) {
      out.push({ label: '', separator: true });
      out.push({
        label: t('spaces.menu.remove', { name: s.name }),
        icon: 'trash',
        danger: true,
        onClick: () => (app.modal = { kind: 'settings', page: 'servers' }),
      });
    }
    return out;
  }

  function openMenu(e: MouseEvent | KeyboardEvent, s: Space) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    e.preventDefault();
    menu.open = { x: r.right + 10, y: Math.max(8, r.top - 6), items: items(s) };
  }

  function onKey(e: KeyboardEvent, s: Space) {
    if (e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) openMenu(e, s);
  }
</script>

<svelte:window bind:innerWidth={width} />

{#if part === 'head'}
  <!-- The open space: its menu on a click (in its own window, its settings, removing it). -->
  <button
    class="space head"
    aria-label={t('servers.rail.current', { name: here.name })}
    aria-haspopup="menu"
    data-testid="rail-space"
    onclick={(e) => openMenu(e, here)}
    oncontextmenu={(e) => openMenu(e, here)}
  >
    <SpacePlate name={here.name} here={here.here} {cut} size={26} />
    <span class="nm" class:cut>{here.here ? here.short : here.name}</span>
  </button>
{:else if !windowSpace}
  <div class="sep"></div>
  <nav class="others" class:compact aria-label={t('servers.rail.spaces')} data-testid="rail-piles" data-compact={compact}>
    {#each others as s (s.id)}
      {@const n = waitingIn(s)}
      {@const off = stateOf(s) !== 'open'}
      {@const colors = spacesState.watched[s.id]?.glance?.colors ?? []}
      <button
        class="space pile"
        disabled={busy}
        aria-label={off ? t('spaces.pileCut', { name: s.name, key: keyOf(s) }) : t('spaces.pile', { name: s.name, n, key: keyOf(s) })}
        aria-keyshortcuts={IS_MAC ? `Control+${s.index}` : `Alt+${s.index}`}
        use:tip={{ title: t('servers.goTo', { name: s.name }), rows: [keyOf(s)], side: 'right', aria: false }}
        onclick={() => void go(s)}
        oncontextmenu={(e) => openMenu(e, s)}
        onkeydown={(e) => onKey(e, s)}
        data-testid="rail-space-other"
        data-space={s.id}
      >
        <span class="stack">
          {#if !compact}
            {#each colors.slice(0, 2) as c, i (i)}<i style:background={c} style:left="{i * 5}px" style:top="{i * 5}px"></i>{/each}
          {/if}
          <span class="mon"><SpacePlate name={s.name} here={s.here} cut={off} size={26} /></span>
          {#if n > 0}<span class="badge" data-testid="pile-count">{n}</span>{/if}
        </span>
        <span class="nm" class:cut={off}>{s.short}</span>
      </button>
    {/each}
  </nav>
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
  .head {
    cursor: pointer;
  }
  .head:hover {
    background: var(--hover);
  }
  .head:focus-visible {
    outline: 2px solid var(--focus, var(--accent));
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
    color: var(--soft);
    font-size: 10px;
    font-weight: 650;
    line-height: 1;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .head .nm {
    color: var(--text);
    font-size: 10.5px;
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
    gap: 6px;
    width: 100%;
  }
  /* A pile: two of its projects as cards set off behind its plate. */
  .stack {
    position: relative;
    width: 36px;
    height: 36px;
  }
  .stack i {
    position: absolute;
    width: 26px;
    height: 26px;
    border-radius: 7px;
    box-shadow: 0 0 0 1.5px var(--frame, var(--bg));
  }
  .mon {
    position: absolute;
    left: 10px;
    top: 10px;
    display: grid;
    border-radius: 8px;
    box-shadow: 0 0 0 1.5px var(--frame, var(--bg));
  }
  .badge {
    position: absolute;
    top: -5px;
    right: -7px;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: 8px;
    background: var(--wait);
    color: #101114;
    font-size: 10px;
    font-weight: 700;
    line-height: 16px;
    text-align: center;
  }
  /* Packed: the plate alone (beyond three spaces, at 1024). */
  .compact {
    gap: 2px;
  }
  .compact .stack {
    width: 26px;
    height: 26px;
  }
  .compact .mon {
    left: 0;
    top: 0;
  }
  .compact .pile {
    padding: 3px 0;
  }
</style>
