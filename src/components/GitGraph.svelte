<script lang="ts">
  import { tick } from 'svelte';
  import { isAgentBranch } from '../lib/branches';
  import { deleteBranch, switchBranch, worktreeReason } from '../lib/branch-actions';
  import { fAgo } from '../lib/format';
  import { t } from '../lib/i18n';
  import { agentCommits, branchCommits, layout, mainLane, trunkTip, type Segment } from '../lib/graph';
  import { api } from '../lib/ipc';
  import { newestReads } from '../lib/reads';
  import { menu, type MenuItem } from '../lib/menu.svelte';
  import { app } from '../lib/state.svelte';
  import type { Agent, BranchInfo, Commit, GitLog, Project } from '../lib/types';
  import { syncInfo } from '../lib/git-sync.svelte';
  import { tip } from '../lib/tip';
  import Icon from './Icon.svelte';
  import { isMulti, repoLabel, touchedRepos } from '../lib/repos';

  // The repository's history, every branch included, with the agent's branch highlighted. A commit's menu (right
  // click, menu key, Shift+F10) starts a branch there, switches to its branches, compares it, deletes its branches.
  let { project, agent }: { project: Project; agent: Agent | null } = $props();

  /** A lane's width, a row's height, the middle of the first lane: the preview's graph. */
  const W = 16;
  const H = 34;
  const X0 = 14;
  /** The branches' colours (the trunk has its own, `--lane-main`). */
  const LANES = 8;

  let log = $state<GitLog | null>(null);
  /** The repository's branches, read with the history; null when they could not be, and there is no earlier reading. */
  let branches = $state<BranchInfo[] | null>(null);
  /** The project `branches` were read for. */
  let branchesOf: string | null = null;
  let error = $state<string | null>(null);
  let list = $state<HTMLElement>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  let first = true;
  let scrolled = false;
  // Answers in any order: the newest one come is shown, a newer read still on its way (ESC-522).
  const reads = newestReads();
  /** Whether a read of `pid`, `aid`, `r` is still of what is shown (another project, agent or repository: not shown). */
  const stillShown = (pid: string, aid: string | null, r: string | null) => pid === projectId && aid === agentId && r === repo;

  const projectId = $derived(project.id);
  const agentId = $derived(agent?.id ?? null);

  // Of several repositories (ESC-169): one repository's history at a time, the one the agent touched first by default.
  const multi = $derived(isMulti(project));
  let picked = $state<string | null>(null);
  const repo = $derived.by(() => {
    if (!multi) return null;
    const repos = project.repos ?? [];
    if (picked !== null && repos.includes(picked)) return picked;
    return (agent ? touchedRepos(project, agent)[0] : undefined) ?? repos[0] ?? null;
  });
  let pickerButton = $state<HTMLButtonElement>();
  function pickRepo() {
    if (!pickerButton) return;
    const repos = project.repos ?? [];
    menu.showAt(pickerButton, [
      { label: t('repos.history.of'), heading: true },
      ...repos.map((r, i) => ({
        label: repoLabel(r),
        checked: r === repo,
        key: i < 9 ? String(i + 1) : undefined,
        detail: app.git[project.id]?.repos?.find((g) => g.repo === r)?.branch,
        onClick: () => {
          picked = r;
          log = null;
        },
      })),
    ]);
  }

  $effect(() => {
    void app.gitTick;
    const [pid, aid, r] = [projectId, agentId, repo];
    clearTimeout(timer);
    timer = setTimeout(() => load(pid, aid, r), first ? 0 : 150);
    first = false;
    return () => clearTimeout(timer);
  });

  async function load(pid: string, aid: string | null, r: string | null) {
    const mine = reads.ask();
    try {
      // The history is what the graph is for: the branches only serve the menu, and fail without taking it down.
      const [result, all] = await Promise.all([api.gitLog(pid, aid, r), api.branchList(pid, r).catch(() => null)]);
      if (!stillShown(pid, aid, r) || !reads.take(mine)) return;
      log = result;
      // A read that fails for a moment leaves the last list in place (the menu keeps its entries); one read for another project is no use.
      if (all) {
        branches = all;
        branchesOf = pid;
      } else if (branchesOf !== pid) {
        branches = null;
        branchesOf = null;
      }
      error = null;
    } catch (e) {
      if (!stillShown(pid, aid, r) || !reads.take(mine)) return;
      error = String(e);
    }
  }

  const commits = $derived(log?.commits ?? []);
  /** The branch the trunk follows when the folder's commit is not in the history read. */
  const base = $derived(agent?.worktree?.baseBranch ?? null);
  const rows = $derived(layout(commits, trunkTip(commits, base)));
  const mine = $derived(branchCommits(commits, log?.head ?? null));
  const cols = $derived(Math.min(12, Math.max(1, ...rows.map((r) => r.width))));
  /** The lanes' column: three lanes at least, as the preview leaves them room. */
  const lanesWidth = $derived(Math.max(56, cols * W + 8));
  const projectAgents = $derived(Object.values(app.agents).filter((a) => a.projectId === project.id && a.worktree));
  const trunk = $derived(mainLane(commits, rows, base));
  /** The commits of each agent's own branch: told by the agent's name rather than its author's. */
  const byAgent = $derived(
    agentCommits(
      commits,
      projectAgents.map((a) => ({ branch: a.worktree!.branch, base: a.worktree!.baseBranch, name: a.name })),
    ),
  );
  /** The colour of a lane: the trunk's grey, else one of the branches' colours. */
  const color = (lane: number) => (lane === trunk ? 'var(--lane-main)' : `var(--lane-${(lane % LANES) + 1})`);
  /** The folder's branch and how far it is from its remote, under the tabs. */
  const repoGit = $derived(repo === null ? null : (app.git[project.id]?.repos?.find((g) => g.repo === repo) ?? null));
  const sync = $derived(
    repoGit ? (repoGit.ahead || repoGit.behind ? { ahead: repoGit.ahead, behind: repoGit.behind } : null) : syncInfo(app.git[project.id]),
  );
  const folderBranch = $derived(repoGit ? repoGit.branch || null : (app.git[project.id]?.branch ?? null));

  // Shows the agent's branch once the history is first loaded.
  $effect(() => {
    if (scrolled || !list || !mine.size) return;
    scrolled = true;
    list.querySelector('[data-mine="true"]')?.scrollIntoView({ block: 'center' });
  });

  const x = (col: number) => X0 + col * W;

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
    /** Its colour: its lane's, grey for a remote. */
    lane: string;
  }

  function labels(c: Commit, lane: number): Label[] {
    return c.refs
      .filter((r) => r !== 'HEAD' && !r.endsWith('/HEAD'))
      .map((r) => {
        if (r.startsWith('tag: '))
          return { text: r.slice(5), title: t('git.graph.tag', { name: r.slice(5) }), kind: 'tag', lane: 'var(--wait)' };
        const owner = Object.values(app.agents).find((a) => a.projectId === project.id && a.worktree?.branch === r);
        const kind =
          r === log?.head
            ? 'head'
            : owner
              ? 'agent'
              : r.includes('/') && !isAgentBranch(r, project.agentBranchPrefix)
                ? 'remote'
                : 'branch';
        return {
          text: r,
          title: owner ? t('git.graph.agentBranch', { branch: r, agent: owner.name }) : r,
          kind,
          lane: kind === 'remote' ? 'var(--muted)' : color(lane),
        };
      });
  }

  const byName = $derived(new Map((branches ?? []).map((b) => [b.name, b])));

  /**
   * Waits until the menu is gone and has given the focus back to the row it came from (keyboard), before a dialog opens:
   * the dialog's focus trap then takes the row as what had the focus, and gives it back there once closed.
   */
  const afterMenu = () => tick();

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
          onClick: async () => {
            await afterMenu();
            app.modal = { kind: 'newBranch', projectId: pid, start: c.hash, repo: repo ?? undefined };
          },
        },
      ],
      others.map((b) => {
        const why = worktreeReason(b);
        return {
          label: t('branches.graph.switchTo', { branch: b.name }),
          disabled: !!why,
          title: why ?? undefined,
          onClick: () => void switchBranch(pid, b.name, false, undefined, repo),
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
          onClick: async () => {
            await afterMenu();
            deleteBranch(pid, b, repo);
          },
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
        onClick: async () => {
          await afterMenu();
          compare(from, to, to === c.hash ? c.hash.slice(0, 7) : to);
        },
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
      repo: repo ?? undefined,
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
      repo: repo ?? undefined,
    };
  }
</script>

<div class="side-top" class:multi>
  {#if multi && repo !== null}
    <button class="select repo" bind:this={pickerButton} aria-haspopup="menu" aria-label={t('repos.history.pick')} onclick={pickRepo}
      ><b class="mono">{repoLabel(repo)}</b><Icon name="down" size={14} /></button
    >
  {/if}
  <div class="caption">
    {#if multi && repo !== null}{t('repos.history.caption', { repo: repoLabel(repo) })}{:else}{t(
        'git.graph.all',
      )}{/if}{#if folderBranch}{' '}· <span class="mono">{folderBranch}</span>{/if}
    {#if sync && (sync.behind || sync.ahead)}
      <span class="ab"
        ><span class="n"><Icon name="pull" size={11} /><span aria-hidden="true">{sync.behind}</span></span><span class="n"
          ><Icon name="up" size={11} /><span aria-hidden="true">{sync.ahead}</span></span
        ><span class="sr">{t('git.graph.sync', { behind: sync.behind, ahead: sync.ahead })}</span></span
      >
    {/if}
  </div>
</div>
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
      class="commit"
      style:grid-template-columns="{lanesWidth}px minmax(0, 1fr) auto"
      data-mine={own}
      use:tip={`${c.hash.slice(0, 7)} · ${c.author}`}
      onclick={() => open(c)}
      oncontextmenu={(e) => onContextMenu(e, c)}
      onkeydown={(e) => onKeydown(e, c)}
    >
      <svg class="lanes" width={lanesWidth} height={H} aria-hidden="true">
        {#each r.top as s, k (k)}
          <path d={path(s, 0, H / 2)} style:stroke={color(s.lane)} />
        {/each}
        {#each r.bottom as s, k (k)}
          <path d={path(s, H / 2, H)} style:stroke={color(s.lane)} />
        {/each}
        <circle cx={x(r.col)} cy={H / 2} r="4.5" style:fill={color(r.lane)} />
      </svg>
      <span class="msg-line">
        <span class="subject">{c.subject}</span>
        {#each labels(c, r.lane) as l (l.text + l.kind)}
          <span class="ref {l.kind}" style:--lane={l.lane} use:tip={l.title}>{l.text}</span>
        {/each}
      </span>
      <span class="who"><span>{byAgent.get(c.hash) ?? c.author}</span> · <span>{fAgo(c.time, app.now)}</span></span>
    </button>
  {/each}
</div>

<style>
  .side-top {
    padding: 12px 14px 8px;
  }
  /* Of several repositories (ESC-169): the repository's menu before the caption. */
  .side-top.multi {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .select.repo {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 8px 0 10px;
    border: none;
    border-radius: 7px;
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }
  .select.repo b {
    font-size: 12px;
    font-weight: 600;
  }
  .select.repo:hover {
    background: rgba(var(--fg-rgb), 0.08);
  }
  .side-top.multi .caption {
    min-width: 0;
  }
  .caption {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: 11.5px;
    white-space: nowrap;
  }
  .caption .mono {
    font-size: 11px;
  }
  .ab {
    display: inline-flex;
    gap: 4px;
    font-variant-numeric: tabular-nums;
  }
  .ab .n {
    display: inline-flex;
    align-items: center;
    gap: 1px;
  }
  .graph {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 6px 8px 14px;
  }
  .msg {
    padding: 30px 10px;
    text-align: center;
    font-size: 12.5px;
    color: var(--muted);
  }
  /* A commit: its place on the lanes, its message and the branches on it, who made it and when. */
  .commit {
    display: grid;
    align-items: center;
    gap: 6px;
    width: 100%;
    height: 34px;
    padding: 0 8px 0 0;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
  }
  .commit:hover {
    background: var(--hover);
  }
  .lanes {
    display: block;
    overflow: visible;
  }
  .lanes path {
    fill: none;
    stroke-width: 2;
    stroke-linecap: round;
  }
  .lanes circle {
    stroke: var(--panel);
    stroke-width: 2;
  }
  .msg-line {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    white-space: nowrap;
  }
  .subject {
    flex: 0 1 auto;
    min-width: 72px;
    overflow: hidden;
    color: var(--soft);
    text-overflow: ellipsis;
  }
  /* A branch gives way to the message only so far: both stay legible in a narrow panel. */
  .ref {
    flex: 0 1 auto;
    display: block;
    min-width: 0;
    max-width: 180px;
    height: 19px;
    overflow: hidden;
    padding: 0 6px;
    border-radius: 5px;
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--lane) 45%, transparent);
    background: color-mix(in oklch, var(--lane) 12%, transparent);
    color: var(--soft);
    font: 11px var(--mono);
    line-height: 19px;
    text-overflow: ellipsis;
  }
  .who {
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
</style>
