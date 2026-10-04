<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { app } from '../../lib/state.svelte';
  import { flows, stopTests } from '../../lib/test-launch.svelte';
  import Modal from './Modal.svelte';

  let { agentId }: { agentId: string } = $props();

  const agent = $derived(app.agents[agentId]);
  const ticket = $derived(app.ticketOf(agentId));
  const flow = $derived(flows.all[agentId]);
  /** A step has a log once it ran: one the backend refused to start has none. */
  const hasLog = (id: string) => !!app.launches[id];
  // "Voir les logs": the processes' first, else the preparation's.
  const logs = $derived.by(() => {
    const withLog = (flow?.lines ?? []).filter((l) => hasLog(l.launchId));
    return (withLog.find((l) => !l.id.includes(':prep:')) ?? withLog[0])?.launchId ?? null;
  });
  const close = () => (app.modal = null);
  const MARK = { running: '…', waiting: '…', ready: '✓', failed: '✕', stopped: '■', skipped: '–' } as const;

  /** The log of a step, in the main area. */
  function viewLog(id: string | null) {
    if (!id) return;
    if (agent && app.ui.activeProject !== agent.projectId) app.selectProject(agent.projectId);
    app.selectLaunch(id);
    close();
  }
</script>

<Modal title={`Tester ${ticket?.key ?? agent?.name ?? ''}`} width={560} onclose={close}>
  {#if flow}
    <ul class="lines" aria-label="Étapes du lancement">
      {#each flow.lines as l (l.id)}
        <li class={l.state}>
          <span class="mark">{MARK[l.state]}</span>
          <span class="label">{l.label}</span>
          <span class="detail mono">{l.detail}</span>
          {#if l.state === 'failed' && hasLog(l.launchId)}
            <button class="link" onclick={() => viewLog(l.launchId)}>Voir le log</button>
          {/if}
        </li>
      {/each}
    </ul>
    {#if flow.error}<p class="error">{flow.error}</p>{/if}
    {#if flow.opened}<p class="opened">Ouvert dans le navigateur : <span class="mono">{flow.opened}</span></p>{/if}
  {:else if agent?.recipe}
    <!-- A recipe and no test: the agent sent another recipe, which dropped the test shown. -->
    <p class="opened">La recette a changé : relance ▶ Tester.</p>
  {/if}
  {#snippet footer()}
    {#if flow?.opened}
      {@const opened = flow.opened}
      <button class="btn" onclick={() => app.run(openUrl(opened))}>Rouvrir</button>
    {/if}
    <button class="btn" disabled={!logs} onclick={() => viewLog(logs)}>Voir les logs</button>
    <button class="btn danger" onclick={() => stopTests(agentId)}>Tout arrêter</button>
    <button class="btn primary" onclick={close}>Fermer</button>
  {/snippet}
</Modal>

<style>
  .lines {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  li {
    display: flex;
    align-items: baseline;
    gap: 10px;
    font-size: 13px;
  }
  .mark {
    width: 14px;
    flex: none;
    text-align: center;
    color: var(--dim);
  }
  li.ready .mark {
    color: var(--ok);
  }
  li.failed .mark,
  li.failed .detail {
    color: var(--del);
  }
  .label {
    flex: none;
    max-width: 45%;
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .detail {
    flex: 1;
    min-width: 0;
    font-size: 11.5px;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .link {
    flex: none;
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    font-size: 12px;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }
  .error {
    margin: 0;
    color: var(--del);
    font-size: 12.5px;
  }
  .opened {
    margin: 0;
    font-size: 12.5px;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
</style>
