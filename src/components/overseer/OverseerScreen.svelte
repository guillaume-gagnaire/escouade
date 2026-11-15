<script lang="ts">
  import { untrack } from 'svelte';
  import { withoutInstruction } from '../../lib/conversation';
  import { conversationOf } from '../../lib/conversations.svelte';
  import { fBytes, fTime } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { voiceServices } from '../../lib/installed';
  import { api } from '../../lib/ipc';
  import { lockLine, locks } from '../../lib/locks.svelte';
  import { orbitTasks, place, type Placed } from '../../lib/orbit';
  import { isWake, overseer, userWords } from '../../lib/overseer.svelte';
  import { DICTATION_KEY, dictationCut, overseerVoice } from '../../lib/overseer-voice.svelte';
  import { keyLabel } from '../../lib/platform';
  import { app } from '../../lib/state.svelte';
  import type { ConvItem, OverseerAction, OverseerNews } from '../../lib/types';
  import Markdown from '../conv/Markdown.svelte';
  import Icon from '../Icon.svelte';
  import Directives from './Directives.svelte';
  import Orbit from './Orbit.svelte';

  // The Overseer's screen (ESC-468, docs/OVERSEER.md « Son écran », design/overseer/ecran-discussion.png): the orbit on
  // the left, the tasks of every project around its orb; the discussion on the right, a pane of glass as an agent's
  // conversation: the user's messages (« dit » or « tapé », their time), its answers and under them what it did, with
  // « Annuler » or « Retirer », the news as cards with ready answers, its instructions (« Consignes · n »); the field at
  // the bottom: Enter sends, Space held (the field empty) or the microphone speaks. Escape closes it.

  const view = $derived(overseer.view);
  const agentId = $derived(view.agentId);
  const conv = $derived(agentId ? conversationOf(agentId) : null);
  $effect(() => conv?.show());
  const agent = $derived(agentId ? app.agents[agentId] : undefined);
  const working = $derived(agent?.status === 'running');
  const voice = overseerVoice;
  const cut = $derived(dictationCut());
  const cutBy = $derived(locks.current.locks[DICTATION_KEY]?.by ?? null);
  // The voice needs Escouade's own (Whisper and Piper, not on Intel Macs) or a service an installed plugin brings.
  let localSpeech = $state(true);
  $effect(() => {
    void api
      .speechInfo()
      .then((i) => (localSpeech = i.available))
      .catch(() => {});
  });
  const canTalk = $derived(localSpeech || voiceServices().length > 0);
  let text = $state('');
  let showDirectives = $state(false);
  let list = $state<HTMLDivElement>();
  let field = $state<HTMLTextAreaElement>();
  let now = $state(Date.now());
  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });

  // ---------- the orbit ----------

  const projects = $derived(app.projects.filter((p) => !p.private));
  const tasks = $derived(place(orbitTasks(app.projects, Object.values(app.tickets), Object.values(app.agents), view.news ?? [], now)));

  /** A card clicked: its agent's conversation (a ticket's, or one alone), else its ticket on its project's board. */
  function openTask(x: Placed) {
    overseer.open = false;
    if (x.agentId && app.agents[x.agentId] && !app.agents[x.agentId].archived) return app.selectAgent(x.agentId);
    app.openBoard(x.projectId);
  }

  // ---------- the discussion ----------

  /** Its answer to a wake-up with nothing to tell: not shown. */
  const QUIET = /^(RAS|Nothing new).?$/i;

  interface Line {
    kind: 'user' | 'text' | 'news';
    id: string;
    text: string;
    said?: boolean;
    ts?: number;
    news?: OverseerNews;
  }

  /** The user's words (their header and the voice's instruction taken off), its answers; its wake-ups left out. */
  function lines(items: ConvItem[]): Line[] {
    const out: Line[] = [];
    for (const it of items) {
      if (it.kind === 'user') {
        if (isWake(it.text)) continue;
        const words = userWords(it.text);
        const said = words ? view.messages.find((m) => m.id === words.id)?.said : false;
        out.push({ kind: 'user', id: it.id, text: withoutInstruction(words?.text ?? it.text).text, said, ts: it.ts });
      } else if (it.kind === 'text' && it.text.trim() && !it.parent && !QUIET.test(it.text.trim())) {
        out.push({ kind: 'text', id: it.id, text: it.text });
      }
    }
    return out;
  }

  /** The lines with the news among them by their time, and after each line what was done until the next one. */
  const thread = $derived.by(() => {
    const ls = lines(conv?.items ?? []);
    // A piece of news goes before the first message of the user that came after it.
    for (const n of view.news ?? []) {
      const i = ls.findIndex((l) => l.ts !== undefined && l.ts > n.at);
      const card: Line = { kind: 'news', id: n.id, text: n.title, ts: n.at, news: n };
      if (i < 0) ls.push(card);
      else ls.splice(i, 0, card);
    }
    const starts = ls.map((l, i) => ({ i, ts: l.ts })).filter((x) => x.ts !== undefined) as { i: number; ts: number }[];
    const after = new Map<number, OverseerAction[]>();
    for (const a of view.actions) {
      const at = [...starts].reverse().find((s) => s.ts <= a.at);
      const key = at ? (starts.find((s) => s.i > at.i)?.i ?? ls.length) : ls.length;
      after.set(key, [...(after.get(key) ?? []), a]);
    }
    return { ls, after };
  });

  const outcomeWord = (a: OverseerAction) => (a.outcome === 'done' ? '' : t(`overseer.panel.${a.outcome}` as 'overseer.panel.undone'));

  async function send() {
    const words = text;
    if (!words.trim() || overseer.sending) return;
    voice.interrupt();
    text = '';
    if (!(await overseer.send(words)) && !text) text = words;
  }

  /** A ready answer of a piece of news: a message to it, or the ticket's diff shown in the window. */
  function answer(n: OverseerNews, what: 'approve' | 'showDiff' | 'resend' | 'look') {
    if (what === 'showDiff') {
      const agentId = app.tickets[n.ticketId]?.agentId ?? null;
      overseer.open = false;
      if (!agentId) return app.openBoard(n.projectId);
      app.modal = { kind: 'diff', projectId: n.projectId, agentId, paths: [], title: t('git.files.changesOf', { name: n.ticket }) };
      return;
    }
    void overseer.send(t(`overseer.news.${what}Say`, { key: n.ticket }));
  }

  const answers = (n: OverseerNews) =>
    n.kind === 'review'
      ? (['approve', 'showDiff', 'resend'] as const)
      : n.kind === 'done'
        ? (['showDiff'] as const)
        : (['look', 'showDiff'] as const);

  $effect(() => {
    void thread.ls.length;
    void view.actions.length;
    void voice.heard;
    queueMicrotask(() => list?.scrollTo({ top: list.scrollHeight }));
  });

  // Its conversation as it changes: the end of its turn has its answer read aloud.
  $effect(() => {
    const items = conv?.items ?? [];
    void items.length;
    untrack(() => voice.observe(items));
  });

  // The screen gone: the microphone closes.
  $effect(() => () => voice.stop());

  // ---------- the keys ----------

  /** Space held in the empty field (or with nothing focused) speaks; let go, it sends; Escape cancels, then closes. */
  function onKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      if (voice.phase === 'talking' || voice.phase === 'sending' || voice.phase === 'speaking') voice.cancel();
      else overseer.open = false;
      return;
    }
    if (e.code !== 'Space' || e.ctrlKey || e.metaKey || e.altKey || !agentId || !canTalk || cut) return;
    const target = e.target as Element | null;
    const inField = target === field;
    const free = target === document.body || !!target?.closest('[data-overseer-screen]');
    // A button or a link that has the focus keeps Space: it presses it.
    const typing =
      target instanceof HTMLInputElement ||
      (target instanceof HTMLTextAreaElement && !inField) ||
      !!target?.closest('button, a, [role="button"], select');
    if (typing || !free || (inField && text !== '')) return;
    e.preventDefault();
    if (e.repeat) return;
    void voice.press(agentId);
  }

  function onKeyUp(e: KeyboardEvent) {
    if (e.code !== 'Space') return;
    if (voice.phase === 'talking' || voice.phase === 'starting') {
      e.preventDefault();
      voice.release();
    }
  }

  const talking = $derived(voice.phase === 'talking' || voice.phase === 'sending');
  /** The bars of the wave the field becomes while the user speaks: the level, shaped. */
  const bars = $derived(
    Array.from({ length: 28 }, (_, i) => 0.25 + Math.abs(Math.sin(i * 1.7 + voice.level * 6)) * voice.level * 0.75 + 0.1),
  );
  const model = $derived(app.settings.overseer?.model ?? '');

  /** A hint around a key: its words before and after it (« Relâche [Espace] pour envoyer »). */
  function around(key: 'hintRelease' | 'hintCancel' | 'hintSpace'): [string, string] {
    const [a, b = ''] = t(`overseer.screen.${key}`, { key: '\u0000' }).split('\u0000');
    return [a, b];
  }
