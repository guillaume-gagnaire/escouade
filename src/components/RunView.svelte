<script lang="ts">
  import { t } from '../lib/i18n';
  import { keyLabel, primaryKey } from '../lib/platform';
  import { launchStatus, log, restartLaunch, startLaunch, stopLaunch } from '../lib/launch-actions';
  import { parseTestId } from '../lib/recipe';
  import { app } from '../lib/state.svelte';
  import { SHELL_GLYPH } from '../lib/term-actions';
  import { getXTerm, logKey, mountTerminal } from '../lib/terminals';
  import type { Project, RunCommand } from '../lib/types';
  import { fTime, joinPath, tildify } from '../lib/format';

  let { cmd, project }: { cmd: RunCommand; project: Project } = $props();
  let box = $state<HTMLDivElement>();
  let searchOpen = $state(false);
  let query = $state('');

  const run = $derived(app.launches[cmd.id]);
  const st = $derived(launchStatus(run));
  const running = $derived(run?.status === 'running');
  const shell = $derived(app.shells.find((s) => s.id === cmd.shell)?.label ?? cmd.shell);
  // A step of a recipe runs in its agent's worktree.
  const root = $derived.by(() => {
    const t = parseTestId(cmd.id);
    const a = t ? app.agents[t.agentId] : undefined;
    return a ? (a.worktree?.path ?? a.cwd) : project.path;
  });
  // The folder is written as the system of its root does: `\` under a Windows folder, `/` on macOS.
  const where = $derived(tildify(cmd.cwd ? joinPath(root, cmd.cwd) : root));
  const since = $derived(running && run ? fTime(run.startedAt) : null);

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
    <div class="who">
      <div class="l1">
        <span class="name">{cmd.name}</span>
        <span class="badge mono" style:color={SHELL_GLYPH[cmd.shell]?.c}>{shell}</span>
        <span class="status" style:color={st.color}>{st.label}</span>
        {#if since}<span class="since">{t('runs.log.since', { time: since })}</span>{/if}
      </div>
      <span class="sub mono" title={where}><span class="cmdline">{cmd.command}</span> <span class="where">· {where}</span></span>
    </div>
    <div style="flex:1"></div>
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
    <button class="btn ghost" title={t('runs.log.searchKey', { key: keyLabel('Ctrl+Shift+F') })} onclick={() => (searchOpen = !searchOpen)}
      >⌕</button
    >
    <button class="btn ghost" onclick={() => getXTerm(logKey(cmd.id))?.term.clear()}>{t('common.clear')}</button>
    {#if running}
      <button class="btn" onclick={() => restartLaunch(project, cmd)}>⟳ {t('runs.action.restart')}</button>
      <button class="btn danger" onclick={() => stopLaunch(cmd.id)}>■ {t('common.stop')}</button>
    {:else if run}
      <button class="btn primary" onclick={() => startLaunch(project, cmd)}>⟳ {t('runs.action.restart')}</button>
    {:else}
      <button class="btn primary" onclick={() => startLaunch(project, cmd)}>▶ {t('runs.action.run')}</button>
    {/if}
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
      <button class="btn primary" onclick={() => startLaunch(project, cmd)}>▶ {t('runs.action.run')}</button>
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
  .head {
    height: 60px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 24px;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
  }
  .who {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }
  .l1 {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .name {
    font-size: 15px;
    font-weight: 700;
  }
  .badge {
    font-size: 11px;
    padding: 2px 7px;
    border-radius: 3px;
    background: var(--elev2);
  }
  .status {
    font-family: var(--mono);
    font-size: 11.5px;
    font-weight: 600;
  }
  .since {
    font-size: 11px;
    color: var(--dim);
  }
  .sub {
    font-size: 11px;
    color: var(--dim);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .cmdline {
    color: var(--muted);
  }
  .search {
    height: 28px;
    width: 220px;
  }
  .btn {
    height: 28px;
    font-size: 12px;
  }
  .box {
    flex: 1;
    min-height: 0;
    padding: 12px 6px 6px 16px;
    background: var(--term);
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
