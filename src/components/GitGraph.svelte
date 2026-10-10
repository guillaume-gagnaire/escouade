<script lang="ts">
  import { isAgentBranch } from '../lib/branches';
  import { deleteBranch, switchBranch, worktreeReason } from '../lib/branch-actions';
  import { fAgo } from '../lib/format';
  import { t } from '../lib/i18n';
  import { branchCommits, layout, type Segment } from '../lib/graph';
  import { api } from '../lib/ipc';
  import { menu, type MenuItem } from '../lib/menu.svelte';
  import { app } from '../lib/state.svelte';
  import type { Agent, BranchInfo, Commit, GitLog, Project } from '../lib/types';

  // The repository's history, every branch included, with the agent's branch highlighted. A commit's menu (right
  // click, menu key, Shift+F10) starts a branch there, switches to its branches, compares it, deletes its branches.
  let { project, agent }: { project: Project; agent: Agent | null } = $props();

  const W = 14;
  const H = 26;
  const COLORS = [
    'oklch(0.72 0.12 48)',
    'oklch(0.74 0.12 235)',
    'oklch(0.76 0.12 150)',
    'oklch(0.72 0.14 330)',
    'oklch(0.82 0.13 80)',
    'oklch(0.7 0.13 290)',
    'oklch(0.75 0.11 190)',
    'oklch(0.72 0.14 25)',
  ];

  let log = $state<GitLog | null>(null);
  /** The repository's branches, read with the history; null when they could not be. */
  let branches = $state<BranchInfo[] | null>(null);
  let error = $state<string | null>(null);
  let list = $state<HTMLElement>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  let first = true;
  let scrolled = false;
  let seq = 0;

  const projectId = $derived(project.id);
  const agentId = $derived(agent?.id ?? null);

  $effect(() => {
    void app.gitTick;
    const [pid, aid] = [projectId, agentId];
    clearTimeout(timer);
    timer = setTimeout(() => load(pid, aid), first ? 0 : 150);
    first = false;
    return () => clearTimeout(timer);
  });

  async function load(pid: string, aid: string | null) {
    const mine = ++seq;
    try {
      // The history is what the graph is for: the branches only serve the menu, and fail without taking it down.
      const [result, all] = await Promise.all([api.gitLog(pid, aid), api.branchList(pid).catch(() => null)]);
      if (mine !== seq) return; // superseded by a newer request
      log = result;
      branches = all;
      error = null;
    } catch (e) {
      if (mine !== seq) return;
      error = String(e);
    }
  }

  const commits = $derived(log?.commits ?? []);
  const rows = $derived(layout(commits));
  const mine = $derived(branchCommits(commits, log?.head ?? null));
  const mineLanes = $derived(new Set(rows.filter((_, i) => mine.has(commits[i].hash)).map((r) => r.lane)));
  const cols = $derived(Math.min(12, Math.max(1, ...rows.map((r) => r.width))));

  // Shows the agent's branch once the history is first loaded.
  $effect(() => {
    if (scrolled || !list || !mine.size) return;
    scrolled = true;
    list.querySelector('[data-mine="true"]')?.scrollIntoView({ block: 'center' });
  });

  const x = (col: number) => col * W + W / 2;

  function path(s: Segment, y0: number, y1: number) {
    const [a, b] = [x(s.from), x(s.to)];
    if (a === b) return `M${a} ${y0}V${y1}`;
    const m = (y0 + y1) / 2;
    return `M${a} ${y0}C${a} ${m} ${b} ${m} ${b} ${y1}`;
  }

  interface Label {
    text: string;
    title: string;
    kind: 'head' | 'agent' | 'branch' | 'remote' | 'tag';
  }

  function labels(c: Commit): Label[] {
    return c.refs
      .filter((r) => r !== 'HEAD' && !r.endsWith('/HEAD'))
      .map((r) => {
        if (r.startsWith('tag: ')) return { text: r.slice(5), title: t('git.graph.tag', { name: r.slice(5) }), kind: 'tag' };
        const owner = Object.values(app.agents).find((a) => a.projectId === project.id && a.worktree?.branch === r);
        const kind = r === log?.head ? 'head' : owner ? 'agent' : r.includes('/') && !isAgentBranch(r) ? 'remote' : 'branch';
        return { text: owner ? owner.name : r, title: owner ? t('git.graph.agentBranch', { branch: r, agent: owner.name }) : r, kind };
      });
  }

  const byName = $derived(new Map((branches ?? []).map((b) => [b.name, b])));

  /** What can be done from a commit: groups of entries (start a branch, move or compare, delete), one separator apart. */
  function entriesOf(c: Commit): MenuItem[] {
    const pid = project.id;
    const here = c.refs.map((r) => byName.get(r)).filter((b): b is BranchInfo => !!b);
    // Only the local branches are acted upon: the remote ones are a name to compare with.
    const others = here.filter((b) => !b.remote && !b.current);
    const current = branches?.find((b) => b.current);

    const groups: MenuItem[][] = [
      [
        {
          label: t('branches.graph.createHere'),
          onClick: () => {
            app.modal = { kind: 'newBranch', projectId: pid, start: c.hash };
          },
        },
      ],
      others.map((b) => {
        const why = worktreeReason(b);
        return {
          label: t('branches.graph.switchTo', { branch: b.name }),
          disabled: !!why,
          title: why ?? undefined,
          onClick: () => void switchBranch(pid, b.name),
        };
      }),
      others.map((b) => {
        const why = worktreeReason(b);
        return {
          label: t('branches.graph.delete'),
          // Which one, when the commit has several.
          hint: others.length > 1 ? b.name : undefined,
          danger: true,
          disabled: !!why,
          title: why ?? undefined,
          onClick: () => deleteBranch(pid, b),
        };
      }),
    ];
    // The commit the folder is on has nothing to be compared with.
    if (!c.refs.includes('HEAD') && !here.some((b) => b.current)) {
      const from = current?.name ?? 'HEAD';
      // The commit is named by its branch when it has one (a local one first), else by itself.
      const to = (here.find((b) => !b.remote) ?? here[0])?.name ?? c.hash;
      groups[1].push({
        label: current ? t('branches.graph.compare') : t('branches.graph.compareHead'),
        onClick: () => compare(from, to, to === c.hash ? c.hash.slice(0, 7) : to),
      });
    }
    return groups.filter((g) => g.length).flatMap((g, i) => (i ? [{ label: '', separator: true }, ...g] : g));
  }

  /** What changes from `from` to `to` (a branch, or a commit), in the diff window; `shown` is how the title names `to`. */
  function compare(from: string, to: string, shown: string) {
    app.modal = {
      kind: 'diff',
      projectId: project.id,
      agentId: null,
      paths: [],
      title: t('branches.graph.compareTitle', { from, to: shown }),
      refs: { from, to },
    };
  }

  const onContextMenu = (e: MouseEvent, c: Commit) => menu.show(e, entriesOf(c));

  function onKeydown(e: KeyboardEvent, c: Commit) {
    if (e.key !== 'ContextMenu' && !(e.key === 'F10' && e.shiftKey)) return;
    e.preventDefault();
    menu.showAt(e.currentTarget as HTMLElement, entriesOf(c));
  }

  function open(c: Commit) {
    app.modal = {
      kind: 'diff',
      projectId: project.id,
      agentId: null,
      paths: [],
      title: `${c.hash.slice(0, 7)} · ${c.subject}`,
      commit: c.hash,
    };
  }
