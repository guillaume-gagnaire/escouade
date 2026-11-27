<script lang="ts">
  // The view of an agent's page (AP-3; docs/AGENT-PAGES.md § 5 and § 12, direction A « Les repères »): a row of
  // « Navigateurs » (`page:p-12`), shown alone or in a pane, beside its agent's conversation when it opened by itself. Its
  // bar: the page's title, its versions (« v2 ▾ », the last by default), its widths, « … ». Under it, the driving band of
  // every row (`DriveBand`), here with the trust mention alone (the driving comes with AB-8); « La page veut ouvrir… »
  // when it asked for a web address; the version the user went back to. The page itself is a webview of its own (its
  // shelter, AP-1), drawn over the window in the room below and hidden whenever something of the window passes over it.
  import { onDestroy, onMount } from 'svelte';
  import { agentpages, answerWants, latestOf, pageOfKey, showVersion, templateName } from '../lib/agentpages.svelte';
  import {
    besideOf,
    boundsOf,
    closePage,
    coveredAt,
    cycleMode,
    devpages,
    modeOf,
    openPageBeside,
    place,
    placementOf,
    setMode,
  } from '../lib/devpage.svelte';
  import { DEVICE_WIDTH, deviceFit, modeKey, PAGE_MODES, type PageMode } from '../lib/devpage-key';
  import { zoomState } from '../lib/display.svelte';
  import { fWhen } from '../lib/format';
  import { t } from '../lib/i18n';
  import { menu, type MenuItem } from '../lib/menu.svelte';
  import { paneCtx } from '../lib/pane-context';
  import { paneDrag } from '../lib/pane-drag.svelte';
  import { IS_MAC, keyLabel } from '../lib/platform';
  import { app } from '../lib/state.svelte';
  import { tip } from '../lib/tip';
  import ChoiceBand from './ChoiceBand.svelte';
  import DriveBand from './DriveBand.svelte';
  import Icon from './Icon.svelte';

  /** The row's key: `page:p-12`. */
  let { pageKey }: { pageKey: string } = $props();
  const key = $derived(pageKey);
  const pageId = $derived(pageOfKey(key) ?? '');

  const pane = paneCtx();
  const ownPane = $derived(pane?.key === `page:${key}`);

  const page = $derived(devpages.pages[key]);
  const info = $derived(agentpages.info[pageId] ?? null);
  /** Its agent by its name now (it may have been renamed since), else the one it had when it showed the page. */
  const agentName = $derived((page?.by.kind === 'page' ? app.agents[page.by.id]?.name : undefined) ?? info?.agentName ?? '');
  const ticketKey = $derived(info?.ticketId ? (app.tickets[info.ticketId]?.key ?? null) : null);
  const last = $derived(info ? latestOf(info) : 1);
  const shownVersion = $derived(info?.shown ?? last);
  const older = $derived(shownVersion < last);
  const modeShortcut = IS_MAC ? '⇧⌘M' : t('devpage.modeKey');
  const MODE_ICON = { full: 'monitor', tablet: 'tablet', mobile: 'phone' } as const;
  const mode = $derived<PageMode>(page ? modeOf(key) : 'full');
  let roomWidth = $state(0);
  const fit = $derived(deviceFit(mode, roomWidth));

  let room: HTMLDivElement | undefined = $state();
  let covered = $state(false);

  /** The page shows: in this view, nothing over it, no content dragged. */
  const visible = $derived(!!page && !covered && !paneDrag.on);

  let frame = 0;
  let placed = '';
  function sync() {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => {
      if (!room) return;
      const rect = room.getBoundingClientRect();
      roomWidth = Math.round(rect.width);
      covered = coveredAt(rect);
      const show = visible && rect.width > 0 && rect.height > 0;
      const f = deviceFit(mode, rect.width);
      const shown = { left: rect.left + f.left, top: rect.top, width: f.width, height: rect.height } as DOMRect;
      const bounds = show ? boundsOf(shown, zoomState.shown) : null;
      const where = placementOf(ownPane, rect, room.closest('.work')?.getBoundingClientRect());
      const said = JSON.stringify([bounds, where]);
      if (said === placed) return;
      placed = said;
      void place(key, bounds, where);
    });
  }

  $effect(() => {
    void [visible, zoomState.shown, mode, paneDrag.on, page?.wants, older, info?.ask, info?.proposal];
    sync();
  });

  // Seen: its point goes.
  $effect(() => {
    if (page?.unseen) page.unseen = false;
  });

  let resize: ResizeObserver | undefined;
  let overlays: MutationObserver | undefined;
  onMount(() => {
    resize = new ResizeObserver(sync);
    if (room) resize.observe(room);
    const work = room?.closest('.work');
    if (work) resize.observe(work);
    overlays = new MutationObserver(sync);
    overlays.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ['style'] });
    window.addEventListener('resize', sync);
  });
  onDestroy(() => {
    cancelAnimationFrame(frame);
    resize?.disconnect();
    overlays?.disconnect();
    window.removeEventListener('resize', sync);
    void place(key, null);
  });

  function onKey(e: KeyboardEvent) {
    if (!page) return;
    if (pane && !pane.active) return;
    if (modeKey(e, IS_MAC)) {
      e.preventDefault();
      cycleMode(key);
    }
  }

  /** « v2 ▾ »: its versions, the last first; the one shown is checked. */
  function versionsMenu(el: HTMLElement) {
    if (!info) return;
    const now = Date.now();
    const items: MenuItem[] = [...info.versions].reverse().map((v) => ({
      label: t('devpage.agentPage.versionItem', { n: v.n }),
      hint: v.n === last ? t('devpage.agentPage.latest') : fWhen(v.at, now),
      checked: v.n === shownVersion,
      onClick: () => void showVersion(key, v.n),
    }));
    menu.showAt(el, items);
  }

  /** « … »: beside or below (or close its pane), its widths when the bar folded them, close the page. */
  function moreMenu(el: HTMLElement) {
    if (!page) return;
    const items: MenuItem[] = [];
    if (ownPane) {
      items.push({
        label: t('shell.panes.close'),
        icon: 'close',
        hint: keyLabel('Ctrl+Shift+W'),
        onClick: () => app.closePaneOf(`page:${key}`),
      });
    } else if (besideOf(key) || app.pageShown !== key) {
      items.push(
        { label: t('shell.panes.openBeside'), icon: 'pane-r', onClick: () => openPageBeside(key, 'right') },
        { label: t('shell.panes.openBelow'), icon: 'pane-b', onClick: () => openPageBeside(key, 'bottom') },
      );
    }
    items.push({ label: '', separator: true });
    for (const m of PAGE_MODES)
      items.push({
        label: t(`devpage.modes.${m}`),
        icon: MODE_ICON[m],
        checked: mode === m,
        hint: m === 'full' ? undefined : t('devpage.width', { width: DEVICE_WIDTH[m] }),
        onClick: () => setMode(key, m),
      });
    items.push({ label: '', separator: true });
    items.push({ label: t('devpage.agentPage.close'), icon: 'close', onClick: () => closePage(key) });
    menu.showAt(el, items);
  }
