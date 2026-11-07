<script lang="ts">
  // The bar of a project's « Pages » window (ESC-311): a tab per worktree whose page was detached from its agent, and
  // the bar of the page shown. The pages themselves are webviews of the backend's, under this bar; this one is
  // Escouade's own interface, the only part of the window that may call its commands.
  import { listen } from '@tauri-apps/api/event';
  import { onDestroy, onMount } from 'svelte';
  import {
    DEVICE_WIDTH,
    deviceFit,
    devtoolsKey,
    modeKey,
    nextMode,
    PAGE_MODES,
    saveMode,
    typedAddress,
    type PageMode,
  } from '../lib/devpage-key';
  import { t } from '../lib/i18n';
  import { api } from '../lib/ipc';
  import { openUrl } from '../lib/opener';
  import { IS_MAC } from '../lib/platform';
  import type { PageView } from '../lib/types';
  import { tip } from '../lib/tip';
  import Icon from './Icon.svelte';

  let { projectId }: { projectId: string } = $props();

  let pages = $state<PageView[]>([]);
  let shown = $state<string | null>(null);
  let typed = $state<string | null>(null);
  const page = $derived(pages.find((p) => p.agentId === shown) ?? null);
  const shortcut = IS_MAC ? '⌥⌘I' : 'F12';
  const MODE_ICON = { full: 'monitor', tablet: 'tablet', mobile: 'phone' } as const;
  const mode = $derived<PageMode>(page?.mode ?? 'full');
  /** The window's width: its page is laid out in it, centred and narrowed as in the agent's view. */
  let width = $state(window.innerWidth);
  const fit = $derived(deviceFit(mode, width));

  /** Shows the page at `m`, kept for its next launches as in the agent's view. */
  function setMode(m: PageMode) {
    if (!page) return;
    saveMode(page.title, m);
    void act(api.devpageMode(page.agentId, m));
  }

  const take = (of: { pages: PageView[]; shown: string | null }) => {
    pages = of.pages.filter((p) => p.detached);
    shown = of.shown;
  };

  let unlisten: (() => void) | undefined;
  onMount(async () => {
    unlisten = await listen<{ pages: PageView[]; shown: string | null }>('devpages', (e) => take(e.payload));
    take(await api.devpageList(projectId));
  });
  onDestroy(() => unlisten?.());

  const act = (p: Promise<unknown>) => p.catch(() => {});

  function submit(e: SubmitEvent) {
    e.preventDefault();
    const to = typed?.trim();
    typed = null;
    if (!to || !page) return;
    const url = typedAddress(to, page.url);
    void act(api.devpageGo(page.agentId, 'to', url));
  }
</script>

<svelte:window
  bind:innerWidth={width}
  onkeydown={(e) => {
    if (page && devtoolsKey(e, IS_MAC)) {
      e.preventDefault();
      void act(api.devpageDevtools(page.agentId));
    } else if (page && modeKey(e, IS_MAC)) {
      e.preventDefault();
      setMode(nextMode(mode));
    }
  }}
/>

<div class="pw">
  <div class="tabs" role="tablist" aria-label={t('devpage.window.tabs')}>
    {#each pages as p (p.agentId)}
      <button role="tab" aria-selected={p.agentId === shown} onclick={() => act(api.devpageShow(p.agentId))}
        ><i class="d"></i>{p.title}</button
      >
    {:else}
      <span class="empty">{t('devpage.window.empty')}</span>
    {/each}
  </div>
  {#if page}
    <div class="bar">
      <button class="ib" aria-label={t('devpage.back')} onclick={() => act(api.devpageGo(page.agentId, 'back'))}
        ><Icon name="left" size={16} /></button
      >
      <button class="ib" aria-label={t('devpage.forward')} onclick={() => act(api.devpageGo(page.agentId, 'forward'))}
        ><Icon name="right" size={16} /></button
      >
      <button class="ib" aria-label={t('devpage.reload')} onclick={() => act(api.devpageGo(page.agentId, 'reload'))}
        ><Icon name="restart" size={16} /></button
      >
      <form class="addr" onsubmit={submit}>
        <i class="d"></i>
        <input
          class="mono"
          aria-label={t('devpage.address')}
          spellcheck="false"
          value={typed ?? page.url}
          oninput={(e) => (typed = e.currentTarget.value)}
          onblur={() => (typed = null)}
        />
      </form>
      <div class="modes" role="radiogroup" aria-label={t('devpage.modesLabel')}>
        {#each PAGE_MODES as m (m)}
          <button
            role="radio"
            aria-checked={mode === m}
            aria-label={t(`devpage.modes.${m}`)}
            use:tip={t(`devpage.modes.${m}`)}
            onclick={() => setMode(m)}><Icon name={MODE_ICON[m]} size={14} /></button
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
      <button class="dt" aria-label={t('devpage.devtoolsHint', { key: shortcut })} onclick={() => act(api.devpageDevtools(page.agentId))}
        ><Icon name="code" size={14} />{t('devpage.devtools')}<kbd>{shortcut}</kbd></button
      >
      <button
        class="ib"
        aria-label={t('devpage.attach')}
        use:tip={t('devpage.attach')}
        onclick={() => act(api.devpageDetach(page.agentId, false))}><Icon name="collapse" size={16} /></button
      >
      <button class="ib" aria-label={t('devpage.system')} use:tip={t('devpage.system')} onclick={() => act(openUrl(page.url))}
        ><Icon name="ext" size={16} /></button
      >
    </div>
  {/if}
</div>

<style>
  :global(html),
  :global(body) {
    background: var(--frame);
  }
  .pw {
    display: flex;
    flex-direction: column;
    height: 84px;
    color: var(--text);
    font-size: 13px;
  }
  .tabs {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 38px;
    padding: 0 10px;
    overflow: hidden;
  }
  .tabs button {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 30px;
    padding: 0 12px;
    border-radius: 8px 8px 0 0;
    color: var(--muted);
    font-weight: 550;
    white-space: nowrap;
  }
  .tabs button[aria-selected='true'] {
    background: var(--panel);
    color: var(--text);
  }
  .empty {
    align-self: center;
    color: var(--muted);
  }
  .d {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--run);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 46px;
    padding: 0 10px;
    background: var(--panel);
    border-bottom: 1px solid var(--line);
  }
  .ib {
    display: inline-grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 7px;
    color: var(--muted);
  }
  .ib:hover,
  .dt:hover {
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
  .addr input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: none;
    color: var(--text);
    font-size: 12.5px;
  }
  .dt {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 9px;
    border-radius: 7px;
    color: var(--soft);
    font-size: 12px;
    font-weight: 550;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .modes {
    display: inline-flex;
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
  .modes button[aria-checked='true'] {
    background: var(--sel);
    color: var(--text);
  }
  .width {
    margin: 0 4px;
    color: var(--muted);
    font-size: 11.5px;
    white-space: nowrap;
  }
  .width.shrunk {
    color: var(--wait);
  }
  .dt kbd {
    padding: 1px 4px;
    border-radius: 4px;
    color: var(--muted);
    font-size: 10.5px;
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
</style>
