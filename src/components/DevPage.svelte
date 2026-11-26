<script lang="ts">
  // The view of a browser (ESC-657, the page of a launch before, ESC-311): its bar, and the room where the page, a
  // webview of its own drawn over the window, is placed. The page is hidden whenever something of the window passes
  // over it (a menu, a list, a dialog, the palette, the compass of a drag), whenever the launch that opened it is not up,
  // and when the view goes. It is drawn alone in the main area or in a pane of the split screen (ESC-419): the webview
  // follows its room there. Its bar: back, forward, reload, the address, then the widths and the devtools beside it,
  // then the system's browser; drawn with the app's controls (`icon-btn`, `btn`, `segmented small`, `field`).
  import { onDestroy, onMount } from 'svelte';
  import {
    answerConsent,
    besideOf,
    boundsOf,
    coveredAt,
    cycleMode,
    devpages,
    devtools,
    devtoolsKey,
    go,
    hostOf,
    modeOf,
    openPageBeside,
    pageItem,
    place,
    loadConsents,
    placementOf,
    setMode,
  } from '../lib/devpage.svelte';
  import { DEVICE_WIDTH, deviceFit, modeKey, PAGE_MODES, typedAddress, type PageMode } from '../lib/devpage-key';
  import { zoomState } from '../lib/display.svelte';
  import { t } from '../lib/i18n';
  import { menu, type MenuItem } from '../lib/menu.svelte';
  import { openUrl } from '../lib/opener';
  import { paneCtx } from '../lib/pane-context';
  import { paneDrag } from '../lib/pane-drag.svelte';
  import { IS_MAC, keyLabel, OS } from '../lib/platform';
  import { app } from '../lib/state.svelte';
  import { anyRunning, flows, testAgent } from '../lib/test-launch.svelte';
  import { tip } from '../lib/tip';
  import ConsentBand from './ConsentBand.svelte';
  import Icon from './Icon.svelte';

  /** The browser's key: its agent's id, or `browser:<id>` for one opened alone. */
  let { pageKey }: { pageKey: string } = $props();
  const key = $derived(pageKey);

  /** Its pane of the split screen, when it is in one. */
  const pane = paneCtx();
  /** In a pane of the split screen (`page:<key>`), not alone in the main area. */
  const ownPane = $derived(pane?.key === `page:${key}`);

  const page = $derived(devpages.pages[key]);
  /** The agent whose launch opened it, if one did. */
  const agent = $derived(page?.by.kind === 'agent' ? app.agents[page.by.id] : undefined);
  const flow = $derived(agent ? flows.all[agent.id] : undefined);
  const project = $derived(app.projects.find((p) => p.id === page?.projectId));
  /** Its launch is starting again: the page waits for it. */
  const waiting = $derived(flow?.phase === 'running');
  /** Its launch is no longer up (stopped, crashed, failed); a browser opened alone has none to wait for. */
  const stopped = $derived(!!agent && !waiting && !anyRunning(agent.id));
  /** Where its devtools go on this system (ESC-657): docked in the page under WebKitGTK, a window of their own elsewhere. */
  const devtoolsWhere = $derived(t(`devpage.devtoolsWhere.${OS}`));
  const shortcut = IS_MAC ? '⌥⌘I' : 'F12';
  const modeShortcut = IS_MAC ? '⇧⌘M' : t('devpage.modeKey');
  const MODE_ICON = { full: 'monitor', tablet: 'tablet', mobile: 'phone' } as const;
  /** The width it is shown at, as chosen for this page (« worktree · processus »). */
  const mode = $derived<PageMode>(page ? modeOf(key) : 'full');
  /** The width of its room, in CSS pixels: a device narrower than it is centred, a wider one narrowed to it. */
  let roomWidth = $state(0);
  const fit = $derived(deviceFit(mode, roomWidth));

  let bar: HTMLDivElement | undefined = $state();
  let room: HTMLDivElement | undefined = $state();
  let covered = $state(false);
  let typed = $state<string | null>(null);

  /** The page shows: loaded or loading, its launch up, in this view, nothing over it, no content dragged. */
  const visible = $derived(!!page && !waiting && !stopped && page.phase !== 'stalled' && !covered && !paneDrag.on);

  let frame = 0;
  function sync() {
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => {
      if (!room) return;
      const rect = room.getBoundingClientRect();
      roomWidth = Math.round(rect.width);
      covered = coveredAt(rect);
      const show = visible && rect.width > 0 && rect.height > 0;
      // A tablet or a phone: its width, centred in the room, the height of the room.
      const f = deviceFit(mode, rect.width);
      const shown = { left: rect.left + f.left, top: rect.top, width: f.width, height: rect.height } as DOMRect;
      const bounds = show ? boundsOf(shown, zoomState.shown) : null;
      // Alone, or in a pane beside or under something else (`list_browsers`, ESC-669).
      const where = placementOf(ownPane, rect, room.closest('.work')?.getBoundingClientRect());
      // Only what changed goes to the backend: the observers below fire far more often than the page moves.
      const said = JSON.stringify([bounds, where]);
      if (said === placed) return;
      placed = said;
      void place(key, bounds, where);
    });
  }
  /** The last place told to the backend. */
  let placed = '';

  $effect(() => {
    // Again whenever what it hangs on changes.
    void [visible, zoomState.shown, page?.phase, mode, paneDrag.on];
    sync();
  });

  let resize: ResizeObserver | undefined;
  let overlays: MutationObserver | undefined;
  onMount(() => {
    // Its room changes size (its pane resized), and moves with no change of size (the pane beside it resized, the
    // sidebar folded): both are followed.
    resize = new ResizeObserver(sync);
    if (room) resize.observe(room);
    const work = room?.closest('.work');
    if (work) resize.observe(work);
    // The window's overlays come and go anywhere in it, and the split screen redraws its panes.
    overlays = new MutationObserver(sync);
    overlays.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ['style'] });
    window.addEventListener('resize', sync);
    // Its band as the backend keeps it: a request may have come while it was elsewhere.
    void loadConsents(key);
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
    // Two pages on screen (two panes): the keys go to the active pane's.
    if (pane && !pane.active) return;
    if (devtoolsKey(e, IS_MAC)) {
      e.preventDefault();
      void devtools(key);
    } else if (modeKey(e, IS_MAC)) {
      e.preventDefault();
      cycleMode(key);
    }
  }

  function submit(e: SubmitEvent) {
    e.preventDefault();
    const to = typed?.trim();
    typed = null;
    if (!to || !page) return;
    const url = typedAddress(to, page.url);
    void go(key, 'to', url);
  }

  const logs = $derived(flow?.lines.find((l) => l.launchId && l.state !== 'ready')?.launchId ?? flow?.lines.at(-1)?.launchId);
  function viewLogs() {
    if (logs) app.selectLaunch(logs);
  }

  /** What the bar, folded by its width (its container query), hides: « … » offers it then. */
  const hidden = (sel: string) => {
    const el = bar?.querySelector(sel);
    return !!el && getComputedStyle(el).display === 'none';
  };

  /** « … »: the split screen's « Ouvrir à côté » and « Ouvrir en dessous », and what the bar folded. */
  function moreMenu(el: HTMLElement) {
    if (!page) return;
    const items: MenuItem[] = [];
    if (hidden('.fwd')) items.push({ label: t('devpage.forward'), icon: 'right', onClick: () => go(key, 'forward') });
    if (hidden('.modes')) {
      if (items.length) items.push({ label: '', separator: true });
      for (const m of PAGE_MODES)
        items.push({
          label: t(`devpage.modes.${m}`),
          icon: MODE_ICON[m],
          checked: mode === m,
          hint: m === 'full' ? undefined : t('devpage.width', { width: DEVICE_WIDTH[m] }),
          onClick: () => setMode(key, m),
        });
    }
    if (items.length) items.push({ label: '', separator: true });
    if (ownPane) {
      items.push({
        label: t('shell.panes.close'),
        icon: 'close',
        hint: keyLabel('Ctrl+Shift+W'),
        onClick: () => app.closePaneOf(`page:${key}`),
      });
    } else if (besideOf(key) || app.pageShown !== key) {
      // Beside what is on screen; shown alone, beside its agent's conversation (a browser opened alone has none).
      items.push(
        {
          label: t('shell.panes.openBeside'),
          icon: 'pane-r',
          onClick: () => openPageBeside(key, 'right'),
        },
        { label: t('shell.panes.openBelow'), icon: 'pane-b', onClick: () => openPageBeside(key, 'bottom') },
      );
    }
    if (items.length) items.push({ label: '', separator: true });
    items.push({
      label: t('devpage.browsers.copy'),
      icon: 'copy',
      onClick: () => app.run(navigator.clipboard.writeText(page.url).then(() => app.toast(t('devpage.browsers.copied'), 'info'))),
    });
    if (hidden('.system')) items.push({ label: '', separator: true });
    if (hidden('.system')) items.push({ label: t('devpage.system'), icon: 'ext', onClick: () => app.run(openUrl(page.url)) });
    menu.showAt(el, items);
  }

  /** Its chip dragged onto the work area: the page into a pane, as a terminal's row (ESC-419). */
  function pressChip(e: PointerEvent) {
    if (!page || ownPane) return;
    paneDrag.press(e, pageItem(key), t('devpage.owner', { title: page.title }));
  }
