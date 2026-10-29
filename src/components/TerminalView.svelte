<script lang="ts">
  import { t } from '../lib/i18n';
  import { keyLabel, primaryKey } from '../lib/platform';
  import { app } from '../lib/state.svelte';
  import { closeTerminal, newTerminal } from '../lib/term-actions';
  import { getXTerm, mountTerminal } from '../lib/terminals';
  import type { Project, TermInfo } from '../lib/types';
  import { tildify } from '../lib/format';
  import { tip } from '../lib/tip';
  import Icon from './Icon.svelte';

  let { term, project }: { term: TermInfo; project: Project } = $props();
  let box = $state<HTMLDivElement>();
  let searchOpen = $state(false);
  let query = $state('');

  const label = $derived(app.shells.find((s) => s.id === term.shell)?.label ?? term.shell);
  const exited = $derived(app.exitedTerms[term.id]);
  /** The folder it opened in: the project's. */
  const folder = $derived(tildify(project.path));

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
    <div class="h-title">
      <div class="row">
        <h1 class="name">{term.name}</h1>
        <span class="chip">{label}</span>
      </div>
      <div class="crumbs"><span class="mono where" use:tip={{ text: folder, cut: true }}>{folder}</span></div>
    </div>
    <div class="actions">
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
      <button
        class="btn icon-only"
        aria-label={t('runs.log.searchKey', { key: keyLabel('Ctrl+Shift+F') })}
        use:tip={t('runs.log.searchKey', { key: keyLabel('Ctrl+Shift+F') })}
        onclick={() => (searchOpen = !searchOpen)}><Icon name="search" size={14} /></button
      >
      <button class="btn" onclick={() => getXTerm(term.id)?.term.clear()}>{t('common.clear')}</button>
      <button
        class="btn icon-only"
        aria-label={t('runs.terminal.close')}
        use:tip={t('runs.terminal.close')}
        onclick={() => closeTerminal(term.id)}><Icon name="close" size={14} /></button
      >
    </div>
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
    <div class="exited" role="status">
      <Icon name="info" size={14} />
      {exited !== null ? t('runs.terminal.exitedCode', { code: exited }) : t('runs.terminal.exited')}
      <button class="btn" onclick={restart}><Icon name="restart" size={14} />{t('runs.action.restart')}</button>
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
  /* The header of the 2.0 design: the terminal's name and its shell, the folder it opened in. */
  .head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 16px;
    min-height: 60px;
    padding: 0 14px 0 22px;
    border-bottom: 1px solid var(--line);
  }
  .h-title {
    display: grid;
    gap: 2px;
    min-width: 0;
    margin-right: auto;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .name {
    min-width: 0;
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 15px;
    font-weight: 620;
    letter-spacing: -0.015em;
  }
  .chip {
    flex: none;
    display: inline-flex;
    align-items: center;
    height: 20px;
    padding: 0 7px;
    border-radius: 6px;
    background: rgba(var(--fg-rgb), 0.06);
    color: var(--soft);
    font-size: 11.5px;
    font-weight: 550;
    white-space: nowrap;
  }
  .crumbs {
    display: flex;
    align-items: center;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    color: var(--muted);
    font-size: 12px;
  }
  .where {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* Code in a line of text, a size below it, as in the preview. */
  .crumbs .mono {
    font-size: 0.92em;
    letter-spacing: -0.01em;
  }
  .actions {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .search {
    height: 30px;
    width: 220px;
  }
  .box {
    flex: 1;
    min-height: 0;
    padding: 16px 8px 8px 22px;
    background: var(--well);
    overflow: hidden;
  }
  .box :global(.xterm) {
    height: 100%;
  }
  .box :global(.xterm-viewport) {
    background: transparent !important;
  }
  /* Its process ended: said over the foot of the terminal, as a notification is. */
  .exited {
    position: absolute;
    left: 50%;
    bottom: 20px;
    z-index: 5;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 8px 7px 12px;
    border-radius: 10px;
    background: var(--raise-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 12px 32px -8px rgba(0, 0, 0, calc(0.7 * var(--shade)));
    color: var(--text);
    font-size: 12.5px;
    white-space: nowrap;
  }
  .exited > :global(.icon) {
    color: var(--muted);
  }
</style>
