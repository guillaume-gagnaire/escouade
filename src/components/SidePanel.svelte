<script lang="ts">
  import { t } from '../lib/i18n';
  import { keyLabel } from '../lib/platform';
  import { app } from '../lib/state.svelte';
  import type { Agent, Project } from '../lib/types';
  import FilesPanel from './FilesPanel.svelte';
  import GitGraph from './GitGraph.svelte';
  import WorkflowPanel from './WorkflowPanel.svelte';
  import PrPanel from './PrPanel.svelte';
  import { prWord } from '../lib/prs';
  import { ciFailed } from '../lib/prview';
  import { workflows } from '../lib/workflows.svelte';
  import { followers, ticketTracks } from '../lib/workflow-view';
  import Icon from './Icon.svelte';
  import { tip } from '../lib/tip';

  // The agent's side panel: its uncommitted files, the repository history, the project's workflow (ESC-194), and the
  // pull request of its branch with its CI, reviews and deployments (ESC-305); without an agent, the project's open ones.
  // `docked`: beside the conversation in the split layout (always shown), else an optional panel.
  let { project, agent, docked = false }: { project: Project; agent: Agent | null; docked?: boolean } = $props();

  let count = $state(0);
  const tab = $derived(app.panelTab);
  /** The agents that follow a workflow of the project (ESC-194), « Tickets » too (ESC-197): the count of its tab. */
  const following = $derived(
    followers([
      ...workflows.list(project.id).filter((tr) => app.agents[tr.agentId]),
      ...ticketTracks(project.id, Object.values(app.tickets), app.agents, 0),
    ]),
  );
  $effect(() => void workflows.loadTracks(project.id));
  /** The host's word (« MR » on GitLab), the agent's once its project's host is known. */
  const word = $derived(agent ? prWord(agent) : 'PR');
  /** The CI of the agent's pull request failed: the tab's dot. */
  const failed = $derived(ciFailed(agent));
</script>

<aside class="git-side" class:docked aria-label={t('common.files')}>
  <div class="utabs" role="tablist">
    <button role="tab" id="side-tab-files" aria-selected={tab === 'files'} onclick={() => (app.panelTab = 'files')}>
      {t('nav.sidePanel.uncommitted')} <span class="n">{count}</span>
    </button>
    <button role="tab" id="side-tab-history" aria-selected={tab === 'history'} onclick={() => (app.panelTab = 'history')}>
      {t('nav.sidePanel.history')}
    </button>
    <button role="tab" id="side-tab-workflow" aria-selected={tab === 'workflow'} onclick={() => (app.panelTab = 'workflow')}>
      {t('workflow.tab')}{#if following}{' '}<span class="n">{following}</span>{/if}
    </button>
    <button role="tab" id="side-tab-hosts" aria-selected={tab === 'hosts'} onclick={() => (app.panelTab = 'hosts')}>
      {t('nav.sidePanel.hosts', { word })}{#if failed && agent}<span
          class="dot"
          role="img"
          aria-label={t('nav.sidePanel.hostsFailed', { word, number: agent.prs[0].number })}
        ></span>{/if}
    </button>
    <!-- Closed as it was opened (ESC-204): « Fichiers » of the header (Ctrl+Shift+B), or the layout side by side (Ctrl+Shift+L). -->
    <button
      class="icon-btn close"
      aria-label={t('nav.sidePanel.close')}
      use:tip={{
        title: t('nav.sidePanel.close'),
        kbd: keyLabel(docked ? 'Ctrl+Shift+L' : 'Ctrl+Shift+B'),
        rows: [t(docked ? 'nav.sidePanel.reopenSplit' : 'nav.sidePanel.reopen')],
        aria: false,
      }}
      onclick={() => {
        // Side by side, the classic layout again, without the panel « Fichiers » may have left open there.
        if (docked) app.toggleLayout();
        app.filesOpen = false;
      }}><Icon name="close" size={14} /></button
    >
  </div>
  <div class="pane" role="tabpanel" aria-labelledby="side-tab-{tab}">
    {#if tab === 'files'}
      <FilesPanel {project} {agent} {docked} bind:count />
    {:else if tab === 'history'}
      <GitGraph {project} {agent} />
    {:else if tab === 'workflow'}
      <WorkflowPanel {project} {agent} />
    {:else}
      <PrPanel {project} {agent} />
    {/if}
  </div>
</aside>

<style>
  .git-side {
    width: 340px;
    flex: none;
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-left: 1px solid var(--line);
    container: git-side / inline-size;
  }
  /* Beside the conversation, as the preview lays it out: 44 % of the work, never under 340 pixels. */
  .git-side.docked {
    width: 44%;
    min-width: 340px;
  }
  /* Tabs underlined in the project's colour. */
  .utabs {
    flex: none;
    display: flex;
    align-items: flex-end;
    gap: 18px;
    height: 60px;
    padding: 0 18px;
    border-bottom: 1px solid var(--line);
  }
  .utabs [role='tab'] {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 40px;
    margin-bottom: -1px;
    padding: 0;
    border: none;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-weight: 550;
    white-space: nowrap;
    cursor: pointer;
  }
  .utabs [role='tab']:hover {
    color: var(--text);
  }
  .utabs [role='tab'][aria-selected='true'] {
    border-bottom-color: var(--p-ink);
    color: var(--text);
  }
  .utabs .n {
    color: var(--dim);
    font-weight: 500;
  }
  /* The CI of the agent's pull request failed. */
  .utabs .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--del);
  }
  /* Narrower still (340 px beside the classic layout, the window of 1028 px), the tabs draw closer, the Workflow tab's
     count goes (choice 132) and « PR et CI » leaves the strip for the × (choice 189): its tab comes back with the width. */
  @container git-side (max-width: 420px) {
    .utabs {
      gap: 12px;
      padding: 0 14px;
    }
    #side-tab-workflow .n,
    #side-tab-hosts {
      display: none;
    }
  }
  .close {
    align-self: center;
    margin-left: auto;
  }
  .pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
</style>
