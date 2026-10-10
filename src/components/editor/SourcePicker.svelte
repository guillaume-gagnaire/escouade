<script lang="ts">
  import { basename } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import type { Agent, GitInfo, Project } from '../../lib/types';
  import StatusDot from '../StatusDot.svelte';

  let {
    project,
    source,
    agents,
    git,
    onpick,
  }: { project: Project; source: string; agents: Agent[]; git: GitInfo | undefined; onpick: (source: string) => void } = $props();

  let open = $state(false);
  let box = $state<HTMLDivElement>();

  const branch = $derived(git?.branch || t('editor.source.noBranch'));
  const current = $derived(source === 'project' ? null : (agents.find((a) => a.id === source) ?? null));
  // `git` tallies the project's own changes together with those of every worktree: take the worktrees out.
  const projectCount = $derived(
    git ? Math.max(0, git.modified + git.added + git.deleted - agents.reduce((n, a) => n + (git.agents[a.id] ?? 0), 0)) : 0,
  );

  function pick(s: string) {
    open = false;
    onpick(s);
  }
</script>

<svelte:window onpointerdown={(e) => open && box && !box.contains(e.target as Node) && (open = false)} />

<div class="picker" bind:this={box}>
  <button
    class="btn src"
    aria-haspopup="menu"
    aria-expanded={open}
    aria-label={t('editor.source.label', { name: current ? current.name : branch })}
    onclick={() => (open = !open)}
  >
    {#if current}<StatusDot status={current.status} size={7} />{:else}<span class="glyph mono">⎇</span>{/if}
    <span class="nm">{current ? current.name : branch}</span>
    <span class="kind mono">{current ? t('editor.source.kindWorktree') : t('editor.source.kindBranch')}</span>
    <span class="chev">▾</span>
  </button>
  {#if open}
    <div class="menu" role="menu" aria-label={t('common.browse')}>
      <span class="title">{t('common.browse')}</span>
      <button class="opt" role="menuitemradio" aria-checked={source === 'project'} onclick={() => pick('project')}>
        <span class="ic mono">⎇</span>
        <span class="txt"
          ><span class="nm">{branch}</span><span class="sub mono">{t('editor.source.projectBranch', { path: project.path })}</span></span
        >
        <span class="count mono" class:some={projectCount > 0}
          >{projectCount ? t('editor.source.delta', { count: projectCount }) : t('editor.source.clean')}</span
        >
        <span class="check">{source === 'project' ? '✓' : ''}</span>
      </button>
      {#each agents as a (a.id)}
        {@const n = git?.agents[a.id] ?? 0}
        <button class="opt" role="menuitemradio" aria-checked={source === a.id} onclick={() => pick(a.id)}>
          <span class="ic"><StatusDot status={a.status} size={7} /></span>
          <span class="txt"
            ><span class="nm">{a.name}</span><span class="sub mono"
              >.claude/worktrees/{a.worktree ? basename(a.worktree.path) : a.name} · {a.model}</span
            ></span
          >
          <span class="count mono" class:some={n > 0}>{n ? t('editor.source.changes', { count: n }) : t('editor.source.clean')}</span>
          <span class="check">{source === a.id ? '✓' : ''}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .picker {
    position: relative;
  }
  .src {
    height: 36px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 10px 0 12px;
  }
  .src .nm {
    font-size: 13.5px;
    font-weight: 700;
  }
  .kind {
    font-size: 10.5px;
    padding: 2px 6px;
    border-radius: 3px;
    background: var(--elev2);
    color: var(--muted);
  }
  .chev {
    font-size: 10px;
    color: var(--dim);
  }
  .glyph {
    font-size: 13px;
    color: var(--muted);
  }
  .menu {
    position: absolute;
    top: 42px;
    left: 0;
    z-index: 30;
    width: 380px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
    border-radius: var(--r);
    border: 1px solid var(--line2);
    background: var(--elev);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.45);
  }
  .title {
    padding: 6px 10px 4px;
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--dim);
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .opt:hover,
  .opt[aria-checked='true'] {
    background: var(--elev2);
  }
  .ic {
    width: 16px;
    flex: none;
    display: flex;
    justify-content: center;
    color: var(--muted);
  }
  .txt {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .txt .nm {
    font-size: 13px;
    font-weight: 600;
  }
  .sub {
    font-size: 10.5px;
    color: var(--dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    font-size: 10.5px;
    color: var(--dim);
  }
  .count.some {
    color: var(--wait);
  }
  .check {
    width: 14px;
    font-size: 12px;
    color: var(--accent);
  }
</style>