</script>

<svelte:window onkeydowncapture={onKeyDown} onkeyupcapture={onKeyUp} />

{#snippet actions(list: OverseerAction[] | undefined)}
  {#if list?.length}
    <ul class="acts">
      {#each list as a (a.id)}
        <li class="act {a.outcome}" data-action={a.tool} data-outcome={a.outcome}>
          <Icon name={a.outcome === 'refused' || a.outcome === 'failed' ? 'shield' : 'check'} size={13} />
          <span class="what">{a.text}</span>
          {#if outcomeWord(a)}<span class="state">{outcomeWord(a)}</span>{/if}
          {#if a.undoable}
            <button class="link" onclick={() => overseer.undo(a.id)}
              >{a.tool === 'add_directive' ? t('overseer.screen.remove') : t('overseer.panel.undo')}</button
            >
          {/if}
          {#if a.why}<p class="why">{a.why}</p>{/if}
        </li>
      {/each}
    </ul>
  {/if}
{/snippet}

<div class="screen" role="dialog" aria-modal="true" aria-label={t('overseer.screen.label')} data-overseer-screen data-overseer-panel>
  <section class="left">
    <header class="top">
      <b>{t('overseer.screen.label')}</b>
      <span class="muted">· {t('overseer.screen.projects', { count: projects.length })}{model ? ` · ${model}` : ''}</span>
    </header>
    <Orbit
      {tasks}
      phase={voice.phase}
      level={voice.level}
      played={voice.played}
      heard={voice.heard?.text ?? null}
      subtitle={voice.subtitle}
      thinking={working}
      open={openTask}
    />
  </section>

  <section class="glass" class:dim={talking}>
    <header class="top right">
      <button class="chip" aria-expanded={showDirectives} onclick={() => (showDirectives = !showDirectives)} data-directives-chip
        >{t('overseer.panel.directives', { n: view.directives.length })}</button
      >
      <span class="quit"><kbd>{keyLabel('Esc')}</kbd> {t('overseer.screen.quit')}</span>
      <button class="icon-btn" aria-label={t('overseer.panel.close')} onclick={() => (overseer.open = false)}>
        <Icon name="close" size={14} />
      </button>
    </header>
    {#if showDirectives}
      <div class="directives"><Directives /></div>
    {/if}
    {#if !view.running}
      <div class="off">
        <p>{view.enabled && view.needsMcp ? t('overseer.settings.needsMcp') : t('overseer.panel.off')}</p>
        <button
          class="btn"
          onclick={() => {
            overseer.open = false;
            app.modal = { kind: 'settings', page: 'overseer' };
          }}>{t('overseer.panel.settings')}</button
        >
      </div>
    {:else}
      <div class="thread" bind:this={list} data-overseer-thread>
        {#if !thread.ls.length && !view.actions.length}
          <p class="empty">{t('overseer.panel.empty')}</p>
        {/if}
        {#each thread.ls as l, i (l.id)}
          {@render actions(thread.after.get(i))}
          {#if l.kind === 'user'}
            <div class="msg user" data-user-message>
              <p>{l.text}</p>
              <span class="meta"
                ><Icon name={l.said ? 'mic' : 'edit'} size={11} />{l.said ? t('overseer.panel.said') : t('overseer.panel.typed')}{l.ts
                  ? ` · ${fTime(l.ts)}`
                  : ''}</span
              >
            </div>
          {:else if l.kind === 'news' && l.news}
            {@const n = l.news}
            <article class="news {n.kind}" data-news={n.kind} data-news-ticket={n.ticket}>
              <span class="label">{t(`overseer.news.${n.kind}`)} · {fTime(n.at)}</span>
              <b class="ttl">{n.ticket} · {n.title}</b>
              <p>
                {n.text || (n.kind === 'review' ? t('overseer.news.reviewText') : n.kind === 'done' ? t('overseer.news.doneText') : '')}
              </p>
              <span class="proj">{n.project}</span>
              <div class="ready">
                {#each answers(n) as what (what)}
                  <button class="pill" data-ready={what} onclick={() => answer(n, what)}>{t(`overseer.news.${what}`)}</button>
                {/each}
              </div>
            </article>
          {:else}
            <div class="reply" data-overseer-reply>
              <span class="who">{t('overseer.screen.he')}</span>
              <Markdown text={l.text} {agentId} />
            </div>
          {/if}
        {/each}
        {@render actions(thread.after.get(thread.ls.length))}
        {#if voice.heard && (voice.phase === 'talking' || voice.phase === 'sending' || voice.phase === 'waiting')}
          <div class="msg user live" data-user-live>
            <p>{voice.heard.text || '…'}</p>
            <span class="meta"><Icon name="mic" size={11} />{t('overseer.screen.live')}</span>
          </div>
        {/if}
        {#if voice.phase === 'speaking'}
          <span class="who speaking" data-he-speaks
            >{t('overseer.screen.he')} · <Icon name="wave" size={11} /> {t('overseer.screen.heSpeaking')}</span
          >
        {:else if working}
          <p class="working">{t('overseer.panel.working')}</p>
        {/if}
      </div>
      <form
        class="ask"
        class:talking
        onsubmit={(e) => {
          e.preventDefault();
          void send();
        }}
      >
        {#if talking}
          <div class="wave" role="img" aria-label={t('overseer.screen.wave')} data-wave>
            {#each bars as b, i (i)}<i style:transform="scaleY({b})"></i>{/each}
          </div>
        {:else}
          <textarea
            bind:this={field}
            rows="2"
            bind:value={text}
            placeholder={t('overseer.panel.placeholder')}
            aria-label={t('overseer.panel.placeholder')}
            oninput={() => voice.interrupt()}
            onkeydown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                void send();
              }
            }}></textarea>
        {/if}
        <div class="foot">
          <span class="hints">
            {#if talking}
              <span>{around('hintRelease')[0]}<kbd>{t('voice.keys.space')}</kbd>{around('hintRelease')[1]}</span>
              <span>{around('hintCancel')[0]}<kbd>{keyLabel('Esc')}</kbd>{around('hintCancel')[1]}</span>
            {:else if voice.phase === 'speaking'}
              <span>{t('overseer.screen.hintInterrupt')}</span>
            {:else}
              <span>{t('overseer.screen.hintEnter')}</span>
              {#if canTalk && !cut}<span>{around('hintSpace')[0]}<kbd>{t('voice.keys.space')}</kbd>{around('hintSpace')[1]}</span>{/if}
            {/if}
          </span>
          {#if canTalk}
            <button
              type="button"
              class="mic"
              class:on={talking}
              disabled={cut || !agentId}
              aria-pressed={talking}
              aria-label={t('overseer.screen.mic')}
              data-overseer-mic
              onpointerdown={() => agentId && void voice.press(agentId)}
              onpointerup={() => voice.release()}
            >
              <Icon name="mic" size={15} />
            </button>
          {/if}
          {#if !talking}
            <button class="btn primary" type="submit" disabled={!text.trim() || overseer.sending}>{t('overseer.panel.send')}</button>
          {/if}
        </div>
        {#if cut}
          <p class="note" data-voice-cut>{t('overseer.voice.cut')}{cutBy ? ` (${lockLine(cutBy)})` : ''}</p>
        {:else if !canTalk}
          <p class="note" data-no-voice>
            {t('overseer.screen.noVoice')}
            <button
              type="button"
              class="link"
              onclick={() => {
                overseer.open = false;
                app.openPlugins();
              }}>{t('overseer.screen.noVoiceOpen')}</button
            >
          </p>
        {:else if voice.phase === 'offer' && voice.offer}
          <p class="note" data-voice-offer>
            {voice.offer.kind === 'model'
              ? t('overseer.voice.offerModel', { model: voice.offer.id, size: fBytes(voice.offer.size) })
              : t('overseer.voice.offerVoice', { voice: voice.offer.id, size: fBytes(voice.offer.size) })}
            <button type="button" class="link" onclick={() => voice.download()}>{t('overseer.voice.download')}</button>
          </p>
        {:else if voice.phase === 'downloading'}
          <p class="note">{t('overseer.voice.downloading')}</p>
        {/if}
      </form>
      {#if overseer.error}<p class="error" role="alert">{overseer.error}</p>{/if}
    {/if}
  </section>
</div>

<style>
  /* The whole window: the orbit on the left, the discussion on the right (direction A, ecran-discussion.png). */
  .screen {
    --ov: var(--brand);
    position: fixed;
    inset: 0;
    z-index: 880;
    display: grid;
    grid-template-columns: minmax(0, 1fr) clamp(340px, 34vw, 480px);
    background: radial-gradient(60% 70% at 36% 52%, color-mix(in oklch, var(--ov) 9%, transparent), transparent 72%), var(--vo-base);
    color: var(--text);
    animation: in 0.25s ease-out;
  }
  @keyframes in {
    from {
      opacity: 0;
    }
  }
  .left {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-width: 0;
    min-height: 0;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 52px;
    padding: 0 22px;
    font-size: 13px;
  }
  .top b {
    font-weight: 650;
  }
  .muted {
    color: var(--muted);
  }
  .right {
    justify-content: flex-end;
    padding: 0 16px 0 22px;
  }
  .quit {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 12px;
  }
  kbd {
    display: inline-block;
    padding: 0 5px;
    margin: 0 4px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    font: inherit;
    font-size: 11.5px;
    color: var(--soft);
  }
  .chip {
    padding: 2px 9px;
    border: none;
    border-radius: 999px;
    background: none;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .chip:hover,
  .chip[aria-expanded='true'] {
    color: var(--text);
  }
  /* The discussion: a pane of glass, as an agent's conversation. */
  /* A column: its head, the instructions unfolded, the thread that scrolls, the field always in view. */
  .glass {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-left: 1px solid rgba(var(--fg-rgb), 0.07);
    background: color-mix(in oklch, var(--panel) 72%, transparent);
    backdrop-filter: blur(18px);
  }
  .glass > .directives {
    padding: 4px 18px 12px;
    max-height: 36vh;
    overflow: auto;
    border-bottom: 1px solid var(--line);
  }
  .off,
  .empty {
    padding: 18px 22px;
    color: var(--muted);
  }
  .thread {
    flex: 1 1 0;
    min-height: 0;
    overflow: auto;
    padding: 6px 18px 14px 22px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    transition: opacity 0.3s;
  }
  /* While the user speaks the rest of the screen fades a little. */
  .dim .thread > :not([data-user-live]) {
    opacity: 0.55;
  }
  .msg {
    align-self: flex-end;
    max-width: 88%;
    padding: 10px 14px 8px;
    border-radius: 10px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
    overflow-wrap: anywhere;
  }
  .msg p {
    margin: 0;
    white-space: pre-wrap;
    line-height: 1.45;
  }
  .meta {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-top: 4px;
    color: var(--muted);
    font-size: 11.5px;
  }
  .msg.live {
    background: color-mix(in oklch, var(--ov) 8%, var(--raise));
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--ov) 55%, transparent);
  }
  .who {
    display: block;
    margin-bottom: 4px;
    color: var(--ov);
    font-size: 11.5px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .who.speaking {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .reply {
    overflow-wrap: anywhere;
  }
  .acts {
    list-style: none;
    margin: -6px 0 0;
    padding: 2px 0 2px 12px;
    border-left: 2px solid color-mix(in oklch, var(--run) 50%, transparent);
    display: grid;
    gap: 4px;
  }
  .act {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 6px;
    font-size: 13px;
    color: var(--soft);
  }
  .act :global(svg) {
    color: var(--run);
    align-self: center;
  }
  .act.refused :global(svg),
  .act.failed :global(svg) {
    color: var(--del);
  }
  .act.undone {
    opacity: 0.6;
  }
  .what {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .state {
    color: var(--muted);
    font-size: 12px;
  }
  .act .why {
    flex-basis: 100%;
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--teal-ink);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }
  /* A piece of news: a card, with ready answers. */
  .news {
    display: grid;
    gap: 6px;
    padding: 12px 14px;
    border-radius: 10px;
    background: color-mix(in oklch, var(--run) 5%, var(--raise));
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--run) 40%, transparent);
  }
  .news.blocked,
  .news.ci {
    background: color-mix(in oklch, var(--wait) 5%, var(--raise));
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .news .label {
    color: var(--run);
    font-size: 11.5px;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }
  .news.blocked .label,
  .news.ci .label {
    color: var(--wait);
  }
  .news p {
    margin: 0;
    color: var(--soft);
    font-size: 13px;
    overflow-wrap: anywhere;
  }
  .proj {
    color: var(--muted);
    font-size: 11.5px;
  }
  .ready {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 2px;
  }
  .pill {
    padding: 3px 10px;
    border: none;
    border-radius: 7px;
    background: rgba(var(--fg-rgb), 0.03);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
  }
  .pill:hover {
    background: rgba(var(--fg-rgb), 0.07);
  }
  .working {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  /* The field: what is typed, or the wave of the voice. */
  .ask {
    flex: none;
    display: grid;
    gap: 8px;
    margin: 0 16px 16px;
    padding: 12px 12px 10px 16px;
    border-radius: 12px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .ask.talking {
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--ov) 60%, transparent);
  }
  textarea {
    resize: none;
    padding: 0;
    border: none;
    background: none;
    color: inherit;
    font: inherit;
    font-size: 14px;
    outline: none;
  }
  .wave {
    display: flex;
    align-items: center;
    gap: 3px;
    height: 44px;
  }
  .wave i {
    width: 3px;
    height: 100%;
    border-radius: 2px;
    background: var(--ov);
    transform-origin: center;
    transition: transform 0.08s linear;
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .hints {
    flex: 1;
    display: flex;
    flex-wrap: wrap;
    gap: 2px 14px;
    color: var(--muted);
    font-size: 12px;
  }
  .mic {
    flex: none;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border: none;
    border-radius: 50%;
    background: color-mix(in oklch, var(--ov) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--ov) 40%, transparent);
    color: var(--ov);
    cursor: pointer;
  }
  .mic.on {
    background: var(--ov);
    color: var(--vo-base);
  }
  .mic:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .note {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .error {
    margin: 0 18px 12px;
    color: var(--del);
    font-size: 12px;
  }
  /* Narrow: the discussion under the orbit. */
  @media (max-width: 760px) {
    .screen {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: minmax(0, 0.9fr) minmax(0, 1.1fr);
    }
    .glass {
      border-left: none;
      border-top: 1px solid rgba(var(--fg-rgb), 0.07);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .screen,
    .wave i {
      animation: none;
      transition: none;
    }
  }
</style>
