<script lang="ts">
  import { untrack } from 'svelte';
  import { fTime } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { VERDICTS, edited, isOwn, notesByFile, publishBlock, tally } from '../../lib/prreview';
  import { openUrl } from '../../lib/opener';
  import { errorText } from '../../lib/refusal';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { AgentReview, NoteChoice, Project, ReviewNote, ReviewVerdict } from '../../lib/types';
  import Icon from '../Icon.svelte';

  // The review of a pull request by an agent (ESC-66), in place of the list of the project's « Pull requests » view: its text,
  // then its comments file by file, each with the lines it is written on. Each is kept, edited or dropped; « Publier » sends
  // what was kept, at once, and is the only way anything reaches the host.
  let { project, review, me }: { project: Project; review: AgentReview; me: string } = $props();

  let verdict = $state<ReviewVerdict>('commented');
  let summary = $state('');
  /** The comment being edited, and what is typed. */
  let editing = $state<number | null>(null);
  let typed = $state('');
  let sending = $state(false);

  const number = $derived(review.number);
  const word = $derived(review.word);
  const files = $derived(notesByFile(review.notes));
  const counts = $derived(tally(review.notes));
  const own = $derived(isOwn(review, me));
  const draft = $derived(review.phase === 'ready');
  const busy = $derived(sending || review.phase === 'publishing');
  const block = $derived(publishBlock(review, verdict, summary));

  // The text follows the draft until the user types: a reading that ends fills it.
  let synced = $state('');
  $effect(() => {
    const text = review.summary;
    untrack(() => {
      if (summary === synced) summary = text;
      synced = text;
    });
  });

  const failed = (e: unknown) => app.toast(errorText(e), 'error');

  async function choose(n: ReviewNote, choice: NoteChoice, body?: string) {
    try {
      await api.prReviewNote(project.id, number, n.id, choice, body);
      if (editing === n.id) editing = null;
    } catch (e) {
      failed(e);
    }
  }

  function edit(n: ReviewNote) {
    editing = n.id;
    typed = n.body;
  }

  async function saveSummary() {
    if (!draft || summary.trim() === review.summary) return;
    try {
      await api.prReviewSummary(project.id, number, summary);
    } catch (e) {
      failed(e);
    }
  }

  async function publish(acceptMoved = false) {
    if (block || busy) return;
    sending = true;
    try {
      await saveSummary();
      const out = await api.prReviewPublish(project.id, number, verdict, acceptMoved);
      if (out.moved) {
        app.modal = {
          kind: 'confirm',
          icon: 'info',
          title: t('prs.review.moved.title', { word }),
          body: t('prs.review.moved.body', { word }),
          confirm: t('prs.review.moved.confirm'),
          onConfirm: () => publish(true),
        };
      } else {
        app.toast(t('prs.review.published', { word, number }), 'ok');
      }
    } catch (e) {
      failed(e);
    } finally {
      sending = false;
    }
  }

  function readAgain() {
    const start = () => app.startPrReview(project.id, number);
    if (!draft || !(counts.kept + counts.dropped)) return void start();
    app.modal = {
      kind: 'confirm',
      title: t('prs.review.again.title'),
      body: t('prs.review.again.body'),
      confirm: t('prs.review.again.confirm'),
      danger: true,
      onConfirm: start,
    };
  }

  function discard() {
    void api.prReviewDiscard(project.id, number).catch(failed);
    app.closePrReview(project.id);
  }
</script>

