<script lang="ts">
  // « Navigateurs » in the sidebar (ESC-657), under « Terminaux » and on its model: one row per browser (a tab), its dot
  // saying its page's state, its × closing it, its menu (beside, below, reload, copy the address, the system's browser,
  // close). A click shows it alone in the main area, Ctrl+click (Cmd) or a drag onto the work area in a pane of the split
  // screen, as a terminal. « + » asks an address in the section itself: a browser of the project at that address, opened
  // alone, beside the pages that the agents' launches open by themselves. In a short window it folds to its header
  // (choice 9), as the terminals do.
  import { tick } from 'svelte';
  import {
    besideOf,
    browsersOf,
    closePage,
    go,
    hasConsents,
    lendBrowser,
    newBrowser,
    openPageBeside,
    pageItem,
    revokeConsents,
    rowName,
    showPage,
    type PageState,
  } from '../lib/devpage.svelte';
  import { agentpages, latestOf, pageOfKey } from '../lib/agentpages.svelte';
  import { driveOf, moveHand } from '../lib/drives.svelte';
  import { initials } from '../lib/project-rail';
  import { t } from '../lib/i18n';
  import { menu, type MenuItem } from '../lib/menu.svelte';
  import { openUrl } from '../lib/opener';
  import { paneDrag } from '../lib/pane-drag.svelte';
  import { itemKey, leaves } from '../lib/panes';
  import { keyLabel } from '../lib/platform';
  import { shortWindow } from '../lib/short-window.svelte';
  import { app } from '../lib/state.svelte';
  import { tip } from '../lib/tip';
  import type { Project } from '../lib/types';
  import FoldTitle from './FoldTitle.svelte';
  import Icon from './Icon.svelte';

  let { project }: { project: Project } = $props();

  let unfolded = $state(false);
  const folded = $derived(shortWindow.now && !unfolded);

  const rows = $derived(browsersOf(project.id));
  /** The contents of the split screen's panes, when it is on screen: each highlighted, the active one more (ESC-392). */
  const paneKeys = $derived(app.panesOn && app.panes ? new Set(leaves(app.panes.root).map(itemKey)) : null);
  const selected = (key: string) => (paneKeys ? app.panes?.active === `page:${key}` : app.pageShown === key);
  const inPane = (key: string) => !!paneKeys?.has(`page:${key}`) && app.panes?.active !== `page:${key}`;

  /** « + » asked an address: the field, at the top of the rows, until it is sent or left. */
  let adding = $state(false);
  let typed = $state('');
  let field = $state<HTMLInputElement>();
  let opening = $state(false);

  async function ask() {
    unfolded = true;
    adding = true;
    typed = '';
    await tick();
    field?.focus();
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (opening) return;
    opening = true;
    try {
      const key = await newBrowser(project.id, typed);
      if (key) {
        adding = false;
        typed = '';
      }
    } finally {
      opening = false;
    }
  }

  function leave() {
    if (!typed.trim()) adding = false;
  }

  /** Its dot, in a word: loading or waiting for the user (amber), loaded (green), not answering (red). */
  const tone = (p: PageState) => (p.waiting ? 'wait' : p.phase === 'ready' ? 'on' : p.phase === 'stalled' ? 'bad' : 'wait');

  /** The agent's name, for a row it opened or that waits for the user (ESC-678). */
  const agentName = (id: string | undefined) => (id ? (app.agents[id]?.name ?? id) : '');

  /** Its tip: what waits for the user, else its address. */
  /** An agent's page (AP-3): its version tag (« v2 »), its agent. */
  const pageInfo = (key: string) => {
    const id = pageOfKey(key);
    return id ? (agentpages.info[id] ?? null) : null;
  };
  /** Who drives it (AB-8), said first: the user always sees that an agent drives. */
  const drivenSay = (key: string) => {
    const d = driveOf(key);
    if (!d) return '';
    const agent = agentName(d.agent ?? undefined);
    if (d.stopped) return t('devpage.drive.rowStopped');
    return d.hand === 'agent' ? t('devpage.drive.rowDriven', { agent }) : t('devpage.drive.rowYours', { agent });
  };
  const tipOf = (key: string, p: PageState) => {
    const said = baseTip(key, p);
    const driven = drivenSay(key);
    return driven ? `${driven} · ${said}` : said;
  };
  const baseTip = (key: string, p: PageState) => {
    const info = pageInfo(key);
    const name = (p.by.kind === 'page' && app.agents[p.by.id]?.name) || info?.agentName || '';
    if (p.by.kind === 'page') return t('devpage.agentPage.rowTip', { title: p.title, agent: name });
    return p.waiting ? t('devpage.browsers.waiting', { name: agentName(p.driver) }) : p.url;
  };

  /** The driving's moves in a row's menu (AB-8): take the hand or give it back, stop letting it drive. */
  function driveItems(key: string): MenuItem[] {
    const d = driveOf(key);
    if (!d || d.stopped) return [];
    return [
      d.hand === 'agent'
        ? { label: t('devpage.drive.take'), icon: 'pause', onClick: () => void moveHand(key, 'take') }
        : { label: t('devpage.drive.give'), icon: 'play', onClick: () => void moveHand(key, 'give') },
      { label: t('devpage.drive.stop'), icon: 'close', onClick: () => void moveHand(key, 'stop') },
      { label: '', separator: true },
    ];
  }

  /**
   * « Laisser piloter par… » (AB-9), on a browser the user opened alone: the live agents of its project, the one that
   * drives it ticked, and « Personne ».
   */
  function lendItems(key: string, p: PageState): MenuItem[] {
    if (p.by.kind !== 'user' || p.opener) return [];
    const agents = Object.values(app.agents)
      .filter((a) => a.projectId === project.id && !a.archived)
      .sort((a, b) => a.createdAt - b.createdAt);
    const submenu: MenuItem[] = agents.length
      ? [
          ...agents.map((a) => ({
            label: a.name,
            checked: p.driver === a.id,
            onClick: () => void lendBrowser(key, a.id),
          })),
          ...(p.driver
            ? ([
                { label: '', separator: true },
                { label: t('devpage.drive.lendNobody'), onClick: () => void lendBrowser(key, null) },
              ] satisfies MenuItem[])
            : []),
        ]
      : [{ label: t('devpage.drive.lendNone'), disabled: true }];
    return [
      { label: t('devpage.drive.lend'), icon: 'play', submenu },
      { label: '', separator: true },
    ];
  }

  function rowMenu(e: MouseEvent, key: string, p: PageState) {
    // Beside what is on screen; a browser opened alone and shown alone has nothing to go beside.
    const beside: MenuItem[] =
      besideOf(key) || app.pageShown !== key
        ? [
            {
              label: t('shell.panes.openBeside'),
              hint: `${keyLabel('Ctrl')}+${t('shell.panes.click')}`,
              onClick: () => openPageBeside(key, 'right'),
            },
            { label: t('shell.panes.openBelow'), onClick: () => openPageBeside(key, 'bottom') },
            { label: '', separator: true },
          ]
        : [];
    // An agent's page: beside or below, and close its row (the page stays kept, its card opens it again).
    if (p.by.kind === 'page') {
      menu.show(e, [
        ...beside,
        ...driveItems(key),
        { label: t('devpage.agentPage.closeRow'), icon: 'close', onClick: () => closePage(key) },
      ]);
      return;
    }
    const items: MenuItem[] = [
      ...beside,
      ...lendItems(key, p),
      ...driveItems(key),
      { label: t('devpage.reload'), icon: 'restart', onClick: () => go(key, 'reload') },
      {
        label: t('devpage.browsers.copy'),
        icon: 'copy',
        onClick: () => app.run(navigator.clipboard.writeText(p.url).then(() => app.toast(t('devpage.browsers.copied'), 'info'))),
      },
      { label: t('devpage.system'), icon: 'ext', onClick: () => app.run(openUrl(p.url)) },
      // The agents' leaves to read it outside the launch (ESC-669), while it has some.
      ...(hasConsents(key)
        ? ([
            { label: '', separator: true },
            { label: t('devpage.consent.revoke'), icon: 'close', onClick: () => void revokeConsents(key) },
          ] satisfies MenuItem[])
        : []),
      { label: '', separator: true },
      { label: t('devpage.browsers.close'), icon: 'close', onClick: () => closePage(key) },
    ];
    menu.show(e, items);
  }

  function click(e: MouseEvent, key: string) {
    // A row that waited is shown now: it waits no more.
    const p = rows.find(([k]) => k === key)?.[1];
    if (p?.waiting) p.waiting = undefined;
    if (e.ctrlKey || e.metaKey) openPageBeside(key, 'right');
    else showPage(key);
  }
