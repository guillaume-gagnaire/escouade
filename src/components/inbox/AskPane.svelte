<script lang="ts">
  import { tick, untrack } from 'svelte';
  import { copyToDisk } from '../../lib/attachments';
  import { insertAt } from '../../lib/dictation';
  import type { DictationTarget } from '../../lib/dictation.svelte';
  import { setDraft } from '../../lib/drafts';
  import { fDate, fSince, fTime } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import type { IconName } from '../../lib/icons';
  import { fLater, inbox, laterTimes, letter, startOption } from '../../lib/inbox/inbox.svelte';
  import { api } from '../../lib/ipc';
  import { menu, type MenuItem } from '../../lib/menu.svelte';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { AskAnswerIn, AskKind, AskVia, AttachedFile, WindowAsk } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import MicButton from '../MicButton.svelte';
  import Markdown from '../conv/Markdown.svelte';
  import AskAttachments from './AskAttachments.svelte';
  import ChoicePlates from './ChoicePlates.svelte';
  import CompareFull from './CompareFull.svelte';
  import PdfReader from './PdfReader.svelte';

  // A request of the box, in the panel (direction B, `b-choix.png`, `b-pdf.png`, `b-validation.png`, `b-tache.png`): who
  // asks, what, since when; its text (an agent's data, sanitized, nothing from the web without a click); its options on
  // their plates, its steps, its files (a PDF read beside the text); the recommendation, set forward but never chosen for
  // the user; what the answer unblocks; then the answer at the bottom, as the composer of a conversation. A request
  // closed shows its trace instead: the answer, who gave it, when, read or not, and its history.
  let { ask }: { ask: WindowAsk } = $props();

  const KIND_ICON: Record<AskKind, IconName> = { choice: 'choice', confirm: 'check-c', answer: 'edit', task: 'check' };

  let now = $state(Date.now());
  $effect(() => {
    const id = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(id);
  });

  // ---------- the answer being written (the pane is made again for another request: `{#key}`) ----------
  const first = untrack(() => ask);
  let focus = $state(startOption(first));
  let chosen = $state<string[]>([]);
  let note = $state('');
  let text = $state(first.recommend?.text ?? '');
  let steps = $state<boolean[]>(first.steps.map(() => false));
  let files = $state<AttachedFile[]>([]);
  let busy = $state(false);
  let voiced = false;
  let comparing = $state(false);
  let picking = $state(false);
  let pickAt = $state('');
  let noteField = $state<HTMLInputElement | HTMLTextAreaElement>();
  let fileInput = $state<HTMLInputElement>();
  let laterButton = $state<HTMLButtonElement>();

  const waiting = $derived(ask.state === 'open' || ask.state === 'snoozed');
  const ownFiles = $derived(ask.files.filter((f) => f.option === null && !f.byUser));
  const pdfs = $derived(ownFiles.filter((f) => f.kind === 'pdf'));
  let reading = $state<string | null>(null);
  $effect(() => {
    if (!reading || !pdfs.some((f) => f.file === reading)) reading = pdfs[0]?.file ?? null;
  });
  const readFile = $derived(pdfs.find((f) => f.file === reading) ?? null);
  const agentName = $derived(ask.author.agent ? (app.agents[ask.author.agent]?.name ?? ask.author.name) : ask.author.name);
  const agentHere = $derived(!!(ask.author.agent && app.agents[ask.author.agent]));
  const projectColor = $derived(app.projects.find((p) => p.id === ask.projectId)?.color ?? 'var(--idle)');
  const recOption = $derived(ask.options.find((o) => o.id === ask.recommend?.option));
  const waitingTickets = $derived(ask.tickets.map((id) => app.tickets[id]?.key).filter((k): k is string => !!k));
  const chief = $derived(ask.delegates[0] ?? null);

  // ---------- the dictation (the microphone of the conversation) ----------
  const dictated: DictationTarget = {
    kind: 'ticket',
    id: `ask-${first.id}`,
    el: () => noteField?.closest('.answer'),
    insert: (said) => {
      voiced = true;
      const f = noteField;
      const long = ask.kind === 'answer';
      const value = long ? text : note;
      const at =
        f && document.activeElement === f
          ? [f.selectionStart ?? value.length, f.selectionEnd ?? value.length]
          : [value.length, value.length];
      const r = insertAt(value, at[0], at[1], said);
      if (long) text = r.text;
      else note = r.text;
      void tick().then(() => {
        f?.focus();
        f?.setSelectionRange(r.caret, r.caret);
      });
    },
  };

  // ---------- answering ----------
  async function send(answer: AskAnswerIn, via: AskVia) {
    if (busy) return;
    busy = true;
    const n = note.trim();
    const done = await app.run(
      api.inboxAnswer(
        ask.id,
        { ...answer, ...(n ? { note: n } : {}), via: voiced ? 'voice' : via },
        $state.snapshot(files) as AttachedFile[],
      ),
    );
    busy = false;
    if (!done) return;
    inbox.put(done);
    inbox.sending = { id: done.id, title: done.title, sendAt: done.answer?.sendAt ?? Date.now() + 10_000 };
    inbox.after(ask.id);
  }

  /** The primary gesture of its sort: « Choisir B », « Oui », « Envoyer », « C'est fait ». */
  function primary(via: AskVia) {
    if (ask.kind === 'choice') {
      const options = ask.multi ? (chosen.length ? chosen : focus ? [focus] : []) : focus ? [focus] : [];
      if (options.length) void send({ options }, via);
    } else if (ask.kind === 'confirm') void send({ yes: true }, via);
    else if (ask.kind === 'answer') {
      if (text.trim()) void send({ text: text.trim() }, via);
    } else void send({ done: true, steps }, via);
  }

  function snooze(until: number | null) {
    void app.run(api.inboxSnooze(ask.id, until)).then((a) => {
      if (!a) return;
      inbox.put(a);
      if (until) {
        app.toast(t('inbox.later.done', { when: fLater(until) }), 'ok');
        inbox.after(ask.id);
      }
    });
  }

  function laterMenu(el: HTMLElement | undefined = laterButton) {
    if (!el) return;
    const r = el.getBoundingClientRect();
    const items: MenuItem[] = [
      { label: t('inbox.later.menu'), heading: true },
      ...laterTimes().map((x) => ({
        label: t(`inbox.later.${x.key}`),
        icon: 'clock' as const,
        detail: fLater(x.at),
        onClick: () => snooze(x.at),
      })),
      { label: t('inbox.later.pick'), icon: 'clock', onClick: () => (picking = true) },
    ];
    menu.open = { x: r.left, y: Math.max(8, r.top - 8 - items.length * 34), items, keyboard: true };
  }

  function pickLater() {
    const at = new Date(pickAt).getTime();
    if (!Number.isFinite(at) || at <= Date.now()) return;
    picking = false;
    snooze(at);
  }

  function delegate(agent: string | null) {
    void app.run(api.inboxDelegate(ask.id, agent)).then((a) => a && inbox.put(a));
  }

  function reopen() {
    void app.run(api.inboxReopen(ask.id)).then((a) => {
      if (!a) return;
      inbox.put(a);
      inbox.select(a.id);
    });
  }

  /** « Changer d'avis… » once read: a message to its agent, written in its composer, which the user sends. */
  function changeMind() {
    const id = ask.author.agent;
    if (!id || !app.agents[id]) return;
    setDraft(id, { text: t('inbox.sent.changeMindDraft', { id: ask.id, title: ask.title }), files: [] });
    app.selectAgent(id);
  }

  async function attach(list: FileList | null) {
    for (const f of list ?? []) {
      const copy = await app.run(copyToDisk(f, 'draft'));
      if (copy) files = [...files, copy];
    }
    if (fileInput) fileInput.value = '';
  }

  /** « Comparer en grand ». */
  function compare() {
    comparing = true;
    api.inboxTrack('compare').catch(() => {});
  }

  function goAgent() {
    if (ask.author.agent && app.agents[ask.author.agent]) app.selectAgent(ask.author.agent);
  }
  function goTicket() {
    const agent = Object.values(app.agents).find((a) => a.ticketId === ask.ticketId && !a.archived);
    if (agent) app.selectAgent(agent.id);
    else app.openBoard(ask.projectId);
  }

  // ---------- the keyboard (§ 8.3): 1 to 6 an option, Entrée the primary gesture, L « Plus tard » ----------
  function typing(e: KeyboardEvent) {
    const el = e.target as HTMLElement | null;
    return !!el?.closest('input, textarea, select, [contenteditable="true"], .xterm, .cm-editor');
  }
  function onWindowKey(e: KeyboardEvent) {
    if (!waiting || comparing || app.modal || menu.open || e.ctrlKey || e.metaKey || e.altKey || typing(e)) return;
    if ((e.target as HTMLElement | null)?.closest('[role="dialog"]')) return;
    const n = Number(e.key);
    if (ask.kind === 'choice' && n >= 1 && n <= ask.options.length) {
      e.preventDefault();
      const id = ask.options[n - 1].id;
      focus = id;
      if (ask.multi) chosen = chosen.includes(id) ? chosen.filter((x) => x !== id) : [...chosen, id];
    } else if (e.key.toLowerCase() === 'l' && !e.shiftKey) {
      e.preventDefault();
      laterMenu();
    } else if (e.key === 'Enter' && !(e.target as HTMLElement | null)?.closest('button, a')) {
      e.preventDefault();
      primary('keyboard');
    }
  }
  function onFieldKey(e: KeyboardEvent) {
    if (e.key !== 'Enter' || e.shiftKey) return;
    // A long answer takes its line breaks: Ctrl Entrée sends it.
    if (ask.kind === 'answer' && e.currentTarget instanceof HTMLTextAreaElement && !(e.ctrlKey || e.metaKey)) return;
    e.preventDefault();
    primary('keyboard');
  }

  // ---------- what a closed request shows ----------
  const answerWhat = $derived.by(() => {
    const a = ask.answer;
    if (!a) return '';
    if (ask.kind === 'choice') return a.options.map((o) => `${letter(o)} · ${ask.options.find((x) => x.id === o)?.label ?? ''}`).join(', ');
    if (ask.kind === 'confirm') return t(a.yes ? 'inbox.sent.yes' : 'inbox.sent.no');
    if (ask.kind === 'task')
      return `${t(a.done ? 'inbox.sent.done' : 'inbox.sent.notDone')} · ${t('inbox.sent.stepsTicked', { done: a.steps.filter(Boolean).length, total: ask.steps.length })}`;
    return t('inbox.sent.quoted', { text: a.text ?? '' });
  });
  const EVENTS = [
    'deposited',
    'updated',
    'snoozed',
    'woke',
    'delegated',
    'answered',
    'reopened',
    'sent',
    'read',
    'withdrawn',
    'expired',
    'retired',
    'filesErased',
  ];