</script>

<svelte:window onkeydown={onKey} />

<section
  class="agentpage"
  aria-label={page ? t('devpage.agentPage.view', { agent: agentName, title: page.title }) : t('devpage.view')}
  data-agent-page={pageId}
>
  {#if page}
    <div class="bar">
      <span class="pt" use:tip={{ text: page.title, aria: false }}>
        <span class="ic"><Icon name="page" size={15} /></span>
        <span class="nm">{page.title}</span>
      </span>
      {#if info}
        <span class="meta"
          >{t('devpage.agentPage.by', { agent: agentName })}{#if ticketKey}<span class="key">{ticketKey}</span>{/if}<span class="tpl"
            >{templateName(info.template)}</span
          ></span
        >
      {/if}
      <button
        class="ver"
        class:old={older}
        aria-haspopup="menu"
        aria-label={t('devpage.agentPage.versions')}
        use:tip={{ text: t('devpage.agentPage.versions'), aria: false }}
        data-testid="page-version"
        onclick={(e) => versionsMenu(e.currentTarget)}
        >{t('devpage.agentPage.version', { n: shownVersion })}<Icon name="down" size={11} /></button
      >
      <span class="sp"></span>
      <div class="segmented small modes" role="radiogroup" aria-label={t('devpage.modesLabel')}>
        {#each PAGE_MODES as m (m)}
          <button
            role="radio"
            class:on={mode === m}
            aria-checked={mode === m}
            aria-label={t(`devpage.modes.${m}`)}
            use:tip={t('devpage.modeHint', {
              mode: t(`devpage.modes.${m}`),
              width: m === 'full' ? '' : ` · ${DEVICE_WIDTH[m]} px`,
              key: modeShortcut,
            })}
            onclick={() => setMode(key, m)}><Icon name={MODE_ICON[m]} size={14} /></button
          >
        {/each}
      </div>
      {#if mode !== 'full'}
        <span class="width" class:shrunk={fit.shrunk}
          >{fit.shrunk ? t('devpage.shrunkShort', { width: fit.width }) : t('devpage.width', { width: DEVICE_WIDTH[mode] })}</span
        >
      {/if}
      <button
        class="icon-btn more"
        aria-label={t('devpage.more')}
        aria-haspopup="menu"
        use:tip={{ text: t('devpage.more'), aria: false }}
        onclick={(e) => moreMenu(e.currentTarget)}><Icon name="more" size={14} /></button
      >
    </div>
    {#if page.phase === 'loading'}<div class="load" aria-hidden="true"></div>{/if}
    <!-- The driving band of every row (AB-8 drives it); on an agent's page, its trust mention. Out of the webview. -->
    <DriveBand agent={agentName} trust />
    {#if page.wants}
      <div class="strip" role="alert">
        <span class="ic"><Icon name="globe" size={14} /></span>
        <p class="say">{t('devpage.agentPage.wants')} <span class="mono url">{page.wants}</span></p>
        <div class="acts">
          <button class="btn ghost" onclick={() => answerWants(key, false)}>{t('devpage.agentPage.wantsIgnore')}</button>
          <button class="btn" onclick={() => answerWants(key, true)}><Icon name="ext" size={13} />{t('devpage.agentPage.wantsOpen')}</button
          >
        </div>
      </div>
    {/if}
    {#if older}
      <div class="strip quiet" role="status" data-testid="page-older">
        <span class="ic"><Icon name="clock" size={14} /></span>
        <p class="say">{t('devpage.agentPage.older', { n: shownVersion, last })}</p>
        <div class="acts">
          <button class="btn" onclick={() => showVersion(key, last)}>{t('devpage.agentPage.toLatest', { last })}</button>
        </div>
      </div>
    {/if}
  {/if}

  <div class="room" bind:this={room} class:device={mode !== 'full' && visible}>
    {#if mode !== 'full' && visible}
      <div class="frame" aria-hidden="true" style:left="{fit.left - 1}px" style:width="{fit.width + 2}px"></div>
    {/if}
    {#if !page}
      <div class="msg"><p>{t('devpage.agentPage.gone')}</p></div>
    {:else if covered || paneDrag.on}
      <div class="msg quiet"><p>{t('devpage.covered')}</p></div>
    {:else if page.phase === 'loading'}
      <div class="msg quiet" role="status"><p>{t('devpage.agentPage.loading')}</p></div>
    {/if}
  </div>
  {#if page && info?.ask}
    <!-- The band of the choice (AP-4): under the page, out of its webview; what the page proposed is sent here alone. -->
    <ChoiceBand page={pageId} askId={info.ask} proposal={info.proposal ?? null} agent={agentName} />
  {/if}
</section>

<style>
  .agentpage {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    min-width: 0;
    container: agentpage / inline-size;
  }
  /* The bar of a browser (DevPage): 46 px, 30 px controls; the page named first, in the project's ink. */
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 46px;
    padding: 0 10px 0 12px;
    border-bottom: 1px solid var(--line);
    flex: none;
    min-width: 0;
  }
  .pt {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 60px;
    flex: 0 1 auto;
    font-size: 13px;
    font-weight: 600;
  }
  .pt .ic {
    display: inline-flex;
    flex: none;
    color: var(--p-ink);
  }
  .nm,
  .meta {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    flex: 0 3 auto;
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 11.5px;
  }
  .meta .key {
    flex: none;
    padding: 0 5px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
    font-weight: 550;
    font-variant-numeric: tabular-nums;
  }
  .meta .tpl::before {
    content: '·';
    margin-right: 6px;
  }
  /* « v2 ▾ »: a hairline chip, as in the design; amber when the user looks at an older one. */
  .ver {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 3px;
    height: 22px;
    padding: 0 4px 0 7px;
    border: 0;
    border-radius: 6px;
    background: none;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
    font: inherit;
    font-size: 11.5px;
    font-weight: 550;
    font-variant-numeric: tabular-nums;
    cursor: pointer;
  }
  .ver:hover {
    color: var(--text);
    box-shadow: inset 0 0 0 1px var(--line-3);
  }
  .ver:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 1px;
  }
  .ver.old {
    color: var(--wait);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .sp {
    flex: 1 1 0;
    min-width: 0;
  }
  .modes {
    flex: none;
  }
  .width {
    flex: none;
    color: var(--muted);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .width.shrunk {
    color: var(--wait);
  }
  .load {
    flex: none;
    height: 2px;
    background: linear-gradient(90deg, var(--p-ink) 0 40%, transparent 40%);
    background-size: 200% 100%;
    animation: agentpage-load 1.2s cubic-bezier(0.22, 1, 0.36, 1) infinite;
  }
  @keyframes agentpage-load {
    from {
      background-position: 100% 0;
    }
    to {
      background-position: -100% 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .load {
      animation: none;
    }
  }
  /* A strip of the window under the bar (ConsentBand's): what the page asked, and the version the user went back to. */
  .strip {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 40px;
    padding: 6px 10px 6px 14px;
    border-bottom: 1px solid var(--line);
    background: var(--wait-bg);
    flex: none;
    min-width: 0;
  }
  .strip.quiet {
    background: rgba(var(--fg-rgb), 0.025);
  }
  .strip .ic {
    display: inline-flex;
    flex: none;
    color: var(--warn-ink);
  }
  .strip.quiet .ic {
    color: var(--muted);
  }
  .say {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: 12.5px;
    line-height: 1.4;
    color: var(--text);
  }
  .url {
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  .acts {
    display: flex;
    flex: none;
    gap: 6px;
  }
  .room {
    position: relative;
    flex: 1;
    min-height: 0;
    display: grid;
    place-items: center;
  }
  .room.device {
    background: var(--well);
  }
  .frame {
    position: absolute;
    top: 0;
    bottom: 0;
    border-inline: 1px solid var(--line-2);
    pointer-events: none;
  }
  .msg {
    max-width: 380px;
    padding: 24px;
    text-align: center;
  }
  .msg p {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
  }
  /* Narrow (half of 1024): the agent and the template go, then the widths into « … ». */
  @container agentpage (max-width: 620px) {
    .meta {
      display: none;
    }
  }
  @container agentpage (max-width: 440px) {
    .modes,
    .width {
      display: none;
    }
  }
</style>
