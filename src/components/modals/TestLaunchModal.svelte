<script lang="ts">
  import { onDestroy } from 'svelte';
  import { engineName } from '../../lib/engines';
  import { t } from '../../lib/i18n';
  import Rich from '../../lib/i18n/Rich.svelte';
  import { isolaApproved, openAddress, recipeApproved, revealHidden } from '../../lib/recipe';
  import { app } from '../../lib/state.svelte';
  import { approveAndTest, flows, openPage, stopTests } from '../../lib/test-launch.svelte';
  import Icon from '../Icon.svelte';
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
  /** The mark of a step: a spinner while it runs or is waited for, else an icon of how it ended. */
  const MARK = { ready: 'check', failed: 'close', stopped: 'stop', skipped: 'circle' } as const;
  /** What the window is at, beside its title. */
  const phase = $derived(flow ? flow.phase : toRead || reading ? 'read' : null);

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

<Modal title={t('runs.testLaunch.title', { name: ticket?.key ?? agent?.name ?? '' })} width={620} onclose={close}>
  {#snippet badge()}{#if phase}<span class="phase {phase}">{t(`runs.testLaunch.phase.${phase}`)}</span>{/if}{/snippet}
  {#if flow}
    <ul class="lines" aria-label={t('runs.testLaunch.steps')}>
      {#each flow.lines as l (l.id)}
        <li class={l.state}>
          <span class="mark"
            >{#if l.state === 'running' || l.state === 'waiting'}<i class="spin"></i>{:else}<Icon
                name={MARK[l.state]}
                size={14}
              />{/if}</span
          >
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
      <p class="opened done">
        <Icon name="ext" size={14} />
        <Rich
          k={flow.where === 'agent'
            ? 'runs.testLaunch.openedHere'
            : flow.where === 'window'
              ? 'runs.testLaunch.openedWindow'
              : 'runs.testLaunch.opened'}>{#snippet address()}<span class="mono">{flow.opened}</span>{/snippet}</Rich
        >
      </p>
    {/if}
  {:else if reading}
    <p class="opened">{t('runs.testLaunch.reading')}</p>
  {:else if toRead && agent}
    {#if changed}
      <p class="notice changed" role="status"><Icon name="info" size={16} /><span>{t('runs.testLaunch.changed')}</span></p>
    {/if}
    {#if agent.isola && isolaConfig !== undefined}
      <p class="notice">
        <Icon name="shield" size={16} />
        <span
          ><Rich k="runs.testLaunch.isolaNotice" harness={engineName(agent.engine)}>{#snippet name()}<b>{agent.name}</b>{/snippet}</Rich
          ></span
        >
      </p>
      <section>
        <h3>{t('runs.testLaunch.isolaConfig')}</h3>
        <pre class="cmd mono">{revealHidden(isolaConfig)}</pre>
      </section>
    {:else if agent.recipe}
      {@const recipe = agent.recipe}
      <p class="notice">
        <Icon name="shield" size={16} />
        <span
          ><Rich k="runs.testLaunch.recipeNotice" harness={engineName(agent.engine)}>{#snippet name()}<b>{agent.name}</b>{/snippet}</Rich
          ></span
        >
      </p>
      {#if recipe.prepare.length}
        <section>
          <h3>{t('runs.testLaunch.prepare')}</h3>
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
          <h3>{t('runs.testLaunch.launch')}</h3>
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
        <h3>{t('runs.testLaunch.opening')}</h3>
        <pre class="cmd mono">{revealHidden(address)}</pre>
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
        <button class="btn primary" disabled={approving || settling || !project} onclick={launch}
          ><Icon name="play" size={14} />{t('runs.action.run')}</button
        >
      {/if}
    {:else}
      {#if flow?.opened}
        {@const opened = flow.opened}
        <button class="btn reopen" onclick={() => agent && flow && openPage(agent, flow, opened)}>{t('runs.testLaunch.reopen')}</button>
      {/if}
      <button class="btn" disabled={!logs} onclick={() => viewLog(logs)}>{t('runs.testLaunch.viewLogs')}</button>
      <button class="btn danger" onclick={() => stopTests(agentId)}>{t('common.stopAll')}</button>
      <button class="btn primary" onclick={close}>{t('common.close')}</button>
    {/if}
  {/snippet}
</Modal>

<style>
  /* The window of « Tester » in the 2.0 design: who wrote the commands first, then each step in a well, then the launch,
     step by step with its time. */
  .phase {
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
  .phase.failed {
    background: oklch(from var(--del) l c h / 0.1);
    color: var(--del);
  }
  .lines {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }
  .lines li {
    display: grid;
    grid-template-columns: 18px minmax(0, 210px) minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    min-height: 34px;
    font-size: 13px;
  }
  .mark {
    display: grid;
    place-items: center;
    color: var(--dim);
  }
  li.ready .mark {
    color: var(--add);
  }
  li.failed .mark,
  li.failed .detail {
    color: var(--del);
  }
  .spin {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 1.75px solid rgba(var(--fg-rgb), 0.15);
    border-top-color: var(--soft);
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }
  .label {
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .detail {
    min-width: 0;
    font-family: var(--mono);
    font-size: 12px;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  li.ready .detail {
    color: var(--soft);
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--p-ink);
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
    display: grid;
    grid-template-columns: 16px 1fr;
    gap: 10px;
    margin: 0;
    padding: 11px 13px;
    border-radius: 10px;
    background: rgba(var(--fg-rgb), 0.035);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .notice > :global(.icon) {
    margin-top: 1px;
    color: var(--wait);
  }
  .notice :global(b) {
    color: var(--text);
    font-weight: 600;
  }
  .notice.changed {
    background: oklch(from var(--del) l c h / 0.08);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.35);
    color: var(--text);
    font-weight: 600;
  }
  .notice.changed > :global(.icon) {
    color: var(--del);
  }
  section {
    display: grid;
    gap: 8px;
  }
  h3 {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
  }
  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
  }
  .steps li {
    display: grid;
    gap: 7px;
    padding: 12px 0;
  }
  .steps li + li {
    border-top: 1px solid var(--line);
  }
  /* What is read here is what the shell reads, in that order: right-to-left letters must not reorder a « ; » or a « | ». */
  .name,
  .cmd,
  .facts dd {
    unicode-bidi: bidi-override;
    direction: ltr;
  }
  .name {
    font-weight: 600;
  }
  /* The whole command, as written: never cut, never reflowed, however long or many its lines. */
  .cmd {
    margin: 0;
    padding: 8px 10px;
    border-radius: 7px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--text);
    font: 12.5px var(--mono);
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .facts {
    margin: 0;
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 3px 14px;
    font-size: 12px;
  }
  .facts dt {
    color: var(--muted);
  }
  .facts dd {
    margin: 0;
    color: var(--soft);
    font-family: var(--mono);
    font-size: 11.5px;
    overflow-wrap: anywhere;
  }
  .opened {
    margin: 0;
    font-size: 12.5px;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .opened.done {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
    padding-top: 14px;
    border-top: 1px solid var(--line);
  }
  .opened.done > :global(.icon) {
    flex: none;
    color: var(--add);
  }
  .opened :global(.mono) {
    color: var(--soft);
  }
  /* « Rouvrir » on the left of the footer, apart from what closes or stops. */
  .reopen {
    margin-right: auto;
  }
</style>
