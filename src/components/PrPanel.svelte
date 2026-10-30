<script lang="ts">
  import { untrack } from 'svelte';
  import { fAgo } from '../lib/format';
  import { t } from '../lib/i18n';
  import { api } from '../lib/ipc';
  import { openOutside } from '../lib/links';
  import { app } from '../lib/state.svelte';
  import {
    askOpenPr,
    canFixChecks,
    canFixReviews,
    canOpenPr,
    fixPr,
    openPr,
    prIcon,
    prKey,
    prTone,
    prWord,
    refreshPrs,
    stateWord,
  } from '../lib/prs';
  import {
    agentOfPr,
    checkDuration,
    checkPhase,
    ciSummary,
    deployState,
    failureTitle,
    hasFailed,
    phaseIcon,
    reallyFailed,
    POLL_MS,
    sortChecks,
    sortComments,
    stillRunning,
  } from '../lib/prview';
  import type { Agent, NoPr, PrCheck, PrDetail, PrFailure, PrRead, Project, PullRequest } from '../lib/types';
  import Icon from './Icon.svelte';
  import { tip } from '../lib/tip';

  // The side panel's « PR et CI » tab (ESC-305). For the agent open: the pull request of its branch (its others under it),
  // its CI (each check, the log of a failed one, « Relancer les échecs » on a click), its reviews and comments in
  // reading, its deployments; without one, « Ouvrir une PR… » or why not. Without an agent: the project's open pull
  // requests. Read when shown, and again every 20 s while a check runs and the tab stays shown: never in the background.
  let { project, agent }: { project: Project; agent: Agent | null } = $props();

  const failedRead = (e: unknown): PrFailure => ({ kind: 'other', message: String(e) });
  function openSettings() {
    app.modal = { kind: 'settings', tab: 'integrations' };
  }

  // ---------- The agent's pull request ----------
  /** Without an agent: the project's open pull requests, as read. */
  let projectRead = $state<PrRead<{ word: string; prs: PullRequest[] }> | null>(null);
  const word = $derived(agent ? prWord(agent) : (projectRead?.data?.word ?? 'PR'));
  const main = $derived(agent?.prs[0] ?? null);
  const others = $derived(agent?.prs.slice(1) ?? []);
  const mainKey = $derived(agent && main ? `${agent.id}|${prKey(main)}` : null);

  // Its pull requests read again when the tab shows it (at most every 20 s, the backend says).
  // By its id: not again at each change of the agent (its status, its tokens).
  const agentId = $derived(agent?.id ?? null);
  $effect(() => {
    if (agentId) untrack(() => refreshPrs(agent));
  });

  let detail = $state<PrDetail | null>(null);
  let failure = $state<PrFailure | null>(null);
  let seq = 0;
  async function loadDetail(quiet = false) {
    const [a, p] = [agent, main];
    if (!a || !p) return;
    const my = ++seq;
    if (!quiet) [detail, failure] = [null, null];
    let read: PrRead<PrDetail>;
    try {
      read = await api.agentPrDetail(a.id, p.number, p.hostRepo);
    } catch (e) {
      read = { failure: failedRead(e) };
    }
    if (my !== seq) return;
    // A poll that fails keeps what was read, and says why.
    if (read.data || !quiet) detail = read.data ?? null;
    failure = read.failure ?? null;
  }
  $effect(() => {
    const k = mainKey;
    untrack(() => {
      openLog = null;
      logs = {};
      if (k) void loadDetail();
      else [detail, failure] = [null, null];
    });
  });
  // Its CI's word changed when its pull requests were read again: the checks again, quietly.
  const rollup = $derived(main ? `${main.checks}|${main.state}` : '');
  let lastRollup = '';
  $effect(() => {
    const r = rollup;
    untrack(() => {
      if (lastRollup && r && r !== lastRollup && detail) void loadDetail(true);
      lastRollup = r;
    });
  });
  // While a check runs, and only while the tab is shown (this component lives with it) and the window is seen.
  $effect(() => {
    if (!detail || !stillRunning(detail.checks)) return;
    const timer = setInterval(() => {
      if (document.hidden) return;
      void loadDetail(true);
      refreshPrs(agent);
    }, POLL_MS);
    return () => clearInterval(timer);
  });

  const checks = $derived(detail ? sortChecks(detail.checks) : []);

  // The log of a failed check, by its job.
  let openLog = $state<string | null>(null);
  let logs = $state<Record<string, PrRead<string> | 'loading'>>({});
  async function toggleLog(c: PrCheck) {
    const job = c.job;
    if (!job || !agent || !main) return;
    if (openLog === job) {
      openLog = null;
      return;
    }
    openLog = job;
    if (logs[job] && logs[job] !== 'loading' && !(logs[job] as PrRead<string>).failure) return;
    logs[job] = 'loading';
    try {
      logs[job] = await api.agentPrLog(agent.id, main.number, job, main.hostRepo);
    } catch (e) {
      logs[job] = { failure: failedRead(e) };
    }
  }

  let rerunning = $state(false);
  async function rerun() {
    if (!agent || !main) return;
    rerunning = true;
    const n = await app.run(api.agentPrRerun(agent.id, main.number, main.hostRepo));
    rerunning = false;
    if (n === undefined) return;
    app.toast(n ? t('prs.view.ci.rerunDone', { count: n }) : t('prs.view.ci.rerunNone'), n ? 'ok' : 'info');
    void loadDetail(true);
  }

  const reviews = $derived(detail ? detail.reviews.filter((r) => r.state !== 'commented' || r.body.trim()) : []);
  const comments = $derived(detail ? sortComments(detail.comments.filter((c) => c.body.trim())) : []);

  // ---------- Without a pull request: « Ouvrir une PR… », or why not ----------
  let noPr = $state<NoPr | null | undefined>(undefined);
  const askWhy = $derived(agent && !main && !canOpenPr(agent) ? agent.id : null);
  $effect(() => {
    const id = askWhy;
    noPr = undefined;
    if (id)
      api
        .agentNoPr(id)
        .then((r) => askWhy === id && (noPr = r))
        .catch((e) => askWhy === id && (noPr = { reason: String(e), settings: false }));
  });

  // ---------- Without an agent: the project's open pull requests ----------
  async function loadProject() {
    const id = project.id;
    projectRead = null;
    let read: PrRead<{ word: string; prs: PullRequest[] }>;
    try {
      read = await api.projectPrs(id);
    } catch (e) {
      read = { failure: failedRead(e) };
    }
    if (project.id === id) projectRead = read;
  }
  $effect(() => {
    if (agent) return;
    void project.id;
    untrack(() => void loadProject());
  });
  const projectAgents = $derived(Object.values(app.agents).filter((a) => a.projectId === project.id));
