<script lang="ts" module>
  import { disposeTerminal, getXTerm, openAgentTerminal, type Room } from '../lib/terminals';

  /** The interface being started, by agent: a view shown twice (classic, then split) starts it once. */
  const starting = new Map<string, Promise<string>>();

  /**
   * The terminal of the agent's Claude Code interface: the one it has when the window holds it, else a new one (the
   * backend stops the one it had, which this window cannot show: a reload lost its output), started at the size of `room`,
   * where it is about to be shown. `manual`: the user asked for it (« Relancer »); the backend refuses one the view starts
   * of its own accord after two stops in a row as soon as it started.
   */
  export function agentTerminal(agentId: string, current: string | null, room?: Room | null, manual = false): Promise<string> {
    if (current && getXTerm(current)) return Promise.resolve(current);
    let p = starting.get(agentId);
    if (!p) {
      p = openAgentTerminal(agentId, room, manual)
        .then((info) => {
          // The one before, if this window still held it, is gone.
          if (current && current !== info.id) disposeTerminal(current);
          return info.id;
        })
        .finally(() => starting.delete(agentId));
      starting.set(agentId, p);
    }
    return p;
  }
</script>

<script lang="ts">
  import { untrack } from 'svelte';
  import { t } from '../lib/i18n';
  import type { DictationTarget } from '../lib/dictation.svelte';
  import { mountTerminal, roomOf, typeIntoTerminal } from '../lib/terminals';
  import MicButton from './MicButton.svelte';
  import { app } from '../lib/state.svelte';
  import { engineName, speakerOf } from '../lib/engines';
  import Rich from '../lib/i18n/Rich.svelte';
  import { api } from '../lib/ipc';
  import Icon from './Icon.svelte';
  import type { Agent } from '../lib/types';

  // « Vue Claude Code »: the agent's conversation in Claude Code's own interface. Escouade still follows it (status,
  // notifications, cost, plan) through its hooks and its transcript.
  let { agent }: { agent: Agent } = $props();
  /** Its harness, as the texts name it. */
  const engine = $derived(engineName(agent.engine));

  /** How often the view asks whether the interface still asks about the trust of its folder. */
  const TRUST_POLL_MS = 1500;

  /** How long the note about a session not found stays, once the interface is up on the new one. */
  const LOST_NOTE_MS = 6000;

  let box = $state<HTMLDivElement>();
  let term = $state<string | null>(null);
  let error = $state<string | null>(null);
  /** How many times the interface was started again (« Relancer », or of its own accord). */
  let asked = $state(0);
  /** Whether the next start is the user's (« Relancer »), not the view's own. */
  let manual = false;
  /** The view started the interface again by itself, the session not found: it does not twice. */
  let retried = $state(false);
  /** The session was not found and the interface starts on a new one: the note says so. */
  let lost = $state(false);

  // Who the agent is, not the object that stands for it: the app replaces that object with each update of the agent,
  // the end of the interface among them (its terminal is null then). An effect reading the object would run again with
  // each one and start an interface that stops, whose end is another update... (ESC-106)
  const agentId = $derived(agent.id);

  const exited = $derived(term !== null && app.exitedTerms[term] !== undefined);
  // How it ended, as the backend tells it: with the agent, which comes after the exit of the terminal.
  const stop = $derived(exited ? agent.tuiExit : null);
  /** It stopped as soon as it started, the session not found, and the view has yet to start it again by itself. */
  const retrying = $derived(!!stop?.quick && stop.sessionLost && !retried);

  /**
   * Its harness: Codex, Copilot CLI and Cursor CLI ask about the trust of the folder, OpenCode's view keeps the thread
   * (choice 37), Cursor's follows less (ESC-49).
   */
  const engineId = $derived(agent.engine);
  const asksAboutFolders = $derived(engineId === 'codex' || engineId === 'copilot' || engineId === 'cursor');
  /** What to do about it, in its words. */
  const trustText = $derived(
    engineId === 'codex'
      ? t('conv.terminal.trust.codex')
      : engineId === 'cursor'
        ? t('conv.terminal.trust.cursor')
        : t('conv.terminal.trust.copilot'),
  );
  /** The interface asks whether to trust the agent's folder: the amber band above the terminal says it. */
  let asksTrust = $state(false);

  // Asked of the backend while the interface is up, until it no longer asks (once answered, it never asks again).
  $effect(() => {
    const id = agentId;
    if (!term || exited || !asksAboutFolders) {
      asksTrust = false;
      return;
    }
    let gone = false;
    let timer: ReturnType<typeof setInterval> | undefined;
    const check = () =>
      api
        .tuiAsksTrust(id)
        .then((asks) => {
          if (gone) return;
          asksTrust = asks;
          if (!asks) clearInterval(timer);
        })
        .catch(() => {});
    void check();
    timer = setInterval(check, TRUST_POLL_MS);
    return () => {
      gone = true;
      clearInterval(timer);
    };
  });

  // Its interface, started when the view opens, and again on « Relancer ».
  $effect(() => {
    const id = agentId;
    void asked;
    const current = untrack(() => agent.terminalId);
    // Its interface starts at the size of its box: Claude Code draws on the rows of its pseudo-console, so one started at some
    // other size stays drawn on fewer rows than the box has until the resize reaches it (a busy window, a message that fails).
    const room = untrack(() => (box ? roomOf(box) : null));
    error = null;
    let gone = false;
    // Asked once: another agent shown in this view (same component) starts its interface of the view's own accord.
    const byUser = manual;
    manual = false;
    agentTerminal(id, current, room, byUser)
      .then((tid) => {
        if (!gone) term = tid;
      })
      .catch((e) => {
        if (!gone) error = String(e);
      });
    return () => {
      gone = true;
    };
  });

  $effect(() => {
    const id = term;
    const el = box;
    if (!id || !el) return;
    mountTerminal(id, el);
    const ro = new ResizeObserver(() => {
      try {
        getXTerm(id)?.fit.fit();
      } catch {
        /* hidden */
      }
    });
    ro.observe(el);
    return () => {
      ro.disconnect();
      mountTerminal(id, null);
    };
  });

  // Dictated text is typed into Claude Code's prompt, as a paste: it is never sent by itself.
  const dictated: DictationTarget = {
    kind: 'terminal',
    id: 'terminal-' + Math.random().toString(36).slice(2),
    el: () => box?.closest('.tui'),
    insert: (said) => {
      if (term) void typeIntoTerminal(term, said.trim());
    },
  };

  /** The old interface is being stopped: the new one is not started before, or the backend would attach to the old one. */
  let restarting = false;

  // The one it shows is gone (stopped first, waited for); a new interface takes its place.
  async function start(byUser: boolean) {
    if (restarting) return;
    restarting = true;
    const old = term;
    term = null;
    manual = byUser;
    try {
      if (old) await disposeTerminal(old);
    } finally {
      restarting = false;
    }
    asked += 1;
  }

  // « Relancer »: a new try, which may be started again by the view if the session is not found.
  function restart() {
    retried = false;
    lost = false;
    void start(true);
  }

  // « Relancer » of « Outils d’Escouade au prochain démarrage », asked from the conversation's banner: a new interface on
  // the same session. What was asked before this view showed it is not asked of it.
  let relaunched = untrack(() => app.relaunched[agentId] ?? 0);
  $effect(() => {
    const n = app.relaunched[agentId] ?? 0;
    if (n === relaunched) return;
    relaunched = n;
    untrack(restart);
  });

  // Claude Code found no conversation to resume and stopped as soon as it started: the backend already replaced the
  // session, so the interface starts again, on the new one. Once: if that one stops as soon as it starts too, the user
  // reads its output and decides (never a loop). Only the stop of the interface shown counts: `exited` is false for
  // the new one, whatever the agent still says of the one before.
  $effect(() => {
    if (!retrying) return;
    untrack(() => {
      retried = true;
      lost = true;
      void start(false);
    });
  });

  // The note goes a few seconds after the interface is up on the new session.
  $effect(() => {
    if (!lost || !term) return;
    const timer = setTimeout(() => (lost = false), LOST_NOTE_MS);
    return () => clearTimeout(timer);
  });