</script>

<section class="sec browsers" class:folded aria-label={t('devpage.browsers.title')}>
  <div class="sec-h">
    <FoldTitle title={t('devpage.browsers.title')} short={shortWindow.now} {folded} ontoggle={() => (unfolded = !unfolded)} />
    <span class="n">{rows.length}</span>
    <span class="acts">
      <button
        class="icon-btn"
        aria-label={t('devpage.browsers.new')}
        aria-expanded={adding}
        use:tip={{ text: t('devpage.browsers.new'), aria: false }}
        onclick={() => (adding ? field?.focus() : ask())}><Icon name="plus" size={14} /></button
      >
    </span>
  </div>
  <div class="rows">
    {#if adding}
      <form class="field add" onsubmit={submit}>
        <Icon name="globe" size={14} />
        <input
          bind:this={field}
          bind:value={typed}
          class="mono"
          aria-label={t('devpage.browsers.field')}
          placeholder={t('devpage.browsers.placeholder')}
          spellcheck="false"
          autocomplete="off"
          disabled={opening}
          onblur={leave}
          onkeydown={(e) => {
            if (e.key === 'Escape') {
              e.stopPropagation();
              adding = false;
            }
          }}
        />
        <button class="icon-btn" type="submit" aria-label={t('devpage.browsers.open')} disabled={opening || !typed.trim()}
          ><Icon name="enter" size={13} /></button
        >
      </form>
    {/if}
    {#each rows as [key, p] (key)}
      {@const tn = tone(p)}
      {@const drive = driveOf(key)}
      <div
        class="row-item browser"
        class:sel={selected(key)}
        class:inpane={inPane(key)}
        class:waits={!!p.waiting}
        class:driven={!!drive && drive.hand === 'agent' && !drive.stopped}
        data-driven={drive ? (drive.stopped ? 'stopped' : drive.hand) : undefined}
        role="button"
        tabindex="0"
        aria-current={selected(key) ? 'true' : undefined}
        data-browser={key}
        use:tip={{ text: tipOf(key, p), aria: false }}
        onclick={(e) => click(e, key)}
        onpointerdown={(e) => paneDrag.press(e, pageItem(key), rowName(p))}
        oncontextmenu={(e) => rowMenu(e, key, p)}
        onkeydown={(e) => e.key === 'Enter' && e.target === e.currentTarget && showPage(key)}
      >
        {#if drive}
          <!-- Driven (AB-8): the agent's initials before the name, in the project's colour; dimmed once the user has the hand. -->
          <span class="av" class:held={drive.hand === 'user' || drive.stopped} aria-hidden="true" data-testid="driven-avatar"
            >{initials(agentName(drive.agent ?? undefined))}</span
          >
          <span class="sr">{drivenSay(key)}</span>
        {/if}
        {#if p.by.kind === 'page'}
          <!-- An agent's page (AP-3): the page's icon for the dot; its point while it waits for the user. -->
          <span class="pg" class:unseen={p.unseen} aria-hidden="true"><Icon name="page" size={13} /></span>
          <span class="nm">{rowName(p)}</span>
          {#if p.unseen}<span class="sr">{t('devpage.agentPage.unseen')}</span>{/if}
          {@const info = pageInfo(key)}
          {#if info}<span class="vtag" data-testid="page-row-version">{t('devpage.agentPage.version', { n: latestOf(info) })}</span>{/if}
        {:else}
          <i class="d {tn}" aria-hidden="true"></i>
          <span class="nm">{rowName(p)}</span>
          <span class="sr"
            >{p.waiting ? t('devpage.browsers.waiting', { name: agentName(p.driver) }) : t(`devpage.browsers.state.${p.phase}`)}</span
          >
          {#if p.by.kind === 'agent'}<span class="by" aria-hidden="true"><Icon name="branch" size={11} /></span>{/if}
          {#if p.opener}<span class="by" use:tip={t('devpage.browsers.opener', { name: agentName(p.opener) })}
              ><Icon name="agents" size={11} /></span
            >{/if}
        {/if}
        <span class="ra">
          <button
            class="icon-btn x"
            aria-label={p.by.kind === 'page' ? t('devpage.agentPage.closeRow') : t('devpage.browsers.close')}
            use:tip={{ text: p.by.kind === 'page' ? t('devpage.agentPage.closeRow') : t('devpage.browsers.close'), aria: false }}
            onclick={(e) => {
              e.stopPropagation();
              closePage(key);
            }}><Icon name="close" size={12} /></button
          >
        </span>
      </div>
    {:else}
      {#if !adding}<span class="none">{t('devpage.browsers.none')}</span>{/if}
    {/each}
  </div>
</section>

<style>
  /* The rows of « Terminaux » (Sidebar.svelte) and of « Lancement » (RunsSection.svelte): the same section, the same rows. */
  .sec {
    display: grid;
    gap: 1px;
    min-height: 0;
    margin: 6px -2px 0;
    padding-top: 8px;
    border-top: 1px solid var(--line);
  }
  .sec-h {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 2px 0 8px;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
  }
  .sec-h .n {
    color: var(--dim);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .sec-h .acts {
    display: flex;
    gap: 2px;
    margin-left: auto;
  }
  .rows {
    display: grid;
    gap: 1px;
    max-height: 30vh;
    overflow: auto;
  }
  .row-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 28px;
    padding: 0 4px 0 9px;
    border-radius: 7px;
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
    outline: none;
  }
  .row-item:hover {
    background: var(--hover);
  }
  .row-item.sel {
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  /* Shown in a pane of the split screen that is not the active one (ESC-392). */
  .row-item.inpane {
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .row-item:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: -2px;
  }
  /* Its page's state, as a launch's dot: loading, loaded, not answering. */
  .d {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--wait);
  }
  .d.on {
    background: var(--run);
  }
  .d.bad {
    background: var(--del);
  }
  /* Its agent wants to show it while the user looks elsewhere (ESC-678): it opens beside the conversation once they
     come back to it. */
  .row-item.waits .nm {
    color: var(--warn-ink);
  }
  .nm {
    min-width: 0;
    overflow: hidden;
    color: var(--soft);
    font-weight: 550;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Driven (AB-8): the agent's initials in the project's colour before the name; held by the user, a quiet outline. */
  .av {
    flex: none;
    display: grid;
    place-items: center;
    min-width: 18px;
    height: 16px;
    padding: 0 3px;
    border-radius: 5px;
    background: var(--p);
    color: #0b0c0e;
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.01em;
    line-height: 1;
  }
  .av.held {
    background: none;
    box-shadow: inset 0 0 0 1px var(--line-3);
    color: var(--muted);
  }
  .row-item.driven .nm {
    color: var(--text);
  }
  /* An agent's page: the page's icon in the project's ink, a point on it while it waits for the user. */
  .pg {
    position: relative;
    display: inline-flex;
    flex: none;
    margin-left: -3px;
    color: var(--p-ink);
  }
  .pg.unseen::after {
    content: '';
    position: absolute;
    top: -2px;
    right: -3px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--wait);
    box-shadow: 0 0 0 2px var(--frame);
  }
  /* Its last version, as the design's « v2 »: a hairline tag. */
  .vtag {
    flex: none;
    height: 17px;
    padding: 0 5px;
    border-radius: 5px;
    display: inline-flex;
    align-items: center;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
    font-size: 11px;
    font-weight: 550;
    font-variant-numeric: tabular-nums;
  }
  /* Opened by an agent's launch: its worktree's mark, as the chip of its bar. */
  .by {
    display: inline-flex;
    flex: none;
    color: var(--dim);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  /* Shown on hover or focus; out of sight otherwise, but still there for the keyboard and a screen reader. */
  .ra {
    display: inline-flex;
    width: 0;
    margin-left: auto;
    overflow: hidden;
  }
  .row-item:hover .ra,
  .row-item.sel .ra,
  .row-item:focus-within .ra {
    width: auto;
  }
  .ra .icon-btn {
    width: 22px;
    height: 22px;
  }
  /* The address asked by « + »: a row of its own, a field of the app (`field`, as the page's address). */
  .add {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 0 2px;
    padding: 0 3px 0 8px;
    color: var(--muted);
    cursor: text;
  }
  .add input {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0;
    border: 0;
    outline: 0;
    background: none;
    color: var(--text);
    font-size: 12px;
  }
  .add input::placeholder {
    color: var(--dim);
  }
  .add .icon-btn {
    width: 22px;
    height: 22px;
  }
  .none {
    padding: 2px 9px 6px;
    color: var(--dim);
    font-size: 12px;
  }
  /* A short window (820 px and less): the section folds to its header (choice 9), until its title unfolds it. */
  .folded .rows {
    display: none;
  }
  .sec.folded {
    padding-bottom: 2px;
  }
</style>
