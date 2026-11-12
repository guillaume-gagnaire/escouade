<script lang="ts">
  // The agent's view « Page » (ESC-311): the bar of its launch's page, and the room where the page, a webview of its
  // own drawn over the window, is placed. The page is hidden whenever something of the window passes over it (a menu,
  // a list, a dialog, the palette, the compass of a drag), whenever its launch is not up, and when the view goes. It is
  // drawn in its agent's conversation, or in a pane of its own beside it (ESC-419): the webview follows its room there.
  // Its bar is drawn with the app's controls (ESC-419): `icon-btn`, `btn`, `segmented small`, `field`, the app's menu.
  import { onDestroy, onMount } from 'svelte';
  import {
    boundsOf,
    coveredAt,
    cycleMode,
    detach,
    devpages,
    devtools,
    devtoolsKey,
    go,
    hostOf,
    modeOf,
    openPageBeside,
    pageItem,
    place,
    setMode,
  } from '../lib/devpage.svelte';
  import { DEVICE_WIDTH, deviceFit, modeKey, PAGE_MODES, typedAddress, type PageMode } from '../lib/devpage-key';
  import { zoomState } from '../lib/display.svelte';
  import { t } from '../lib/i18n';
  import { api } from '../lib/ipc';
  import { menu, type MenuItem } from '../lib/menu.svelte';
  import { openUrl } from '../lib/opener';
  import { paneCtx } from '../lib/pane-context';
  import { paneDrag } from '../lib/pane-drag.svelte';
  import { IS_MAC, keyLabel } from '../lib/platform';
  import { app } from '../lib/state.svelte';
  import { anyRunning, flows, testAgent } from '../lib/test-launch.svelte';
  import { tip } from '../lib/tip';
  import type { Agent } from '../lib/types';
  import Icon from './Icon.svelte';

  let { agent }: { agent: Agent } = $props();

  /** Its pane of the split screen, when it is in one (its own, or its conversation's). */
  const pane = paneCtx();
  /** In a pane of its own (`page:<agent>`), not in its conversation's. */
  const ownPane = $derived(pane?.key === `page:${agent.id}`);

  const page = $derived(devpages.pages[agent.id]);
  const flow = $derived(flows.all[agent.id]);
  const project = $derived(app.projects.find((p) => p.id === agent.projectId));
  /** Its launch is starting again: the page waits for it. */
  const waiting = $derived(flow?.phase === 'running');
  /** Its launch is no longer up (stopped, crashed, failed). */
  const stopped = $derived(!waiting && !anyRunning(agent.id));
  const shortcut = IS_MAC ? '⌥⌘I' : 'F12';
  const modeShortcut = IS_MAC ? '⇧⌘M' : t('devpage.modeKey');
  const MODE_ICON = { full: 'monitor', tablet: 'tablet', mobile: 'phone' } as const;
  /** The width it is shown at, as chosen for this page (« worktree · processus »). */
  const mode = $derived<PageMode>(page ? modeOf(agent.id) : 'full');
  /** The width of its room, in CSS pixels: a device narrower than it is centred, a wider one narrowed to it. */
  let roomWidth = $state(0);
  const fit = $derived(deviceFit(mode, roomWidth));

  let bar: HTMLDivElement | undefined = $state();
  let room: HTMLDivElement | undefined = $state();
  let covered = $state(false);
  let typed = $state<string | null>(null);

  /** The page shows: loaded or loading, its launch up, in this view, nothing over it, no content dragged. */
  const visible = $derived(!!page && !page.detached && !waiting && !stopped && page.phase !== 'stalled' && !covered && !paneDrag.on);

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
      // Only what changed goes to the backend: the observers below fire far more often than the page moves.
      const said = JSON.stringify(bounds);
      if (said === placed) return;
      placed = said;
      void place(agent.id, bounds);
    });
  }
  /** The last place told to the backend. */
  let placed = '';

  $effect(() => {
    // Again whenever what it hangs on changes.
    void [visible, zoomState.shown, page?.phase, page?.detached, mode, paneDrag.on];
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
  });
  onDestroy(() => {
    cancelAnimationFrame(frame);
    resize?.disconnect();
    overlays?.disconnect();
    window.removeEventListener('resize', sync);
    void place(agent.id, null);
  });

  function onKey(e: KeyboardEvent) {
    if (!page || page.detached) return;
    // Two pages on screen (two panes): the keys go to the active pane's.
    if (pane && !pane.active) return;
    if (devtoolsKey(e, IS_MAC)) {
      e.preventDefault();
      void devtools(agent.id);
    } else if (modeKey(e, IS_MAC)) {
      e.preventDefault();
      cycleMode(agent.id);
    }
  }

  function submit(e: SubmitEvent) {
    e.preventDefault();
    const to = typed?.trim();
    typed = null;
    if (!to || !page) return;
    const url = typedAddress(to, page.url);
    void go(agent.id, 'to', url);
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
    if (hidden('.fwd')) items.push({ label: t('devpage.forward'), icon: 'right', onClick: () => go(agent.id, 'forward') });
    if (hidden('.modes')) {
      if (items.length) items.push({ label: '', separator: true });
      for (const m of PAGE_MODES)
        items.push({
          label: t(`devpage.modes.${m}`),
          icon: MODE_ICON[m],
          checked: mode === m,
          hint: m === 'full' ? undefined : t('devpage.width', { width: DEVICE_WIDTH[m] }),
          onClick: () => setMode(agent.id, m),
        });
    }
    if (items.length) items.push({ label: '', separator: true });
    if (ownPane) {
      items.push({
        label: t('shell.panes.close'),
        icon: 'close',
        hint: keyLabel('Ctrl+Shift+W'),
        onClick: () => app.closePaneOf(`page:${agent.id}`),
      });
    } else {
      items.push(
        {
          label: t('shell.panes.openBeside'),
          icon: 'pane-r',
          onClick: () => openPageBeside(agent.id, 'right'),
        },
        { label: t('shell.panes.openBelow'), icon: 'pane-b', onClick: () => openPageBeside(agent.id, 'bottom') },
      );
    }
    if (hidden('.detach') || hidden('.system')) items.push({ label: '', separator: true });
    if (hidden('.detach'))
      items.push({
        label: page.detached ? t('devpage.attach') : t('devpage.detach'),
        icon: 'expand',
        onClick: () => detach(agent.id, !page.detached),
      });
    if (hidden('.system')) items.push({ label: t('devpage.system'), icon: 'ext', onClick: () => app.run(openUrl(page.url)) });
    menu.showAt(el, items);
  }

  /** Its chip dragged onto the work area: the page into a pane, as a terminal's row (ESC-419). */
  function pressChip(e: PointerEvent) {
    if (!page || page.detached || ownPane) return;
    paneDrag.press(e, pageItem(agent.id), t('devpage.owner', { title: page.title }));
  }
