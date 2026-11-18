<script lang="ts">
  import { t } from '../../lib/i18n';
  import type { IconName } from '../../lib/icons';
  import { inbox, itemKey, stoppedTitle, stoppedWhat } from '../../lib/inbox/inbox.svelte';
  import { goToSource, placeOf } from '../../lib/inbox/act';
  import { api } from '../../lib/ipc';
  import { menu } from '../../lib/menu.svelte';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import Icon from '../Icon.svelte';
  import AskPane from './AskPane.svelte';
  import InboxViewPane from './InboxViewPane.svelte';

  // The panel of the box « À toi » (direction B, `b-pleine.png`): its bar (« ‹ 3 sur 9 › », J suivante, K précédente,
  // L plus tard; « Aller à l'agent », « Le ticket »), then one request at a time, in full; the box empty when nothing
  // waits (`b-vide.png`). An answer sent opens the next one; its 10 seconds show at the bottom, with « Annuler ».

  const cur = $derived(inbox.current);
  const pos = $derived(inbox.position());

  // Opened on the first that waits, and again when the one open is gone.
  $effect(() => {
    if (inbox.loaded && !inbox.current && inbox.rows.length) inbox.select(null, false);
  });

  // The 10 seconds of the last answer.
  let now = $state(Date.now());
  $effect(() => {
    if (!inbox.sending) return;
    now = Date.now();
    const id = setInterval(() => {
      now = Date.now();
      if (inbox.sending && now >= inbox.sending.sendAt) inbox.sending = null;
    }, 250);
    return () => clearInterval(id);
  });
  const seconds = $derived(inbox.sending ? Math.max(0, Math.ceil((inbox.sending.sendAt - now) / 1000)) : 0);

  function undo() {
    const s = inbox.sending;
    if (!s) return;
    inbox.sending = null;
    void app.run(api.inboxReopen(s.id)).then((a) => {
      if (!a) return;
      inbox.put(a);
      inbox.select(a.id);
      app.toast(t('inbox.sent.undone'), 'ok');
    });
  }

  function onKey(e: KeyboardEvent) {
    if (app.modal || menu.open || e.ctrlKey || e.metaKey || e.altKey) return;
    const el = e.target as HTMLElement | null;
    if (el?.closest('input, textarea, select, [contenteditable="true"], [role="dialog"], .xterm, .cm-editor')) return;
    const k = e.key.toLowerCase();
    if (k === 'j' || (e.key === 'ArrowDown' && !el?.closest('.queue'))) inbox.step(1);
    else if (k === 'k' || (e.key === 'ArrowUp' && !el?.closest('.queue'))) inbox.step(-1);
    else if (e.key === 'Escape' && !document.querySelector('[aria-modal="true"]')) app.closeInbox();
    else return;
    e.preventDefault();
  }

  const goAgent = (id: string | null | undefined) => id && app.agents[id] && app.selectAgent(id);
  const STOP_ICON: Record<string, IconName> = { permission: 'shield', question: 'question', plan: 'read', delete: 'trash' };
</script>

<svelte:window onkeydown={onKey} />

