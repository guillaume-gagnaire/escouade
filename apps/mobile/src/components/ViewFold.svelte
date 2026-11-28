<script lang="ts">
  // A view of the box unfolded (MO-5, ESC-513): what waits elsewhere (a ticket to approve or blocked, a decision, the
  // docs to share, a review, an agent passed, a task scheduled), with the gestures of its source, as the desktop's box
  // offers them (`gesturesOf`). « Valider » a ticket (a merge follows), take an agent, accept a task that launches
  // agents: under the lock (M3). « Renvoyer » takes what to change. The squad lead's marks (ESC-557): « Le chef s'en
  // occupe » and « Réglé par… », with « Remettre dans la boîte ». « Plus tard » for what does not stop an agent.
  import Icon from '$shared/components/Icon.svelte';
  import { fLater } from '$shared/lib/inbox/rules';
  import { gesturesOf, type Gesture, type InboxItem, type ViewAct } from '$shared/lib/inbox/views';
  import { t } from '$shared/lib/i18n';
  import { actOnView, reopenView, snooze, type Outcome } from '../lib/act';
  import { keepDraft } from '../lib/kept';
  import { openTalk } from '../lib/nav.svelte';
  import type { SpaceRef } from '../lib/thread';
  import LaterMenu from './LaterMenu.svelte';
  import Word from './Word.svelte';

  let { space, view, cut, onSettled }: { space: SpaceRef; view: InboxItem; cut: boolean; onSettled: () => void } = $props();

  const gestures = $derived(gesturesOf(view));
  let rejecting = $state(false);
  let comment = $state('');
  let later = $state(false);
  let busy = $state(false);
  let said = $state<string | null>(null);
  const draftKey = $derived(`${space.id}:reject:${view.key}`);

  const noteKey = $derived.by(() => {
    switch (view.source) {
      case 'ticketReview':
      case 'ticketBlocked':
        return 'inboxViews.note.ticket' as const;
      case 'decision':
        return 'inboxViews.note.decision' as const;
      case 'docs':
        return 'inboxViews.note.docs' as const;
      case 'review':
        return 'inboxViews.note.review' as const;
      default:
        return null;
    }
  });

  const labelOf = (g: Gesture) =>
    g.id === 'retarget'
      ? t('inboxViews.act.retarget', { branch: view.otherLine ?? '' })
      : g.id === 'openPr'
        ? t('inboxViews.act.openPr', { word: view.word ?? 'PR' })
        : t(g.label);

  async function finish(o: Outcome) {
    busy = false;
    if (o.ok) {
      keepDraft(draftKey, '');
      onSettled();
    } else said = 'locked' in o ? t('mobile.lock.cancelled') : o.error;
  }

  async function act(g: Gesture) {
    said = null;
    if (g.id === 'openPr') {
      if (view.url) window.open(view.url, '_blank', 'noopener');
      return;
    }
    if (g.id === 'inConversation') {
      if (view.agentId) openTalk(space.id, view.agentId);
      return;
    }
    if (g.id === 'reject' && !rejecting) {
      rejecting = true;
      return;
    }
    let a: ViewAct;
    switch (g.id) {
      case 'reject':
        if (!comment.trim()) return;
        a = { act: 'reject', comment: comment.trim() };
        break;
      case 'retarget':
        a = { act: 'retarget', target: view.otherLine ?? '' };
        break;
      case 'take':
        a = { act: 'take', engine: null };
        break;
      default:
        a = { act: g.id } as ViewAct;
    }
    busy = true;
    await finish(await actOnView(space, view, a, t('mobile.lock.view', { act: labelOf(g), title: view.ticketKey ?? view.title })));
  }

  async function putOff(at: number) {
    busy = true;
    later = false;
    await finish(await snooze(space, { view }, at));
  }

  const marked = $derived(view.mark);
  const shown = $derived(gestures.filter((g) => g.inBox || g.id === 'openPr' || g.id === 'inConversation'));
</script>

