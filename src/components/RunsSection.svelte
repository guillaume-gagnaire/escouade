<script lang="ts">
  import { paneDrag } from '../lib/pane-drag.svelte';
  import { t } from '../lib/i18n';
  import { launchStatus, restartLaunch, runHere, runIn, startAll, stopAll, stopLaunch } from '../lib/launch-actions';
  import { menu } from '../lib/menu.svelte';
  import { inWorktree, isolaCommand, parseWorktreeRunId, recipeCommands } from '../lib/recipe';
  import { shortWindow } from '../lib/short-window.svelte';
  import { app } from '../lib/state.svelte';
  import type { Project, RunCommand } from '../lib/types';
  import { tip } from '../lib/tip';
  import FoldTitle from './FoldTitle.svelte';
  import Icon from './Icon.svelte';

  // « Lancement » in the sidebar (2.0 design): a row per launch command with its state, relaunch and stop on hover, ▶ for
  // one at rest (a menu « Dans le projet / Dans le worktree de … » once an agent has a worktree, choice 30), then each
  // agent's launches under its name. In a short window it folds to its header, a crash still said (choice 9), and its title
  // unfolds it.

  let { project }: { project: Project } = $props();

  let unfolded = $state(false);
  const folded = $derived(shortWindow.now && !unfolded);

  const cmds = $derived(project.runCommands);
  const running = $derived(cmds.filter((c) => app.launches[c.id]?.status === 'running').length);
  const selected = $derived(app.runCommand?.id ?? null);
  // Each agent's launches, under its name, once one of them ran: the project's commands run in its worktree,
  // then its test launches.
  const groups = $derived(
    Object.values(app.agents)
      .filter((a) => a.projectId === project.id)
      .map((a) => {
        const shell = app.shells[0]?.id ?? '';
        const c = recipeCommands(a, shell);
        const isola = a.isola ? [isolaCommand(a, shell)] : [];
        const inIts = cmds.map((x) => inWorktree(x, a));
        return { agent: a, cmds: [...inIts, ...isola, ...c.prepare, ...c.processes].filter((x) => app.launches[x.id]) };
      })
      .filter((g) => g.cmds.length > 0),
  );
  /** The commands of the section that crashed: said in its header, even folded. */
  const crashed = $derived([...cmds, ...groups.flatMap((g) => g.cmds)].filter((c) => app.launches[c.id]?.status === 'crashed').length);

  /** The state of a row in a word: running (or stopping), crashed, or at rest. */
  function tone(c: RunCommand): 'on' | 'stopping' | 'bad' | '' {
    const l = app.launches[c.id];
    if (l?.status === 'running') return l.stopping ? 'stopping' : 'on';
    return l?.status === 'crashed' ? 'bad' : '';
  }

  function act(e: Event, f: () => unknown) {
    // The row itself opens the log: its buttons only act.
    e.stopPropagation();
    f();
  }

  function configure() {
    app.modal = { kind: 'settings', page: 'launch', projectId: project.id };
  }
  /** The same settings, where Claude reads the project at once to fill in the commands (to read before saving). */
  function propose() {
    app.modal = { kind: 'settings', page: 'launch', projectId: project.id, suggest: true };
  }
  const isRunning = (c: RunCommand) => app.launches[c.id]?.status === 'running';

  /** The agents of the project with a worktree to run a command in (not the archived ones), as the list shows them. */
  const worktreeAgents = $derived(app.projectAgents.filter((a) => a.projectId === project.id && a.worktree));

  /** ▶ of one of the project's commands: in the project, or in the worktree of one of its agents (a menu, when one has one). */
  function run(e: Event, c: RunCommand) {
    e.stopPropagation();
    const own = project.runCommands.some((x) => x.id === c.id);
    if (!own || !worktreeAgents.length) {
      runHere(project, c);
      return;
    }
    menu.showAt(e.currentTarget as HTMLElement, [
      { label: t('runs.where.project'), icon: 'folder', onClick: () => runIn(project, c, null) },
      { label: '', separator: true },
      ...worktreeAgents.map((a) => ({
        label: t('runs.where.worktree', { name: a.name }),
        icon: 'branch' as const,
        onClick: () => runIn(project, c, a.id),
      })),
    ]);
  }
</script>

