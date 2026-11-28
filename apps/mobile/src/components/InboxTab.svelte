<script lang="ts">
  // « À toi » (MO-1, MO-5, direction B, `docs/MOBILE.md` § 3.3): the thread of what waits in the spaces the chip shows,
  // grouped as the desktop's box: « Tes agents se sont arrêtés » (the longest waiting first), « Aujourd'hui »,
  // « Plus tôt »; then « Le chef s'en occupe », « Plus tard », « Répondu récemment ». A line unfolds on the spot; a line
  // settled opens the next. A line that changes keeps its place and its dot until it is seen. A space that is cut
  // keeps what it said last, dated (M13), and answers nothing.
  import Icon from '$shared/components/Icon.svelte';
  import { fTime } from '$shared/lib/format';
  import { t } from '$shared/lib/i18n';
  import type { WindowAsk } from '$shared/lib/types';
  import { reopenAsk, seeAsk } from '../lib/act';
  import { clock, currentThread, hasDot, markSeen, openNext } from '../lib/feed.svelte';
  import { nav, unfold } from '../lib/nav.svelte';
  import { ALL, countsOnlySpaces, phone, shownSpaces, stateOf } from '../lib/phone-spaces.svelte';
  import { GROUPS, waitingLines, type Group, type Line as L, type SpaceRef } from '../lib/thread';
  import Empty from './Empty.svelte';
  import Line from './Line.svelte';

  const shown = $derived(shownSpaces(phone.spaces, phone.filter));
  const thread = $derived.by(() => {
    void clock.now;
    return currentThread();
  });
  const hidden = $derived(countsOnlySpaces(shown));
  const one = $derived(phone.filter === ALL ? null : shown[0]);
  const waiting = $derived(waitingLines(thread).length);
  const any = $derived(GROUPS.some((g) => thread[g].length));
  /** The spaces shown that are cut, with when they said last. */
  const stale = $derived(shown.filter((s) => stateOf(s) !== 'open' && s.at > 0 && s.glance));
  const cutIds = $derived(new Set(shown.filter((s) => stateOf(s) !== 'open').map((s) => s.id)));

  let gone = $state(false);
  let sent = $state<{ space: SpaceRef; ask: WindowAsk; until: number } | null>(null);
  let tick = $state(Date.now());

  const HEAD: Record<Group, { title: () => string; icon?: 'shield' | 'clock' }> = {
    stopped: { title: () => t('inbox.queue.stopped'), icon: 'shield' },
    today: { title: () => t('inbox.queue.today'), icon: 'clock' },
    earlier: { title: () => t('inbox.queue.earlier') },
    claimed: { title: () => t('inboxViews.mark.group') },
    later: { title: () => t('inbox.queue.later') },
    recent: { title: () => t('inbox.queue.recent') },
  };

  function toggle(l: L) {
    gone = false;
    if (nav.open === l.key) return unfold(null);
    unfold(l.key);
    markSeen(l);
    if (l.kind === 'ask' && l.ask) seeAsk(l.space, l.ask);
  }

  // The line open, gone from the thread without a gesture of this phone: settled elsewhere, said once.
  $effect(() => {
    const key = nav.open;
    if (!key) return;
    const there = GROUPS.some((g) => thread[g].some((l) => l.key === key));
    if (!there) {
      gone = true;
      unfold(null);
    }
  });

  // An unfolded line, seen in the state it is in now (it changed while open): its dot stays off.
  $effect(() => {
    const key = nav.open;
    for (const g of GROUPS) for (const l of thread[g]) if (l.key === key) markSeen(l);
  });

  function settled(key: string) {
    gone = false;
    openNext(thread, key);
    // The line just answered opens the next one, which comes into view.
    queueMicrotask(() => document.querySelector(`[data-key="${CSS.escape(nav.open ?? '')}"]`)?.scrollIntoView({ block: 'nearest' }));
  }

  function answered(space: SpaceRef, ask: WindowAsk) {
    sent = { space, ask, until: Date.now() + 10_000 };
  }

  $effect(() => {
    if (!sent) return;
    const timer = setInterval(() => {
      tick = Date.now();
      if (sent && tick >= sent.until) sent = null;
    }, 500);
    return () => clearInterval(timer);
  });

  async function undo() {
    if (!sent) return;
    const s = sent;
    sent = null;
    await reopenAsk(s.space, s.ask);
  }
</script>

{#each hidden as s (s.id)}
  <p class="note">{t('mobile.inbox.countsOnly', { name: s.name, waiting: s.glance?.waiting ?? 0 })}</p>
{/each}
{#each stale as s (s.id)}
  <p class="note stale" data-testid="stale">{t('mobile.thread.stale', { time: fTime(s.at), name: s.name })}</p>
{/each}
{#if gone}<p class="note" role="status">{t('mobile.thread.settled')}</p>{/if}

{#each GROUPS as g (g)}
  {#if thread[g].length}
    <section class="grp" data-group={g}>
      <h2 class="gh" class:waits={g === 'stopped'}>
        {#if HEAD[g].icon}<Icon name={HEAD[g].icon} size={15} />{/if}
        {HEAD[g].title()} <span class="c">{thread[g].length}</span>
      </h2>
      <ul>
        {#each thread[g] as l (l.key)}
          <Line
            line={l}
            open={nav.open === l.key}
            dot={hasDot(l)}
            cut={cutIds.has(l.space.id)}
            now={clock.now}
            onToggle={() => toggle(l)}
            onSettled={() => settled(l.key)}
            onAnswered={(a) => answered(l.space, a)}
          />
        {/each}
      </ul>
    </section>
  {/if}
{/each}

{#if !waiting && !hidden.length}
  <Empty
    icon="inbox"
    title={one ? t('mobile.inbox.emptyIn', { name: one.name }) : t('mobile.inbox.empty')}
    text={any ? '' : t('mobile.inbox.emptyText')}
  />
{/if}

{#if sent}
  <div class="toast" role="status">
    <span>{t('mobile.answer.sentUndo', { seconds: Math.max(0, Math.ceil((sent.until - tick) / 1000)) })}</span>
    <button type="button" onclick={undo}>{t('mobile.answer.undo')}</button>
  </div>
{/if}

<style>
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .gh {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 0;
    padding: 16px var(--gut) 6px;
    font-size: var(--small);
    font-weight: 650;
    color: var(--muted);
  }
  .gh.waits {
    color: var(--wait);
  }
  .c {
    font-variant-numeric: tabular-nums;
    opacity: 0.8;
  }
  .note {
    margin: 12px var(--gut) 0;
    padding: 10px 12px;
    border-radius: var(--r-m);
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: var(--small);
    color: var(--soft);
  }
  .stale {
    color: var(--wait);
  }
  .toast {
    position: sticky;
    bottom: 8px;
    z-index: 2;
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 12px var(--gut) 0;
    padding: 6px 6px 6px 14px;
    border-radius: var(--r-m);
    background: var(--raise);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 8px 24px -8px rgba(0, 0, 0, calc(0.5 * var(--shade)));
    font-size: 14px;
  }
  .toast span {
    flex: 1;
  }
  .toast button {
    min-height: var(--target);
    padding: 0 12px;
    border: 0;
    border-radius: 10px;
    background: transparent;
    color: var(--teal-ink);
    font: inherit;
    font-weight: 650;
  }
</style>