<div class="fold">
  {#if marked}
    <p class="mark">
      <Icon name={marked.kind === 'claimed' ? 'clock' : 'check'} size={15} />
      {marked.kind === 'claimed'
        ? t('inboxViews.mark.claimed', { name: marked.byName, when: fLater(marked.at) })
        : t('inboxViews.mark.settled', { name: marked.byName, when: fLater(marked.at) })}
    </p>
    {#if marked.note}<p class="quote">{t('inboxViews.mark.note', { note: marked.note })}</p>{/if}
    <p class="note">{marked.kind === 'claimed' ? t('inboxViews.mark.claimedNote') : t('inboxViews.mark.settledNote')}</p>
  {/if}

  {#if view.scheduled}
    <dl class="sched">
      <dt>{t('inboxViews.scheduled.rhythm')}</dt>
      <dd>{view.scheduled.cron}</dd>
      <dt>{t('inboxViews.scheduled.prompt')}</dt>
      <dd>{view.scheduled.description}</dd>
    </dl>
  {/if}

  {#if view.partial || view.conflict || view.blocked || view.target}
    <p class="tags">
      {#if view.target}<span>{t('inboxViews.to', { branch: view.target })}</span>{/if}
      {#if view.partial}<span>{t('inboxViews.partial')}</span>{/if}
      {#if view.conflict}<span class="w">{t('inboxViews.conflict')}</span>{/if}
      {#if view.blocked}<span class="w">{view.blocked}</span>{/if}
    </p>
  {/if}

  {#if view.snoozedUntil && !marked}
    <p class="note">{t('inbox.ask.snoozedUntil', { when: fLater(view.snoozedUntil) })}</p>
  {/if}

  {#if cut}<p class="note warn" role="status">{t('mobile.answer.offline')}</p>{/if}

  {#if marked?.kind !== 'settled'}
    {#if rejecting}
      <Word
        bind:value={comment}
        label={t('inboxViews.act.rejectLabel')}
        draft={draftKey}
        placeholder={t('inboxViews.act.rejectPlaceholder')}
      />
    {/if}
    {#if shown.length}
      <div class="acts" class:one={shown.length === 1}>
        {#each shown as g (g.id)}
          <button
            type="button"
            class="b"
            class:go={g.primary || (rejecting && g.id === 'reject')}
            disabled={busy || cut || (rejecting && g.id === 'reject' && !comment.trim())}
            onclick={() => act(g)}
          >
            {#if g.inBox && ['approve', 'take'].includes(g.id)}<Icon name="lock" size={16} />{/if}
            {rejecting && g.id === 'reject' ? t('inboxViews.act.rejectSend') : labelOf(g)}
          </button>
        {/each}
      </div>
    {/if}
    {#if noteKey}<p class="note">{t(noteKey)}</p>{/if}
  {/if}

  {#if later}
    <LaterMenu disabled={busy || cut} onPick={putOff} onCancel={() => (later = false)} />
  {/if}

  <div class="more">
    {#if marked || view.snoozedUntil}
      <button type="button" class="m" disabled={busy || cut} onclick={async () => finish(await reopenView(space, view))}>
        <Icon name="undo" size={16} />{t('mobile.answer.reopenView')}
      </button>
    {:else}
      <button type="button" class="m" disabled={cut} onclick={() => (later = !later)}
        ><Icon name="clock" size={16} />{t('mobile.answer.later')}</button
      >
    {/if}
    {#if view.agentId}
      <button type="button" class="m end" onclick={() => openTalk(space.id, view.agentId ?? '')}>
        {t('mobile.answer.inConversation')}<Icon name="right" size={14} />
      </button>
    {/if}
  </div>

  {#if said}<p class="said" role="status">{said}</p>{/if}
</div>

<style>
  .fold {
    display: grid;
    gap: 10px;
    padding: 2px 0 4px;
  }
  .mark {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--teal-ink);
  }
  .quote {
    margin: 0;
    font-size: 14px;
    color: var(--soft);
  }
  .sched {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 4px 10px;
    margin: 0;
    font-size: 14px;
  }
  .sched dt {
    color: var(--muted);
  }
  .sched dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 0;
  }
  .tags span {
    padding: 2px 8px;
    border-radius: 999px;
    font-size: 12px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
  }
  .tags .w {
    color: var(--wait);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .acts {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--gap-target);
  }
  .acts.one {
    grid-template-columns: 1fr;
  }
  .b {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: calc(var(--target) + 2px);
    padding: 0 14px;
    border: 0;
    border-radius: var(--r-m);
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font: inherit;
    font-weight: 650;
    font-size: 16px;
  }
  .b.go {
    background: var(--text);
    color: var(--panel);
    box-shadow: none;
  }
  .b:disabled,
  .m:disabled {
    opacity: 0.45;
  }
  .more {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .m {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-height: var(--target);
    padding: 0 10px;
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-weight: 600;
  }
  .m.end {
    margin-left: auto;
  }
  .note {
    margin: 0;
    font-size: var(--small);
    color: var(--muted);
  }
  .warn {
    color: var(--wait);
  }
  .said {
    margin: 0;
    font-size: var(--small);
    color: var(--soft);
  }
</style>