</script>

{#snippet failed(f: PrFailure, again: (() => unknown) | null)}
  <div class="state err" role="alert">
    <Icon name="info" size={14} />
    <div class="state-t">
      <b>{failureTitle(f)}</b>
      {#if f.kind === 'refused' || f.kind === 'rateLimit' || f.kind === 'unreachable'}<span class="why">{f.message}</span>{/if}
      {#if f.kind === 'refused' || f.settings || again}
        <span class="acts">
          {#if f.kind === 'refused' || f.settings}
            <button class="btn" onclick={openSettings}>{t('prs.view.noPr.settings')}</button>
          {/if}
          {#if again && f.kind !== 'unavailable'}<button class="btn" onclick={again}>{t('prs.view.retry')}</button>{/if}
        </span>
      {/if}
    </div>
  </div>
{/snippet}

{#snippet prRow(p: PullRequest, w: string, onOpen: () => void, label: string)}
  <li class="row">
    <button class="row-main" aria-label={label} onclick={onOpen}>
      <span class="ic {prTone(p)}"><Icon name={prIcon(p)} size={13} /></span>
      <span class="num mono">#{p.number}</span>
      <span class="ttl">{p.title}</span>
      <span class="meta">
        {stateWord(p)} · {t(`prs.checks.${p.checks ?? 'none'}`)}{#if p.mergeable === false}{' · '}<span class="bad"
            >{t('prs.view.conflict', { base: p.base })}</span
          >{/if}{#if p.created}{' · '}{fAgo(p.created, app.now)}{/if}{#if p.author}{' · '}{t('prs.view.by', { author: p.author })}{/if}
      </span>
    </button>
    <button
      class="icon-btn ext"
      aria-label={t('prs.chipLabel', { word: w, number: p.number, state: stateWord(p) })}
      use:tip={{ text: t('prs.view.openInBrowser'), aria: false }}
      onclick={() => openPr(p)}><Icon name="ext" size={13} /></button
    >
  </li>
{/snippet}

<div class="hosts" data-hosts={agent ? 'agent' : 'project'}>
  {#if agent}
    {#if main}
      {@const p = main}
      <!-- Its pull request: number, title, state, branch → target, conflict, age and author. -->
      <section class="head" aria-label={t('prs.tipHead', { word, number: p.number, title: p.title })}>
        <div class="h1">
          <span class="ic {prTone(p)}"><Icon name={prIcon(p)} size={14} /></span>
          <span class="num mono">#{p.number}</span>
          <span class="st {p.state === 'open' && p.draft ? 'draft' : p.state}">{stateWord(p)}</span>
          <button
            class="icon-btn ext"
            aria-label={t('prs.chipLabel', { word, number: p.number, state: stateWord(p) })}
            use:tip={{ text: t('prs.view.openInBrowser'), aria: false }}
            onclick={() => openPr(p)}><Icon name="ext" size={14} /></button
          >
        </div>
        <b class="title">{p.title}</b>
        <span class="caption mono">{p.head} → {p.base}</span>
        {#if p.mergeable === false}
          <span class="conflict"><Icon name="close" size={12} />{t('prs.view.conflict', { base: p.base })}</span>
        {/if}
        {#if p.created || p.author}
          <span class="caption"
            >{#if p.created}{t('prs.view.opened', {
                ago: fAgo(p.created, app.now),
              })}{/if}{#if p.created && p.author}{' '}{/if}{#if p.author}{t('prs.view.by', { author: p.author })}{/if}</span
          >
        {/if}
        {#if !agent.archived && (canFixChecks(p) || canFixReviews(p))}
          <!-- The corrections of ESC-64: handed to the agent on a click, never by themselves. -->
          <span class="acts">
            {#if canFixChecks(p)}
              <button
                class="btn"
                use:tip={t('prs.fixChecksHint', { agent: agent.name, word, number: p.number })}
                onclick={() => agent && fixPr(agent, p, 'checks')}>{t('prs.fixChecks')}</button
              >
            {/if}
            {#if canFixReviews(p)}
              <button
                class="btn"
                use:tip={t('prs.fixReviewsHint', { agent: agent.name, word, number: p.number })}
                onclick={() => agent && fixPr(agent, p, 'reviews')}>{t('prs.fixReviews')}</button
              >
            {/if}
          </span>
        {/if}
      </section>

      {#if failure}
        {@render failed(failure, () => loadDetail())}
      {/if}
      {#if !detail && !failure}
        <p class="state" role="status">{t('prs.view.loading', { word })}</p>
      {:else if detail}
        {@const d = detail}
        {#if d.can.checks}
          <section class="sec" aria-labelledby="hosts-ci">
            <div class="sec-h">
              <h3 id="hosts-ci">{t('prs.view.ci.title')}</h3>
              {#if checks.length}<span class="sum" class:bad={reallyFailed(checks)}>{ciSummary(checks)}</span>{/if}
              {#if d.can.rerun && p.state === 'open' && hasFailed(checks)}
                <button class="btn rerun" disabled={rerunning} use:tip={t('prs.view.ci.rerunHint')} onclick={rerun}
                  ><Icon name="restart" size={13} />{rerunning ? t('prs.view.ci.rerunning') : t('prs.view.ci.rerun')}</button
                >
              {/if}
            </div>
            {#if !checks.length}
              <p class="none">{t('prs.view.ci.none')}</p>
            {:else}
              <ul class="list" aria-label={t('prs.view.ci.title')}>
                {#each checks as c, i (`${c.name}|${c.job ?? i}`)}
                  {@const phase = checkPhase(c)}
                  {@const dur = checkDuration(c, app.now)}
                  {@const withLog = phase === 'failure' && !!c.job && d.can.jobLogs}
                  <li class="check {phase}" data-check={c.name}>
                    <span class="ic {phase}"><Icon name={phaseIcon[phase]} size={13} /></span>
                    {#if withLog}
                      <button
                        class="cn as-btn"
                        aria-expanded={openLog === c.job}
                        use:tip={{ text: t('prs.view.ci.logShow'), aria: false }}
                        onclick={() => toggleLog(c)}>{c.name}</button
                      >
                    {:else}
                      <span class="cn">{c.name}</span>
                    {/if}
                    <span class="cs">{t(`prs.view.ci.phase.${phase}`)}</span>
                    {#if dur}<span class="cd mono">{dur}</span>{/if}
                    {#if c.url}
                      {@const url = c.url}
                      <button
                        class="icon-btn ext"
                        aria-label={`${c.name} · ${t('prs.view.ci.onHost')}`}
                        use:tip={{ text: t('prs.view.ci.onHost'), aria: false }}
                        onclick={() => openOutside(url)}><Icon name="ext" size={12} /></button
                      >
                    {/if}
                  </li>
                  {#if withLog && openLog === c.job}
                    {@const log = logs[c.job ?? '']}
                    <li class="log" aria-label={t('prs.view.ci.log', { name: c.name })}>
                      {#if !log || log === 'loading'}
                        <p class="state" role="status">{t('prs.view.ci.logLoading')}</p>
                      {:else if log.failure}
                        {@render failed(log.failure, () => {
                          openLog = null;
                          return toggleLog(c);
                        })}
                      {:else if log.data?.trim()}
                        <span class="log-h">{t('prs.view.ci.logTail')}</span>
                        <pre class="mono">{log.data}</pre>
                      {:else}
                        <p class="none">{t('prs.view.ci.logNone')}</p>
                      {/if}
                    </li>
                  {/if}
                {/each}
              </ul>
            {/if}
          </section>
        {/if}

        {#if d.can.reviews}
          <section class="sec" aria-labelledby="hosts-reviews">
            <div class="sec-h"><h3 id="hosts-reviews">{t('prs.view.reviews.title')}</h3></div>
            {#if !reviews.length && !comments.length}
              <p class="none">{t('prs.view.reviews.none')}</p>
            {:else}
              <ul class="list">
                {#each reviews as r, i (i)}
                  <li class="say">
                    <span class="who"
                      ><b>{r.author}</b>
                      <span class="verdict {r.state}">{t(`prs.view.reviews.${r.state}`)}</span></span
                    >
                    {#if r.body.trim()}<p class="body">{r.body.trim()}</p>{/if}
                  </li>
                {/each}
                {#each comments as c (c.id)}
                  <li class="say" class:resolved={c.resolved}>
                    <span class="who"
                      ><b>{c.author}</b>
                      {#if c.path}<span class="where mono">{c.path}{c.line ? `:${c.line}` : ''}</span>{/if}
                      {#if c.resolved}<span class="tag">{t('prs.view.reviews.resolved')}</span>{/if}</span
                    >
                    <p class="body">{c.body.trim()}</p>
                  </li>
                {/each}
              </ul>
            {/if}
            <p class="note">{t('prs.view.reviews.readOnly')}</p>
          </section>
        {/if}

        {#if d.can.deployments && d.deployments.length}
          <section class="sec" aria-labelledby="hosts-deploy">
            <div class="sec-h"><h3 id="hosts-deploy">{t('prs.view.deployments.title')}</h3></div>
            <ul class="list">
              {#each d.deployments as dep, i (i)}
                <li class="dep">
                  <span class="cn">{dep.environment}</span>
                  <span class="st {dep.state}">{deployState(dep.state)}</span>
                  {#if dep.url}
                    {@const url = dep.url}
                    <button class="url mono" aria-label={t('prs.view.deployments.open', { url })} onclick={() => openOutside(url)}
                      >{url.replace(/^https?:\/\//, '')}<Icon name="ext" size={12} /></button
                    >
                  {/if}
                </li>
              {/each}
            </ul>
          </section>
        {/if}
      {/if}

      {#if others.length}
        <section class="sec" aria-labelledby="hosts-others">
          <div class="sec-h"><h3 id="hosts-others">{t('prs.view.others', { word })}</h3></div>
          <ul class="list">
            {#each others as o (prKey(o))}
              {@render prRow(o, word, () => openPr(o), t('prs.tipHead', { word, number: o.number, title: o.title }))}
            {/each}
          </ul>
        </section>
      {/if}
    {:else}
      <!-- No pull request on its branch: « Ouvrir une PR… » when it can, else why not. -->
      <div class="empty">
        <Icon name="pr" size={18} />
        <b>{t('prs.view.noPr.title', { word })}</b>
        {#if canOpenPr(agent)}
          {@const a = agent}
          <button class="btn primary" onclick={() => askOpenPr(a)}>{t('prs.open', { word })}</button>
        {:else if noPr}
          <span class="why">{noPr.reason}</span>
          {#if noPr.settings}<button class="btn" onclick={openSettings}>{t('prs.view.noPr.settings')}</button>{/if}
        {:else if noPr === undefined}
          <span class="why" role="status">…</span>
        {/if}
      </div>
    {/if}
  {:else}
    <!-- No agent open: the project's open pull requests; an agent's opens it. -->
    <section class="sec" aria-labelledby="hosts-project">
      <div class="sec-h">
        <h3 id="hosts-project">{t('prs.view.project.title', { word })}</h3>
        {#if projectRead?.data}<span class="sum">{projectRead.data.prs.length}</span>{/if}
      </div>
      {#if !projectRead}
        <p class="state" role="status">{t('prs.view.project.loading', { word })}</p>
      {:else if projectRead.failure}
        {@render failed(projectRead.failure, loadProject)}
      {:else if projectRead.data && !projectRead.data.prs.length}
        <p class="none">{t('prs.view.project.none', { word })}</p>
      {:else if projectRead.data}
        <ul class="list">
          {#each projectRead.data.prs as p (prKey(p))}
            {@const owner = agentOfPr(projectAgents, p)}
            {@render prRow(
              p,
              word,
              () => (owner ? app.selectAgent(owner.id) : openPr(p)),
              owner ? t('prs.view.project.ofAgent', { agent: owner.name }) : t('prs.tipHead', { word, number: p.number, title: p.title }),
            )}
          {/each}
        </ul>
      {/if}
    </section>
  {/if}
</div>

<style>
  /* The panel's lists (FilesPanel, WorkflowPanel): rows of 30 px, state glyphs in the colours of the card's numbers. */
  .hosts {
    flex: 1;
    min-height: 0;
    overflow: auto;
    scrollbar-width: thin;
    display: flex;
    flex-direction: column;
    padding-bottom: 12px;
  }
  .hosts > * {
    flex: none;
  }
  .mono {
    font-family: var(--mono);
    font-size: 0.92em;
    letter-spacing: -0.01em;
  }
  .caption {
    overflow: hidden;
    color: var(--muted);
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .head {
    display: grid;
    gap: 5px;
    padding: 14px 14px 10px;
    border-bottom: 1px solid var(--line);
  }
  .h1 {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
  }
  .h1 .num {
    color: var(--text);
    font-weight: 600;
  }
  .h1 .ext {
    margin-left: auto;
  }
  .title {
    color: var(--text);
    font-weight: 600;
    line-height: 1.35;
    overflow-wrap: anywhere;
  }
  .conflict {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--del);
    font-size: 12px;
    font-weight: 550;
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 4px;
  }
  .acts .btn,
  .rerun {
    height: 26px;
    padding: 0 9px;
    font-size: 12px;
  }
  /* The glyphs, as the card draws a pull request's CI. */
  .ic {
    display: inline-grid;
    place-items: center;
    flex: none;
    color: var(--muted);
  }
  .ic.ok,
  .ic.success {
    color: var(--run);
  }
  .ic.fail,
  .ic.failure {
    color: var(--del);
  }
  .ic.wait,
  .ic.running,
  .ic.queued {
    color: var(--wait);
  }
  .ic.merged {
    color: var(--syn-k);
  }
  .st {
    display: inline-flex;
    align-items: center;
    height: 18px;
    padding: 0 6px;
    border-radius: 5px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
    font-size: 11px;
    font-weight: 550;
  }
  .st.open,
  .st.success {
    color: var(--run);
  }
  .st.merged {
    color: var(--syn-k);
  }
  .st.failure,
  .st.error {
    color: var(--del);
  }
  .st.draft,
  .st.closed,
  .st.inactive {
    color: var(--muted);
  }
  .sec {
    padding: 10px 8px 0;
  }
  .sec-h {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 28px;
    padding: 0 6px 4px;
  }
  .sec-h h3 {
    margin: 0;
    color: var(--soft);
    font-size: 12px;
    font-weight: 600;
  }
  .sum {
    color: var(--muted);
    font-size: 11.5px;
  }
  .sum.bad {
    color: var(--del);
  }
  .rerun {
    margin-left: auto;
  }
  .list {
    display: grid;
    gap: 1px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .check,
  .dep {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 30px;
    padding: 0 6px 0 8px;
    border-radius: 7px;
  }
  .check:hover,
  .dep:hover {
    background: var(--hover);
  }
  .cn {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    color: var(--text);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .as-btn {
    padding: 0;
    border: none;
    background: none;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .as-btn:hover {
    text-decoration: underline;
  }
  .cs {
    flex: none;
    color: var(--muted);
    font-size: 11.5px;
  }
  .check.failure .cs {
    color: var(--del);
  }
  .cd {
    flex: none;
    color: var(--dim);
    font-size: 11px;
  }
  .ext {
    width: 22px;
    height: 22px;
    border-radius: 5px;
  }
  .log {
    display: grid;
    gap: 4px;
    margin: 0 0 6px 29px;
  }
  .log-h {
    color: var(--dim);
    font-size: 11px;
  }
  .log pre {
    max-height: 320px;
    margin: 0;
    padding: 8px 10px;
    overflow: auto;
    border-radius: 7px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--code-fg);
    font-size: 11px;
    line-height: 1.45;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .say {
    display: grid;
    gap: 3px;
    padding: 6px 8px;
    border-radius: 7px;
  }
  .say.resolved {
    opacity: 0.6;
  }
  .who {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
    font-size: 12px;
  }
  .who b {
    color: var(--text);
    font-weight: 600;
  }
  .verdict {
    color: var(--muted);
  }
  .verdict.approved {
    color: var(--run);
  }
  .verdict.changesRequested {
    color: var(--wait);
  }
  .where {
    min-width: 0;
    overflow: hidden;
    color: var(--p-ink);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag {
    color: var(--dim);
    font-size: 11px;
  }
  .body {
    margin: 0;
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.45;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    display: -webkit-box;
    -webkit-line-clamp: 6;
    line-clamp: 6;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .note,
  .none {
    margin: 4px 8px 0;
    color: var(--dim);
    font-size: 11.5px;
  }
  .url {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    max-width: 55%;
    padding: 0;
    border: none;
    background: none;
    color: var(--p-ink);
    font-size: 11px;
    cursor: pointer;
  }
  .url:hover {
    text-decoration: underline;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    border-radius: 7px;
  }
  .row:hover {
    background: var(--hover);
  }
  .row-main {
    display: grid;
    grid-template-columns: auto auto minmax(0, 1fr);
    align-items: center;
    column-gap: 7px;
    flex: 1;
    min-width: 0;
    padding: 6px 4px 6px 8px;
    border: none;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .row-main .num {
    color: var(--soft);
  }
  .row-main .ttl {
    overflow: hidden;
    color: var(--text);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row-main .meta {
    grid-column: 3;
    overflow: hidden;
    color: var(--muted);
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row .ext {
    margin-right: 4px;
  }
  .bad {
    color: var(--del);
  }
  /* Waiting, empty and failed: the panel's quiet words, a failure in its colour. */
  .state {
    margin: 16px 14px 0;
    color: var(--muted);
    font-size: 12.5px;
  }
  .state.err {
    display: flex;
    gap: 9px;
    margin: 12px 14px 0;
    padding: 10px 12px;
    border-radius: 9px;
    background: color-mix(in oklch, var(--del) 8%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--del) 30%, transparent);
    color: var(--del);
  }
  .state-t {
    display: grid;
    gap: 4px;
    min-width: 0;
  }
  .state-t b {
    color: var(--text);
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .why {
    color: var(--muted);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  .empty {
    display: grid;
    justify-items: center;
    gap: 8px;
    margin: 28px 18px 0;
    color: var(--muted);
    text-align: center;
    font-size: 12.5px;
  }
  .empty b {
    color: var(--soft);
    font-weight: 600;
  }
</style>