{#snippet runRow(c: RunCommand)}
  {@const st = launchStatus(app.launches[c.id])}
  {@const tn = tone(c)}
  <div
    class="row-item run"
    class:sel={selected === c.id}
    role="button"
    tabindex="0"
    aria-current={selected === c.id ? 'true' : undefined}
    data-run={c.id}
    use:tip={c.command}
    onclick={(e) => (e.ctrlKey || e.metaKey ? app.openBeside({ kind: 'launch', id: c.id }) : app.selectLaunch(c.id))}
    onpointerdown={(e) => paneDrag.press(e, { kind: 'launch', id: c.id }, c.name)}
    onkeydown={(e) => e.key === 'Enter' && e.target === e.currentTarget && app.selectLaunch(c.id)}
  >
    <i class="d {tn}" aria-hidden="true"></i>
    <span class="nm name">{c.name}</span>
    {#if parseWorktreeRunId(c.id)}<span class="wt">{t('runs.section.inItsWorktree')}</span>{/if}
    <span class="st {tn}">{st.label}</span>
    <span class="ra" class:keep={tn === 'bad'}>
      {#if isRunning(c)}
        <button
          class="icon-btn"
          use:tip={{ text: t('runs.action.restart'), aria: false }}
          aria-label={t('runs.action.restart')}
          onclick={(e) => act(e, () => restartLaunch(project, c))}><Icon name="restart" size={14} /></button
        >
        <button
          class="icon-btn"
          use:tip={{ text: t('common.stop'), aria: false }}
          aria-label={t('common.stop')}
          onclick={(e) => act(e, () => stopLaunch(c.id))}><Icon name="stop" size={14} /></button
        >
      {:else}
        <button
          class="icon-btn"
          use:tip={{ text: t('runs.action.run'), aria: false }}
          aria-label={t('runs.action.run')}
          aria-haspopup={project.runCommands.some((x) => x.id === c.id) && worktreeAgents.length ? 'menu' : undefined}
          onclick={(e) => run(e, c)}><Icon name="play" size={14} /></button
        >
      {/if}
    </span>
  </div>
{/snippet}

<section class="sec runs" class:folded aria-label={t('runs.section.title')}>
  <div class="sec-h">
    <FoldTitle title={t('runs.section.title')} short={shortWindow.now} {folded} ontoggle={() => (unfolded = !unfolded)} />
    {#if cmds.length}<span class="n">{running}/{cmds.length}</span>{/if}
    {#if crashed}<span class="sec-bad">{t('runs.section.crashed', { count: crashed })}</span>{/if}
    <span class="acts">
      {#if cmds.length}
        {#if running}
          <button class="mini-btn" onclick={() => stopAll(project)}>{t('common.stopAll')}</button>
        {:else}
          <button class="mini-btn" onclick={() => startAll(project)}>{t('runs.action.runAll')}</button>
        {/if}
      {/if}
      <button
        class="icon-btn"
        use:tip={{ text: t('runs.section.configure'), aria: false }}
        aria-label={t('runs.section.configure')}
        onclick={configure}><Icon name="sliders" size={14} /></button
      >
    </span>
  </div>
  <div class="rows">
    {#each cmds as c (c.id)}
      {@render runRow(c)}
    {:else}
      <div class="none">
        <span>{t('runs.section.none')}</span>
        <button class="link" onclick={configure}>{t('runs.section.set')}</button>
        <button class="link" onclick={propose}><Icon name="sparkle" size={12} />{t('runs.section.suggest')}</button>
      </div>
    {/each}
    {#each groups as g (g.agent.id)}
      <div class="group-label group">{g.agent.name}</div>
      {#each g.cmds as c (c.id)}
        {@render runRow(c)}
      {/each}
    {/each}
  </div>
</section>

<style>
  .mini-btn,
  .link {
    padding: 0;
    border: none;
    background: transparent;
    cursor: pointer;
  }
  .sec {
    display: grid;
    gap: 1px;
    min-height: 0;
    margin: 6px -2px 0;
    padding-top: 8px;
    border-top: 1px solid var(--line);
  }
  .sec-h {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 2px 0 8px;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
  }
  .sec-h .n {
    color: var(--dim);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .sec-h .acts {
    display: flex;
    gap: 2px;
    margin-left: auto;
  }
  /* A crash, said in the header: always, the rows may be folded. */
  .sec-bad {
    overflow: hidden;
    color: var(--del);
    font-weight: 500;
    text-overflow: ellipsis;
  }
  .mini-btn {
    height: 24px;
    padding: 0 8px;
    white-space: nowrap;
    border-radius: 6px;
    color: var(--muted);
    font-size: 12px;
    font-weight: 550;
  }
  .mini-btn:hover {
    background: var(--hover);
    color: var(--text);
  }
  .rows {
    display: grid;
    gap: 1px;
    max-height: 30vh;
    overflow: auto;
  }
  .row-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 28px;
    padding: 0 4px 0 9px;
    border-radius: 7px;
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
    outline: none;
  }
  .row-item:hover {
    background: var(--hover);
  }
  .row-item.sel {
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .row-item:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: -2px;
  }
  .d {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--idle);
  }
  .d.on {
    background: var(--run);
  }
  .d.stopping {
    background: var(--wait);
  }
  .d.bad {
    background: var(--del);
  }
  .nm {
    min-width: 0;
    overflow: hidden;
    color: var(--soft);
    font-weight: 550;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .wt {
    flex: none;
    color: var(--muted);
    font-size: 11.5px;
    white-space: nowrap;
  }
  .st {
    flex: none;
    margin-left: auto;
    color: var(--muted);
    font-size: 11.5px;
    white-space: nowrap;
  }
  .st.on {
    color: var(--run);
  }
  .st.stopping {
    color: var(--wait);
  }
  .st.bad {
    color: var(--del);
  }
  /* Shown on hover or focus; out of sight otherwise, but still there for the keyboard and a screen reader. */
  .ra {
    display: inline-flex;
    width: 0;
    overflow: hidden;
  }
  .row-item:hover .ra,
  .row-item.sel .ra,
  .row-item:focus-within .ra,
  .ra.keep {
    width: auto;
    margin-left: 2px;
  }
  .ra .icon-btn {
    width: 22px;
    height: 22px;
  }
  .group-label {
    padding: 6px 9px 2px;
    overflow: hidden;
    color: var(--dim);
    font: 11.5px/1.4 var(--mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .none {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px 10px;
    padding: 2px 9px 6px;
    color: var(--dim);
    font-size: 12px;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--p-ink);
    font-size: 12px;
  }
  .link:hover {
    text-decoration: underline;
  }

  /* A short window (820 px and less): the section folds to its header, a crash still said (choice 9), until its title
     unfolds it. */
  .folded .rows,
  .folded .mini-btn {
    display: none;
  }
  .sec.folded {
    padding-bottom: 2px;
  }
</style>
