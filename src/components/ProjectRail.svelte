<script lang="ts">
  import { untrack } from 'svelte';
  import { fDate } from '../lib/format';
  import { t } from '../lib/i18n';
  import { api } from '../lib/ipc';
  import { menu, type MenuItem } from '../lib/menu.svelte';
  import { keyLabel } from '../lib/platform';
  import { dropProject, moveProject, projectMenu, tileState } from '../lib/project-rail';
  import { RAIL_MIN, railWidth } from '../lib/rail-width.svelte';
  import { app } from '../lib/state.svelte';
  import { tip } from '../lib/tip';
  import type { Project } from '../lib/types';
  import Icon from './Icon.svelte';
  import ProjectTile from './ProjectTile.svelte';
  import Splitter from './Splitter.svelte';

  // The projects' rail (ESC-158), as Slack's workspaces, from the title bar to the status bar: « Vue d'ensemble » on top,
  // a tile per open project (the list scrolls when they are many, the active one kept in view, a pill for those out of
  // view that wait), « + » (the assistant, or a menu with the closed projects), then the stats and the settings. At the
  // first start, « + » and the settings alone. Its right edge sets its width (ESC-168): from 160 px, every tile has its name
  // on its right, a row each.

  /** A shortcut as the rail writes it: « Ctrl 1 », « Ctrl Maj A ». */
  const kbd = (s: string) => keyLabel(s).split('+').join(' ');

  let list = $state<HTMLElement>();
  const tileEls: Record<string, HTMLButtonElement | undefined> = $state({});
  let dragId = $state<string | null>(null);
  let dropIndex = $state<number | null>(null);
  /** Projects out of view above and below the list, once it scrolls. */
  let edges = $state.raw<{ up: string[]; down: string[] }>({ up: [], down: [] });

  const first = $derived(app.projects.length === 0);
  const activeId = $derived(app.ui.view === 'project' ? app.ui.activeProject : null);
  const tiles = $derived(app.projects.map((p, i) => ({ p, s: tileState(p), i })));
  /** The one tile the Tab key reaches: the active one, else the first. */
  const focusId = $derived(app.projects.some((p) => p.id === activeId) ? activeId : (app.projects[0]?.id ?? null));

  /** Which tiles are out of view, above and below, and the fades of the edges with more. */
  function measure() {
    if (!list) return;
    const top = list.scrollTop;
    const bottom = top + list.clientHeight;
    const up: string[] = [];
    const down: string[] = [];
    for (const p of app.projects) {
      const el = tileEls[p.id];
      if (!el) continue;
      if (el.offsetTop + el.offsetHeight / 2 < top) up.push(p.id);
      else if (el.offsetTop + el.offsetHeight / 2 > bottom) down.push(p.id);
    }
    edges = { up, down };
  }

  $effect(() => {
    if (!list) return;
    const ro = new ResizeObserver(() => measure());
    ro.observe(list);
    return () => ro.disconnect();
  });
  $effect(() => {
    void app.projects.map((p) => p.id).join();
    untrack(measure);
  });

  // The active tile comes into view, however its project was chosen: a click, Ctrl+1…9, a notification, « Vue d'ensemble ».
  $effect(() => {
    void app.chosen;
    const el = activeId ? tileEls[activeId] : undefined;
    if (el) untrack(() => el.scrollIntoView?.({ block: 'nearest' }));
  });

  /** The projects out of view, at an edge, whose agents wait (or need a look): their pill says how many agents. */
  const pill = (ids: string[]) => {
    const waiting = tiles.filter(({ p, s }) => ids.includes(p.id) && (s.waiting > 0 || !!s.alert));
    return waiting.length
      ? {
          ids: waiting.map((x) => x.p.id),
          names: waiting.map((x) => x.p.name),
          count: waiting.reduce((n, x) => n + Math.max(1, x.s.waiting), 0),
        }
      : null;
  };
  const above = $derived(pill(edges.up));
  const below = $derived(pill(edges.down));

  function select(p: Project) {
    app.selectProject(p.id);
  }

  let nav = $state<HTMLElement>();
  /** Where a menu of the rail opens: beside its edge, or beside the tile of a narrow rail. */
  const besideRail = (r: DOMRect) => Math.max(r.right, (nav?.getBoundingClientRect().right ?? 0) - 8) + 12;

  function tileMenu(e: MouseEvent, p: Project) {
    // Its tooltip goes: the menu stands where it was.
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    e.preventDefault();
    e.stopPropagation();
    menu.open = { x: besideRail(r), y: Math.max(8, r.top - 6), items: projectMenu(p) };
  }

  function onTileKey(e: KeyboardEvent, p: Project, i: number) {
    if (e.altKey && (e.key === 'ArrowUp' || e.key === 'ArrowDown')) {
      e.preventDefault();
      // The tile keeps the focus as it moves.
      if (moveProject(p.id, e.key === 'ArrowUp' ? -1 : 1)) queueMicrotask(() => tileEls[p.id]?.focus());
      return;
    }
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    const to =
      e.key === 'ArrowUp' ? i - 1 : e.key === 'ArrowDown' ? i + 1 : e.key === 'Home' ? 0 : e.key === 'End' ? app.projects.length - 1 : null;
    if (to === null) return;
    e.preventDefault();
    const next = app.projects[Math.max(0, Math.min(app.projects.length - 1, to))];
    if (next) tileEls[next.id]?.focus();
  }

  function onDragOver(e: DragEvent, i: number) {
    if (dragId === null) return;
    e.preventDefault();
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    dropIndex = e.clientY < r.top + r.height / 2 ? i : i + 1;
  }

  function onDrop(e: DragEvent) {
    e.preventDefault();
    if (dragId !== null && dropIndex !== null) dropProject(dragId, dropIndex);
    dragId = dropIndex = null;
  }

  /** « + »: the assistant, or a short menu when projects were closed with their history kept (choice 47). */
  async function add(e: MouseEvent) {
    const button = e.currentTarget as HTMLElement;
    const closed = (await api.closedProjects().catch(() => [])).filter((c) => !c.unreadable);
    if (!closed.length) {
      app.modal = { kind: 'newProject' };
      return;
    }
    const items: MenuItem[] = [
      {
        label: t('nav.rail.addMenu.new'),
        icon: 'plus',
        detail: t('nav.rail.addMenu.newDetail'),
        onClick: () => (app.modal = { kind: 'newProject' }),
      },
      { label: '', separator: true },
      ...closed.slice(0, 5).map((c): MenuItem => ({
        label: c.name,
        swatch: c.color,
        detail: t('nav.rail.addMenu.closedOn', { date: fDate(c.closedAt) }),
        hint: t('nav.rail.addMenu.reopen'),
        onClick: () => (app.modal = { kind: 'newProject', path: c.path, historyId: c.id }),
      })),
      {
        label: t('nav.rail.addMenu.all'),
        icon: 'sliders',
        detail: t('nav.rail.addMenu.allDetail'),
        onClick: () => (app.modal = { kind: 'settings', tab: 'projects' }),
      },
    ];
    const r = button.getBoundingClientRect();
    menu.open = { x: besideRail(r), y: Math.max(8, r.top - 6), items, keyboard: true };
  }

  /** A pill of projects out of view: a click brings the first of them into view. */
  function reveal(ids: string[]) {
    tileEls[ids[0]]?.scrollIntoView?.({ block: 'nearest', behavior: 'smooth' });
  }
