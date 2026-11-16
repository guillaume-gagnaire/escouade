<script lang="ts">
  import { tick, untrack, type Snippet } from 'svelte';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { failureTitle } from '../../lib/prview';
  import {
    fileTree,
    firstLine,
    isViewed,
    setViewed,
    statusLetter,
    treeOrder,
    viewedCount,
    viewedKey,
    type FilesScope,
  } from '../../lib/prfiles';
  import { shortSha, subjectOf } from '../../lib/prpage';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { PrContents, PrContentsFile, PrFailure, PrLine, PrRead } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import PrFileDiff from './PrFileDiff.svelte';

  // The tab « Fichiers » of a pull request's view (ESC-467, PR-3 of the PR in Escouade, direction A of design/pr-2-3). A
  // bar: the commits shown (the whole pull request, or one), the tree's button, « 3/12 vus ». The tree of the files
  // changed on the left, filtered, following the scrolling; their diffs on the right, git's, each « Vu » kept by pull
  // request and by commit (`ui.prViewed`): a file marked in the whole pull request is no longer once a commit changes it.
  // « Ouvrir dans l'éditeur » opens a file, at a line, in the VS Code view of `source` (the agent's worktree that holds the
  // branch, else the project's folder; null: none). `under` draws what goes under a line (PR-4, PR-5); `above` what goes
  // above a file's lines, `head` above the list (ESC-474: the threads apart). `threads` gives a file's open threads and its
  // lines that have one; `reveal` brings a thread into view (« Suivante »); `onComment` offers « + » on the lines.
  let {
    projectId,
    number,
    hostRepo,
    contents,
    scope = $bindable('pr'),
    source,
    under,
    above,
    head,
    threads,
    reveal = null,
    onComment,
  }: {
    projectId: string;
    number: number;
    hostRepo?: string;
    contents: PrContents;
    scope?: FilesScope;
    source: string | null;
    under?: Snippet<[PrContentsFile, PrLine, number]>;
    above?: Snippet<[PrContentsFile]>;
    head?: Snippet;
    threads?: (f: PrContentsFile) => { open: number; marked: Record<number, boolean> };
    reveal?: { key: string; path: string | null; index: number; seq: number } | null;
    onComment?: (f: PrContentsFile, line: PrLine, index: number, extend: boolean) => void;
  } = $props();

  let filter = $state('');
  let closedDirs = $state<Record<string, boolean>>({});
  /** The tree shown: null follows the width (shown when wide enough), else the user's choice. */
  let treeChoice = $state<boolean | null>(null);
  let width = $state(1000);
  // Not measured yet (0): wide, as most windows are.
  const wide = $derived(width === 0 || width >= 760);
  const treeOn = $derived(treeChoice ?? wide);
  let current = $state<string | null>(null);
  let listBox = $state<HTMLElement | null>(null);
  let treeBox = $state<HTMLElement | null>(null);
  /** The files of the commits read, by hash. */
  let commitFiles = $state<Record<string, PrRead<PrContentsFile[]> | 'reading'>>({});

  const key = $derived(viewedKey(projectId, hostRepo, number));
  const viewed = $derived(app.ui.prViewed?.[key]);
  const sha = $derived(scope === 'pr' ? null : scope.sha);
  const read = $derived(sha ? commitFiles[sha] : null);
  const files = $derived<PrContentsFile[] | null>(sha ? (read && read !== 'reading' ? (read.data ?? null) : null) : contents.files);
  const failure = $derived<PrFailure | null>(sha && read && read !== 'reading' ? (read.failure ?? null) : null);
  const rows = $derived(files ? fileTree(files, filter, closedDirs) : []);
  const ordered = $derived(files ? treeOrder(files, filter) : []);
  const seen = $derived(files ? viewedCount(viewed, scope, files) : 0);
  const totals = $derived((files ?? []).reduce((s, f) => ({ add: s.add + f.additions, del: s.del + f.deletions }), { add: 0, del: 0 }));

  // A commit chosen: its files read once.
  $effect(() => {
    const s = sha;
    if (!s || commitFiles[s]) return;
    commitFiles[s] = 'reading';
    api
      .prCommitFiles(projectId, number, s, hostRepo)
      .catch((e): PrRead<PrContentsFile[]> => ({ failure: { kind: 'other', message: String(e) } as PrFailure }))
      .then((r) => (commitFiles[s] = r));
  });

  // Another scope: from the top.
  $effect(() => {
    void scope;
    current = null;
    listBox?.scrollTo?.({ top: 0 });
  });

  function mark(f: PrContentsFile, on: boolean) {
    app.setPrViewed(key, setViewed(viewed, scope, f, on, Date.now()));
  }

  function openIn(f: PrContentsFile, line: number | null) {
    if (!source) return;
    const at = line ?? firstLine(f) ?? undefined;
    void app.openEditor({ projectId, source, path: f.path, ...(at ? { line: at } : {}) });
  }

  /** A path inside a selector's quotes. */
  const quoted = (s: string) => s.replace(/["\\]/g, '\\$&');

  function jump(path: string) {
    current = path;
    const el = listBox?.querySelector(`section[data-path="${quoted(path)}"]`);
    el?.scrollIntoView?.({ block: 'start' });
    if (!wide) treeChoice = false;
  }

  // The tree follows the scrolling: the file whose diff is at the top of the list is the current one.
  let frame = 0;
  function onScroll() {
    if (frame) return;
    frame = requestAnimationFrame(() => {
      frame = 0;
      if (!listBox) return;
      const top = listBox.getBoundingClientRect().top + 40;
      for (const el of listBox.querySelectorAll<HTMLElement>('section.file')) {
        if (el.getBoundingClientRect().bottom > top) {
          if (current !== el.dataset.path) current = el.dataset.path ?? null;
          break;
        }
      }
    });
  }
  $effect(() => {
    const c = current;
    if (!c || !treeBox) return;
    treeBox.querySelector(`[data-tree-path="${quoted(c)}"]`)?.scrollIntoView?.({ block: 'nearest' });
  });

  const scopeValue = $derived(sha ?? 'pr');

  // « Suivante » (ESC-474): its file shown (the filter emptied when it hides it), then the thread brought into view and
  // focused once its part is drawn.
  let seenReveal = 0;
  $effect(() => {
    const r = reveal;
    if (!r || r.seq === seenReveal) return;
    seenReveal = r.seq;
    untrack(() => {
      if (r.path && !ordered.some((f) => f.path === r.path)) filter = '';
      if (r.path) current = r.path;
      void bring(r.key);
    });
  });

  async function bring(key: string) {
    for (let i = 0; i < 20; i++) {
      await tick();
      const el = listBox?.querySelector<HTMLElement>(`[data-thread-key="${quoted(key)}"]`);
      if (el) {
        el.scrollIntoView?.({ block: 'center' });
        el.focus?.({ preventScroll: true });
        return;
      }
      await new Promise((r) => requestAnimationFrame(() => r(null)));
    }
  }
</script>

<div class="files" bind:clientWidth={width}>
  <div class="bar">
    <button
      class="icon-btn"
      aria-pressed={treeOn}
      aria-label={treeOn ? t('prs.page.files.hideTree') : t('prs.page.files.showTree')}
      use:tip={treeOn ? t('prs.page.files.hideTree') : t('prs.page.files.showTree')}
      onclick={() => (treeChoice = !treeOn)}><Icon name="pane-l" size={15} /></button
    >
    <label class="pick">
      <span class="sr">{t('prs.page.files.scope')}</span>
      <select
        value={scopeValue}
        onchange={(e) => {
          const v = e.currentTarget.value;
          scope = v === 'pr' ? 'pr' : { sha: v };
        }}
      >
        <option value="pr">{t('prs.page.files.allCommits', { count: contents.commits.length, word: contents.word })}</option>
        {#each contents.commits as k (k.sha)}
          <option value={k.sha}>{shortSha(k.sha)} · {subjectOf(k.message)}</option>
        {/each}
      </select>
    </label>
    {#if files}
      <span class="sum">
        {t('prs.page.files.count', { count: files.length })}
        <span class="mono add">+{totals.add}</span><span class="mono del">−{totals.del}</span>
      </span>
      <span class="sp"></span>
      <span class="seen" role="status" aria-label={t('prs.page.files.viewedCount', { seen, total: files.length })}>
        <span class="meter" aria-hidden="true"><i style:width="{files.length ? (100 * seen) / files.length : 0}%"></i></span>
        {t('prs.page.files.viewedCount', { seen, total: files.length })}
      </span>
    {/if}
  </div>

  {#if failure}
    <div class="msg err" role="alert"><Icon name="info" size={14} />{failureTitle(failure)} <span class="why">{failure.message}</span></div>
  {:else if !files}
    <div class="msg" role="status"><span class="spinner" aria-hidden="true"></span>{t('prs.page.files.reading')}</div>
  {:else if !contents.diff || contents.diffFailure}
    <div class="msg" role="note">
      <Icon name="info" size={14} />{t('prs.page.files.noDiff', { word: contents.word })}
      {#if contents.diffFailure}<span class="why">{contents.diffFailure}</span>{/if}
    </div>
  {:else}
    <div class="split" class:with-tree={treeOn} class:over={treeOn && !wide}>
      {#if treeOn}
        <nav class="tree" aria-label={t('prs.page.files.tree')} bind:this={treeBox}>
          <input
            class="field filter"
            type="search"
            placeholder={t('prs.page.files.filter')}
            aria-label={t('prs.page.files.filter')}
            bind:value={filter}
          />
          <ul>
            {#each rows as r (r.kind + r.path)}
              <li style:--depth={r.depth}>
                {#if r.kind === 'dir'}
                  <button class="node dir" aria-expanded={!closedDirs[r.path]} onclick={() => (closedDirs[r.path] = !closedDirs[r.path])}
                    ><Icon name={closedDirs[r.path] ? 'right' : 'down'} size={11} /><Icon name="folder" size={13} /><span class="nm"
                      >{r.name}</span
                    ></button
                  >
                {:else}
                  {@const v = isViewed(viewed, scope, r.file)}
                  <button
                    class="node"
                    class:cur={current === r.path}
                    class:seen={v}
                    data-tree-path={r.path}
                    aria-current={current === r.path ? 'true' : undefined}
                    onclick={() => jump(r.path)}
                  >
                    <span class="st {r.file.status}" aria-label={t(`prs.page.files.status.${r.file.status}`)}
                      >{statusLetter(r.file.status)}</span
                    >
                    <span class="nm">{r.name}</span>
                    {#if v}<span class="ok" aria-label={t('prs.page.files.viewed')}><Icon name="check" size={11} /></span>{/if}
                    <span class="n mono"
                      >{#if r.file.binary}{t('prs.page.files.binTag')}{:else}<span class="add">+{r.file.additions}</span>
                        <span class="del">−{r.file.deletions}</span>{/if}</span
                    >
                  </button>
                {/if}
              </li>
            {:else}
              <li class="none">{t('prs.page.files.noMatch')}</li>
            {/each}
          </ul>
        </nav>
      {/if}
      <div class="list" bind:this={listBox} onscroll={onScroll}>
        {#if head && scope === 'pr'}{@render head()}{/if}
        {#each ordered as f (f.path)}
          <PrFileDiff
            file={f}
            viewed={isViewed(viewed, scope, f)}
            root={listBox}
            canOpen={!!source}
            onViewed={(on) => mark(f, on)}
            onOpen={(line) => openIn(f, line)}
            under={scope === 'pr' ? under : undefined}
            above={scope === 'pr' ? above : undefined}
            openThreads={threads?.(f).open ?? 0}
            marked={threads?.(f).marked}
            reveal={reveal && reveal.path === f.path ? reveal : null}
            onComment={onComment && scope === 'pr' ? (l, i, extend) => onComment(f, l, i, extend) : undefined}
          />
        {:else}
          <p class="msg">{files.length ? t('prs.page.files.noMatch') : t('prs.page.files.none')}</p>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .files {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .bar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    height: 42px;
    padding: 0 14px 0 10px;
    border-bottom: 1px solid var(--line);
    font-size: 12.5px;
    color: var(--muted);
    min-width: 0;
  }
  .pick select {
    height: 28px;
    max-width: 320px;
    padding: 0 8px;
    border-radius: 7px;
    border: 0;
    color: var(--text);
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
    font: inherit;
    text-overflow: ellipsis;
  }
  .pick select:focus-visible {
    outline: 2px solid var(--brain);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  .sum {
    display: inline-flex;
    gap: 7px;
    white-space: nowrap;
  }
  .add {
    color: var(--add);
  }
  .del {
    color: var(--del);
  }
  .sp {
    flex: 1;
  }
  .seen {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    white-space: nowrap;
    color: var(--soft);
    font-variant-numeric: tabular-nums;
  }
  .meter {
    width: 64px;
    height: 5px;
    border-radius: 3px;
    background: var(--raise-2);
    overflow: hidden;
  }
  .meter i {
    display: block;
    height: 100%;
    background: var(--run);
  }
  .split {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    position: relative;
  }
  .split.with-tree {
    grid-template-columns: 260px minmax(0, 1fr);
  }
  .split.over {
    grid-template-columns: minmax(0, 1fr);
  }
  .tree {
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--line);
    background: var(--bg);
  }
  .split.over .tree {
    position: absolute;
    inset: 0 auto 0 0;
    z-index: 3;
    width: min(300px, 85%);
    box-shadow: var(--shadow-pop, 0 8px 30px rgb(0 0 0 / 0.35));
  }
  .filter {
    margin: 10px 10px 6px;
    height: 28px;
  }
  .tree ul {
    flex: 1;
    min-height: 0;
    overflow: auto;
    list-style: none;
    margin: 0;
    padding: 0 6px 10px;
  }
  .node {
    all: unset;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    height: 26px;
    padding: 0 6px 0 calc(6px + var(--depth) * 12px);
    border-radius: 6px;
    font-size: 12px;
    color: var(--soft);
    cursor: pointer;
    min-width: 0;
  }
  .node:hover {
    background: var(--hover);
  }
  .node:focus-visible {
    outline: 2px solid var(--brain);
    outline-offset: -2px;
  }
  .node.dir {
    color: var(--muted);
  }
  .node.cur {
    color: var(--text);
    background: var(--raise-2);
  }
  .node.seen .nm {
    color: var(--muted);
  }
  .nm {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ok {
    color: var(--run);
    display: inline-flex;
  }
  .n {
    flex: none;
    font-size: 10.5px;
    color: var(--dim);
  }
  .st {
    flex: none;
    width: 12px;
    text-align: center;
    font-family: var(--mono);
    font-size: 10.5px;
    font-weight: 700;
    color: var(--wait);
  }
  .st.added {
    color: var(--add);
  }
  .st.removed {
    color: var(--del);
  }
  .st.renamed,
  .st.copied {
    color: var(--info);
  }
  .none {
    padding: 10px;
    color: var(--muted);
    font-size: 12px;
  }
  .list {
    min-height: 0;
    overflow: auto;
    /* No padding above: a file's head sticks flush to the top, nothing shows over it. */
    padding: 0 14px 24px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .list > :global(:first-child) {
    margin-top: 12px;
  }
  .msg {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 24px;
    color: var(--muted);
    font-size: 12.5px;
  }
  .msg.err {
    color: var(--del);
  }
  .why {
    color: var(--muted);
    font-size: 12px;
  }
</style>
