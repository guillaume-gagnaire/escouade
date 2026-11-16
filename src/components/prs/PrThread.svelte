<script lang="ts">
  import { fAgo } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { initials, threadPlace, threadState } from '../../lib/prpage';
  import { threadKey, writeFailure, type PrWrite, type PrWriting } from '../../lib/prthreads';
  import { app } from '../../lib/state.svelte';
  import type { PrFailure, PrThread } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import Markdown from '../conv/Markdown.svelte';
  import PrComposer from './PrComposer.svelte';

  // A thread of a pull request under its line in the tab « Fichiers » (ESC-474): its state, its messages (a third
  // party's Markdown, rendered as the app renders it: nothing remote loads without a click), and what the host writes in
  // the user's name on a click: « Répondre » (a draft kept in `app.prDrafts[replyKey]`), « Résoudre », « Rouvrir ». A
  // resolved one is folded in a line until unfolded. A write that fails is said in the thread, its draft kept. A button
  // the host does not write is not shown, and the thread says so.
  let {
    thread,
    host,
    writing,
    replyKey,
    send,
    current = false,
    showPlace = false,
  }: {
    thread: PrThread;
    host: string;
    writing: PrWriting;
    replyKey: string;
    send: (w: PrWrite) => Promise<PrFailure | null>;
    current?: boolean;
    showPlace?: boolean;
  } = $props();

  let unfolded = $state(false);
  let replying = $state(false);
  let busy = $state<'reply' | 'resolve' | 'reopen' | null>(null);
  let failure = $state<string | null>(null);

  const key = $derived(threadKey(thread));
  const place = $derived(threadPlace(thread));
  const folded = $derived(thread.state === 'resolved' && !unfolded);
  const hasDraft = $derived(!!app.prDrafts[replyKey]?.trim());
  const first = $derived(thread.comments[0]);
  /** What the host cannot do here, said instead of its button. */
  const cannot = $derived(
    thread.state === 'resolved'
      ? writing.reopen || !thread.id
        ? ''
        : t('prs.page.threads.cannotReopen', { host })
      : writing.resolve || !thread.id
        ? ''
        : t('prs.page.threads.cannotResolve', { host }),
  );

  async function act(w: PrWrite, what: 'reply' | 'resolve' | 'reopen') {
    busy = what;
    failure = null;
    const f = await send(w);
    busy = null;
    if (f) {
      failure = writeFailure(f, host, typeof navigator === 'undefined' ? true : navigator.onLine);
      return;
    }
    if (what === 'reply') {
      delete app.prDrafts[replyKey];
      replying = false;
    }
  }

  const ago = (s?: number) => (s ? fAgo(s, app.now) : '');
</script>

