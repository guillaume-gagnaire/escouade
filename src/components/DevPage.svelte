<script lang="ts">
  // The agent's view « Page » (ESC-311): the bar of its launch's page, and the room where the page, a webview of its
  // own drawn over the window, is placed. The page is hidden whenever something of the window passes over it (a menu,
  // a list, a dialog, the palette), whenever its launch is not up, and when the view goes.
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
    place,
    setMode,
  } from '../lib/devpage.svelte';
  import { DEVICE_WIDTH, deviceFit, modeKey, PAGE_MODES, typedAddress, type PageMode } from '../lib/devpage-key';
  import { zoomState } from '../lib/display.svelte';
  import { t } from '../lib/i18n';
  import { api } from '../lib/ipc';
  import { openUrl } from '../lib/opener';
  import { IS_MAC } from '../lib/platform';
  import { app } from '../lib/state.svelte';
  import { anyRunning, flows, testAgent } from '../lib/test-launch.svelte';
  import { tip } from '../lib/tip';
  import type { Agent } from '../lib/types';
  import Icon from './Icon.svelte';

  let { agent }: { agent: Agent } = $props();

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
  let more = $state(false);

  let room: HTMLDivElement | undefined = $state();
  let covered = $state(false);
  let typed = $state<string | null>(null);

  /** The page shows: loaded or loading, its launch up, in this view, nothing over it. */
  const visible = $derived(!!page && !page.detached && !waiting && !stopped && page.phase !== 'stalled' && !covered);

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
      void place(agent.id, show ? boundsOf(shown, zoomState.shown) : null);
    });
  }

  $effect(() => {
    // Again whenever what it hangs on changes.
    void [visible, zoomState.shown, page?.phase, page?.detached, mode];
    sync();
  });

  let resize: ResizeObserver | undefined;
  let overlays: MutationObserver | undefined;
  onMount(() => {
    resize = new ResizeObserver(sync);
    if (room) resize.observe(room);
    // The window's overlays come and go anywhere in it.
    overlays = new MutationObserver(sync);
    overlays.observe(document.body, { childList: true, subtree: true });
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
    if (devtoolsKey(e, IS_MAC)) {
      e.preventDefault();
      void devtools(agent.id);
    } else if (modeKey(e, IS_MAC)) {
      e.preventDefault();
      cycleMode(agent.id);
    } else if (e.key === 'Escape' && more) more = false;
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
</script>

<svelte:window onkeydown={onKey} />

<section class="devpage" aria-label={page ? t('devpage.owner', { title: page.title }) : t('devpage.view')}>
  {#if page}
    <div class="bar">
      <button class="ib" aria-label={t('devpage.back')} use:tip={t('devpage.back')} onclick={() => go(agent.id, 'back')}
        ><Icon name="left" size={16} /></button
      >
      <button class="ib fwd" aria-label={t('devpage.forward')} use:tip={t('devpage.forward')} onclick={() => go(agent.id, 'forward')}
        ><Icon name="right" size={16} /></button
      >
      <button class="ib" aria-label={t('devpage.reload')} use:tip={t('devpage.reload')} onclick={() => go(agent.id, 'reload')}
        ><Icon name="restart" size={16} /></button
      >
      <form class="addr" onsubmit={submit}>
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
      <span class="owner" use:tip={t('devpage.apart')}><Icon name="branch" size={12} />{page.title}</span>
      <span class="sep" aria-hidden="true"></span>
      <div class="modes wide-only" role="radiogroup" aria-label={t('devpage.modesLabel')}>
        {#each PAGE_MODES as m (m)}
          <button
            role="radio"
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
      <button
        class="dt"
        aria-label={t('devpage.devtoolsHint', { key: shortcut })}
        use:tip={t('devpage.devtoolsHint', { key: shortcut })}
        onclick={() => devtools(agent.id)}
        ><Icon name="code" size={14} /><span class="lbl">{t('devpage.devtools')}</span><kbd>{shortcut}</kbd></button
      >
      <button
        class="ib wide-only"
        aria-label={page.detached ? t('devpage.attach') : t('devpage.detach')}
        use:tip={page.detached ? t('devpage.attach') : t('devpage.detach')}
        onclick={() => detach(agent.id, !page.detached)}><Icon name="expand" size={16} /></button
      >
      <button class="ib wide-only" aria-label={t('devpage.system')} use:tip={t('devpage.system')} onclick={() => app.run(openUrl(page.url))}
        ><Icon name="ext" size={16} /></button
      >
      <!-- Narrow: the secondary buttons in « … ». -->
      <div class="more-wrap narrow-only">
        <button class="ib" aria-label={t('devpage.more')} aria-haspopup="menu" aria-expanded={more} onclick={() => (more = !more)}
          ><Icon name="more" size={16} /></button
        >
        {#if more}
          <div class="menu" role="menu" aria-label={t('devpage.more')}>
            <button role="menuitem" onclick={() => ((more = false), go(agent.id, 'forward'))}
              ><Icon name="right" size={14} />{t('devpage.forward')}</button
            >
            <div class="menu-sep" role="separator"></div>
            {#each PAGE_MODES as m (m)}
              <button role="menuitemradio" aria-checked={mode === m} onclick={() => ((more = false), setMode(agent.id, m))}
                ><Icon name={MODE_ICON[m]} size={14} />{t(`devpage.modes.${m}`)}{#if m !== 'full'}<span class="mw"
                    >{DEVICE_WIDTH[m]} px</span
                  >{/if}</button
              >
            {/each}
            <div class="menu-sep" role="separator"></div>
            <button role="menuitem" onclick={() => ((more = false), detach(agent.id, !page.detached))}
              ><Icon name="expand" size={14} />{page.detached ? t('devpage.attach') : t('devpage.detach')}</button
            >
            <button role="menuitem" onclick={() => ((more = false), app.run(openUrl(page.url)))}
              ><Icon name="ext" size={14} />{t('devpage.system')}</button
            >
          </div>
        {/if}
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
    {:else if covered}
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
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 46px;
    padding: 0 10px;
    border-bottom: 1px solid var(--line);
    flex: none;
  }
  .ib {
    display: inline-grid;
    place-items: center;
    flex: none;
    width: 28px;
    height: 28px;
    border-radius: 7px;
    color: var(--muted);
  }
  .ib:hover {
    background: var(--hover);
    color: var(--text);
  }
  .addr {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 30px;
    margin: 0 4px;
    padding: 0 10px;
    border-radius: 7px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .addr:focus-within {
    box-shadow: inset 0 0 0 1px var(--ring, var(--p-ink));
  }
  .addr input {
    flex: 1;
    min-width: 0;
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
  .owner {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    max-width: 220px;
    height: 28px;
    padding: 0 9px;
    border-radius: 7px;
    overflow: hidden;
    color: var(--soft);
    font-size: 12px;
    font-weight: 550;
    white-space: nowrap;
    text-overflow: ellipsis;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .sep {
    flex: none;
    width: 1px;
    height: 18px;
    margin: 0 4px;
    background: var(--line);
  }
  .dt {
    display: inline-flex;
    align-items: center;
    flex: none;
    gap: 6px;
    height: 28px;
    padding: 0 9px;
    border-radius: 7px;
    color: var(--soft);
    font-size: 12px;
    font-weight: 550;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .dt:hover {
    background: var(--hover);
    color: var(--text);
  }
  .dt kbd {
    padding: 1px 4px;
    border-radius: 4px;
    color: var(--muted);
    font: 500 10.5px inherit;
    box-shadow: inset 0 0 0 1px var(--line-2);
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
  .acts {
    display: flex;
    justify-content: center;
    gap: 8px;
  }
  .modes {
    display: inline-flex;
    flex: none;
    gap: 2px;
    padding: 2px;
    border-radius: 8px;
    background: rgba(var(--fg-rgb), 0.035);
  }
  .modes button {
    display: inline-grid;
    place-items: center;
    width: 26px;
    height: 24px;
    border-radius: 6px;
    color: var(--muted);
  }
  .modes button:hover {
    color: var(--text);
  }
  .modes button[aria-checked='true'] {
    background: var(--sel);
    color: var(--text);
  }
  .width {
    flex: none;
    margin: 0 4px;
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
  .narrow-only {
    display: none;
  }
  .more-wrap {
    position: relative;
  }
  .menu {
    position: absolute;
    top: 34px;
    right: 0;
    z-index: 20;
    min-width: 240px;
    padding: 5px;
    border-radius: 10px;
    background: var(--raise);
    box-shadow:
      0 0 0 1px var(--line-2),
      0 14px 34px -8px rgba(0, 0, 0, 0.6);
  }
  .menu button {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    height: 30px;
    padding: 0 9px;
    border-radius: 7px;
    color: var(--soft);
    text-align: left;
  }
  .menu button:hover,
  .menu button[aria-checked='true'] {
    background: var(--hover);
    color: var(--text);
  }
  .menu .mw {
    margin-left: auto;
    color: var(--muted);
    font-size: 11.5px;
  }
  .menu-sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--line);
  }
  /* Narrow: forward, the owner, the widths, the window and the browser go into « … »; the address keeps its room. */
  @container conv (max-width: 760px) {
    .fwd,
    .owner,
    .wide-only,
    .dt .lbl,
    .dt kbd {
      display: none;
    }
    .narrow-only {
      display: block;
    }
  }
</style>