</script>

<svelte:window onkeydown={onWindowKey} />

<div class="pane-ask" data-testid="ask-pane" data-ask={ask.id} data-state={ask.state}>
  <div class="fb">
    <div class="rh">
      <span class="kind"><Icon name={KIND_ICON[ask.kind]} size={13} />{t(`inbox.kind.${ask.kind}`)}</span>
      {#if ask.urgency === 'today' && waiting}<span class="pill wt"><Icon name="clock" size={11} />{t('inbox.ask.todayPill')}</span>{/if}
      {#if ask.woke}<span class="pill">{t('inbox.ask.woke')}</span>{/if}
      <span class="sp"></span>
      <span class="who">
        <span class="pdot" style:background={projectColor}></span>
        <span>{ask.projectName}</span>
        {#if ask.private}<span class="pill k">{t('inbox.ask.privateMark')}</span>{/if}
        <span aria-hidden="true">·</span>
        <b>{ask.author.agent ? agentName : t('inbox.ask.external')}</b>
        {#if ask.author.chef}<span>· {t('inbox.ask.chef')}</span>{/if}
        {#if ask.ticketKey}<span aria-hidden="true">·</span><span class="tk">{ask.ticketKey}</span>{/if}
        <span>· {t('inbox.ask.ago', { time: fSince(ask.createdAt, now) })}</span>
      </span>
    </div>
    <h2 id="ask-title-{ask.id}">{ask.title}</h2>

    {#if ask.state === 'snoozed' && ask.snoozedUntil}
      <p class="band">
        <Icon name="clock" size={13} />{t('inbox.ask.snoozedUntil', { when: fLater(ask.snoozedUntil, now) })}
        <button class="link" onclick={() => snooze(null)}>{t('inbox.ask.wakeNow')}</button>
      </p>
    {/if}
    {#if ask.delegatedTo && waiting}
      <p class="band">
        <Icon name="user" size={13} />{t('inbox.ask.delegatedTo', { name: ask.delegatedName ?? '' })}
        <button class="link" onclick={() => delegate(null)}>{t('inbox.ask.takeBack')}</button>
      </p>
    {/if}

    {#snippet facts()}
      {#if ask.kind === 'task' && ask.steps.length}
        <ol class="steps" aria-label={t('inbox.answer.steps')}>
          {#each ask.steps as s, i (i)}
            <li class="st" class:done={steps[i] || ask.answer?.steps[i]}>
              <label>
                <input
                  type="checkbox"
                  checked={waiting ? steps[i] : !!ask.answer?.steps[i]}
                  disabled={!waiting}
                  onchange={(e) => (steps[i] = e.currentTarget.checked)}
                  data-testid="ask-step"
                />
                <span>{s}</span>
              </label>
            </li>
          {/each}
        </ol>
      {/if}
      <AskAttachments {ask} files={ownFiles} bind:reading />
      {#if ask.recommend && (recOption || ask.recommend.why)}
        <p class="recline">
          <Icon name="sparkle" size={13} />
          <span>
            {#if recOption}<b>{t('inbox.ask.recommendedLine', { option: `${letter(recOption.id)} · ${recOption.label}` })}</b>{' '}{/if}{ask
              .recommend.why}
          </span>
        </p>
      {/if}
      {#if ask.unblocks || waitingTickets.length}
        <p class="unblk">
          <Icon name="lock" size={13} />
          <span class="ell"><b>{t('inbox.ask.unblocks')}</b> {ask.unblocks}</span>
          {#each waitingTickets as k (k)}<span class="tchip" use:tip={t('inbox.ask.waitingTickets')}>{k}</span>{/each}
        </p>
      {/if}
    {/snippet}

    {#if ask.body}<div class="body"><Markdown text={ask.body} /></div>{/if}
    {#if ask.kind === 'choice' && ask.options.length}
      <ChoicePlates
        {ask}
        bind:focus
        {chosen}
        onzoom={(id) => ((focus = id), compare())}
        onpick={(id) => waiting && send({ options: [id] }, 'click')}
      />
    {/if}
    {#if readFile}
      <div class="split">
        <div class="col">{@render facts()}</div>
        {#key readFile.file}<PdfReader {ask} file={readFile} />{/key}
      </div>
    {:else}
      {@render facts()}
    {/if}

    {#if !waiting}
      <section class="trace" aria-label={t('inbox.sent.history')} data-testid="ask-trace">
        {#if ask.answer}
          <p class="yours">
            <Icon name="check" size={13} />
            <b>{t('inbox.sent.yourAnswer')}</b>
            <span data-testid="ask-answer">{answerWhat}</span>
            <span class="muted"
              >· {ask.answer.byName ? t('inbox.sent.delegated', { name: ask.answer.byName }) : t('inbox.sent.you')} · {fLater(
                ask.answer.at,
                now,
              )} · {t(`inbox.sent.via.${ask.answer.via}`)}</span
            >
          </p>
          {#if ask.answer.note}<p class="muted">{t('inbox.sent.noteLine', { note: ask.answer.note })}</p>{/if}
          <p class="status">
            {#if ask.state === 'read'}
              <span class="ok"><Icon name="check-c" size={13} />{t('inbox.sent.readBy', { agent: agentName })}</span>
              {#if agentHere}<button class="btn" onclick={changeMind} data-testid="ask-change-mind">{t('inbox.sent.changeMind')}</button
                >{/if}
            {:else}
              <span
                >{ask.answer.sentAt
                  ? t('inbox.sent.sentTo', { agent: agentName })
                  : t('inbox.sent.waitingRead', { agent: agentName })}</span
              >
              <button class="btn" onclick={reopen} data-testid="ask-reopen"><Icon name="undo" size={13} />{t('inbox.sent.reopen')}</button>
            {/if}
          </p>
          {#if ask.decision}
            <p class="status">
              <span>{t('inbox.sent.decision', { agent: agentName })}</span>
              <button class="btn ghost" onclick={() => app.openDocs(ask.projectId, 'decisions')}>{t('inbox.sent.openDecisions')}</button>
            </p>
          {/if}
        {:else if ask.state === 'withdrawn'}
          <p>{t('inbox.sent.withdrawn', { agent: agentName, reason: ask.reason ?? '' })}</p>
        {:else if ask.state === 'expired'}
          <p>{t('inbox.sent.expired', { date: fDate(ask.closedAt ?? ask.expiresAt) })}</p>
        {:else if ask.state === 'retired'}
          <p>{t('inbox.sent.retired', { reason: ask.reason ?? '' })}</p>
        {/if}
        <ol class="hist">
          {#each ask.history as h, i (i)}
            <li>
              {t('inbox.sent.at', {
                what: EVENTS.includes(h.what) ? t(`inbox.sent.events.${h.what as 'read'}`) : h.what,
                when: `${fTime(h.at)}`,
              })}{h.detail ? ` · ${h.detail}` : ''}
            </li>
          {/each}
        </ol>
      </section>
    {/if}
  </div>

  {#if waiting}
    <form class="ff" onsubmit={(e) => (e.preventDefault(), primary('click'))} aria-labelledby="ask-title-{ask.id}">
      <div class="answer" data-testid="ask-answer-bar">
        {#if ask.kind === 'choice'}
          <div class="opts" role={ask.multi ? 'group' : 'radiogroup'} aria-label={t('inbox.answer.options')}>
            {#each ask.options as o, i (o.id)}
              {@const on = ask.multi ? chosen.includes(o.id) : focus === o.id}
              <button
                type="button"
                class="opt"
                class:on
                role={ask.multi ? 'checkbox' : 'radio'}
                aria-checked={on}
                aria-keyshortcuts={String(i + 1)}
                onclick={() => {
                  focus = o.id;
                  if (ask.multi) chosen = chosen.includes(o.id) ? chosen.filter((x) => x !== o.id) : [...chosen, o.id];
                }}
                data-testid="ask-option"
              >
                <span class="l">{letter(o.id)}</span><span class="ell">{o.label}</span>
                {#if ask.recommend?.option === o.id}<span class="pill rec">{t('inbox.ask.recommended')}</span>{/if}
              </button>
            {/each}
            <span class="sp"></span>
            {#if ask.files.some((f) => f.option !== null && f.kind === 'image')}
              <button type="button" class="btn ghost" onclick={compare} data-testid="ask-compare"
                ><Icon name="expand" size={13} />{t('inbox.answer.compare')}</button
              >
            {/if}
          </div>
        {/if}
        <div class="row">
          {#if ask.kind === 'answer'}
            <textarea
              class="field ta"
              rows="3"
              placeholder={t('inbox.answer.text')}
              aria-label={t('inbox.answer.text')}
              bind:value={text}
              bind:this={noteField}
              onkeydown={onFieldKey}
              data-testid="ask-text"></textarea>
          {:else}
            <span class="field">
              <input
                placeholder={ask.kind === 'task' ? t('inbox.answer.noteTask') : t('inbox.answer.note')}
                aria-label={ask.kind === 'task' ? t('inbox.answer.noteTask') : t('inbox.answer.note')}
                maxlength="2000"
                bind:value={note}
                bind:this={noteField}
                onkeydown={onFieldKey}
                data-testid="ask-note"
              />
              <button type="button" class="clip" aria-label={t('inbox.answer.attach')} onclick={() => fileInput?.click()}
                ><Icon name="clip" size={13} /></button
              >
            </span>
          {/if}
          <input
            class="vh"
            type="file"
            multiple
            bind:this={fileInput}
            onchange={(e) => attach(e.currentTarget.files)}
            tabindex="-1"
            aria-hidden="true"
          />
          <MicButton target={dictated} />
          {#if picking}
            <span class="pick">
              <input type="datetime-local" aria-label={t('inbox.later.pickTitle')} bind:value={pickAt} />
              <button type="button" class="btn" onclick={pickLater}>{t('inbox.later.pickOk')}</button>
              <button type="button" class="btn ghost" onclick={() => (picking = false)}>{t('inbox.later.cancel')}</button>
            </span>
          {:else}
            <button
              type="button"
              class="btn"
              bind:this={laterButton}
              aria-haspopup="menu"
              aria-keyshortcuts="L"
              onclick={(e) => laterMenu(e.currentTarget)}
              data-testid="ask-later"><Icon name="clock" size={13} />{t('inbox.answer.later')}<Icon name="down" size={11} /></button
            >
          {/if}
          {#if ask.kind === 'confirm'}
            <button type="button" class="btn" disabled={busy} onclick={() => send({ yes: false }, 'click')} data-testid="ask-no"
              >{ask.noLabel || t('inbox.answer.no')}</button
            >
            <button type="submit" class="btn primary" disabled={busy} data-testid="ask-yes"
              ><Icon name="check" size={13} />{ask.yesLabel || t('inbox.answer.yes')}<kbd>{t('inbox.answer.enter')}</kbd></button
            >
          {:else if ask.kind === 'task'}
            <button type="button" class="btn" disabled={busy} onclick={() => send({ done: false, steps }, 'click')} data-testid="ask-wont"
              >{t('inbox.answer.wontDo')}</button
            >
            <button type="submit" class="btn primary" disabled={busy} data-testid="ask-done"
              ><Icon name="check" size={13} />{t('inbox.answer.done')}<kbd>{t('inbox.answer.enter')}</kbd></button
            >
          {:else if ask.kind === 'answer'}
            <button type="submit" class="btn primary" disabled={busy || !text.trim()} data-testid="ask-send"
              >{t('inbox.answer.send')}<kbd>{t('inbox.answer.ctrlEnter')}</kbd></button
            >
          {:else}
            <button
              type="submit"
              class="btn primary"
              disabled={busy || (ask.multi ? !chosen.length && !focus : !focus)}
              data-testid="ask-choose"
            >
              {ask.multi && chosen.length > 1
                ? t('inbox.answer.chooseMany', { count: chosen.length })
                : t('inbox.answer.choose', { option: letter(focus) })}<kbd>{t('inbox.answer.enter')}</kbd>
            </button>
          {/if}
        </div>
        {#if files.length}
          <ul class="mine" aria-label={t('inbox.answer.attached')}>
            {#each files as f (f.path)}
              <li>
                <Icon name="clip" size={11} /><span class="ell">{f.name}</span>
                <button
                  type="button"
                  aria-label={t('inbox.answer.removeFile', { name: f.name })}
                  onclick={() => (files = files.filter((x) => x !== f))}><Icon name="close" size={11} /></button
                >
              </li>
            {/each}
          </ul>
        {/if}
        <div class="foot">
          <Icon name={ask.gate ? 'lock' : 'info'} size={12} />
          <span class="say">
            {#if ask.gate}{t('inbox.answer.gate', { agent: agentName })}
            {:else if ask.kind === 'task'}{t('inbox.answer.taskHint')}
            {:else if ask.author.agent}{t('inbox.answer.goesTo', { agent: agentName })}
            {:else}{t('inbox.answer.goesToClient')}{/if}
          </span>
          {#if chief && !ask.gate && ask.kind !== 'task' && !ask.delegatedTo}
            <button type="button" class="link" onclick={() => delegate(chief[0])} data-testid="ask-delegate"
              ><Icon name="user" size={12} />{t('inbox.answer.delegate', { name: chief[1] })}</button
            >
          {/if}
        </div>
      </div>
    </form>
  {/if}
</div>

{#if comparing && ask.kind === 'choice'}
  <CompareFull
    {ask}
    bind:focus
    bind:note
    onclose={() => (comparing = false)}
    onchoose={(id) => {
      comparing = false;
      void send({ options: [id] }, 'keyboard');
    }}
  />
{/if}

<style>
  .pane-ask {
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    min-height: 0;
    min-width: 0;
    flex: 1;
  }
  .fb {
    display: grid;
    align-content: start;
    gap: 16px;
    min-height: 0;
    padding: 22px 28px 14px;
    overflow-y: auto;
  }
  .rh {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .kind {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 550;
    white-space: nowrap;
  }
  .who {
    display: inline-flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px 6px;
    min-width: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .who b {
    color: var(--soft);
    font-weight: 600;
  }
  .tk {
    color: var(--soft);
    font-size: 11.5px;
    font-weight: 600;
  }
  .pdot {
    width: 8px;
    height: 8px;
    border-radius: 2.5px;
  }
  .sp {
    flex: 1;
  }
  h2 {
    margin: -4px 0 0;
    font-size: 21px;
    font-weight: 640;
    letter-spacing: -0.01em;
    line-height: 1.3;
    text-wrap: balance;
  }
  .body {
    max-width: 72ch;
    color: var(--text-ai);
    line-height: 1.6;
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 21px;
    padding: 0 7px;
    border-radius: 6px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
    font-size: 11.5px;
    font-weight: 600;
    white-space: nowrap;
  }
  .pill.wt {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--wait);
  }
  .pill.k {
    color: var(--muted);
    font-weight: 550;
  }
  .pill.rec {
    height: 19px;
    border-radius: 999px;
    background: oklch(from var(--brand) l c h / 0.08);
    box-shadow: inset 0 0 0 1px oklch(from var(--brand) l c h / 0.45);
    color: var(--teal-ink);
  }
  .band {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 8px 11px;
    border-radius: 9px;
    background: rgba(var(--fg-rgb), 0.025);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    font-size: 12.5px;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 0;
    border: none;
    background: none;
    color: var(--soft);
    font: 550 12px var(--font);
    cursor: pointer;
  }
  .link:hover {
    color: var(--text);
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .recline {
    display: flex;
    align-items: flex-start;
    gap: 9px;
    margin: 0;
    padding: 10px 12px;
    border-radius: 10px;
    background: oklch(from var(--brand) l c h / 0.06);
    box-shadow: inset 0 0 0 1px oklch(from var(--brand) l c h / 0.3);
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .recline :global(.icon) {
    margin-top: 2px;
    color: var(--teal-ink);
  }
  .recline b {
    color: var(--teal-ink);
    font-weight: 640;
  }
  .unblk {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    margin: 0;
    padding: 8px 11px;
    border-radius: 9px;
    background: rgba(var(--fg-rgb), 0.025);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    font-size: 12.5px;
  }
  .unblk > :global(.icon) {
    color: var(--teal-ink);
  }
  .unblk b {
    color: var(--text);
    font-weight: 600;
  }
  .tchip {
    display: inline-flex;
    align-items: center;
    height: 20px;
    padding: 0 6px;
    border-radius: 5px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
    font-size: 11.5px;
    font-weight: 600;
    white-space: nowrap;
  }
  .split {
    display: grid;
    grid-template-columns: minmax(0, 0.9fr) minmax(0, 1.1fr);
    gap: 22px;
    min-height: 420px;
  }
  .split .col {
    display: grid;
    align-content: start;
    gap: 14px;
    min-width: 0;
  }
  .steps {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .st label {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    align-items: start;
    gap: 10px;
    padding: 9px 11px;
    border-radius: 9px;
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    line-height: 1.5;
    cursor: pointer;
  }
  .st input {
    width: 16px;
    height: 16px;
    margin: 2px 0 0;
    accent-color: var(--run);
  }
  .st.done label {
    color: var(--muted);
  }
  .trace {
    display: grid;
    gap: 8px;
    padding: 12px 14px;
    border-radius: 10px;
    background: rgba(var(--fg-rgb), 0.025);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 12.5px;
  }
  .trace p {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 8px;
    margin: 0;
  }
  .trace .yours :global(.icon),
  .trace .ok {
    color: var(--run);
  }
  .trace .ok {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .muted {
    color: var(--muted);
  }
  .hist {
    margin: 4px 0 0;
    padding: 0 0 0 16px;
    color: var(--dim);
    font-size: 11.5px;
  }
  .ff {
    padding: 0 20px 16px;
  }
  .answer {
    display: grid;
    gap: 10px;
    min-width: 0;
    padding: 12px;
    border-radius: 12px;
    background: var(--raise);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 12px 32px -12px rgba(0, 0, 0, calc(0.5 * var(--shade)));
  }
  .opts {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .opt {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    max-width: 280px;
    height: 32px;
    padding: 0 12px 0 8px;
    border: none;
    border-radius: 8px;
    background: rgba(var(--fg-rgb), 0.03);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
    font: 550 12.5px var(--font);
    white-space: nowrap;
    cursor: pointer;
  }
  .opt .l {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    flex: none;
    border-radius: 6px;
    box-shadow: inset 0 0 0 1px var(--line-3);
    font-size: 11.5px;
    font-weight: 700;
  }
  .opt.on {
    background: oklch(from var(--brand) l c h / 0.08);
    box-shadow: inset 0 0 0 1.5px var(--brand);
    color: var(--text);
  }
  .opt.on .l {
    background: var(--brand);
    box-shadow: none;
    color: var(--frame);
  }
  .opt:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 1px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .field {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 32px;
    padding: 0 6px 0 10px;
    border-radius: 7px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .field:focus-within {
    box-shadow: inset 0 0 0 1.5px var(--p-ink);
  }
  .field input {
    flex: 1;
    min-width: 0;
    border: none;
    background: none;
    color: var(--text);
    font: 12.5px var(--font);
    outline: none;
  }
  .field.ta {
    height: auto;
    min-height: 64px;
    padding: 8px 10px;
    border: none;
    color: var(--text);
    font: 12.5px/1.5 var(--font);
    resize: vertical;
    outline: none;
  }
  .clip {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: none;
    color: var(--muted);
    cursor: pointer;
  }
  .clip:hover {
    background: var(--hover);
    color: var(--text);
  }
  .pick {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .pick input {
    height: 30px;
    padding: 0 8px;
    border: none;
    border-radius: 7px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font: 12px var(--font);
    color-scheme: inherit;
  }
  .btn kbd {
    margin-left: 4px;
  }
  .mine {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .mine li {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    max-width: 240px;
    height: 24px;
    padding: 0 4px 0 8px;
    border-radius: 6px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
    font-size: 12px;
  }
  .mine button {
    display: grid;
    place-items: center;
    padding: 2px;
    border: none;
    background: none;
    color: var(--muted);
    cursor: pointer;
  }
  .foot {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    min-width: 0;
    color: var(--dim);
    font-size: 11.5px;
  }
  .foot .say {
    flex: 1;
    min-width: 0;
    line-height: 1.45;
  }
  .foot > :global(.icon) {
    margin-top: 2px;
  }
  .foot .link {
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
  .vh {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  @media (max-width: 1100px) {
    .fb {
      gap: 12px;
      padding: 16px 20px 10px;
    }
    .split {
      grid-template-columns: minmax(0, 1fr);
    }
    .split :global(.pdfv) {
      min-height: 420px;
    }
    /* The seal on its plate already says the recommendation: the option keeps its name. */
    .opt .pill.rec,
    .row .btn kbd {
      display: none;
    }
    .opt {
      max-width: 200px;
      padding-right: 10px;
    }
    .opts .btn.ghost {
      padding: 0 8px;
    }
  }
</style>