</script>

<svelte:window onkeydown={onKey} />

<section class="devpage" aria-label={page ? t('devpage.owner', { title: page.title }) : t('devpage.view')}>
  {#if page}
    <div class="bar" bind:this={bar}>
      <div class="nav">
        <button class="icon-btn" aria-label={t('devpage.back')} use:tip={t('devpage.back')} onclick={() => go(key, 'back')}
          ><Icon name="left" size={14} /></button
        >
        <button class="icon-btn fwd" aria-label={t('devpage.forward')} use:tip={t('devpage.forward')} onclick={() => go(key, 'forward')}
          ><Icon name="right" size={14} /></button
        >
        <button class="icon-btn" aria-label={t('devpage.reload')} use:tip={t('devpage.reload')} onclick={() => go(key, 'reload')}
          ><Icon name="restart" size={14} /></button
        >
      </div>
      <form class="field addr" onsubmit={submit}>
        <i class="dot" class:wait={page.phase === 'loading' || waiting} class:bad={page.phase === 'stalled' || stopped}></i>
        <input
          class="mono"
          aria-label={t('devpage.address')}
          spellcheck="false"
          autocomplete="off"
          value={typed ?? page.url}
          oninput={(e) => (typed = e.currentTarget.value)}
          onblur={() => (typed = null)}
          onkeydown={(e) => e.key === 'Escape' && ((typed = null), e.currentTarget.blur())}
        />
      </form>
      {#if agent}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <span class="chip owner" class:grab={!ownPane} use:tip={t('devpage.apart')} onpointerdown={pressChip}
          ><Icon name="branch" size={12} /><span class="t">{page.title}</span></span
        >
      {/if}
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
        <span
          class="width"
          class:shrunk={fit.shrunk}
          use:tip={fit.shrunk ? t('devpage.shrunk', { width: fit.width, device: DEVICE_WIDTH[mode] }) : undefined}
          >{fit.shrunk ? t('devpage.shrunkShort', { width: fit.width }) : t('devpage.width', { width: DEVICE_WIDTH[mode] })}</span
        >
      {/if}
      <div class="acts">
        <button
          class="btn dt"
          aria-label={t('devpage.devtoolsHint', { key: shortcut })}
          aria-describedby="devtools-where-{key}"
          use:tip={`${t('devpage.devtoolsHint', { key: shortcut })}. ${devtoolsWhere}`}
          onclick={() => devtools(key)}
          ><Icon name="code" size={14} /><span class="lbl">{t('devpage.devtools')}</span><kbd>{shortcut}</kbd></button
        >
        <span class="sr" id="devtools-where-{key}">{devtoolsWhere}</span>
        <button
          class="btn icon-only system"
          aria-label={t('devpage.system')}
          use:tip={t('devpage.system')}
          onclick={() => app.run(openUrl(page.url))}><Icon name="ext" size={14} /></button
        >
        <button
          class="btn icon-only more"
          aria-label={t('devpage.more')}
          aria-haspopup="menu"
          use:tip={t('devpage.more')}
          onclick={(e) => moreMenu(e.currentTarget)}><Icon name="more" size={14} /></button
        >
      </div>
    </div>
    {#if page.phase === 'loading' && !waiting && !stopped}<div class="load" aria-hidden="true"></div>{/if}
    <!-- An agent asks to read this page, outside its project's launch (ESC-669): native, above the page. -->
    <ConsentBand asks={page.consents?.asks ?? []} onanswer={(ask, allow) => answerConsent(key, ask, allow)} />
  {/if}

  <div class="room" bind:this={room} class:on={visible} class:device={mode !== 'full' && visible}>
    {#if mode !== 'full' && visible}
      <div class="frame" aria-hidden="true" style:left="{fit.left - 1}px" style:width="{fit.width + 2}px"></div>
    {/if}
    {#if !page}
      <div class="msg"><p>{t('devpage.waitingText')}</p></div>
    {:else if waiting}
      <div class="msg" role="status">
        <i class="dot wait"></i><b>{t('devpage.waiting')}</b>
        <p>{t('devpage.waitingText')}</p>
        <div class="acts"><button class="btn" disabled={!logs} onclick={viewLogs}>{t('devpage.viewLogs')}</button></div>
      </div>
    {:else if stopped}
      <div class="msg" role="status">
        <i class="dot bad"></i><b>{t('devpage.stopped')}</b>
        <p>{t('devpage.stoppedText')}</p>
        <div class="acts">
          <button class="btn primary" disabled={!project || !agent} onclick={() => project && agent && testAgent(agent, project)}
            >{t('devpage.relaunch')}</button
          >
          <button class="btn" disabled={!logs} onclick={viewLogs}>{t('devpage.viewLogs')}</button>
        </div>
      </div>
    {:else if page.phase === 'stalled'}
      <div class="msg" role="status">
        <i class="dot bad"></i><b>{t('devpage.stalled', { host: hostOf(page.url) })}</b>
        <!-- A browser opened alone has no launch, so no logs to look at. -->
        <p>{agent ? t('devpage.stalledText') : t('devpage.stalledAlone')}</p>
        <div class="acts">
          <button class="btn primary" onclick={() => go(key, 'reload')}>{t('devpage.reload')}</button>
          {#if agent}<button class="btn" disabled={!logs} onclick={viewLogs}>{t('devpage.viewLogs')}</button>{/if}
        </div>
      </div>
    {:else if covered || paneDrag.on}
      <div class="msg quiet"><p>{t('devpage.covered')}</p></div>
    {/if}
  </div>
</section>

<style>
  .devpage {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    min-width: 0;
    /* Its bar folds by its own width: in its conversation as in a pane of its own. */
    container: devpage / inline-size;
  }
  /* A toolbar of the 2.0 design, as the terminal's and the editor's: 30 px controls, 6 px apart. */
  .bar {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 46px;
    padding: 0 12px 0 10px;
    border-bottom: 1px solid var(--line);
    flex: none;
  }
  .nav {
    display: flex;
    flex: none;
    gap: 2px;
  }
  /* The address: a field of the app, the state of its page as a dot before it. */
  .addr {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0 2px;
    cursor: text;
  }
  .addr input {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0;
    border: 0;
    outline: 0;
    background: none;
    color: var(--text);
    font-size: 12.5px;
  }
  .dot {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--run);
  }
  .dot.wait {
    background: var(--wait);
  }
  .dot.bad {
    background: var(--del);
  }
  /* « worktree · processus », as the shell of a terminal is named in its header; dragged, the page goes into a pane. */
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    max-width: 220px;
    height: 20px;
    padding: 0 7px;
    border-radius: 6px;
    background: rgba(var(--fg-rgb), 0.06);
    color: var(--soft);
    font-size: 11.5px;
    font-weight: 550;
    white-space: nowrap;
  }
  .chip.grab {
    cursor: grab;
  }
  .chip .t {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .chip :global(.icon) {
    flex: none;
    color: var(--muted);
  }
  .modes {
    flex: none;
  }
  .acts {
    display: flex;
    flex: none;
    align-items: center;
    gap: 6px;
  }
  .dt kbd {
    margin-right: -3px;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  .load {
    flex: none;
    height: 2px;
    background: linear-gradient(90deg, var(--p-ink) 0 40%, transparent 40%);
    background-size: 200% 100%;
    animation: devpage-load 1.2s cubic-bezier(0.22, 1, 0.36, 1) infinite;
  }
  @keyframes devpage-load {
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
  .room {
    position: relative;
    flex: 1;
    min-height: 0;
    display: grid;
    place-items: center;
  }
  .msg {
    max-width: 380px;
    padding: 24px;
    text-align: center;
  }
  .msg b {
    display: block;
    margin: 10px 0 4px;
    font-size: 13.5px;
  }
  .msg p {
    margin: 0 0 12px;
    color: var(--muted);
    font-size: 12.5px;
  }
  .msg .dot {
    display: inline-block;
    width: 9px;
    height: 9px;
  }
  .msg.quiet p {
    margin: 0;
  }
  .msg .acts {
    justify-content: center;
    gap: 8px;
  }
  .width {
    flex: none;
    color: var(--muted);
    font-size: 11.5px;
    font-feature-settings: 'tnum';
    white-space: nowrap;
  }
  .width.shrunk {
    color: var(--wait);
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
  /* Narrow: forward, the owner and the browser go into « … », the widths stay beside the address (ESC-657). */
  @container devpage (max-width: 760px) {
    .fwd,
    .owner,
    .system,
    .dt .lbl,
    .dt kbd {
      display: none;
    }
    .dt {
      width: 30px;
      padding: 0;
    }
  }
  /* Narrower still (a pane): the widths go into « … » too; the address keeps its room. */
  @container devpage (max-width: 560px) {
    .modes {
      display: none;
    }
  }
</style>