</script>

<div class="tui">
  {#if asksTrust}
    <div class="band trust" role="status">
      <Icon name="shield" size={14} />
      <span class="grow"
        ><b>{t('conv.terminal.trust.title', { engine })}</b>
        {trustText}</span
      >
    </div>
  {:else if engineId === 'cursor'}
    <!-- What its transcript tells, and what it does not: the band says it until the view goes. -->
    <div class="band limited" role="note">
      <Icon name="info" size={14} />
      <span class="grow"><Rich k="conv.terminal.limited">{#snippet title()}<b>{t('conv.terminal.limitedTitle')}</b>{/snippet}</Rich></span>
    </div>
  {:else if engineId === 'opencode'}
    <div class="band keeps" role="note">
      <Icon name="check-c" size={14} />
      <span class="grow"><Rich k="conv.terminal.keeps">{#snippet title()}<b>{t('conv.terminal.keepsTitle')}</b>{/snippet}</Rich></span>
    </div>
  {/if}
  <!-- The terminal and its microphone, over its top right corner: under the band, if there is one. -->
  <div class="well">
    <div class="box" bind:this={box} aria-label={t('conv.header.toTerminal', { engine })}></div>
    {#if term && !exited}
      <div class="voice"><MicButton target={dictated} below /></div>
    {/if}
  </div>
  {#if error}
    <div class="note err" role="alert">
      {t('conv.terminal.failed', { error, engine })}
      <button class="btn" onclick={restart}>{t('conv.terminal.restart')}</button>
    </div>
  {:else if exited}
    <!-- Nothing while it starts again by itself: the next note is the one of its new session. -->
    {#if !retrying}
      <div class="note">
        {#if !stop?.quick}
          {t('conv.terminal.exited', { engine })}
        {:else if typeof stop.code === 'number'}
          {t('conv.terminal.exitedAtOnce', { code: stop.code, engine })}
        {:else}
          {t('conv.terminal.exitedAtOnceNoCode', { engine })}
        {/if}
        <button class="btn" onclick={restart}>{t('conv.terminal.restart')}</button>
      </div>
    {/if}
  {:else if lost}
    <div class="note">{t('conv.terminal.sessionLost', { speaker: speakerOf(agent.engine) })}</div>
  {:else if !term}
    <div class="note">{t('conv.terminal.starting', { engine })}</div>
  {/if}
</div>

<style>
  .tui {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .well {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .box {
    flex: 1;
    min-height: 0;
    padding: 16px 8px 8px 22px;
    background: var(--well);
  }
  /* Under a band, the terminal starts below it, the panel between them. */
  .band + .well {
    margin-top: 10px;
  }
  /* Above the terminal: what the harness asks there (amber), or what Escouade keeps doing meanwhile (OpenCode). */
  .band {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 10px 14px 0;
    padding: 9px 12px;
    border-radius: 10px;
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .band b {
    font-weight: 700;
  }
  .band .grow {
    flex: 1;
    min-width: 0;
  }
  .trust {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .trust > :global(.icon) {
    flex: none;
    color: var(--wait);
  }
  .keeps,
  .limited {
    margin: 10px 0 0;
    padding: 8px 10px 8px 12px;
    background: rgba(var(--fg-rgb), 0.035);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .keeps > :global(.icon) {
    flex: none;
    color: var(--run);
  }
  .limited > :global(.icon) {
    flex: none;
    color: var(--muted);
  }
  /* Over the top right corner of the terminal, above its layers. */
  .voice {
    position: absolute;
    top: 8px;
    right: 14px;
    z-index: 5;
  }
  .voice :global(.mic) {
    background: var(--panel);
    border: 1px solid var(--line);
  }
  .note {
    position: absolute;
    left: 50%;
    bottom: 16px;
    transform: translateX(-50%);
    /* Above the terminal's layers (xterm's link layer is a canvas over the whole box): « Relancer » is clicked, not the layer. */
    z-index: 5;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 14px;
    border-radius: var(--r);
    border: 1px solid var(--line);
    background: var(--panel);
    font-size: 12.5px;
    color: var(--muted);
  }
  .note.err {
    color: var(--del);
  }
</style>