{#if folded}
  <div class="thread folded" class:current data-thread-key={key} data-state={thread.state}>
    <button class="unfold" aria-expanded="false" aria-label={t('prs.page.threads.unfold')} onclick={() => (unfolded = true)}>
      <span class="ts resolved"><Icon name="check-c" size={13} />{threadState(thread)}</span>
      {#if showPlace && place}<span class="mono place">{place}</span>{/if}
      <span class="muted">· {t('prs.page.resolvedFold', { count: thread.comments.length })}</span>
      <span class="ell muted">{first?.body ?? ''}</span>
    </button>
    {#if writing.reopen && thread.id}
      <button class="btn small" disabled={!!busy} onclick={() => act({ kind: 'reopen', thread: thread.id }, 'reopen')}
        >{busy === 'reopen' ? t('prs.page.threads.reopening') : t('prs.page.threads.reopen')}</button
      >
    {/if}
    {#if failure}<p class="fail" role="alert"><Icon name="info" size={13} />{failure}</p>{/if}
  </div>
{:else}
  <article
    class="thread"
    class:current
    data-thread-key={key}
    data-state={thread.state}
    tabindex="-1"
    aria-label={place ? t('prs.page.threads.label', { place }) : t('prs.page.threads.conversationLabel')}
  >
    <header class="th-head">
      <span class="ts {thread.state}"
        ><Icon name={thread.state === 'open' ? 'circle' : thread.state === 'resolved' ? 'check-c' : 'clock'} size={13} />{threadState(
          thread,
        )}</span
      >
      {#if showPlace && place}<span class="mono place">{place}</span>{/if}
      {#if thread.state === 'resolved'}
        <button class="icon-btn fold" aria-label={t('prs.page.threads.fold')} aria-expanded="true" onclick={() => (unfolded = false)}
          ><Icon name="up" size={12} /></button
        >
      {/if}
    </header>
    {#each thread.comments as m (m.id)}
      <div class="msg">
        <span class="av" aria-hidden="true">{initials(m.author)}</span>
        <div class="who">
          <b>{m.author}</b>{#if m.created}<span class="t">{ago(m.created)}</span>{/if}
        </div>
        <div class="bd third"><Markdown text={m.body} /></div>
      </div>
    {/each}
    {#if failure}<p class="fail" role="alert"><Icon name="info" size={13} />{failure}</p>{/if}
    {#if replying}
      <PrComposer
        draftKey={replyKey}
        label={t('prs.page.threads.reply')}
        submit={t('prs.page.threads.sendReply')}
        {host}
        busy={busy === 'reply'}
        autofocus
        onSend={(body) => first && act({ kind: 'reply', comment: first.id, body }, 'reply')}
        onCancel={() => {
          replying = false;
          failure = null;
        }}
      />
    {:else}
      <footer class="acts">
        {#if writing.reply && first?.id}
          <button class="btn small" onclick={() => (replying = true)}
            >{hasDraft ? t('prs.page.threads.replyDraft') : t('prs.page.threads.reply')}</button
          >
        {/if}
        {#if thread.state !== 'resolved' && writing.resolve && thread.id}
          <button class="btn small" disabled={!!busy} onclick={() => act({ kind: 'resolve', thread: thread.id }, 'resolve')}
            ><Icon name="check" size={12} />{busy === 'resolve' ? t('prs.page.threads.resolving') : t('prs.page.threads.resolve')}</button
          >
        {/if}
        {#if thread.state === 'resolved' && writing.reopen && thread.id}
          <button class="btn small" disabled={!!busy} onclick={() => act({ kind: 'reopen', thread: thread.id }, 'reopen')}
            >{busy === 'reopen' ? t('prs.page.threads.reopening') : t('prs.page.threads.reopen')}</button
          >
        {/if}
        {#if !writing.reply && first?.id}<span class="cannot">{t('prs.page.threads.cannotReply', { host })}</span>{/if}
        {#if cannot}<span class="cannot">{cannot}</span>{/if}
      </footer>
    {/if}
  </article>
{/if}

<style>
  .thread {
    margin: 6px 12px 8px 44px;
    border-radius: 10px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
    overflow: hidden;
    min-width: 0;
    font-family: var(--font, inherit);
    white-space: normal;
  }
  .thread:focus {
    outline: none;
  }
  .thread.current {
    box-shadow:
      inset 0 0 0 1px var(--brain),
      0 0 0 1px var(--brain);
  }
  .th-head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px 6px 12px;
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
  .msg {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr);
    gap: 3px 9px;
    padding: 9px 12px 7px;
  }
  .msg + .msg {
    border-top: 1px solid var(--line);
  }
  .av {
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
  .who {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    font-size: 12.5px;
  }
  .who b {
    font-weight: 620;
  }
  .t {
    color: var(--muted);
    font-size: 11.5px;
  }
  .bd {
    grid-column: 2;
    color: var(--text-ai);
    min-width: 0;
    font-size: 13px;
  }
  .third :global(p:first-child) {
    margin-top: 0;
  }
  .third :global(p:last-child) {
    margin-bottom: 0;
  }
  .acts {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    padding: 4px 12px 10px 43px;
  }
  .btn.small {
    height: 26px;
    padding: 0 9px;
    font-size: 12px;
  }
  .cannot {
    color: var(--muted);
    font-size: 11.5px;
  }
  .fail {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    margin: 0 12px 8px;
    padding: 6px 9px;
    border-radius: 8px;
    font-size: 12px;
    color: var(--del);
    background: oklch(from var(--del) l c h / 0.07);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.35);
  }
  .thread.folded {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    padding: 4px 8px 4px 4px;
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .unfold {
    all: unset;
    box-sizing: border-box;
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding: 4px 8px;
    border-radius: 8px;
    font-size: 12px;
    color: var(--muted);
    cursor: pointer;
  }
  .unfold:hover {
    background: var(--raise);
  }
  .unfold:focus-visible {
    outline: 2px solid var(--brain);
  }
  .thread.folded .fail {
    flex-basis: 100%;
  }
  .muted {
    color: var(--muted);
  }
  .ell {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
