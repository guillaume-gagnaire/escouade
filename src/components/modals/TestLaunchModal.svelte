<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { onDestroy } from 'svelte';
  import { t } from '../../lib/i18n';
  import Rich from '../../lib/i18n/Rich.svelte';
  import { isolaApproved, openAddress, recipeApproved, revealHidden } from '../../lib/recipe';
  import { app } from '../../lib/state.svelte';
  import { approveAndTest, flows, stopTests } from '../../lib/test-launch.svelte';
  import Modal from './Modal.svelte';

  let { agentId }: { agentId: string } = $props();

  /** How long « Lancer » stays shut once what is shown has changed: what was in view for less than that is not read. */
  const SETTLE_MS = 1000;

  const agent = $derived(app.agents[agentId]);
  const project = $derived(app.projects.find((p) => p.id === agent?.projectId));
  /** « Lancer » was pressed: the approval is on its way, and the test starts as soon as it is given. */
  let approving = $state(false);
  /** The .isola.toml of the agent's worktree, read when "▶ Tester" was pressed (undefined until then). */
  const isolaConfig = $derived(flows.isolaConfig[agentId]);
  // What the agent wrote and would run in the user's shell is read before it runs: its recipe, and with isola (which
  // runs the commands of the worktree's .isola.toml, a file the agent can write too) that file.
  const toRead = $derived.by(() => {
    if (!agent) return false;
    if (agent.isola) return isolaConfig !== undefined && (approving || !isolaApproved(agent, isolaConfig));
    return !!agent.recipe && (approving || !recipeApproved(agent));
  });
  const ticket = $derived(app.ticketOf(agentId));
  const flow = $derived(flows.all[agentId]);
  /** The file is being read. */
  const reading = $derived(!!agent?.isola && !flow && isolaConfig === undefined);
  /** The address the browser will open. */
  const address = $derived(agent?.isola ? agent.recipe?.open.trim() || null : agent ? openAddress(agent) : null);

  // What is shown, to tell when it changes under the user's eyes: the agent sent another recipe, or edited its
  // .isola.toml, while it was read. Said above it, and « Lancer » waits a moment, so that a click aimed at what was
  // there is not an approval of what replaced it.
  const shown = $derived(
    !agent
      ? null
      : agent.isola
        ? isolaConfig === undefined
          ? null
          : JSON.stringify([isolaConfig, agent.recipe?.open ?? ''])
        : agent.recipe
          ? JSON.stringify(agent.recipe)
          : null,
  );
  let seen: string | null = null;
  let changed = $state(false);
  let settling = $state(false);
  let settle: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const now = shown;
    if (now === null) return;
    if (seen !== null && seen !== now) {
      changed = true;
      settling = true;
      clearTimeout(settle);
      settle = setTimeout(() => (settling = false), SETTLE_MS);
    }
    seen = now;
  });
  onDestroy(() => clearTimeout(settle));

  /** A step has a log once it ran: one the backend refused to start has none. */
  const hasLog = (id: string) => !!app.launches[id];
  // "Voir les logs": the processes' first, else the preparation's.
  const logs = $derived.by(() => {
    const withLog = (flow?.lines ?? []).filter((l) => hasLog(l.launchId));
    return (withLog.find((l) => !l.id.includes(':prep:')) ?? withLog[0])?.launchId ?? null;
  });
  const close = () => (app.modal = null);
  const MARK = { running: '…', waiting: '…', ready: '✓', failed: '✕', stopped: '■', skipped: '–' } as const;

  /** « Lancer »: what is shown is approved, then it runs. */
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
  const where = (dir: string) => (dir.trim() && dir.trim() !== '.' ? revealHidden(dir) : t('runs.testLaunch.root'));

  /** The log of a step, in the main area. */
  function viewLog(id: string | null) {
    if (!id) return;
    if (agent && app.ui.activeProject !== agent.projectId) app.selectProject(agent.projectId);
    app.selectLaunch(id);
    close();
  }
</script>

