<script lang="ts">
  import { launchStatus, restartLaunch, startAll, startLaunch, stopAll, stopLaunch } from '../lib/launch-actions';
  import { isolaCommand, recipeCommands } from '../lib/recipe';
  import { app } from '../lib/state.svelte';
  import type { Project, RunCommand } from '../lib/types';

  let { project }: { project: Project } = $props();

  const cmds = $derived(project.runCommands);
  const running = $derived(cmds.filter((c) => app.launches[c.id]?.status === 'running').length);
  const selected = $derived(app.runCommand?.id ?? null);
  // Each agent's test launches, under its name, once one of them ran.
  const groups = $derived(
    Object.values(app.agents)
      .filter((a) => a.projectId === project.id && (a.recipe || a.isola))
      .map((a) => {
        const shell = app.shells[0]?.id ?? '';
        const c = recipeCommands(a, shell);
        const isola = a.isola ? [isolaCommand(a, shell)] : [];
        return { agent: a, cmds: [...isola, ...c.prepare, ...c.processes].filter((x) => app.launches[x.id]) };
      })
      .filter((g) => g.cmds.length > 0),
  );

  function act(e: Event, f: () => unknown) {
    // The row itself opens the log: its buttons only act.
    e.stopPropagation();
    f();
  }

  function configure() {
    app.modal = { kind: 'settings', tab: 'projects', projectId: project.id, section: 'launch' };
  }
  /** The same settings, where Claude reads the project at once to fill in the commands (to read before saving). */
  function propose() {
    app.modal = { kind: 'settings', tab: 'projects', projectId: project.id, section: 'launch', suggest: true };
  }
  const isRunning = (c: RunCommand) => app.launches[c.id]?.status === 'running';
</script>

{#snippet runRow(c: RunCommand)}
  {@const st = launchStatus(app.launches[c.id])}
  <div
    class="run"
    class:sel={selected === c.id}
    role="button"
    tabindex="0"
    title={c.command}
    onclick={() => app.selectLaunch(c.id)}
    onkeydown={(e) => e.key === 'Enter' && e.target === e.currentTarget && app.selectLaunch(c.id)}
  >
    <span class="dot" class:live={isRunning(c)} style:background={st.color}></span>
    <span class="name">{c.name}</span>
    <span class="status" style:color={st.color}>{st.label}</span>
    {#if isRunning(c)}
      <button class="ctl" title="Relancer" aria-label="Relancer" onclick={(e) => act(e, () => restartLaunch(project, c))}>⟳</button>
      <button class="ctl" title="Stopper" aria-label="Stopper" onclick={(e) => act(e, () => stopLaunch(c.id))}>■</button>
    {:else}
      <button class="ctl go" title="Lancer" aria-label="Lancer" onclick={(e) => act(e, () => startLaunch(project, c))}>▶</button>
    {/if}
  </div>
{/snippet}

<div class="runs">
  <div class="head">
    <span class="section-label">Lancement</span>
    {#if cmds.length}<span class="count">{running}/{cmds.length}</span>{/if}
    <div style="flex:1"></div>
    {#if cmds.length}
      {#if running}
        <button class="all" onclick={() => stopAll(project)}>Tout arrêter</button>
      {:else}
        <button class="all" onclick={() => startAll(project)}>Tout lancer</button>
      {/if}
    {/if}
    <button class="gear" title="Commandes de lancement…" aria-label="Commandes de lancement…" onclick={configure}>
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"
        ><circle cx="12" cy="12" r="3" /><path
          d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"
        /></svg
      >
    </button>
  </div>
  <div class="list">
    {#each cmds as c (c.id)}
      {@render runRow(c)}
    {:else}
      <div class="none">
        <span>Aucune commande.</span>
        <button class="link" onclick={configure}>Configurer</button>
        <button class="link" onclick={propose}>✦ Proposer des commandes</button>
      </div>
    {/each}
    {#each groups as g (g.agent.id)}
      <div class="group mono">{g.agent.name}</div>
      {#each g.cmds as c (c.id)}
        {@render runRow(c)}
      {/each}
    {/each}
  </div>
</div>

<style>
  .runs {
    border-top: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    padding-bottom: 8px;
    max-height: 30%;
    min-height: 0;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 14px 8px 18px;
  }
  .count {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--dim);
  }
  .all {
    height: 24px;
    padding: 0 8px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: var(--elev);
    font-size: 11.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .gear {
    width: 26px;
    height: 26px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: var(--elev);
    color: var(--muted);
    cursor: pointer;
  }
  .all:hover,
  .gear:hover {
    border-color: var(--accent);
    color: var(--text);
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 0 10px;
    overflow: auto;
  }
  .run {
    display: flex;
    align-items: center;
    gap: 9px;
    height: 32px;
    padding: 0 4px 0 12px;
    border-radius: var(--r-sm);
    border: 1px solid transparent;
    cursor: pointer;
    flex: none;
  }
  .run:hover:not(.sel) {
    background: color-mix(in oklch, var(--elev) 55%, transparent);
  }
  .run.sel {
    background: var(--elev);
    border-color: var(--line2);
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    flex: none;
  }
  .dot.live {
    box-shadow: 0 0 0 3px color-mix(in oklch, var(--ok) 25%, transparent);
  }
  .name {
    flex: 1;
    min-width: 0;
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status {
    font-family: var(--mono);
    font-size: 10.5px;
    white-space: nowrap;
  }
  .ctl {
    width: 22px;
    height: 22px;
    border: none;
    border-radius: 3px;
    background: transparent;
    color: var(--muted);
    font-size: 11px;
    cursor: pointer;
    flex: none;
  }
  .ctl.go {
    color: var(--ok);
  }
  .ctl:hover {
    background: var(--elev2);
    color: var(--text);
  }
  .group {
    padding: 8px 8px 2px;
    font-size: 10.5px;
    color: var(--dim);
  }
  .none {
    display: flex;
    flex-wrap: wrap;
    gap: 2px 10px;
    padding: 2px 8px 4px;
    font-size: 12px;
    color: var(--dim);
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--accent);
    font-size: 12px;
    cursor: pointer;
  }
</style>
