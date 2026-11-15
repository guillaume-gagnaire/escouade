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
  import { openPrPage } from '../../lib/prs';
  import { POLL_MS, stillRunning } from '../../lib/prview';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { PrContents, PrDetail, PrFailure, PrRead, PrThread, Project, PullRequest } from '../../lib/types';
  import CiChecks from '../CiChecks.svelte';
  import Icon from '../Icon.svelte';
  import Markdown from '../conv/Markdown.svelte';

  // The view of a pull request (ESC-457, PR-2 of the PR in Escouade, direction A of design/pr-2-3): in the work area, in
  // place of the list « Pull requests », in reading only. Its header (number, title, state, branches, ticket, author,
  // « Ouvrir sur <hébergeur> »), its tabs Conversation (the description, a third party's, then the timeline of the commits,
  // comments, threads and reviews), Commits, Contrôles (the tab « PR et CI »'s checks, their log and « Relancer les
  // échecs ») and Fichiers (PR-3's, greyed). The aside says its reviewers, its labels and where it stands; the merge box
  // (PR-8) will go at its foot. Read when shown and again when the host says it changed (`prChanged`).
  let {
    project,
    number,
    hostRepo,
    listed = null,
  }: { project: Project; number: number; hostRepo?: string; listed?: PullRequest | null } = $props();

  let contents = $state<PrRead<PrContents> | null>(null);
  let detail = $state<PrRead<PrDetail> | null>(null);
  let reading = $state(false);
  let readAt = $state<number | null>(null);
  let tab = $state<PrTab>('conversation');
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
    files: c?.reading.files ? String(c.files.length) : '',
  });

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
      void load();
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
    const live: PrTab[] = PR_TABS.filter((x) => x !== 'files');
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

<main class="pr" aria-busy={reading} aria-label={t('prs.page.label', { word, number })}>
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
      <!-- The place of « Ta relecture » (PR-7) and of the merge (PR-8). -->
      <button class="btn" disabled={!pr} onclick={openOnHost}><Icon name="ext" size={14} />{t('prs.page.openOnHost', { host })}</button>
    </div>
  </header>

  <div class="tabs" role="tablist" aria-label={t('prs.page.tabs.label', { word })}>
    {#each PR_TABS as id, i (id)}
      {@const off = id === 'files'}
      <button
        role="tab"
        id="pr-tab-{id}"
        aria-selected={tab === id}
        aria-controls="pr-panel"
        aria-disabled={off || undefined}
        tabindex={tab === id ? 0 : -1}
        class:off
        use:tip={off ? t('prs.page.tabs.filesSoon', { word }) : null}
        onclick={() => !off && (tab = id)}
        onkeydown={(e) => onTabKey(e, i)}
        >{t(`prs.page.tabs.${id}`)}{#if tabCount[id]}<span class="c num">{tabCount[id]}</span>{/if}</button
      >
    {/each}
    <span class="ro">{t('prs.page.readOnly')}</span>
  </div>

  {#if !contents}
    <div class="state" role="status">
      <span class="spinner" aria-hidden="true"></span>
      <p>{t('prs.page.loading', { word, number })}</p>
    </div>
  {:else if !c && failure}
    <div class="state">{@render failed(failure, false)}</div>
  {:else if c}
    <div class="body">
      <div class="col" id="pr-panel" role="tabpanel" aria-labelledby="pr-tab-{tab}">
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
                  {#if opened[k.sha]}<pre class="whole third">{k.message}</pre>{/if}
                </li>
              {/each}
            </ul>
          {/if}
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

      <aside class="aside">
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
  .tabs .off {
    opacity: 0.45;
    cursor: default;
  }
  .tabs .c {
    color: var(--dim);
    font-size: 11.5px;
    font-weight: 500;
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
    padding: 0 12px;
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
</style>
