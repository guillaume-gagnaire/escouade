<script lang="ts">
  import { untrack } from 'svelte';
  import { fAgo, fList, fTime } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import {
    PR_TABS,
    agentWithPr,
    checksCount,
    conversationCount,
    hasBody,
    initials,
    missingParts,
    pageFailure,
    reviewers,
    shortSha,
    statePill,
    subjectOf,
    threadPlace,
    threadState,
    ticketOf,
    timeline,
    type PrTab,
  } from '../../lib/prpage';
  import { treeOrder, viewedCount, viewedKey, type FilesScope } from '../../lib/prfiles';
  import {
    NO_WRITING,
    draftKey,
    extendPick,
    lineTarget,
    nextStop,
    openOrder,
    pickOf,
    pickPlace,
    placeThreads,
    threadCounts,
    threadKey,
    writeFailure,
    type LinePick,
    type PrWrite,
  } from '../../lib/prthreads';
  import { openPrPage } from '../../lib/prs';
  import {
    SEVERITIES,
    isLive,
    keptNotes,
    mergeStops,
    noteKey,
    noteOrder,
    notePlace,
    onItsLine,
    placeNotes,
    severityCounts,
    shownBy,
    sortNotes,
    type NextScope,
    type SeverityFilter,
  } from '../../lib/prreview';
  import { readPref, writePref } from '../../lib/prefs';
  import { POLL_MS, stillRunning } from '../../lib/prview';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type {
    PrContents,
    PrContentsFile,
    PrDetail,
    PrFailure,
    PrLine,
    PrRead,
    PrThread,
    Project,
    PullRequest,
    ReviewNote,
    Severity,
  } from '../../lib/types';
  import CiChecks from '../CiChecks.svelte';
  import Icon from '../Icon.svelte';
  import Markdown from '../conv/Markdown.svelte';
  import PrAgentNote from './PrAgentNote.svelte';
  import PrComposer from './PrComposer.svelte';
  import PrFiles from './PrFiles.svelte';
  import PrThreadView from './PrThread.svelte';

  // The view of a pull request (ESC-457, PR-2 of the PR in Escouade, direction A of design/pr-2-3): in the work area, in
  // place of the list « Pull requests », in reading only. Its header (number, title, state, branches, ticket, author,
  // « Ouvrir sur <hébergeur> »), its tabs Conversation (the description, a third party's, then the timeline of the commits,
  // comments, threads and reviews), Commits (each one's files a click away), Contrôles (the tab « PR et CI »'s checks,
  // their log and « Relancer les échecs ») and Fichiers (ESC-467, PR-3: `PrFiles`). The aside says its reviewers, its labels and where it stands; the merge box
  // (PR-8) will go at its foot. Read when shown and again when the host says it changed (`prChanged`).
  let {
    project,
    number,
    hostRepo,
    listed = null,
    initialTab = 'conversation',
  }: { project: Project; number: number; hostRepo?: string; listed?: PullRequest | null; initialTab?: PrTab } = $props();

  let contents = $state<PrRead<PrContents> | null>(null);
  let detail = $state<PrRead<PrDetail> | null>(null);
  let reading = $state(false);
  let readAt = $state<number | null>(null);
  let tab = $state<PrTab>(untrack(() => initialTab));
  /** What the tab « Fichiers » shows: the whole pull request, or one of its commits. */
  let scope = $state<FilesScope>('pr');
  /** The commits whose whole message is shown, by their hash. */
  let opened = $state<Record<string, boolean>>({});
  /** The resolved or outdated threads unfolded, by their key. */
  let unfolded = $state<Record<string, boolean>>({});
  let online = $state(typeof navigator === 'undefined' ? true : navigator.onLine);

  const c = $derived(contents?.data ?? null);
  const d = $derived(detail?.data ?? null);
  const pr = $derived(c?.pr ?? listed);
  const word = $derived(c?.word ?? app.prWords[project.id] ?? 'PR');
  const host = $derived(app.codeHosts[project.id]?.name ?? t('prs.page.host'));
  const items = $derived(timeline(c, d));
  const missing = $derived(missingParts(c));
  const people = $derived(reviewers(pr, d));
  const ticket = $derived(ticketOf(Object.values(app.tickets), project.id, number, hostRepo));
  const agent = $derived(agentWithPr(Object.values(app.agents), project.id, number, hostRepo));
  const failure = $derived(contents?.failure ?? null);
  const changes = $derived(app.prChanges[`${project.id}#${number}`] ?? 0);
  const tabCount = $derived<Record<PrTab, string>>({
    conversation: c ? String(conversationCount(c)) : '',
    commits: c?.reading.commits ? String(c.commits.length) : '',
    checks: checksCount(d),
    files:
      c?.diff && !c.diffFailure
        ? `${viewedCount(app.ui.prViewed?.[viewedKey(project.id, hostRepo, number)], 'pr', c.files)}/${c.files.length}`
        : '',
  });
  // « Ouvrir dans l'éditeur »: in the worktree of the agent that holds the branch (its own, or its ticket's), else the project's folder.
  const holder = $derived(agent ?? (ticket?.agentId ? (app.agents[ticket.agentId] ?? null) : null));
  const source = $derived(holder?.worktree?.path && !holder.archived ? holder.id : 'project');

  /** A commit's files: the tab « Fichiers » on it. */
  function filesOf(sha: string) {
    scope = { sha };
    tab = 'files';
  }

  // ---------- The threads (ESC-474, PR-4) ----------

  /** What the host writes in the user's name, on a click: no button for the rest. */
  const writing = $derived(c?.writing ?? NO_WRITING);
  /** The threads under their line in the whole pull request's diff, the others apart. */
  const placed = $derived(c ? placeThreads(c.files, c.threads) : { under: {}, stale: {}, elsewhere: [] });
  const counts = $derived(c ? threadCounts(c.threads) : { open: 0, resolved: 0 });

  // ---------- The reviewer agent's remarks (ESC-490, PR-5) ----------

  /** The review of this pull request by an agent: one of the project's own pull requests. */
  const review = $derived(hostRepo ? null : (app.prReviews[project.id]?.[number] ?? null));
  const notes = $derived(review?.notes ?? []);
  /** The severities shown (all when none is chosen), and the order of the summary. */
  let sevFilter = $state<SeverityFilter>({});
  let noteSort = $state<'severity' | 'place'>('severity');
  const NEXT_SCOPES: NextScope[] = ['both', 'host', 'agent'];
  const savedScope = readPref('prNextScope');
  let nextScope = $state<NextScope>(NEXT_SCOPES.includes(savedScope as NextScope) ? (savedScope as NextScope) : 'both');
  const shownNotes = $derived(notes.filter(shownBy(sevFilter)));
  const sevCounts = $derived(severityCounts(notes));
  const liveCount = $derived(notes.filter(isLive).length);
  const kept = $derived(keptNotes(notes, c ? treeOrder(c.files, '').map((f) => f.path) : []));
  const notePlaced = $derived(c ? placeNotes(c.files, shownNotes) : { under: {}, onFile: {}, stale: {}, top: [] });
  const stops = $derived.by(() => {
    if (!c) return [];
    const ordered = treeOrder(c.files, '');
    return mergeStops(ordered, openOrder(ordered, placed), noteOrder(ordered, notePlaced), nextScope);
  });
  /** Each file's open threads and remarks (under its lines and apart) and its lines that have one. */
  const NO_THREADS = { open: 0, marked: {} as Record<number, boolean> };
  const fileThreads = $derived.by(() => {
    const out: Record<string, { open: number; marked: Record<number, boolean> }> = {};
    for (const [path, lines] of Object.entries(placed.under)) {
      const it = (out[path] ??= { open: 0, marked: {} });
      for (const [i, ts] of Object.entries(lines)) {
        it.marked[Number(i)] = true;
        it.open += ts.filter((th) => th.state !== 'resolved').length;
      }
    }
    for (const [path, ts] of Object.entries(placed.stale)) {
      const it = (out[path] ??= { open: 0, marked: {} });
      it.open += ts.filter((th) => th.state !== 'resolved').length;
    }
    for (const [path, lines] of Object.entries(notePlaced.under)) {
      const it = (out[path] ??= { open: 0, marked: {} });
      for (const [i, ns] of Object.entries(lines)) {
        it.marked[Number(i)] = true;
        it.open += ns.filter(isLive).length;
      }
    }
    for (const group of [notePlaced.onFile, notePlaced.stale])
      for (const [path, ns] of Object.entries(group)) {
        const it = (out[path] ??= { open: 0, marked: {} });
        it.open += ns.filter(isLive).length;
      }
    return out;
  });
  let starting = $state(false);
  let batchOpen = $state(false);
  let batchBusy = $state(false);
  let batchFailure = $state<string | null>(null);

  function setNextScope(s: NextScope) {
    nextScope = s;
    writePref('prNextScope', s);
  }

  function toggleSeverity(s: Severity) {
    sevFilter = { ...sevFilter, [s]: !sevFilter[s] };
  }

  /** « Faire relire par un agent », or read again: the remarks before stay. */
  async function askReview() {
    starting = true;
    try {
      await api.prReviewStart(project.id, number);
    } catch (e) {
      app.toast(String(e), 'error');
    } finally {
      starting = false;
    }
  }

  /** A remark kept, dropped or opened again: null when done, else why not. */
  async function decide(n: ReviewNote, choice: 'open' | 'kept' | 'dropped'): Promise<string | null> {
    try {
      const r = await api.prReviewNote(project.id, number, n.id, choice);
      (app.prReviews[project.id] ??= {})[number] = r;
      return null;
    } catch (e) {
      return String(e);
    }
  }

  /** Remarks published on the host, as the user's comments, in order: null when all went, else why it stopped. */
  async function sendNotes(items: { n: ReviewNote; body?: string }[]): Promise<string | null> {
    let out: Awaited<ReturnType<typeof api.prReviewSend>>;
    try {
      out = await api.prReviewSend(
        project.id,
        number,
        items.map(({ n, body }) => ({ id: n.id, body, onLine: !!c && onItsLine(c.files, n) })),
      );
    } catch (e) {
      return String(e);
    }
    (app.prReviews[project.id] ??= {})[number] = out.review;
    if (out.sent.length) {
      app.toast(t('prs.review.published', { count: out.sent.length, host }), 'ok');
      // Published, each one is the host's thread now: read again to show it.
      await loadContents();
    }
    return out.failure ? writeFailure(out.failure, host, online) : null;
  }

  const publishNote = (n: ReviewNote, body: string) => sendNotes([{ n, body }]);

  async function publishKept() {
    batchBusy = true;
    batchFailure = null;
    const why = await sendNotes(kept.map((n) => ({ n })));
    batchBusy = false;
    if (why) batchFailure = t('prs.review.batch.stopped', { why });
    else batchOpen = false;
  }

  /** A remark of the summary, in the tab « Fichiers » under its line. */
  function showNote(n: ReviewNote) {
    sevFilter = {};
    const files = c?.files ?? [];
    const all = placeNotes(files, notes);
    const at = n.path ? Object.entries(all.under[n.path] ?? {}).find(([, ns]) => ns.some((x) => x.id === n.id)) : undefined;
    show(noteKey(n), n.path && files.some((f) => f.path === n.path) ? n.path : null, at ? Number(at[0]) : -1);
  }
  /** The lines a comment is being written on. */
  let composing = $state<LinePick | null>(null);
  let composeBusy = $state(false);
  let composeFailure = $state<string | null>(null);
  let convBusy = $state(false);
  let convFailure = $state<string | null>(null);
  /** The thread « Suivante » reached last, and what brings it into view. */
  let currentThread = $state<string | null>(null);
  let reveal = $state<{ key: string; path: string | null; index: number; seq: number } | null>(null);

  const keyOf = (target: string) => draftKey(project.id, hostRepo, number, target);

  /** A write the user clicked, on the host: null when done (the pull request read again), else why not. */
  async function send(w: PrWrite): Promise<PrFailure | null> {
    let r: PrRead<boolean>;
    try {
      r = await api.prWrite(project.id, number, w, hostRepo);
    } catch (e) {
      return { kind: 'other', message: String(e) };
    }
    if (r.failure) return r.failure;
    await loadContents();
    return null;
  }

  /** « Suivante » (or the previous one): the next open thread, in the tab « Fichiers ». */
  function next(step: 1 | -1 = 1) {
    const stop = nextStop(stops, currentThread, step);
    if (!stop) return;
    show(stop.key, stop.path, stop.index);
  }

  function show(key: string, path: string | null, index: number) {
    currentThread = key;
    scope = 'pr';
    tab = 'files';
    reveal = { key, path, index, seq: (reveal?.seq ?? 0) + 1 };
  }

  /** A thread of the conversation, in the tab « Fichiers » under its line. */
  function showThread(th: PrThread) {
    const key = threadKey(th);
    const path = th.path ?? null;
    const at = path ? Object.entries(placed.under[path] ?? {}).find(([, ts]) => ts.some((x) => threadKey(x) === key)) : undefined;
    show(key, path && c?.files.some((f) => f.path === path) ? path : null, at ? Number(at[0]) : -1);
  }

  /** « + » on a line: a comment on it, or (Maj+clic) up to it from the line picked. */
  function onComment(f: PrContentsFile, l: PrLine, i: number, extend: boolean) {
    const pick = pickOf(f.path, l, i);
    if (!pick || !writing.lineComment) return;
    const was = composing;
    composing = extend ? extendPick(was, pick) : pick;
    // A range made from a line picked: its draft goes with it.
    if (was && composing !== pick) {
      const from = keyOf(lineTarget(was.path, was.side, was.line));
      const to = keyOf(lineTarget(composing.path, composing.side, composing.line));
      if (from !== to && app.prDrafts[from] && !app.prDrafts[to]) {
        app.prDrafts[to] = app.prDrafts[from];
        delete app.prDrafts[from];
      }
    }
    composeFailure = null;
  }

  async function sendLine(body: string) {
    const p = composing;
    if (!p) return;
    composeBusy = true;
    composeFailure = null;
    const f = await send({ kind: 'lineComment', path: p.path, line: p.line, startLine: p.startLine, side: p.side, body });
    composeBusy = false;
    if (f) {
      composeFailure = writeFailure(f, host, online);
      return;
    }
    delete app.prDrafts[keyOf(lineTarget(p.path, p.side, p.line))];
    composing = null;
  }

  async function sendConversation(body: string) {
    convBusy = true;
    convFailure = null;
    const f = await send({ kind: 'comment', body });
    convBusy = false;
    if (f) convFailure = writeFailure(f, host, online);
    else delete app.prDrafts[keyOf('pr')];
  }

  /** N and P go from an open thread to the next, outside a field. */
  function onKey(e: KeyboardEvent) {
    if (e.ctrlKey || e.metaKey || e.altKey || e.defaultPrevented) return;
    const el = e.target as HTMLElement | null;
    if (el?.closest('input, textarea, select, [contenteditable="true"], [contenteditable=""]')) return;
    if (e.key !== 'n' && e.key !== 'p') return;
    if (!stops.length) return;
    e.preventDefault();
    next(e.key === 'n' ? 1 : -1);
  }

  async function loadContents() {
    const at = { id: project.id, n: number };
    let next: PrRead<PrContents>;
    try {
      next = await api.prContents(at.id, at.n, hostRepo);
    } catch (e) {
      next = { failure: { kind: 'other', message: String(e) } as PrFailure };
    }
    if (project.id !== at.id || number !== at.n) return;
    // A read that fails keeps what was shown, and says why above it.
    if (next.failure && contents?.data) next = { data: contents.data, failure: next.failure };
    contents = next;
    if (next.data && !next.failure) {
      readAt = Date.now();
      app.prWords[at.id] = next.data.word;
    }
  }

  async function loadDetail() {
    const at = { id: project.id, n: number };
    let next: PrRead<PrDetail>;
    try {
      next = await api.prDetail(at.id, at.n, hostRepo);
    } catch (e) {
      next = { failure: { kind: 'other', message: String(e) } as PrFailure };
    }
    if (project.id !== at.id || number !== at.n) return;
    if (next.failure && detail?.data) next = { data: detail.data, failure: next.failure };
    detail = next;
  }

  async function load() {
    if (reading) return;
    reading = true;
    try {
      await Promise.all([loadContents(), loadDetail()]);
    } finally {
      reading = false;
    }
  }

  // Shown: read. Another pull request: from the start.
  $effect(() => {
    void project.id;
    void number;
    void hostRepo;
    untrack(() => {
      contents = null;
      detail = null;
      readAt = null;
      opened = {};
      unfolded = {};
      scope = 'pr';
      composing = null;
      composeFailure = null;
      convFailure = null;
      currentThread = null;
      reveal = null;
      sevFilter = {};
      batchOpen = false;
      batchFailure = null;
      void load();
      if (!hostRepo) app.loadPrReviews(project.id).catch(() => {});
    });
  });

  // The host said it changed (the following of the pull requests saw it): read again.
  let seenChanges = -1;
  $effect(() => {
    const n = changes;
    untrack(() => {
      if (seenChanges >= 0 && n !== seenChanges) void load();
      seenChanges = n;
    });
  });

  // A check runs: its checks read again while the window is visible, as the tab « PR et CI » does.
  $effect(() => {
    if (!d || !stillRunning(d.checks)) return;
    const timer = setInterval(() => {
      if (document.visibilityState === 'visible') void loadDetail();
    }, POLL_MS);
    return () => clearInterval(timer);
  });

  async function rerun() {
    const n = await app.run(api.prRerun(project.id, number, hostRepo));
    if (n === undefined) return;
    app.toast(n ? t('prs.view.ci.rerunDone', { count: n }) : t('prs.view.ci.rerunNone'), n ? 'ok' : 'info');
    void loadDetail();
  }

  function openOnHost() {
    if (pr) void openPrPage({ url: pr.url, number: pr.number, word });
  }

  function onTabKey(e: KeyboardEvent, i: number) {
    const live: PrTab[] = [...PR_TABS];
    const at = live.indexOf(PR_TABS[i]);
    let next = -1;
    if (e.key === 'ArrowRight') next = (at + 1) % live.length;
    else if (e.key === 'ArrowLeft') next = (at - 1 + live.length) % live.length;
    if (next < 0) return;
    e.preventDefault();
    tab = live[next];
    document.getElementById(`pr-tab-${tab}`)?.focus();
  }

  const ago = (seconds?: number) => (seconds ? fAgo(seconds, app.now) : '');