</script>

<div class="graph" bind:this={list}>
  {#if error}
    <div class="msg">{error}</div>
  {:else if !log}
    <div class="msg">{t('common.loading')}</div>
  {:else if !commits.length}
    <div class="msg">{t('git.graph.empty')}</div>
  {/if}
  {#each commits as c, i (c.hash)}
    {@const r = rows[i]}
    {@const own = mine.has(c.hash)}
    <button
      class="row"
      class:own
      data-mine={own}
      title="{c.hash.slice(0, 7)} · {c.author}"
      onclick={() => open(c)}
      oncontextmenu={(e) => onContextMenu(e, c)}
      onkeydown={(e) => onKeydown(e, c)}
    >
      <svg class="lanes" width={cols * W} height={H} aria-hidden="true">
        {#each r.top as s, k (k)}
          <path d={path(s, 0, H / 2)} stroke={COLORS[s.lane % COLORS.length]} class:dim={!mineLanes.has(s.lane)} />
        {/each}
        {#each r.bottom as s, k (k)}
          <path d={path(s, H / 2, H)} stroke={COLORS[s.lane % COLORS.length]} class:dim={!mineLanes.has(s.lane)} />
        {/each}
        <circle cx={x(r.col)} cy={H / 2} r={own ? 4.5 : 3.5} fill={COLORS[r.lane % COLORS.length]} class:dim={!own} />
      </svg>
      {#each labels(c) as l (l.text + l.kind)}
        <span class="ref {l.kind}" title={l.title}>{l.text}</span>
      {/each}
      <span class="subject">{c.subject}</span>
      <span class="when">{fAgo(c.time, app.now)}</span>
    </button>
  {/each}
</div>

<style>
  .graph {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 4px 8px 12px;
  }
  .msg {
    padding: 30px 10px;
    text-align: center;
    font-size: 12.5px;
    color: var(--muted);
  }
  .row {
    width: 100%;
    height: 26px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px 0 2px;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    text-align: left;
    cursor: pointer;
    min-width: 0;
    color: var(--muted);
  }
  .row:hover {
    background: var(--elev);
  }
  .row.own {
    color: var(--text);
  }
  .lanes {
    flex: none;
    overflow: hidden;
  }
  .lanes path {
    fill: none;
    stroke-width: 2;
  }
  .lanes .dim {
    opacity: 0.35;
  }
  .subject {
    flex: 1;
    min-width: 0;
    font-size: 12.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .when {
    flex: none;
    font-size: 11px;
    color: var(--dim);
  }
  .ref {
    flex: none;
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 1px 6px;
    border-radius: 3px;
    font-family: var(--mono);
    font-size: 10.5px;
    background: var(--elev2);
    color: var(--muted);
  }
  .ref.head {
    background: var(--accent);
    color: var(--accent-ink);
    font-weight: 700;
  }
  .ref.agent {
    background: color-mix(in oklch, var(--info) 22%, transparent);
    color: var(--text);
  }
  .ref.remote {
    background: transparent;
    border: 1px solid var(--line2);
  }
  .ref.tag {
    background: color-mix(in oklch, var(--wait) 20%, transparent);
    color: var(--text);
  }
</style>
