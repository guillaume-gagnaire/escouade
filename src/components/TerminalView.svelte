<script lang="ts">
  import { t } from '../lib/i18n';
  import { keyLabel, primaryKey } from '../lib/platform';
  import { app } from '../lib/state.svelte';
  import { closeTerminal, newTerminal, SHELL_GLYPH } from '../lib/term-actions';
  import { getXTerm, mountTerminal } from '../lib/terminals';
  import type { Project, TermInfo } from '../lib/types';
  import { tildify } from '../lib/format';

  let { term, project }: { term: TermInfo; project: Project } = $props();
  let box = $state<HTMLDivElement>();
  let searchOpen = $state(false);
  let query = $state('');

  const label = $derived(app.shells.find((s) => s.id === term.shell)?.label ?? term.shell);
  const exited = $derived(app.exitedTerms[term.id]);

  $effect(() => {
    const id = term.id;
    const el = box;
    if (!el) return;
    mountTerminal(id, el);
    const ro = new ResizeObserver(() => {
      try {
        getXTerm(id)?.fit.fit();
      } catch {
        /* hidden */
      }
    });
    ro.observe(el);
    return () => {
      ro.disconnect();
      mountTerminal(id, null);
    };
  });

  function find(next = true) {
    const x = getXTerm(term.id);
    if (!x || !query) return;
    if (next) x.search.findNext(query);
    else x.search.findPrevious(query);
  }

  async function restart() {
    const shell = term.shell;
    closeTerminal(term.id);
    await newTerminal(project.id, shell);
  }
</script>

<main class="tv">
  <header class="head">
    <div class="who">
      <div class="l1">
        <span class="name">{term.name}</span>
        <span class="badge mono" style:color={SHELL_GLYPH[term.shell]?.c}>{label}</span>
      </div>
      <span class="sub mono">{tildify(project.path)}</span>
    </div>
    <div style="flex:1"></div>
    {#if searchOpen}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="field search"
        placeholder={t('common.searchEllipsis')}
        bind:value={query}
        autofocus
        onkeydown={(e) => {
          if (e.key === 'Enter') find(!e.shiftKey);
          if (e.key === 'Escape') {
            searchOpen = false;
            getXTerm(term.id)?.term.focus();
          }
        }}
      />
    {/if}
    <button class="btn ghost" title={t('runs.log.searchKey', { key: keyLabel('Ctrl+Shift+F') })} onclick={() => (searchOpen = !searchOpen)}
      >⌕</button
    >
    <button class="btn ghost" onclick={() => getXTerm(term.id)?.term.clear()}>{t('common.clear')}</button>
    <button class="btn danger" onclick={() => closeTerminal(term.id)}>{t('runs.terminal.close')}</button>
  </header>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="box"
    bind:this={box}
    onkeydown={(e) => {
      if (primaryKey(e) && e.shiftKey && e.key.toLowerCase() === 'f') {
        e.preventDefault();
        searchOpen = true;
      }
    }}
  ></div>
  {#if exited !== undefined}
    <div class="exited">
      {exited !== null ? t('runs.terminal.exitedCode', { code: exited }) : t('runs.terminal.exited')}
      <button class="btn" onclick={restart}>{t('runs.action.restart')}</button>
      <button class="btn ghost" onclick={() => closeTerminal(term.id)}>{t('common.close')}</button>
    </div>
  {/if}
</main>

<style>
  .tv {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    position: relative;
  }
  .head {
    height: 60px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 24px;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
  }
  .who {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .l1 {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .name {
    font-size: 15px;
    font-weight: 700;
  }
  .badge {
    font-size: 11px;
    padding: 2px 7px;
    border-radius: 3px;
    background: var(--elev2);
  }
  .sub {
    font-size: 11px;
    color: var(--dim);
  }
  .search {
    height: 28px;
    width: 220px;
  }
  .btn {
    height: 28px;
    font-size: 12px;
  }
  .box {
    flex: 1;
    min-height: 0;
    padding: 12px 6px 6px 16px;
    background: var(--term);
    overflow: hidden;
  }
  .box :global(.xterm) {
    height: 100%;
  }
  .box :global(.xterm-viewport) {
    background: transparent !important;
  }
  .exited {
    position: absolute;
    left: 50%;
    bottom: 20px;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-radius: var(--r);
    border: 1px solid var(--line2);
    background: var(--elev);
    font-size: 12.5px;
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.4);
  }
</style>