</script>

<svelte:window onkeydown={onKey} />

<section class="devpage" aria-label={page ? t('devpage.owner', { title: page.title }) : t('devpage.view')}>
  {#if page}
    <div class="bar" bind:this={bar}>
      <div class="nav">
        <button class="icon-btn" aria-label={t('devpage.back')} use:tip={t('devpage.back')} onclick={() => go(agent.id, 'back')}
          ><Icon name="left" size={14} /></button
        >
        <button
          class="icon-btn fwd"
          aria-label={t('devpage.forward')}
          use:tip={t('devpage.forward')}
          onclick={() => go(agent.id, 'forward')}><Icon name="right" size={14} /></button
        >
        <button class="icon-btn" aria-label={t('devpage.reload')} use:tip={t('devpage.reload')} onclick={() => go(agent.id, 'reload')}
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
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <span class="chip owner" class:grab={!ownPane} use:tip={t('devpage.apart')} onpointerdown={pressChip}
        ><Icon name="branch" size={12} /><span class="t">{page.title}</span></span
      >
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
            onclick={() => setMode(agent.id, m)}><Icon name={MODE_ICON[m]} size={14} /></button
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
          use:tip={t('devpage.devtoolsHint', { key: shortcut })}
          onclick={() => devtools(agent.id)}
          ><Icon name="code" size={14} /><span class="lbl">{t('devpage.devtools')}</span><kbd>{shortcut}</kbd></button
        >
        <button
          class="btn icon-only detach"
          aria-label={page.detached ? t('devpage.attach') : t('devpage.detach')}
          use:tip={page.detached ? t('devpage.attach') : t('devpage.detach')}
          onclick={() => detach(agent.id, !page.detached)}><Icon name="expand" size={14} /></button
        >
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
  {/if}

  <div class="room" bind:this={room} class:on={visible} class:device={mode !== 'full' && visible}>
    {#if mode !== 'full' && visible}
      <div class="frame" aria-hidden="true" style:left="{fit.left - 1}px" style:width="{fit.width + 2}px"></div>
    {/if}
    {#if !page}
      <div class="msg"><p>{t('devpage.waitingText')}</p></div>
    {:else if page.detached}
      <div class="msg" role="status">
        <b>{t('devpage.detached', { project: project?.name ?? '' })}</b>
        <div class="acts">
          <button class="btn primary" onclick={() => app.run(api.devpageShow(agent.id))}>{t('devpage.showWindow')}</button>
          <button class="btn" onclick={() => detach(agent.id, false)}>{t('devpage.attach')}</button>
        </div>
      </div>
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
          <button class="btn primary" disabled={!project} onclick={() => project && testAgent(agent, project)}
            >{t('devpage.relaunch')}</button
          >
          <button class="btn" disabled={!logs} onclick={viewLogs}>{t('devpage.viewLogs')}</button>
        </div>
      </div>
    {:else if page.phase === 'stalled'}
      <div class="msg" role="status">
        <i class="dot bad"></i><b>{t('devpage.stalled', { host: hostOf(page.url) })}</b>
        <p>{t('devpage.stalledText')}</p>
        <div class="acts">
          <button class="btn primary" onclick={() => go(agent.id, 'reload')}>{t('devpage.reload')}</button>
          <button class="btn" disabled={!logs} onclick={viewLogs}>{t('devpage.viewLogs')}</button>
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
  /* Narrow: forward, the owner, the widths, the window and the browser go into « … »; the address keeps its room. */
  @container devpage (max-width: 760px) {
    .fwd,
    .owner,
    .modes,
    .detach,
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
</style>
