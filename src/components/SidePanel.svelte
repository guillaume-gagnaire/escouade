<script lang="ts">
  import { t } from '../lib/i18n';
  import { app } from '../lib/state.svelte';
  import type { Agent, Project } from '../lib/types';
  import FilesPanel from './FilesPanel.svelte';
  import GitGraph from './GitGraph.svelte';

  // The agent's side panel: its uncommitted files, or the repository history.
  // `docked`: right half of the split layout (always shown), else an optional 330 px panel.
  let { project, agent, docked = false }: { project: Project; agent: Agent | null; docked?: boolean } = $props();

  let count = $state(0);
  const tab = $derived(app.panelTab);
</script>

<aside class="panel" class:docked>
  <div class="head">
    <div class="tabs" role="tablist">
      <button role="tab" aria-selected={tab === 'files'} class:on={tab === 'files'} onclick={() => (app.panelTab = 'files')}>
        {t('nav.sidePanel.uncommitted')} <span class="count">{count}</span>
      </button>
      <button role="tab" aria-selected={tab === 'history'} class:on={tab === 'history'} onclick={() => (app.panelTab = 'history')}>
        {t('nav.sidePanel.history')}
      </button>
    </div>
    <div style="flex:1"></div>
    {#if !docked}<button class="icon-btn" title={t('common.close')} onclick={() => (app.filesOpen = false)}>×</button>{/if}
  </div>
  {#if tab === 'files'}
    <FilesPanel {project} {agent} {docked} bind:count />
  {:else}
    <GitGraph {project} {agent} />
  {/if}
</aside>

<style>
  .panel {
    width: 330px;
    flex: none;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border-left: 1px solid var(--line);
    min-height: 0;
  }
  .panel.docked {
    width: auto;
    flex: 1 1 0;
    min-width: 0;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 14px 10px 12px;
  }
  .tabs {
    display: flex;
    gap: 2px;
  }
  .tabs button {
    height: 28px;
    padding: 0 10px;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    cursor: pointer;
    white-space: nowrap;
  }
  .tabs button:hover {
    color: var(--text);
  }
  .tabs button.on {
    background: var(--elev);
    color: var(--text);
  }
  .count {
    margin-left: 4px;
    font-family: var(--mono);
    letter-spacing: 0;
    color: var(--dim);
  }
</style>