<section class="focus" aria-label={t('inbox.queue.title')} data-testid="inbox">
  <!-- A request left since the last reading, said without taking the focus (§ accessibility). -->
  <p class="vh" role="status" aria-live="polite">{inbox.announce ? t('inbox.queue.newOne', inbox.announce) : ''}</p>
  {#if cur}
    <header class="fh">
      <button
        class="nb"
        aria-label={t('inbox.ask.prev')}
        use:tip={t('inbox.ask.prev')}
        onclick={() => inbox.step(-1)}
        disabled={pos.total < 2}><Icon name="left" size={13} /></button
      >
      <span class="nth" data-testid="inbox-nth">{pos.n ? t('inbox.ask.nth', { n: pos.n, total: pos.total }) : ''}</span>
      <button
        class="nb"
        aria-label={t('inbox.ask.next')}
        use:tip={t('inbox.ask.next')}
        onclick={() => inbox.step(1)}
        disabled={pos.total < 2}><Icon name="right" size={13} /></button
      >
      <span class="keys" aria-hidden="true">
        <span><kbd>{t('inbox.ask.kJ')}</kbd>{t('inbox.ask.keyNext')}</span>
        <span><kbd>{t('inbox.ask.kK')}</kbd>{t('inbox.ask.keyPrev')}</span>
        <span><kbd>{t('inbox.ask.kL')}</kbd>{t('inbox.ask.keyLater')}</span>
      </span>
      <span class="sp"></span>
      {#if cur.kind === 'ask'}
        {#if cur.ask.author.agent && app.agents[cur.ask.author.agent]}
          <button class="btn ghost" onclick={() => goAgent(cur.ask.author.agent)} data-testid="inbox-to-agent"
            ><Icon name="agents" size={13} />{t('inbox.ask.toAgent')}</button
          >
        {/if}
        {#if cur.ask.ticketId}
          <button
            class="btn ghost"
            onclick={() => {
              const a = Object.values(app.agents).find((x) => x.ticketId === cur.ask.ticketId && !x.archived);
              if (a) app.selectAgent(a.id);
              else app.openBoard(cur.ask.projectId);
            }}
            data-testid="inbox-to-ticket"><Icon name="kanban" size={13} />{t('inbox.ask.toTicket')}</button
          >
        {/if}
      {:else if cur.kind === 'view'}
        {@const place = placeOf(cur.view)}
        {#if place}
          <button class="btn ghost" onclick={() => goToSource(cur.view)} data-testid="ivw-source"
            ><Icon name={place.icon} size={13} />{place.label}</button
          >
        {/if}
      {:else}
        <button class="btn ghost" onclick={() => goAgent(cur.agent.id)}><Icon name="agents" size={13} />{t('inbox.ask.toAgent')}</button>
      {/if}
    </header>
    {#if cur.kind === 'ask'}
      {#key cur.ask.id}
        <AskPane ask={cur.ask} />
      {/key}
    {:else if cur.kind === 'view'}
      <!-- What waits elsewhere (ESC-513), settled here with the gestures of its source; done, the next row opens. -->
      {#key cur.view.key}
        <InboxViewPane view={cur.view} ondone={() => inbox.after(itemKey(cur))} />
      {/key}
    {:else}
      <!-- An agent stopped with no view of its request: where to answer it. -->
      {@const what = stoppedWhat(cur.agent)}
      <div class="stopped" data-testid="inbox-stopped-pane">
        <span class="kind wt"
          ><Icon name={STOP_ICON[what]} size={13} />{t(`inbox.queue.stoppedWhat.${what}`)} · {t('inbox.queue.stoppedLine')}</span
        >
        <h2>{t('inbox.ask.stoppedTitle', { agent: cur.agent.name })}</h2>
        {#if stoppedTitle(cur.agent)}<pre class="cmd">{stoppedTitle(cur.agent)}</pre>{/if}
        <p class="muted">{t('inbox.ask.stoppedBody', { what: t(`inbox.queue.stoppedWhat.${what}`) })}</p>
        <div>
          <button class="btn primary" onclick={() => goAgent(cur.agent.id)}
            ><Icon name="agents" size={13} />{t('inbox.ask.openConversation')}</button
          >
        </div>
      </div>
    {/if}
  {:else}
    <div class="empty" data-testid="inbox-empty">
      <div class="in">
        <h2>{t('inbox.empty.title')}</h2>
        <p class="lead">{t('inbox.empty.lead')}</p>
        <dl class="kinds" aria-label={t('inbox.empty.kinds')}>
          {#each [['choice', 'choice'], ['confirm', 'check-c'], ['answer', 'edit'], ['task', 'check']] as const as [k, icon] (k)}
            <div>
              <Icon name={icon} />
              <dt>{t(`inbox.empty.${k}`)}</dt>
              <dd>{t(`inbox.empty.${k}Desc`)}</dd>
            </div>
          {/each}
        </dl>
        <p class="note"><Icon name="bell" size={12} />{t('inbox.empty.notify')}</p>
      </div>
    </div>
  {/if}
  {#if inbox.sending}
    <div class="sending" role="status" data-testid="inbox-sending">
      <Icon name="clock" size={13} />
      <span class="ell">{t('inbox.sent.goingIn', { title: inbox.sending.title, seconds })}</span>
      <button class="btn" onclick={undo} data-testid="inbox-undo"><Icon name="undo" size={13} />{t('inbox.sent.undo')}</button>
    </div>
  {/if}
</section>

<style>
  .focus {
    position: relative;
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  .fh {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    padding: 10px 16px 10px 20px;
    border-bottom: 1px solid var(--line);
    flex: none;
  }
  .nb {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: none;
    color: var(--muted);
    cursor: pointer;
  }
  .nb:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }
  .nb:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .nth {
    color: var(--soft);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .keys {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    margin-left: 10px;
    overflow: hidden;
    color: var(--dim);
    font-size: 11.5px;
    white-space: nowrap;
  }
  .keys span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .sp {
    flex: 1;
  }
  .stopped {
    display: grid;
    align-content: start;
    gap: 14px;
    padding: 22px 28px;
  }
  .stopped h2,
  .empty h2 {
    margin: 0;
    font-size: 21px;
    font-weight: 640;
    letter-spacing: -0.015em;
  }
  .kind {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    font-weight: 600;
  }
  .kind.wt {
    color: var(--wait);
  }
  .cmd {
    margin: 0;
    padding: 8px 10px;
    border-radius: 8px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--code-fg);
    font: 12px var(--mono);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .muted {
    margin: 0;
    color: var(--muted);
  }
  .empty {
    flex: 1;
    display: grid;
    place-items: center;
    min-height: 0;
    padding: 28px;
    overflow: auto;
  }
  .empty .in {
    display: grid;
    gap: 16px;
    width: min(560px, 100%);
  }
  .lead {
    margin: 0;
    color: var(--muted);
    line-height: 1.6;
  }
  .kinds {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px 18px;
    margin: 0;
    padding: 14px 16px;
    border-radius: 10px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .kinds div {
    display: grid;
    align-content: start;
    grid-template-columns: 16px minmax(0, 1fr);
    gap: 2px 8px;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.45;
  }
  .kinds :global(.icon) {
    margin-top: 2px;
    color: var(--teal-ink);
  }
  .kinds dt {
    color: var(--soft);
    font-size: 12.5px;
    font-weight: 600;
  }
  .kinds dd {
    grid-column: 2;
    margin: 0;
  }
  .note {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .sending {
    position: absolute;
    left: 50%;
    top: 54px;
    z-index: 3;
    display: flex;
    align-items: center;
    gap: 10px;
    max-width: calc(100% - 40px);
    padding: 8px 8px 8px 14px;
    border-radius: 10px;
    background: var(--raise-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 12px 32px -12px rgba(0, 0, 0, calc(0.6 * var(--shade)));
    color: var(--soft);
    font-size: 12.5px;
    transform: translateX(-50%);
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
    .keys {
      display: none;
    }
  }
</style>