</script>

<svelte:window
  ononline={() => {
    online = true;
    if (contents?.failure) void load();
  }}
  onoffline={() => (online = false)}
/>

{#snippet avatar(name: string, agentic = false)}
  <span class="av" class:agentic aria-hidden="true">{initials(name)}</span>
{/snippet}

{#snippet failed(f: PrFailure, banner: boolean)}
  <div class="err" class:banner role="alert">
    <Icon name="info" size={15} />
    <div class="err-t">
      <b>{pageFailure(f, word, number, online)}</b>
      {#if f.kind === 'refused' || f.kind === 'rateLimit' || f.kind === 'unreachable'}<span class="why">{f.message}</span>{/if}
    </div>
    {#if f.kind === 'refused' || f.settings}
      <button class="btn" onclick={() => (app.modal = { kind: 'settings', page: 'services' })}>{t('prs.view.noPr.settings')}</button>
    {/if}
    {#if f.kind !== 'unavailable'}
      <button class="btn" onclick={() => void load()}><Icon name="restart" size={14} />{t('prs.view.retry')}</button>
    {/if}
  </div>
{/snippet}

{#snippet message(name: string, body: string, when?: number, place?: string)}
  <div class="msg">
    {@render avatar(name)}
    <div class="who">
      <b>{name}</b>
      {#if place}<span class="mono place">{place}</span>{/if}
      {#if when}<span class="t">{ago(when)}</span>{/if}
    </div>
    <div class="bd third"><Markdown text={body} /></div>
  </div>
{/snippet}

{#snippet lineThread(th: PrThread, showPlace: boolean)}
  <PrThreadView
    thread={th}
    {host}
    {writing}
    replyKey={keyOf(`reply:${threadKey(th)}`)}
    {send}
    {showPlace}
    current={currentThread === threadKey(th)}
  />
{/snippet}

{#snippet lineComposer(p: LinePick)}
  <div class="new-comment">
    <p class="nc-head"><Icon name="edit" size={12} />{t('prs.page.threads.commentOn', { place: pickPlace(p) })}</p>
    {#if composeFailure}<p class="fail" role="alert"><Icon name="info" size={13} />{composeFailure}</p>{/if}
    <PrComposer
      draftKey={keyOf(lineTarget(p.path, p.side, p.line))}
      label={t('prs.page.threads.commentOn', { place: `${p.path} · ${pickPlace(p)}` })}
      submit={t('prs.page.threads.send')}
      {host}
      busy={composeBusy}
      autofocus
      onSend={sendLine}
      onCancel={() => (composing = null)}
    />
  </div>
{/snippet}

{#snippet agentNote(n: ReviewNote, showPlace: boolean)}
  <PrAgentNote
    note={n}
    {host}
    {word}
    onLine={!!c && onItsLine(c.files, n)}
    current={currentThread === noteKey(n)}
    {showPlace}
    {decide}
    publish={publishNote}
  />
{/snippet}

<!-- Under a line of the tab « Fichiers » (ESC-474): its threads, the agent's remarks (ESC-490), then the comment being written on it. -->
{#snippet under(file: PrContentsFile, _l: PrLine, i: number)}
  {@const here = placed.under[file.path]?.[i]}
  {@const mine = notePlaced.under[file.path]?.[i]}
  {@const writingHere = !!composing && composing.path === file.path && composing.index === i}
  {#if here || mine || writingHere}
    <div class="under">
      {#each here ?? [] as th (threadKey(th))}{@render lineThread(th, false)}{/each}
      {#each mine ?? [] as n (n.id)}{@render agentNote(n, false)}{/each}
      {#if writingHere && composing}{@render lineComposer(composing)}{/if}
    </div>
  {/if}
{/snippet}

<!-- Above a file's lines: the agent's remarks on the whole file, then its threads and remarks whose line is no longer in the diff. -->
{#snippet above(file: PrContentsFile)}
  {@const apart = placed.stale[file.path]}
  {@const onFile = notePlaced.onFile[file.path]}
  {@const staleNotes = notePlaced.stale[file.path]}
  {#if onFile?.length}
    <div class="apart" role="group" aria-label={t('prs.review.onFile')}>
      <p class="apart-h"><Icon name="file" size={12} />{t('prs.review.onFile')}</p>
      {#each onFile as n (n.id)}{@render agentNote(n, false)}{/each}
    </div>
  {/if}
  {#if apart?.length}
    <div class="apart" role="group" aria-label={t('prs.page.threads.stale', { count: apart.length })}>
      <p class="apart-h"><Icon name="clock" size={12} />{t('prs.page.threads.stale', { count: apart.length })}</p>
      {#each apart as th (threadKey(th))}{@render lineThread(th, true)}{/each}
    </div>
  {/if}
  {#if staleNotes?.length}
    <div class="apart" role="group" aria-label={t('prs.review.stale', { count: staleNotes.length })}>
      <p class="apart-h"><Icon name="clock" size={12} />{t('prs.review.stale', { count: staleNotes.length })}</p>
      {#each staleNotes as n (n.id)}{@render agentNote(n, true)}{/each}
    </div>
  {/if}
{/snippet}

<!-- Above the files: the threads on files the diff does not change, the agent's remarks on the whole pull request. -->
{#snippet head()}
  {#if review && (review.phase === 'reading' || review.phase === 'failed')}{@render reviewState()}{/if}
  {#if notePlaced.top.length}
    <div class="apart top" role="group" aria-label={t('prs.review.elsewhere', { count: notePlaced.top.length, word })}>
      <p class="apart-h"><Icon name="sparkle" size={12} />{t('prs.review.elsewhere', { count: notePlaced.top.length, word })}</p>
      {#each notePlaced.top as n (n.id)}{@render agentNote(n, true)}{/each}
    </div>
  {/if}
  {#if placed.elsewhere.length}
    <div class="apart top" role="group" aria-label={t('prs.page.threads.elsewhere', { count: placed.elsewhere.length })}>
      <p class="apart-h"><Icon name="clock" size={12} />{t('prs.page.threads.elsewhere', { count: placed.elsewhere.length })}</p>
      {#each placed.elsewhere as th (threadKey(th))}{@render lineThread(th, true)}{/each}
    </div>
  {/if}
{/snippet}

<!-- The reading under way, or failed (its remarks before stay). -->
{#snippet reviewState()}
  {#if review?.phase === 'reading'}
    <div class="rv-state" role="status">
      <span class="spinner" aria-hidden="true"></span>
      <div><b>{t('prs.review.reading', { word, number })}</b><span class="muted">{t('prs.review.readingHint')}</span></div>
    </div>
  {:else if review?.phase === 'failed'}
    <div class="rv-state ko" role="alert">
      <Icon name="info" size={15} />
      <div><b>{t('prs.review.failed', { why: review.error ?? '' })}</b></div>
      <button class="btn small" disabled={starting} onclick={askReview}><Icon name="restart" size={13} />{t('prs.review.again')}</button>
    </div>
  {/if}
{/snippet}

<!-- The agent's review in the conversation: what it read, at which commit, its remarks by severity, its view, a link to each. -->
{#snippet reviewSummary()}
  {#if review}
    <section class="rv-sum" aria-label={t('prs.review.summary.label')}>
      <div class="dh">
        <span class="av agentic" aria-hidden="true"><Icon name="sparkle" size={12} /></span>
        <b>{t('prs.review.reviewer')}</b>
        <span class="tag agent-tag" use:tip={t('prs.review.tagHint', { host })}>{t('prs.review.tag')}</span>
        {#if review.headSha}<span class="muted">{t('prs.review.summary.read', { word, sha: shortSha(review.headSha) })}</span>{/if}
        {#if review.readAt}<span class="t">· {ago(Math.floor(review.readAt / 1000))}</span>{/if}
        <span class="muted by">{review.by}</span>
      </div>
      {@render reviewState()}
      <div class="db">
        <p class="sum-counts">
          {t('prs.review.summary.counts', { count: notes.length })}{#each SEVERITIES as s (s)}{#if sevCounts[s]}{' · '}<span
                class="sevc {s}">{sevCounts[s]} {t(`prs.review.severity.${s}`)}</span
              >{/if}{/each}
        </p>
        {#if review.summary.trim()}
          <h4>{t('prs.review.summary.view')}</h4>
          <div class="third"><Markdown text={review.summary} /></div>
        {/if}
        {#if review.leftOut.length}
          <p class="muted small" use:tip={review.leftOut.join('\n')}>{t('prs.review.summary.leftOut', { count: review.leftOut.length })}</p>
        {/if}
        {#if notes.length}
          <div class="sum-bar">
            <h4>{t('prs.review.summary.remarks')}</h4>
            {@render severityChips()}
            <label class="sort"
              ><select bind:value={noteSort} aria-label={t('prs.review.sort')}>
                <option value="severity">{t('prs.review.sorts.severity')}</option>
                <option value="place">{t('prs.review.sorts.place')}</option>
              </select></label
            >
          </div>
          <ul class="sum-list">
            {#each sortNotes(shownNotes, noteSort, c ? treeOrder(c.files, '').map((f) => f.path) : []) as n (n.id)}
              <li data-choice={n.choice}>
                <span class="sev-dot {n.severity}">{t(`prs.review.severity.${n.severity}`)}</span>
                <button class="link go" onclick={() => showNote(n)} aria-label={t('prs.review.summary.go', { place: notePlace(n, word) })}
                  ><span class="mono">{notePlace(n, word)}</span></button
                >
                <span class="ell">{n.body}</span>
                <span class="st">{t(`prs.review.summary.state.${n.choice}`)}</span>
              </li>
            {:else}
              <li class="muted">{t('prs.review.summary.noneShown')}</li>
            {/each}
          </ul>
        {:else if review.phase !== 'reading'}
          <p class="muted">{t('prs.review.summary.none')}</p>
        {/if}
      </div>
    </section>
  {/if}
{/snippet}

{#snippet severityChips()}
  <div class="chips" role="group" aria-label={t('prs.review.filter')}>
    {#each SEVERITIES as s (s)}
      <button
        class="chip {s}"
        aria-pressed={!!sevFilter[s]}
        use:tip={t('prs.review.filterHint', { severity: t(`prs.review.severity.${s}`) })}
        onclick={() => toggleSeverity(s)}>{t(`prs.review.severity.${s}`)}<span class="num">{sevCounts[s]}</span></button
      >
    {/each}
  </div>
{/snippet}

{#snippet threadCard(th: PrThread, key: string)}
  {@const place = threadPlace(th)}
  {#if th.state !== 'open' && !unfolded[key]}
    <button class="thread folded" data-state={th.state} aria-expanded="false" onclick={() => (unfolded[key] = true)}>
      <span class="ts {th.state}"><Icon name={th.state === 'resolved' ? 'check-c' : 'clock'} size={13} />{threadState(th)}</span>
      {#if place}<span class="mono place">{place}</span>{/if}
      <span class="muted">· {t('prs.page.resolvedFold', { count: th.comments.length })}</span>
      <span class="ell muted">{th.comments[0]?.body ?? ''}</span>
    </button>
  {:else}
    <article class="thread" data-state={th.state} aria-label={place || t('prs.page.commented')}>
      <header class="th-head">
        <span class="ts {th.state}"
          ><Icon name={th.state === 'open' ? 'circle' : th.state === 'resolved' ? 'check-c' : 'clock'} size={13} />{threadState(th)}</span
        >
        {#if place}<span class="mono place">{place}</span>{/if}
        {#if th.path && c?.diff && !c.diffFailure}
          <button class="link to-files" onclick={() => showThread(th)}>{t('prs.page.threads.showInFiles')}</button>
        {/if}
        {#if th.state !== 'open'}
          <button class="icon-btn fold" aria-label={threadState(th)} aria-expanded="true" onclick={() => (unfolded[key] = false)}
            ><Icon name="up" size={12} /></button
          >
        {/if}
      </header>
      {#if th.excerpt}<pre class="excerpt mono">{th.excerpt}</pre>{/if}
      {#each th.comments as m (m.id)}
        {@render message(m.author, m.body, m.created)}
      {/each}
    </article>
  {/if}
{/snippet}

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<main class="pr" aria-busy={reading} aria-label={t('prs.page.label', { word, number })} onkeydown={onKey}>
  <header class="hd">
    <button
      class="icon-btn back"
      aria-label={t('prs.page.back', { word })}
      use:tip={t('prs.page.back', { word })}
      onclick={() => app.closePrView(project.id)}><Icon name="left" size={15} /></button
    >
    <div class="tt">
      <div class="t1">
        <h1><span class="n num">#{number}</span>{pr?.title ?? ''}</h1>
        {#if pr}{@const pill = statePill(pr)}<span class="pill-st {pill.tone}"
            ><Icon name={pill.tone === 'merged' ? 'merge' : 'pr'} size={12} />{pill.text}</span
          >{/if}
      </div>
      {#if pr}
        <div class="t2">
          <span class="mono">{pr.head}</span><span aria-hidden="true">→</span><span class="mono">{pr.base}</span>
          {#if ticket}<span aria-hidden="true">·</span>
            <button class="link" onclick={() => app.openBoard(project.id)}>{t('prs.page.ticket', { key: ticket.key })}</button>{/if}
          {#if agent}<span aria-hidden="true">·</span><span>{t('prs.page.byAgent', { agent: agent.name })}</span>
          {:else if pr.author}<span aria-hidden="true">·</span><span>{t('prs.page.by', { author: pr.author })}</span>{/if}
          {#if c?.reading.commits}<span aria-hidden="true">·</span><span>{t('prs.page.commitsCount', { count: c.commits.length })}</span
            >{/if}
        </div>
      {/if}
    </div>
    <div class="right">
      {#if readAt}<span class="read">{t('prs.page.readAt', { time: fTime(readAt) })}</span>{/if}
      <button
        class="icon-btn"
        class:spin={reading}
        aria-label={t('prs.page.refresh', { word })}
        use:tip={t('prs.page.refresh', { word })}
        onclick={() => void load()}><Icon name="restart" size={15} /></button
      >
      {#if !hostRepo && pr?.state === 'open'}
        <button
          class="btn agentic"
          disabled={starting || review?.phase === 'reading'}
          use:tip={review ? t('prs.review.againHint', { word }) : t('prs.review.askHint', { word })}
          onclick={askReview}><Icon name="sparkle" size={14} />{review ? t('prs.review.again') : t('prs.review.ask')}</button
        >
      {/if}
      <!-- The place of « Ta relecture » (PR-7) and of the merge (PR-8). -->
      <button class="btn" disabled={!pr} onclick={openOnHost}><Icon name="ext" size={14} />{t('prs.page.openOnHost', { host })}</button>
    </div>
  </header>

  <div class="tabs">
    <div class="tl" role="tablist" aria-label={t('prs.page.tabs.label', { word })}>
      {#each PR_TABS as id, i (id)}
        <button
          role="tab"
          id="pr-tab-{id}"
          aria-selected={tab === id}
          aria-controls="pr-panel"
          tabindex={tab === id ? 0 : -1}
          aria-label={id === 'files' && tabCount.files ? t('prs.page.tabs.filesViewed', { count: tabCount.files }) : undefined}
          onclick={() => (tab = id)}
          onkeydown={(e) => onTabKey(e, i)}
          >{t(`prs.page.tabs.${id}`)}{#if tabCount[id]}<span class="c num">{tabCount[id]}</span>{/if}</button
        >
      {/each}
    </div>
    {#if (c?.reading.threads && counts.open + counts.resolved > 0) || liveCount}
      <div class="nav-th" role="group" aria-label={t('prs.page.threads.open', { count: counts.open })}>
        <span class="cnt"
          >{t('prs.page.threads.open', { count: counts.open })}{#if counts.resolved}{' · '}{t('prs.page.threads.resolvedCount', {
              count: counts.resolved,
            })}{/if}{#if liveCount}{' · '}<span class="agent-cnt"
              ><Icon name="sparkle" size={11} />{t('prs.review.summary.counts', { count: liveCount })}</span
            >{/if}</span
        >
        {#if stops.length}
          <button
            class="icon-btn"
            aria-label={t('prs.page.threads.previous')}
            use:tip={t('prs.page.threads.previous')}
            onclick={() => next(-1)}><Icon name="up" size={13} /></button
          >
          <button class="btn small" aria-keyshortcuts="N" use:tip={t('prs.page.threads.nextHint')} onclick={() => next(1)}
            >{t('prs.page.threads.next')}<Icon name="down" size={12} /></button
          >
        {/if}
      </div>
    {:else}
      <span class="ro">{t('prs.page.writesOnClick', { host })}</span>
    {/if}
  </div>

  {#if !contents}
    <div class="state" role="status">
      <span class="spinner" aria-hidden="true"></span>
      <p>{t('prs.page.loading', { word, number })}</p>
    </div>
  {:else if !c && failure}
    <div class="state">{@render failed(failure, false)}</div>
  {:else if c}
    <div class="body" class:wide={tab === 'files'}>
      <div class="col" class:flat={tab === 'files'} id="pr-panel" role="tabpanel" aria-labelledby="pr-tab-{tab}">
        {#if failure}{@render failed(failure, true)}{/if}
        {#if missing.length}
          <p class="note" role="note"><Icon name="info" size={14} />{t('prs.page.missing', { host, parts: fList(missing) })}</p>
        {/if}

        {#if tab === 'conversation'}
          <div class="conv">
            {#if c.reading.description}
              <section class="desc" aria-label={t('prs.page.thirdParty')}>
                <div class="dh">
                  {@render avatar(c.pr.author ?? '?')}
                  <b>{c.pr.author ?? ''}</b>
                  <span class="tag third-tag" use:tip={t('prs.page.thirdPartyHint', { host, word })}
                    ><Icon name="shield" size={11} />{t('prs.page.thirdParty')}</span
                  >
                  <span class="muted"
                    >{c.pr.created ? t('prs.page.opened', { word, ago: ago(c.pr.created) }) : t('prs.page.openedNoDate', { word })}</span
                  >
                </div>
                <div class="db third">
                  {#if c.description?.body.trim()}
                    <Markdown text={c.description.body} />
                  {:else}
                    <p class="muted">{t('prs.page.noDescription')}</p>
                  {/if}
                </div>
              </section>
            {/if}

            {@render reviewSummary()}

            {#each items as it (it.key)}
              {#if it.kind === 'commits'}
                <div class="ev">
                  <span class="gl"><Icon name="push" size={12} /></span>
                  <span><b>{it.author}</b> {t('prs.page.pushed', { count: it.commits.length })}</span>
                  {#each it.commits.slice(0, 3) as k (k.sha)}<span class="mono sha">{shortSha(k.sha)}</span>{/each}
                  {#if it.commits.length === 1}<span class="ell">{subjectOf(it.commits[0].message)}</span>{/if}
                  {#if it.at}<span class="t">· {ago(it.at)}</span>{/if}
                </div>
              {:else if it.kind === 'review'}
                <div class="review" data-state={it.review.state}>
                  <div class="ev">
                    <span class="gl {it.review.state}"
                      ><Icon
                        name={it.review.state === 'approved' ? 'check' : it.review.state === 'changesRequested' ? 'close' : 'read'}
                        size={12}
                      /></span
                    >
                    <span><b>{it.review.author}</b> {t(`prs.page.review.${it.review.state}`)}</span>
                  </div>
                  {#if it.review.body.trim()}
                    <div class="thread">{@render message(it.review.author, it.review.body)}</div>
                  {/if}
                </div>
              {:else if it.kind === 'comment'}
                <div class="thread">
                  {#each it.thread.comments as m (m.id)}{@render message(m.author, m.body, m.created)}{/each}
                </div>
              {:else}
                {@render threadCard(it.thread, it.key)}
              {/if}
            {:else}
              {#if c.reading.threads}<p class="muted empty">{t('prs.page.emptyConversation')}</p>{/if}
            {/each}
            {#if writing.comment}
              <section class="thread conv-new" aria-label={t('prs.page.threads.commentPr', { word })}>
                {#if convFailure}<p class="fail" role="alert"><Icon name="info" size={13} />{convFailure}</p>{/if}
                <PrComposer
                  draftKey={keyOf('pr')}
                  label={t('prs.page.threads.commentPr', { word })}
                  submit={t('prs.page.threads.send')}
                  {host}
                  busy={convBusy}
                  onSend={sendConversation}
                />
              </section>
            {/if}
          </div>
        {:else if tab === 'commits'}
          {#if !c.commits.length}
            <p class="muted empty">{t('prs.page.commits.empty')}</p>
          {:else}
            <ul class="commits" aria-label={t('prs.page.commits.label', { word })}>
              {#each c.commits as k (k.sha)}
                {@const more = hasBody(k.message)}
                <li class="commit">
                  <button
                    class="row"
                    aria-expanded={more ? !!opened[k.sha] : undefined}
                    aria-label={more ? (opened[k.sha] ? t('prs.page.commits.hideMessage') : t('prs.page.commits.showMessage')) : undefined}
                    disabled={!more}
                    onclick={() => (opened[k.sha] = !opened[k.sha])}
                  >
                    {@render avatar(k.author)}
                    <span class="ell subj">{subjectOf(k.message)}</span>
                    {#if more}<Icon name={opened[k.sha] ? 'up' : 'down'} size={12} />{/if}
                    <span class="by">{k.author}</span>
                    <span class="mono sha">{shortSha(k.sha)}</span>
                    <span class="t">{ago(k.created)}</span>
                  </button>
                  {#if c.diff && !c.diffFailure}
                    <button
                      class="icon-btn files-of"
                      aria-label={t('prs.page.commits.files', { sha: shortSha(k.sha) })}
                      use:tip={t('prs.page.commits.files', { sha: shortSha(k.sha) })}
                      onclick={() => filesOf(k.sha)}><Icon name="file" size={14} /></button
                    >
                  {/if}
                  {#if opened[k.sha]}<pre class="whole third">{k.message}</pre>{/if}
                </li>
              {/each}
            </ul>
          {/if}
        {:else if tab === 'files'}
          {#if review && (notes.length || review.phase !== 'ready')}
            <div class="agent-bar" role="region" aria-label={t('prs.review.head')}>
              <span class="ab-h"><Icon name="sparkle" size={13} />{t('prs.review.head')}</span>
              {@render severityChips()}
              {#if kept.length}
                <button class="btn small primary" disabled={batchBusy} onclick={() => ((batchOpen = true), (batchFailure = null))}
                  ><Icon name="push" size={12} />{t('prs.review.batch.button', { count: kept.length })}</button
                >
              {/if}
              <label class="scope"
                ><span class="muted">{t('prs.page.threads.next')}</span>
                <select
                  aria-label={t('prs.review.nextScope')}
                  value={nextScope}
                  onchange={(e) => setNextScope((e.currentTarget as HTMLSelectElement).value as NextScope)}
                >
                  {#each NEXT_SCOPES as s (s)}<option value={s}>{t(`prs.review.nextScopes.${s}`, { host })}</option>{/each}
                </select></label
              >
            </div>
            {#if batchOpen && kept.length}
              <div class="batch" role="dialog" aria-modal="false" aria-label={t('prs.review.batch.title', { count: kept.length })}>
                <p class="b-t">{t('prs.review.batch.title', { count: kept.length })}</p>
                <p class="muted">{t('prs.review.batch.body', { host })}</p>
                <ol>
                  {#each kept as n (n.id)}
                    <li>
                      <span class="sev-dot {n.severity}">{t(`prs.review.severity.${n.severity}`)}</span>
                      <span class="mono">{notePlace(n, word)}</span>
                      <span class="ell">{n.body}</span>
                    </li>
                  {/each}
                </ol>
                {#if batchFailure}<p class="fail" role="alert"><Icon name="info" size={13} />{batchFailure}</p>{/if}
                <div class="b-acts">
                  <button class="btn small primary" disabled={batchBusy} onclick={publishKept}
                    >{batchBusy ? t('prs.review.publishing') : t('prs.review.batch.confirm', { count: kept.length })}</button
                  >
                  <button class="btn small" disabled={batchBusy} onclick={() => (batchOpen = false)}>{t('prs.review.cancel')}</button>
                </div>
              </div>
            {/if}
          {/if}
          <PrFiles
            projectId={project.id}
            {number}
            {hostRepo}
            contents={c}
            bind:scope
            {source}
            {under}
            {above}
            {head}
            threads={(f) => fileThreads[f.path] ?? NO_THREADS}
            {reveal}
            onComment={writing.lineComment ? onComment : undefined}
          />
        {:else if tab === 'checks'}
          {#if !detail}
            <div class="state" role="status"><span class="spinner" aria-hidden="true"></span></div>
          {:else if !d && detail.failure}
            {@render failed(detail.failure, false)}
          {:else if d}
            {#if detail.failure}{@render failed(detail.failure, true)}{/if}
            <CiChecks
              checks={d.checks}
              jobLogs={d.can.jobLogs}
              canRerun={d.can.rerun && c.pr.state === 'open'}
              loadLog={(job) => api.prLog(project.id, number, job, hostRepo)}
              onRerun={rerun}
              titleId="pr-view-ci"
              tall
            />
          {/if}
        {/if}
      </div>

      <aside class="aside" class:gone={tab === 'files'}>
        <section class="sec">
          <h3>{t('prs.page.aside.reviewers')}</h3>
          {#each people as r (r.name)}
            <div class="rvr">
              {@render avatar(r.name)}<span class="ell">{r.name}</span>
              <span class="st {r.state}">{t(`prs.page.aside.reviewer.${r.state}`)}</span>
            </div>
          {:else}
            <p class="muted">{t('prs.page.aside.noReviewer')}</p>
          {/each}
        </section>
        {#if c.reading.description}
          <section class="sec">
            <h3>{t('prs.page.aside.labels')}</h3>
            {#if c.description?.labels?.length}
              <div class="lbls">
                {#each c.description.labels as l (l)}<span class="lbl"><i></i>{l}</span>{/each}
              </div>
            {:else}
              <p class="muted">{t('prs.page.aside.noLabel')}</p>
            {/if}
          </section>
        {/if}
        <section class="sec">
          <h3>{t('prs.page.aside.about', { word })}</h3>
          <div class="kv"><Icon name="pr" size={13} />{c.pr.draft ? t('prs.page.aside.draft') : t('prs.page.aside.ready')}</div>
          <div class="kv" class:ko={c.pr.mergeable === false}>
            <Icon name="branch" size={13} />{c.pr.mergeable === false
              ? t('prs.page.aside.conflict', { base: c.pr.base })
              : c.pr.mergeable
                ? t('prs.page.aside.upToDate', { base: c.pr.base })
                : t('prs.page.aside.mergeUnknown', { base: c.pr.base })}
          </div>
          {#if ticket}
            <div class="kv">
              <Icon name="kanban" size={13} />{t('prs.page.aside.ticketIn', {
                key: ticket.key,
                column: t(`board.columns.${ticket.column}`),
              })}
            </div>
          {/if}
        </section>
        <!-- The place of the box « prête à merger ? » (PR-8). -->
      </aside>
    </div>
  {/if}
</main>

<style>
  .pr {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    container-type: inline-size;
  }
  .hd {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px 10px 12px;
  }
  .back {
    color: var(--muted);
  }
  .tt {
    display: grid;
    gap: 3px;
    min-width: 0;
  }
  .t1 {
    display: flex;
    align-items: center;
    gap: 9px;
    min-width: 0;
  }
  h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 620;
    letter-spacing: -0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  h1 .n {
    color: var(--muted);
    font-weight: 500;
    margin-right: 5px;
  }
  .pill-st {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 22px;
    padding: 0 8px;
    border-radius: 6px;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--muted);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .pill-st.open {
    color: var(--run);
    box-shadow: inset 0 0 0 1px oklch(from var(--run) l c h / 0.4);
    background: oklch(from var(--run) l c h / 0.07);
  }
  .pill-st.merged {
    color: var(--brain);
    box-shadow: inset 0 0 0 1px oklch(from var(--brain) l c h / 0.4);
  }
  .pill-st.closed {
    color: var(--del);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.4);
  }
  .t2 {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
  }
  .t2 .mono {
    color: var(--soft);
  }
  .link {
    all: unset;
    cursor: pointer;
    color: var(--muted);
  }
  .link:hover,
  .link:focus-visible {
    color: var(--text);
    text-decoration: underline;
  }
  .right {
    margin-left: auto;
    flex: none;
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .read {
    color: var(--dim);
    font-size: 12px;
    white-space: nowrap;
  }
  .spin :global(svg) {
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .tabs {
    flex: none;
    display: flex;
    gap: 2px;
    height: 38px;
    padding: 0 14px;
    border-bottom: 1px solid var(--line);
    align-items: stretch;
  }
  .tabs [role='tab'] {
    all: unset;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 0 10px;
    color: var(--muted);
    font-weight: 550;
    font-size: 12.5px;
    box-shadow: inset 0 -2px 0 transparent;
    cursor: pointer;
  }
  .tabs [role='tab']:hover {
    color: var(--soft);
  }
  .tabs [role='tab']:focus-visible {
    outline: 2px solid var(--focus, var(--brain));
    outline-offset: -4px;
  }
  .tabs [aria-selected='true'] {
    color: var(--text);
    box-shadow: inset 0 -2px 0 var(--text);
  }
  .tabs .c {
    color: var(--dim);
    font-size: 11.5px;
    font-weight: 500;
  }
  .tl {
    flex: none;
    display: flex;
    gap: 2px;
    align-items: stretch;
    min-width: 0;
  }
  /* Its count gives way to the tabs: cut, never over them. */
  .nav-th {
    margin-left: auto;
    min-width: 0;
    padding-left: 10px;
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--soft);
    white-space: nowrap;
  }
  .nav-th .cnt {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .btn.small {
    height: 26px;
    padding: 0 9px;
    font-size: 12px;
  }
  /* Under a line of the tab « Fichiers »: kept in the width of the diff's box while its lines scroll sideways. */
  .under {
    position: sticky;
    left: 0;
    width: var(--under-w, 100%);
    box-sizing: border-box;
    padding: 2px 0;
    font-family: var(--font);
    line-height: 1.5;
    white-space: normal;
  }
  .apart {
    display: grid;
    gap: 2px;
    padding: 6px 0 4px;
    border-bottom: 1px solid var(--line);
    font-family: var(--font);
  }
  .apart.top {
    margin-bottom: 10px;
    border: none;
    border-radius: 10px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    background: var(--well);
  }
  .apart-h {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 12px;
    color: var(--dim);
    font-size: 11.5px;
    font-weight: 600;
  }
  .new-comment {
    margin: 6px 12px 8px 44px;
    border-radius: 10px;
    background: var(--raise);
    box-shadow:
      inset 0 0 0 1px var(--brain),
      0 0 0 1px oklch(from var(--brain) l c h / 0.3);
  }
  .nc-head {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    padding: 7px 12px 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--brain);
  }
  .fail {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin: 8px 12px 0;
    padding: 6px 9px;
    border-radius: 8px;
    font-size: 12px;
    color: var(--del);
    background: oklch(from var(--del) l c h / 0.07);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.35);
  }
  .to-files {
    font-size: 11.5px;
  }
  .ro {
    margin-left: auto;
    align-self: center;
    color: var(--dim);
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 268px;
  }
  .col {
    min-width: 0;
    overflow: auto;
    padding: 12px 14px 18px 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .body.wide {
    grid-template-columns: minmax(0, 1fr);
  }
  .aside.gone {
    display: none;
  }
  .col.flat {
    padding: 0;
    overflow: hidden;
    gap: 0;
  }
  .col.flat > :global(.err.banner),
  .col.flat > .note {
    margin: 10px 14px 0;
  }
  .commit {
    position: relative;
  }
  .files-of {
    position: absolute;
    top: 5px;
    right: 6px;
  }
  .aside {
    border-left: 1px solid var(--line);
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    font-size: 12.5px;
    overflow: auto;
  }
  @container (max-width: 860px) {
    .body {
      grid-template-columns: minmax(0, 1fr);
    }
    .aside {
      display: none;
    }
    .ro,
    .read {
      display: none;
    }
  }
  .aside h3 {
    margin: 0 0 7px;
    font-size: 11.5px;
    font-weight: 600;
    color: var(--muted);
  }
  .sec {
    padding-bottom: 14px;
    border-bottom: 1px solid var(--line);
  }
  .sec:last-of-type {
    border-bottom: 0;
  }
  .sec p {
    margin: 0;
  }
  .rvr {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 26px;
    min-width: 0;
  }
  .rvr .st {
    margin-left: auto;
    flex: none;
    color: var(--muted);
    font-size: 12px;
  }
  .rvr .st.approved {
    color: var(--run);
  }
  .rvr .st.changesRequested {
    color: var(--del);
  }
  .rvr .st.requested {
    color: var(--wait);
  }
  .lbls {
    display: flex;
    gap: 5px;
    flex-wrap: wrap;
  }
  .lbl {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 22px;
    padding: 0 8px;
    border-radius: 11px;
    font-size: 11.5px;
    font-weight: 550;
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .lbl i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--brain);
  }
  .kv {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 24px;
    color: var(--soft);
  }
  .kv.ko {
    color: var(--del);
  }
  .av {
    flex: none;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    font-size: 9.5px;
    font-weight: 700;
    color: var(--soft);
    background: var(--raise-2);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .conv {
    display: grid;
    gap: 12px;
    max-width: 820px;
  }
  .desc {
    border-radius: 10px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    overflow: hidden;
  }
  .dh {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 12px;
    background: var(--raise);
    border-bottom: 1px solid var(--line);
    font-size: 12.5px;
    white-space: nowrap;
    min-width: 0;
  }
  .db {
    padding: 12px 14px;
    color: var(--text-ai);
  }
  .tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 10.5px;
    font-weight: 650;
    padding: 1px 6px;
    border-radius: 4px;
    color: var(--wait);
    background: var(--wait-bg);
  }
  .ev {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-left: 10px;
    color: var(--muted);
    font-size: 12.5px;
    min-width: 0;
    white-space: nowrap;
  }
  .ev b {
    color: var(--text);
    font-weight: 600;
  }
  .gl {
    flex: none;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
  }
  .gl.approved {
    color: var(--run);
  }
  .gl.changesRequested {
    color: var(--del);
  }
  .sha {
    color: var(--soft);
    font-size: 11.5px;
  }
  .t {
    color: var(--muted);
    font-size: 11.5px;
    white-space: nowrap;
  }
  .ell {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .review {
    display: grid;
    gap: 8px;
  }
  .thread {
    border-radius: 10px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
    overflow: hidden;
    min-width: 0;
  }
  .th-head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px 7px 12px;
    border-bottom: 1px solid var(--line);
    font-size: 12px;
  }
  .fold {
    margin-left: auto;
  }
  .ts {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-weight: 600;
    font-size: 11.5px;
    color: var(--wait);
  }
  .ts.resolved {
    color: var(--run);
  }
  .ts.outdated {
    color: var(--dim);
  }
  .place {
    color: var(--soft);
    font-size: 11.5px;
  }
  .excerpt {
    margin: 0;
    padding: 6px 12px;
    background: var(--well);
    border-bottom: 1px solid var(--line);
    font-size: 11.5px;
    white-space: pre-wrap;
    overflow-x: auto;
    color: var(--soft);
  }
  .thread.folded {
    all: unset;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-radius: 10px;
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--muted);
    font-size: 12px;
    cursor: pointer;
    min-width: 0;
  }
  .thread.folded:hover {
    background: var(--raise);
  }
  .thread.folded:focus-visible {
    outline: 2px solid var(--brain);
  }
  .msg {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr);
    gap: 3px 9px;
    padding: 10px 12px 8px;
  }
  .msg + .msg {
    border-top: 1px solid var(--line);
  }
  .who {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    white-space: nowrap;
  }
  .who b {
    font-weight: 620;
  }
  .bd {
    grid-column: 2;
    color: var(--text-ai);
    min-width: 0;
  }
  .third :global(p:first-child) {
    margin-top: 0;
  }
  .third :global(p:last-child) {
    margin-bottom: 0;
  }
  .muted {
    color: var(--muted);
  }
  .empty {
    margin: 4px 0;
  }
  .note {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 8px 12px;
    border-radius: 8px;
    color: var(--soft);
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    font-size: 12.5px;
  }
  .commits {
    list-style: none;
    margin: 0;
    padding: 0;
    border-radius: 10px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    overflow: hidden;
    max-width: 980px;
  }
  .commit + .commit {
    border-top: 1px solid var(--line);
  }
  .commit .row {
    all: unset;
    box-sizing: border-box;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 9px;
    height: 34px;
    padding: 0 42px 0 12px;
    font-size: 12.5px;
    cursor: pointer;
  }
  .commit .row:disabled {
    cursor: default;
  }
  .commit .row:not(:disabled):hover {
    background: var(--raise);
  }
  .commit .row:focus-visible {
    outline: 2px solid var(--brain);
    outline-offset: -2px;
  }
  .subj {
    flex: 1;
  }
  .by {
    color: var(--muted);
    white-space: nowrap;
  }
  .whole {
    margin: 0;
    padding: 8px 12px 10px 43px;
    font-family: var(--mono);
    font-size: 12px;
    white-space: pre-wrap;
    color: var(--soft);
    background: var(--well);
  }
  .state {
    flex: 1;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 10px;
    color: var(--muted);
    padding: 24px;
  }
  .err {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 14px;
    border-radius: 10px;
    color: var(--del);
    background: oklch(from var(--del) l c h / 0.07);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.35);
    max-width: 620px;
  }
  .err-t {
    display: grid;
    gap: 2px;
    margin-right: auto;
  }
  .err-t b {
    color: var(--text);
    font-weight: 600;
  }
  .why {
    color: var(--muted);
    font-size: 12px;
  }
  /* The reviewer agent (ESC-490): its colour is the brain's, its tag and its icon say it without the colour. */
  .btn.agentic {
    color: var(--brain);
    box-shadow: inset 0 0 0 1px oklch(from var(--brain) l c h / 0.45);
  }
  .agent-cnt {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--brain);
  }
  .av.agentic {
    color: var(--brain);
    background: oklch(from var(--brain) l c h / 0.14);
    box-shadow: inset 0 0 0 1px oklch(from var(--brain) l c h / 0.45);
  }
  .agent-tag {
    color: var(--brain);
    background: oklch(from var(--brain) l c h / 0.12);
  }
  .agent-bar {
    flex: none;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px 10px;
    padding: 7px 14px;
    border-bottom: 1px solid var(--line);
    font-size: 12px;
  }
  .ab-h {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--brain);
    font-weight: 600;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .chip {
    all: unset;
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 22px;
    padding: 0 8px;
    border-radius: 11px;
    font-size: 11.5px;
    font-weight: 550;
    color: var(--soft);
    box-shadow: inset 0 0 0 1px var(--line-2);
    cursor: pointer;
  }
  .chip .num {
    color: var(--muted);
  }
  .chip:hover {
    background: var(--raise);
  }
  .chip:focus-visible {
    outline: 2px solid var(--brain);
  }
  .chip[aria-pressed='true'] {
    color: var(--text);
    background: var(--raise-2);
    box-shadow: inset 0 0 0 1px var(--text);
  }
  .chip.security {
    color: var(--del);
  }
  .chip.bug {
    color: var(--wait);
  }
  .scope {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .scope select,
  .sort select {
    height: 24px;
    font: inherit;
    font-size: 12px;
    color: var(--text);
    background: var(--raise);
    border: none;
    border-radius: 6px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    padding: 0 6px;
  }
  .batch {
    flex: none;
    margin: 8px 14px 4px;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--raise);
    box-shadow:
      inset 0 0 0 1px var(--brain),
      0 6px 18px oklch(0 0 0 / 0.18);
    display: grid;
    gap: 6px;
    font-size: 12.5px;
  }
  .batch p {
    margin: 0;
  }
  .b-t {
    font-weight: 620;
  }
  .batch ol {
    margin: 0;
    padding-left: 20px;
    display: grid;
    gap: 3px;
    max-height: 180px;
    overflow: auto;
  }
  .batch li,
  .sum-list li {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .b-acts {
    display: flex;
    gap: 6px;
  }
  .sev-dot {
    flex: none;
    font-size: 10.5px;
    font-weight: 650;
    padding: 1px 6px;
    border-radius: 4px;
    color: var(--muted);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .sev-dot.security {
    color: var(--del);
    background: oklch(from var(--del) l c h / 0.1);
    box-shadow: none;
  }
  .sev-dot.bug {
    color: var(--wait);
    background: var(--wait-bg);
    box-shadow: none;
  }
  .rv-sum {
    border-radius: 10px;
    box-shadow:
      inset 3px 0 0 var(--brain),
      inset 0 0 0 1px oklch(from var(--brain) l c h / 0.45);
    overflow: hidden;
  }
  .rv-sum .by {
    margin-left: auto;
    font-size: 11.5px;
  }
  .rv-sum h4 {
    margin: 10px 0 4px;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
  .sum-counts {
    margin: 0;
    font-size: 12.5px;
    color: var(--soft);
  }
  .sevc.security {
    color: var(--del);
  }
  .sevc.bug {
    color: var(--wait);
  }
  .sum-bar {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  .sum-bar h4 {
    margin-right: 4px;
  }
  .sort {
    margin-left: auto;
  }
  .sum-list {
    list-style: none;
    margin: 6px 0 0;
    padding: 0;
    display: grid;
    gap: 4px;
    font-size: 12.5px;
  }
  .sum-list li[data-choice='dropped'],
  .sum-list li[data-choice='published'] {
    color: var(--muted);
  }
  .sum-list .st {
    margin-left: auto;
    flex: none;
    color: var(--muted);
    font-size: 11.5px;
  }
  .go {
    flex: none;
    color: var(--soft);
    font-size: 12px;
  }
  .small {
    font-size: 12px;
  }
  .rv-state {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 10px 12px;
    padding: 9px 12px;
    border-radius: 8px;
    font-size: 12.5px;
    background: oklch(from var(--brain) l c h / 0.07);
    box-shadow: inset 0 0 0 1px oklch(from var(--brain) l c h / 0.35);
  }
  .rv-state > div {
    display: grid;
    gap: 2px;
    margin-right: auto;
  }
  .rv-state.ko {
    color: var(--del);
    background: oklch(from var(--del) l c h / 0.07);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.35);
  }
  .rv-state.ko b {
    color: var(--text);
  }
</style>