<Modal title={t('runs.testLaunch.title', { name: ticket?.key ?? agent?.name ?? '' })} width={560} onclose={close}>
  {#if flow}
    <ul class="lines" aria-label={t('runs.testLaunch.steps')}>
      {#each flow.lines as l (l.id)}
        <li class={l.state}>
          <span class="mark">{MARK[l.state]}</span>
          <span class="label">{l.label}</span>
          <span class="detail mono">{l.detail}</span>
          {#if l.state === 'failed' && hasLog(l.launchId)}
            <button class="link" onclick={() => viewLog(l.launchId)}>{t('runs.testLaunch.viewLog')}</button>
          {/if}
        </li>
      {/each}
    </ul>
    {#if flow.error}<p class="error">{flow.error}</p>{/if}
    {#if flow.opened}
      <p class="opened">
        <Rich k="runs.testLaunch.opened">{#snippet address()}<span class="mono">{flow.opened}</span>{/snippet}</Rich>
      </p>
    {/if}
  {:else if reading}
    <p class="opened">{t('runs.testLaunch.reading')}</p>
  {:else if toRead && agent}
    {#if changed}
      <p class="notice changed" role="status">{t('runs.testLaunch.changed')}</p>
    {/if}
    {#if agent.isola && isolaConfig !== undefined}
      <p class="notice">{t('runs.testLaunch.isolaNotice', { name: agent.name })}</p>
      <section>
        <h3 class="section-label">{t('runs.testLaunch.isolaConfig')}</h3>
        <pre class="cmd mono">{revealHidden(isolaConfig)}</pre>
      </section>
    {:else if agent.recipe}
      {@const recipe = agent.recipe}
      <p class="notice">{t('runs.testLaunch.recipeNotice', { name: agent.name })}</p>
      {#if recipe.prepare.length}
        <section>
          <h3 class="section-label">{t('runs.testLaunch.prepare')}</h3>
          <ol class="steps">
            {#each recipe.prepare as step, i (i)}
              <li>
                <pre class="cmd mono">{revealHidden(step.command)}</pre>
                <dl class="facts">
                  <dt>{t('common.folder')}</dt>
                  <dd class="mono">{where(step.dir)}</dd>
                </dl>
              </li>
            {/each}
          </ol>
        </section>
      {/if}
      {#if recipe.processes.length}
        <section>
          <h3 class="section-label">{t('runs.testLaunch.launch')}</h3>
          <ul class="steps">
            {#each recipe.processes as p, i (i)}
              <li>
                <span class="name">{revealHidden(p.name.trim()) || t('runs.recipe.process', { n: i + 1 })}</span>
                <pre class="cmd mono">{revealHidden(p.command)}</pre>
                <dl class="facts">
                  <dt>{t('common.folder')}</dt>
                  <dd class="mono">{where(p.dir)}</dd>
                  {#each Object.entries(p.env) as [key, value] (key)}
                    <dt>{t('runs.testLaunch.variable')}</dt>
                    <dd class="mono">{revealHidden(`${key}=${value}`)}</dd>
                  {/each}
                  {#if p.url.trim()}
                    <dt>{t('runs.testLaunch.address')}</dt>
                    <dd class="mono">{revealHidden(p.url)}</dd>
                  {/if}
                </dl>
              </li>
            {/each}
          </ul>
        </section>
      {/if}
    {/if}
    {#if address}
      <section>
        <h3 class="section-label">{t('runs.testLaunch.opening')}</h3>
        <p class="cmd mono">{revealHidden(address)}</p>
      </section>
    {/if}
  {:else if agent?.recipe && !agent.isola}
    <!-- A recipe and no test: the agent sent another recipe, which dropped the test shown. -->
    <p class="opened">{t('runs.testLaunch.recipeChanged')}</p>
  {/if}
  {#snippet footer()}
    {#if !flow && (toRead || reading)}
      <button class="btn" disabled={approving} onclick={close}>{t('common.cancel')}</button>
      {#if toRead}
        <button class="btn primary" disabled={approving || settling || !project} onclick={launch}>{t('runs.action.run')}</button>
      {/if}
    {:else}
      {#if flow?.opened}
        {@const opened = flow.opened}
        <button class="btn" onclick={() => app.run(openUrl(opened))}>{t('runs.testLaunch.reopen')}</button>
      {/if}
      <button class="btn" disabled={!logs} onclick={() => viewLog(logs)}>{t('runs.testLaunch.viewLogs')}</button>
      <button class="btn danger" onclick={() => stopTests(agentId)}>{t('common.stopAll')}</button>
      <button class="btn primary" onclick={close}>{t('common.close')}</button>
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
  .notice.changed {
    border-left-color: var(--del);
    font-weight: 600;
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
  /* What is read here is what the shell reads, in that order: right-to-left letters must not reorder a « ; » or a « | ». */
  .name,
  .cmd,
  .facts dd {
    unicode-bidi: bidi-override;
    direction: ltr;
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