</script>

<nav class="rail" aria-label={t('nav.rail.label')} data-tip-edge bind:this={nav}>
  {#if !first}
    <button
      class="tile global"
      aria-pressed={app.ui.view === 'overview'}
      aria-label={t('nav.rail.overview')}
      use:tip={{ title: t('nav.rail.overview'), kbd: kbd('Ctrl+Shift+A'), rows: [t('nav.rail.overviewTip')], side: 'right', aria: false }}
      onclick={() => app.toggleOverview()}
      ><Icon name="overview" /><span class="lbl" aria-hidden="true"><span class="tx">{t('nav.rail.overview')}</span></span></button
    >
    <div class="sep"></div>
    {#if above}
      <button
        class="more"
        aria-label={t('nav.rail.moreUp', { names: above.names.join(', '), count: above.count })}
        onclick={() => reveal(above.ids)}><Icon name="up" size={11} />{above.count}</button
      >
    {/if}
    <div
      class="list"
      class:more-up={edges.up.length > 0}
      class:more-down={edges.down.length > 0}
      bind:this={list}
      onscroll={measure}
      role="tablist"
      aria-orientation="vertical"
      aria-label={t('nav.rail.list')}
    >
      {#each tiles as { p, s, i } (p.id)}
        {#if dropIndex === i && dragId !== p.id && dragId !== app.projects[i - 1]?.id}<div class="drop" aria-hidden="true"></div>{/if}
        <ProjectTile
          project={p}
          state={s}
          selected={p.id === activeId}
          shortcut={i < 9 ? kbd(`Ctrl+${i + 1}`) : null}
          dragging={dragId === p.id}
          focusable={p.id === focusId}
          bind:el={tileEls[p.id]}
          onclick={() => select(p)}
          oncontextmenu={(e) => tileMenu(e, p)}
          onauxclick={(e) => e.button === 1 && tileMenu(e, p)}
          onkeydown={(e) => onTileKey(e, p, i)}
          ondragstart={(e) => {
            dragId = p.id;
            e.dataTransfer?.setData('text/plain', p.id);
          }}
          ondragover={(e) => onDragOver(e, i)}
          ondrop={onDrop}
          ondragend={() => (dragId = dropIndex = null)}
        />
      {/each}
      {#if dropIndex === app.projects.length && dragId !== app.projects.at(-1)?.id}<div class="drop" aria-hidden="true"></div>{/if}
    </div>
    {#if below}
      <button
        class="more"
        aria-label={t('nav.rail.moreDown', { names: below.names.join(', '), count: below.count })}
        onclick={() => reveal(below.ids)}><Icon name="down" size={11} />{below.count}</button
      >
    {/if}
  {/if}
  <button
    class="tile add"
    aria-haspopup="menu"
    aria-label={t('nav.rail.add')}
    use:tip={{ title: t('nav.rail.add'), rows: [t('nav.rail.addTip')], side: 'right', aria: false }}
    onclick={add}><Icon name="plus" /><span class="lbl" aria-hidden="true"><span class="tx">{t('nav.rail.add')}</span></span></button
  >
  <div class="spacer"></div>
  {#if !first}
    <button
      class="tile global"
      aria-pressed={app.ui.view === 'stats'}
      aria-label={t('nav.rail.stats')}
      use:tip={{ title: t('nav.rail.stats'), rows: [t('nav.rail.statsTip')], side: 'right', aria: false }}
      onclick={() => (app.ui.view === 'stats' && app.ui.activeProject ? app.selectProject(app.ui.activeProject) : app.openStats())}
      ><Icon name="stats" /><span class="lbl" aria-hidden="true"><span class="tx">{t('nav.rail.stats')}</span></span></button
    >
    <button
      class="tile global"
      aria-pressed={app.ui.view === 'plugins'}
      aria-label={t('plugins.nav.tile')}
      use:tip={{ title: t('plugins.nav.tile'), rows: [t('plugins.nav.tileTip')], side: 'right', aria: false }}
      onclick={() => (app.ui.view === 'plugins' && app.ui.activeProject ? app.selectProject(app.ui.activeProject) : app.openPlugins())}
      ><Icon name="puzzle" /><span class="lbl" aria-hidden="true"><span class="tx">{t('plugins.nav.tile')}</span></span></button
    >
  {/if}
  <button
    class="tile global"
    aria-label={t('nav.rail.settingsLabel', { key: keyLabel('Ctrl+,') })}
    use:tip={{ title: t('nav.rail.settings'), kbd: kbd('Ctrl+,'), side: 'right', aria: false }}
    onclick={() => (app.modal = { kind: 'settings' })}
    ><Icon name="sliders" /><span class="lbl" aria-hidden="true"><span class="tx">{t('nav.rail.settings')}</span></span></button
  >
  <div class="edge">
    <Splitter
      value={railWidth.shown}
      min={RAIL_MIN}
      max={railWidth.max}
      onreset={() => railWidth.reset()}
      label={t('nav.rail.width')}
      title={t('nav.rail.widthTip')}
      onresize={(w) => railWidth.set(w)}
      oncommit={(w) => railWidth.commit(w)}
    />
  </div>
</nav>

<style>
  .rail {
    grid-column: 1;
    grid-row: 2;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    min-height: 0;
    padding: 6px 0 8px;
    /* Its width chosen (ESC-168): the labels and the tiles follow it. */
    position: relative;
    container: rail / inline-size;
  }
  /* Positioned: the tiles' offsetTop is measured from it (measure). */
  .list {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    flex: 0 1 auto;
    min-height: 0;
    width: 100%;
    padding: 6px 0;
    overflow-y: auto;
    scrollbar-width: none;
  }
  .list::-webkit-scrollbar {
    display: none;
  }
  /* More projects above or below: the edge fades. */
  .list.more-down {
    mask-image: linear-gradient(to bottom, #000 calc(100% - 28px), transparent);
  }
  .list.more-up {
    mask-image: linear-gradient(to bottom, transparent, #000 28px);
  }
  .list.more-up.more-down {
    mask-image: linear-gradient(to bottom, transparent, #000 28px, #000 calc(100% - 28px), transparent);
  }
  .sep {
    flex: none;
    width: 24px;
    height: 1px;
    background: var(--line-2);
  }
  .spacer {
    flex: 1;
    min-height: 0;
  }
  /* The tiles of what no project owns: the overview, « + », the stats and the settings. */
  .tile {
    position: relative;
    flex: none;
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    padding: 0;
    border: none;
    border-radius: 10px;
    background: rgba(var(--fg-rgb), 0.06);
    color: var(--muted);
    cursor: pointer;
    transition:
      background-color 0.15s,
      color 0.15s,
      box-shadow 0.2s var(--ease);
  }
  .tile:hover {
    color: var(--text);
    background: rgba(var(--fg-rgb), 0.1);
  }
  .tile::before {
    content: '';
    position: absolute;
    /* On the rail's edge, whatever its width (ESC-168): the tile is in its middle. */
    left: calc((36px - 100cqi) / 2);
    top: 50%;
    width: 4px;
    height: 22px;
    border-radius: 0 3px 3px 0;
    background: var(--line-3);
    transform: translateY(-50%) scaleY(0);
    transition: transform 0.2s var(--ease);
  }
  .tile:hover::before {
    transform: translateY(-50%) scaleY(0.4545);
  }
  .tile[aria-pressed='true']::before {
    transform: translateY(-50%) scaleY(1);
    background: var(--p-ink);
  }
  .tile[aria-pressed='true'] {
    box-shadow:
      0 0 0 2px var(--frame),
      0 0 0 4px var(--p-ink);
    color: var(--p-ink);
    background: var(--p-soft);
  }
  .tile:focus-visible {
    outline: none;
    box-shadow:
      0 0 0 2px var(--frame),
      0 0 0 4px var(--p-ink);
  }
  .tile.add {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .tile.add:hover {
    background: var(--hover);
    box-shadow: inset 0 0 0 1px var(--line-3);
  }
  /* Where a dragged tile lands. */
  .drop {
    flex: none;
    width: 32px;
    height: 3px;
    margin: -5.5px 0;
    border-radius: 2px;
    background: var(--p-ink);
  }
  /* Projects out of view that wait: an amber pill at that edge of the list. */
  .more {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 2px;
    height: 18px;
    margin: -4px 0;
    padding: 0 6px 0 4px;
    border: none;
    border-radius: 9px;
    background: var(--wait);
    color: var(--on-wait);
    font: 650 11px/1 var(--font);
    font-variant-numeric: tabular-nums;
    cursor: pointer;
  }
  .more :global(.icon) {
    stroke-width: 2;
  }

  /* The labels (ESC-168), of the projects' tiles and of the others. Narrow, a project's goes under its tile when it has
     one, on one line, cut in « … »; the active one's in the project's ink (its colour, legible on the frame). */
  .rail :global(.lbl) {
    position: absolute;
    top: calc(100% + 4px);
    left: 50%;
    display: none;
    width: calc(100cqi - 8px);
    transform: translateX(-50%);
    color: var(--muted);
    font: 550 10.5px/13px var(--font);
    letter-spacing: 0;
    text-align: center;
    white-space: nowrap;
    /* Part of its button, though drawn out of its box: a click on it opens what the tile opens (a click passing through
       it reached nothing, the rail widened to a list most of all). */
    cursor: pointer;
  }
  .rail :global(.lbl .tx) {
    display: block;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .rail :global(.lbl .n) {
    display: none;
  }
  .rail :global(.tile.labelled) {
    margin-bottom: 17px;
  }
  .rail :global(.tile.labelled .lbl) {
    display: block;
  }
  .rail :global(.tile:hover .lbl) {
    color: var(--text);
  }
  .rail :global(.tile[aria-selected='true'] .lbl),
  .rail :global(.tile[aria-pressed='true'] .lbl) {
    color: var(--p-ink);
    font-weight: 650;
  }
  /* The right edge, which sets the width: the line of the other handles, on the rail's edge, seen once hovered. */
  .edge {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    display: flex;
  }
  .edge :global(.splitter) {
    background: transparent;
  }
  /* Its grip inside the rail: out of it, it would widen what the rail holds. */
  .edge :global(.splitter::before) {
    inset: 0 0 0 -6px;
  }

  /* Widened to a list: every tile has its label on its right (the text chosen, else the name), a row each, its agents
     waiting at the row's end; the tiles and their badges stay as they are. */
  @container rail (min-width: 160px) {
    .tile,
    .more {
      align-self: flex-start;
      margin-left: 8px;
    }
    .sep {
      align-self: flex-start;
      margin-left: 14px;
    }
    .drop {
      align-self: flex-start;
      margin-left: 10px;
    }
    .tile::before {
      left: -8px;
    }
    .rail :global(.tile.labelled) {
      margin-bottom: 0;
    }
    .rail :global(.lbl),
    .rail :global(.tile.labelled .lbl) {
      top: 50%;
      left: calc(100% + 10px);
      display: flex;
      align-items: center;
      gap: 8px;
      width: calc(100cqi - 64px);
      transform: translateY(-50%);
      font-size: 13px;
      line-height: 18px;
      text-align: left;
    }
    .rail :global(.lbl .tx) {
      flex: 1;
    }
    .rail :global(.lbl .n) {
      display: grid;
      place-items: center;
      flex: none;
      min-width: 18px;
      height: 18px;
      padding: 0 5px;
      border-radius: 9px;
      background: var(--wait);
      color: var(--on-wait);
      font-size: 11px;
      font-weight: 650;
      font-variant-numeric: tabular-nums;
    }
  }
</style>
