<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { openAddress, recipeApproved, revealHidden } from '../../lib/recipe';
  import { app } from '../../lib/state.svelte';
  import { approveAndTest, flows, stopTests } from '../../lib/test-launch.svelte';
  import Modal from './Modal.svelte';

  let { agentId }: { agentId: string } = $props();

  const agent = $derived(app.agents[agentId]);
  const project = $derived(app.projects.find((p) => p.id === agent?.projectId));
  /** « Lancer » was pressed: the approval is on its way, and the test starts as soon as it is given. */
  let approving = $state(false);
  // A recipe is the agent's own text, run in the user's shell: it is read before it runs. isola's test has none (its
  // command is fixed and the services are the project's).
  const toRead = $derived(!!agent?.recipe && !agent.isola && (approving || !recipeApproved(agent)));
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

  /** « Lancer »: the recipe shown is approved, then it runs. */
  async function launch() {
    if (!agent || !project || approving) return;
    approving = true;
    try {
      await approveAndTest(agent, project);
    } finally {
      approving = false;
    }
  }

  /** What the recipe says of a folder: relative to the worktree, empty for its root. */
  const where = (dir: string) => (dir.trim() && dir.trim() !== '.' ? revealHidden(dir) : 'la racine du worktree');

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
  {:else if toRead && agent?.recipe}
    {@const recipe = agent.recipe}
    {@const address = openAddress(agent)}
    <p class="notice">
      Ces commandes ont été écrites par {agent.name}. Elles tournent dans ton shell, hors du mode de permission de Claude Code.
    </p>
    {#if recipe.prepare.length}
      <section>
        <h3 class="section-label">Préparation</h3>
        <ol class="steps">
          {#each recipe.prepare as step, i (i)}
            <li>
              <pre class="cmd mono">{revealHidden(step.command)}</pre>
              <dl class="facts">
                <dt>Dossier</dt>
                <dd class="mono">{where(step.dir)}</dd>
              </dl>
            </li>
          {/each}
        </ol>
      </section>
    {/if}
    {#if recipe.processes.length}
      <section>
        <h3 class="section-label">Lancement</h3>
        <ul class="steps">
          {#each recipe.processes as p, i (i)}
            <li>
              <span class="name">{revealHidden(p.name.trim()) || `processus ${i + 1}`}</span>
              <pre class="cmd mono">{revealHidden(p.command)}</pre>
              <dl class="facts">
                <dt>Dossier</dt>
                <dd class="mono">{where(p.dir)}</dd>
                {#each Object.entries(p.env) as [key, value] (key)}
                  <dt>Variable</dt>
                  <dd class="mono">{revealHidden(`${key}=${value}`)}</dd>
                {/each}
                {#if p.url.trim()}
                  <dt>Adresse</dt>
                  <dd class="mono">{revealHidden(p.url)}</dd>
                {/if}
              </dl>
            </li>
          {/each}
        </ul>
      </section>
    {/if}
    {#if address}
      <section>
        <h3 class="section-label">Ouverture</h3>
        <p class="cmd mono">{revealHidden(address)}</p>
      </section>
    {/if}
  {:else if agent?.recipe}
    <!-- A recipe and no test: the agent sent another recipe, which dropped the test shown. -->
    <p class="opened">La recette a changé : relance ▶ Tester.</p>
  {/if}
  {#snippet footer()}
    {#if !flow && toRead}
      <button class="btn" disabled={approving} onclick={close}>Annuler</button>
      <button class="btn primary" disabled={approving || !project} onclick={launch}>Lancer</button>
    {:else}
      {#if flow?.opened}
        {@const opened = flow.opened}
        <button class="btn" onclick={() => app.run(openUrl(opened))}>Rouvrir</button>
      {/if}
      <button class="btn" disabled={!logs} onclick={() => viewLog(logs)}>Voir les logs</button>
      <button class="btn danger" onclick={() => stopTests(agentId)}>Tout arrêter</button>
      <button class="btn primary" onclick={close}>Fermer</button>
    {/if}
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
  .notice {
    margin: 0;
    padding: 8px 12px;
    border-left: 2px solid var(--wait);
    background: var(--wait-soft);
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text);
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  h3 {
    margin: 0;
  }
  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .steps li {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 4px;
  }
  .name {
    font-size: 13px;
    font-weight: 600;
  }
  /* The whole command, as written: never cut, never reflowed, however long or many its lines. */
  .cmd {
    margin: 0;
    padding: 6px 10px;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--elev);
    font-size: 11.5px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .facts {
    margin: 0;
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 2px 10px;
    font-size: 11.5px;
  }
  .facts dt {
    color: var(--dim);
  }
  .facts dd {
    margin: 0;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .opened {
    margin: 0;
    font-size: 12.5px;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
</style>