{#snippet excerpt(n: ReviewNote)}
  {#if n.excerpt.length}
    <div class="code" role="img" aria-label={t('prs.review.excerpt', { path: n.path, line: n.line })}>
      {#each n.excerpt as l, i (i)}
        <div class="cl" class:add={l.kind === '+'} class:del={l.kind === '-'} class:on={i === n.excerpt.length - 1}>
          <span class="ln">{l.line ?? ''}</span><span class="k">{l.kind === ' ' || l.kind === '@' ? '' : l.kind}</span><span class="tx"
            >{l.text}</span
          >
        </div>
      {/each}
    </div>
  {/if}
{/snippet}

{#snippet noteCard(n: ReviewNote)}
  <article
    class="note"
    class:kept={n.choice === 'kept'}
    class:dropped={n.choice === 'dropped'}
    aria-label={t('prs.review.noteLabel', { path: n.path, line: n.line })}
  >
    {#if n.choice !== 'dropped'}{@render excerpt(n)}{/if}
    <div class="where">
      <span class="ref">:{n.line}</span>
      {#if n.outside && n.choice !== 'dropped'}<span class="outside"><Icon name="info" size={12} />{t('prs.review.outside')}</span>{/if}
    </div>
    {#if editing === n.id}
      <!-- svelte-ignore a11y_autofocus -->
      <textarea class="fix" rows="4" aria-label={t('prs.review.editLabel')} bind:value={typed} autofocus></textarea>
      <div class="ctl">
        <button class="btn small primary" disabled={!typed.trim()} onclick={() => choose(n, 'kept', typed)}
          ><Icon name="check" size={13} />{t('prs.review.saveKeep')}</button
        >
        <button class="btn small" onclick={() => (editing = null)}>{t('prs.review.cancel')}</button>
        {#if edited(n) || typed !== n.proposed}
          <button class="btn small ghost" onclick={() => (typed = n.proposed)}>{t('prs.review.backToAgent')}</button>
        {/if}
      </div>
    {:else}
      <p class="txt">{n.body}</p>
      {#if draft}
        <div class="ctl">
          {#if n.choice === 'dropped'}
            <button class="btn small" onclick={() => choose(n, 'open')}>{t('prs.review.takeBack')}</button>
          {:else}
            {#if n.choice === 'open'}
              <button class="btn small primary" onclick={() => choose(n, 'kept')}
                ><Icon name="check" size={13} />{t('prs.review.keep')}</button
              >
            {/if}
            <button class="btn small" onclick={() => edit(n)}><Icon name="edit" size={13} />{t('prs.review.edit')}</button>
            <button class="btn small" onclick={() => choose(n, 'dropped')}><Icon name="close" size={13} />{t('prs.review.drop')}</button>
          {/if}
          {#if n.choice !== 'open'}
            <span class="said" class:ok={n.choice === 'kept'}>
              {n.choice === 'dropped' ? t('prs.review.dropped') : edited(n) ? t('prs.review.edited') : t('prs.review.kept')}
            </span>
          {/if}
        </div>
      {:else if n.choice !== 'open'}
        <div class="ctl">
          <span class="said" class:ok={n.choice === 'kept'}>
            {n.choice === 'dropped' ? t('prs.review.dropped') : edited(n) ? t('prs.review.edited') : t('prs.review.kept')}
          </span>
        </div>
      {/if}
    {/if}
  </article>
{/snippet}

<main class="rv">
  <header class="head">
    <button
      class="icon-btn"
      aria-label={t('prs.review.back', { word })}
      use:tip={t('prs.review.back', { word })}
      onclick={() => app.closePrReview(project.id)}><Icon name="left" size={16} /></button
    >
    <div class="h-title">
      <h1><span class="num">#{number}</span> {review.title}</h1>
      <div class="crumbs">
        {#if review.by}<span>{t('prs.review.by', { who: review.by })}</span><span aria-hidden="true">·</span>{/if}
        <span class="mono">{review.head} → {review.base}</span>
      </div>
    </div>
    {#if review.phase !== 'reading'}
      <button class="btn" disabled={busy} onclick={readAgain}><Icon name="restart" size={14} />{t('prs.review.again.button')}</button>
    {/if}
    <button
      class="icon-btn"
      aria-label={t('prs.view.openInBrowser')}
      use:tip={t('prs.view.openInBrowser')}
      onclick={() => void openUrl(review.url)}><Icon name="ext" size={15} /></button
    >
    <button class="btn ghost" disabled={busy} onclick={discard}
      >{review.phase === 'reading' ? t('prs.review.cancel') : t('prs.review.discard')}</button
    >
  </header>

  {#if review.phase === 'reading'}
    <div class="state" role="status">
      <span class="spinner" aria-hidden="true"></span>
      <p class="state-t">{t('prs.review.reading', { word, number })}</p>
      <p>{t('prs.review.readingHint')}</p>
    </div>
  {:else if review.phase === 'failed'}
    <div class="state">
      <div class="err" role="alert">
        <Icon name="info" size={15} />
        <div class="err-t">
          <b>{t('prs.review.failed')}</b>
          <span class="why">{review.error}</span>
        </div>
        <button class="btn" onclick={readAgain}><Icon name="restart" size={14} />{t('prs.view.retry')}</button>
      </div>
    </div>
  {:else}
    <div class="body">
      <div class="col">
        {#if review.error}
          <div class="err" role="alert">
            <Icon name="info" size={15} />
            <div class="err-t"><span>{review.error}</span></div>
          </div>
        {/if}
        {#if review.phase === 'published'}
          <p class="done" role="status">
            <Icon name="check" size={14} />{t('prs.review.publishedAt', { host: review.host, time: fTime(review.publishedAt ?? 0) })}
          </p>
        {/if}
        {#if review.leftOut.length}
          <p class="left-out">
            <Icon name="info" size={13} />
            <span use:tip={{ text: review.leftOut.join('\n') }}>{t('prs.review.leftOut', { count: review.leftOut.length })}</span>
          </p>
        {/if}
        <label class="sum">
          <span class="lab">{t('prs.review.summary')}</span>
          <textarea rows="3" bind:value={summary} readonly={!draft} onblur={saveSummary}></textarea>
        </label>
        {#each files as f (f.path)}
          <section class="file" aria-label={f.path}>
            <div class="fname">
              <span class="mono">{f.path}</span>
              <span class="fcount">{t('prs.review.comments', { count: f.notes.length })}</span>
            </div>
            {#each f.notes as n (n.id)}{@render noteCard(n)}{/each}
          </section>
        {:else}
          <p class="none">{t('prs.review.none')}</p>
        {/each}
      </div>
    </div>
    {#if review.phase !== 'published'}
      <footer class="foot">
        <span class="left">
          {#if counts.open}<b>{t('prs.review.toDecide', { count: counts.open })}</b>{' · '}{/if}{t('prs.review.keptCount', {
            count: counts.kept,
          })}{' · '}{t('prs.review.droppedCount', { count: counts.dropped })}
          <span class="promise">{t('prs.review.promise')}</span>
        </span>
        <div class="segmented" role="radiogroup" aria-label={t('prs.review.verdict')}>
          {#each VERDICTS as v (v)}
            <button
              role="radio"
              aria-checked={verdict === v}
              class:on={verdict === v}
              disabled={own && v !== 'commented'}
              use:tip={own && v !== 'commented' ? t('prs.review.ownHint', { word }) : ''}
              onclick={() => (verdict = v)}>{t(`prs.review.verdicts.${v}`)}</button
            >
          {/each}
        </div>
        <span use:tip={block ?? ''}>
          <button class="btn primary" disabled={!!block || busy} onclick={() => publish()}
            ><Icon name="up" size={14} />{busy ? t('prs.review.publishing') : t('prs.review.publish', { host: review.host })}</button
          >
        </span>
      </footer>
    {/if}
  {/if}
</main>

<style>
  .rv {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 60px;
    padding: 0 14px;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
  }
  .h-title {
    display: grid;
    gap: 2px;
    min-width: 0;
    margin-right: auto;
  }
  h1 {
    margin: 0;
    overflow: hidden;
    font-size: 15px;
    font-weight: 620;
    letter-spacing: -0.015em;
    text-overflow: ellipsis;
  }
  h1 .num {
    color: var(--muted);
    font-weight: 520;
  }
  .crumbs {
    display: flex;
    gap: 6px;
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: 12px;
  }
  .crumbs .mono {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .state {
    flex: 1;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 8px;
    padding: 40px;
    color: var(--muted);
    text-align: center;
  }
  .state p {
    margin: 0;
    max-width: 46ch;
  }
  .state-t {
    color: var(--text);
    font-size: 14px;
    font-weight: 560;
  }
  .spinner {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid var(--line-2);
    border-top-color: var(--soft);
    animation: turn 0.9s linear infinite;
  }
  @keyframes turn {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spinner {
      animation: none;
    }
  }
  .err {
    display: flex;
    align-items: center;
    gap: 12px;
    max-width: 600px;
    padding: 12px 14px;
    border-radius: 10px;
    color: var(--del);
    text-align: left;
    background: color-mix(in oklch, var(--del) 7%, var(--panel));
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--del) 35%, transparent);
  }
  .err-t {
    flex: 1;
    display: grid;
    gap: 3px;
    color: var(--text);
  }
  .err b {
    font-weight: 600;
  }
  .why {
    color: var(--soft);
  }

  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 18px 22px 24px;
  }
  .col {
    display: grid;
    gap: 18px;
    max-width: 860px;
    margin: 0 auto;
  }
  .done,
  .left-out {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: 12.5px;
  }
  .done {
    color: var(--run);
  }
  .left-out {
    color: var(--muted);
  }
  .sum {
    display: grid;
    gap: 6px;
  }
  .lab {
    color: var(--muted);
    font-size: 12px;
    font-weight: 520;
  }
  .sum textarea,
  .fix {
    box-sizing: border-box;
    width: 100%;
    resize: vertical;
    line-height: 1.5;
  }
  .file {
    display: grid;
    gap: 8px;
  }
  .fname {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
    color: var(--soft);
    font-size: 12.5px;
  }
  .fname .mono {
    overflow-wrap: anywhere;
  }
  .fcount {
    flex: none;
    color: var(--dim);
    font-size: 12px;
  }
  .none {
    margin: 0;
    color: var(--muted);
  }

  .note {
    overflow: hidden;
    border-radius: 9px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .note.kept {
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--run) 34%, transparent);
  }
  .note.dropped .txt {
    color: var(--muted);
    text-decoration: line-through;
  }
  .code {
    padding: 6px 0;
    overflow-x: auto;
    background: var(--well);
    border-bottom: 1px solid var(--line);
    font-family: var(--mono);
    font-size: 12px;
    line-height: 1.55;
  }
  .cl {
    display: grid;
    grid-template-columns: 46px 16px 1fr;
    width: max-content;
    min-width: 100%;
    color: var(--soft);
    white-space: pre;
  }
  .cl.add {
    background: color-mix(in oklch, var(--add) 10%, transparent);
  }
  .cl.del {
    background: color-mix(in oklch, var(--del) 10%, transparent);
  }
  .cl.on {
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--p) 50%, transparent);
  }
  .ln {
    padding-right: 10px;
    color: var(--dim);
    text-align: right;
    user-select: none;
  }
  .k {
    color: var(--muted);
    user-select: none;
  }
  .add .k {
    color: var(--add);
  }
  .del .k {
    color: var(--del);
  }
  .where {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px 0;
    color: var(--dim);
    font-size: 12px;
  }
  .ref {
    font-family: var(--mono);
  }
  .outside {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--wait);
  }
  .txt {
    max-width: 75ch;
    margin: 0;
    padding: 4px 12px 10px;
    line-height: 1.5;
    overflow-wrap: anywhere;
    white-space: pre-wrap;
  }
  .fix {
    display: block;
    width: calc(100% - 24px);
    margin: 6px 12px 10px;
  }
  .ctl {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px 10px;
  }
  .btn.small {
    height: 26px;
    padding: 0 9px;
    font-size: 12px;
  }
  .said {
    margin-left: auto;
    color: var(--muted);
    font-size: 12px;
  }
  .said.ok {
    color: var(--run);
  }

  .foot {
    flex: none;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 22px;
    border-top: 1px solid var(--line);
  }
  .left {
    min-width: 0;
    margin-right: auto;
    color: var(--muted);
    font-size: 12.5px;
  }
  .left b {
    color: var(--text);
    font-weight: 560;
  }
  .promise::before {
    content: ' · ';
  }
  /* Narrow: the promise gives its room to the verdict. */
  @media (max-width: 1150px) {
    .promise {
      display: none;
    }
  }
</style>
