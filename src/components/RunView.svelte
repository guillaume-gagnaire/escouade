<script lang="ts">
  import { t } from '../lib/i18n';
  import { keyLabel, primaryKey } from '../lib/platform';
  import { launchStatus, log, restartLaunch, runHere, stopLaunch } from '../lib/launch-actions';
  import { parseTestId, parseWorktreeRunId } from '../lib/recipe';
  import { app } from '../lib/state.svelte';
  import { getXTerm, logKey, mountTerminal } from '../lib/terminals';
  import type { Project, RunCommand } from '../lib/types';
  import { fTime, joinPath, tildify } from '../lib/format';
  import { tip } from '../lib/tip';
  import Icon from './Icon.svelte';

  let { cmd, project }: { cmd: RunCommand; project: Project } = $props();
  let box = $state<HTMLDivElement>();
  let searchOpen = $state(false);
  let query = $state('');

  const run = $derived(app.launches[cmd.id]);
  const st = $derived(launchStatus(run));
  const running = $derived(run?.status === 'running');
  const shell = $derived(app.shells.find((s) => s.id === cmd.shell)?.label ?? cmd.shell);
  // A step of a recipe, or a launch command run in a worktree, runs in its agent's.
  const root = $derived.by(() => {
    const t = parseTestId(cmd.id) ?? parseWorktreeRunId(cmd.id);
    const a = t ? app.agents[t.agentId] : undefined;
    return a ? (a.worktree?.path ?? a.cwd) : project.path;
  });
  // The folder is written as the system of its root does: `\` under a Windows folder, `/` on macOS.
  const where = $derived(tildify(cmd.cwd ? joinPath(root, cmd.cwd) : root));
  const since = $derived(running && run ? fTime(run.startedAt) : null);
  /** The tone of its state: running, stopping, crashed, or at rest. */
  const tone = $derived(run?.status === 'running' ? (run.stopping ? 'wait' : '') : run?.status === 'crashed' ? 'err' : 'idle');
  /** The agent whose worktree it runs in (a launch command run there, choice 30), named beside the folder. */
  const owner = $derived.by(() => {
    const w = parseWorktreeRunId(cmd.id);
    return w ? (app.agents[w.agentId]?.name ?? null) : null;
  });
  /** Its own name: a launch command run in a worktree is listed as « Front · <agent> », and named here « Front ». */
  const title = $derived.by(() => {
    const w = parseWorktreeRunId(cmd.id);
    return (w && project.runCommands.find((c) => c.id === w.commandId)?.name) || cmd.name;
  });

  $effect(() => {
    const id = cmd.id;
    const el = box;
    if (!el) return;
    log(id);
    mountTerminal(logKey(id), el);
    const ro = new ResizeObserver(() => {
      try {
        getXTerm(logKey(id))?.fit.fit();
      } catch {
        /* hidden */
      }
    });
    ro.observe(el);
    return () => {
      ro.disconnect();
      mountTerminal(logKey(id), null);
    };
  });

  function find(next = true) {
    const x = getXTerm(logKey(cmd.id));
    if (!x || !query) return;
    if (next) x.search.findNext(query);
    else x.search.findPrevious(query);
  }
</script>

<main class="rv">
  <header class="head">
    <div class="h-title">
      <div class="row">
        <h1 class="name">{title}</h1>
        <span class="state {tone}"><i></i>{st.label}</span>
        {#if since}<span class="since">{t('runs.log.since', { time: since })}</span>{/if}
      </div>
      <div class="crumbs" use:tip={{ text: `${cmd.command} · ${where}`, cut: true }}>
        <span class="mono cmdline">{cmd.command}</span><span aria-hidden="true">·</span><span class="mono where">{where}</span>
        {#if owner}<span aria-hidden="true">·</span><span class="owner">{t('runs.where.inWorktree', { name: owner })}</span>{/if}
        <span aria-hidden="true">·</span><span class="shell">{shell}</span>
      </div>
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
            if (e.key === 'Escape') searchOpen = false;
          }}
        />
      {/if}
      <button
        class="btn icon-only"
        aria-label={t('runs.log.searchKey', { key: keyLabel('Ctrl+Shift+F') })}
        use:tip={t('runs.log.searchKey', { key: keyLabel('Ctrl+Shift+F') })}
        onclick={() => (searchOpen = !searchOpen)}><Icon name="search" size={14} /></button
      >
      <button class="btn" onclick={() => getXTerm(logKey(cmd.id))?.term.clear()}>{t('common.clear')}</button>
      {#if running}
        <button class="btn" onclick={() => restartLaunch(project, cmd)}><Icon name="restart" size={14} />{t('runs.action.restart')}</button>
        <button class="btn danger" onclick={() => stopLaunch(cmd.id)}><Icon name="stop" size={14} />{t('common.stop')}</button>
      {:else if run}
        <button class="btn primary" onclick={() => runHere(project, cmd)}
          ><Icon name="restart" size={14} />{t('runs.action.restart')}</button
        >
      {:else}
        <button class="btn primary" onclick={() => runHere(project, cmd)}><Icon name="play" size={14} />{t('runs.action.run')}</button>
      {/if}
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
  {#if !run}
    <div class="idle">
      <span>{t('runs.log.notRun')}</span>
      <button class="btn primary" onclick={() => runHere(project, cmd)}><Icon name="play" size={14} />{t('runs.action.run')}</button>
    </div>
  {/if}
</main>

<style>
  .rv {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    position: relative;
  }
  /* The header of the 2.0 design, as the conversation's: name, state and since when; the command, where, the shell. */
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
  .state {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 20px;
    padding: 0 8px 0 7px;
    border-radius: 10px;
    background: oklch(from var(--run) l c h / 0.1);
    color: var(--run);
    font-size: 11.5px;
    font-weight: 550;
    white-space: nowrap;
  }
  .state i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }
  .state.wait {
    background: oklch(from var(--wait) l c h / 0.1);
    color: var(--wait);
  }
  .state.idle {
    background: rgba(var(--fg-rgb), 0.05);
    color: var(--muted);
  }
  .state.err {
    background: oklch(from var(--del) l c h / 0.1);
    color: var(--del);
  }
  .since {
    flex: none;
    white-space: nowrap;
    color: var(--muted);
    font-size: 12px;
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    overflow: hidden;
    white-space: nowrap;
    color: var(--muted);
    font-size: 12px;
  }
  .crumbs > span {
    flex: none;
  }
  .crumbs .cmdline {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--soft);
  }
  .crumbs .where {
    flex: 0 1 auto;
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
  .idle {
    position: absolute;
    inset: 60px 0 0 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    color: var(--muted);
    font-size: 13px;
    pointer-events: none;
  }
  .idle .btn {
    pointer-events: auto;
  }
</style>
