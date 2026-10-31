<script lang="ts">
  // The bar of a project's « Pages » window (ESC-311): a tab per worktree whose page was detached from its agent, and
  // the bar of the page shown. The pages themselves are webviews of the backend's, under this bar; this one is
  // Escouade's own interface, the only part of the window that may call its commands.
  import { listen } from '@tauri-apps/api/event';
  import { onDestroy, onMount } from 'svelte';
  import { devtoolsKey, typedAddress } from '../lib/devpage-key';
  import { t } from '../lib/i18n';
  import { api } from '../lib/ipc';
  import { openUrl } from '../lib/opener';
  import { IS_MAC } from '../lib/platform';
  import type { PageView } from '../lib/types';
  import Icon from './Icon.svelte';

  let { projectId }: { projectId: string } = $props();

  let pages = $state<PageView[]>([]);
  let shown = $state<string | null>(null);
  let typed = $state<string | null>(null);
  const page = $derived(pages.find((p) => p.agentId === shown) ?? null);
  const shortcut = IS_MAC ? '⌥⌘I' : 'F12';

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
  onkeydown={(e) => {
    if (page && devtoolsKey(e, IS_MAC)) {
      e.preventDefault();
      void act(api.devpageDevtools(page.agentId));
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
      <button class="dt" aria-label={t('devpage.devtoolsHint', { key: shortcut })} onclick={() => act(api.devpageDevtools(page.agentId))}
        ><Icon name="code" size={14} />{t('devpage.devtools')}<kbd>{shortcut}</kbd></button
      >
      <button
        class="ib"
        aria-label={t('devpage.attach')}
        title={t('devpage.attach')}
        onclick={() => act(api.devpageDetach(page.agentId, false))}><Icon name="collapse" size={16} /></button
      >
      <button class="ib" aria-label={t('devpage.system')} title={t('devpage.system')} onclick={() => act(openUrl(page.url))}
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
  .dt kbd {
    padding: 1px 4px;
    border-radius: 4px;
    color: var(--muted);
    font-size: 10.5px;
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
</style>
