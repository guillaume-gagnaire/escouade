<script lang="ts">
  import { fDate, fSince, fTime } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import type { IconName } from '../../lib/icons';
  import { fLater, imagesOf, inbox, itemKey, letter, stoppedTitle, stoppedWhat, type QueueItem } from '../../lib/inbox/inbox.svelte';
  import { app } from '../../lib/state.svelte';
  import type { AskKind, WindowAsk } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import AskThumb from './AskThumb.svelte';

  // The queue of the box « À toi » (direction B, `b-pleine.png`): « À toi » and its count, the scope (« Tous », « Ce
  // projet »), the meter (a cell per row: green handled, amber stopped, white the one open), « 3 traitées aujourd'hui · il
  // en reste 9 », then the groups: the agents stopped (amber), « Aujourd'hui », « Plus tôt », « Plus tard » (folded) and
  // « Répondu récemment ». A choice shows the renders of its options under its line (from direction D).

  const KIND_ICON: Record<AskKind, IconName> = { choice: 'choice', confirm: 'check-c', answer: 'edit', task: 'check' };
  const STOP_ICON: Record<string, IconName> = { permission: 'shield', question: 'question', plan: 'read', delete: 'trash' };

  let now = $state(Date.now());
  $effect(() => {
    const id = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(id);
  });

  let laterOpen = $state(false);
  const g = $derived(inbox.groups);
  const left = $derived(inbox.rows.length);
  const done = $derived(inbox.doneToday);
  /** The meter: one cell per row handled today and per row left, 24 at most. */
  const cells = $derived.by(() => {
    const out: ('d' | 'w' | 'c' | '')[] = [];
    for (let i = 0; i < done; i++) out.push('d');
    for (const r of inbox.rows) out.push(itemKey(r) === inbox.selected ? 'c' : r.kind === 'stopped' ? 'w' : '');
    return out.slice(Math.max(0, out.length - 24));
  });

  const colorOf = (projectId: string) => app.projects.find((p) => p.id === projectId)?.color ?? 'var(--idle)';

  /** What its line under the title says: its author, then its files or its kind and urgency. */
  function meta(a: WindowAsk): string[] {
    const out: string[] = [];
    const imgs = a.files.filter((f) => f.kind === 'image').length;
    const pdfs = a.files.filter((f) => f.kind === 'pdf').length;
    const others = a.files.length - imgs - pdfs;
    const files = [
      pdfs ? t('inbox.queue.pdfs', { count: pdfs }) : '',
      imgs ? t('inbox.queue.images', { count: imgs }) : '',
      others ? t('inbox.queue.others', { count: others }) : '',
    ].filter(Boolean);
    if (files.length) out.push(files.join(', '));
    else out.push(t(`inbox.kind.${a.kind}`));
    if (a.woke) out.push(t('inbox.queue.woke'));
    else if (a.urgency === 'today') out.push(t('inbox.queue.todayMark'));
    return out;
  }

  /** A closed request's line: its answer, who, when, read or not. */
  function answerLine(a: WindowAsk): string {
    const ans = a.answer;
    if (a.state === 'withdrawn') return t('inbox.sent.withdrawn', { agent: a.author.name, reason: a.reason ?? '' });
    if (a.state === 'expired') return t('inbox.sent.expired', { date: fDate(a.closedAt ?? a.expiresAt) });
    if (a.state === 'retired') return t('inbox.sent.retired', { reason: a.reason ?? '' });
    if (!ans) return '';
    const what =
      a.kind === 'choice'
        ? ans.options
            .map((o) => {
              const opt = a.options.find((x) => x.id === o);
              return opt ? `${letter(o)} · ${opt.label}` : letter(o);
            })
            .join(', ')
        : a.kind === 'confirm'
          ? t(ans.yes ? 'inbox.sent.yes' : 'inbox.sent.no')
          : a.kind === 'task'
            ? t(ans.done ? 'inbox.sent.done' : 'inbox.sent.notDone')
            : t('inbox.sent.quoted', { text: ans.text ?? '' });
    const who = ans.byName ? t('inbox.sent.delegated', { name: ans.byName }) : t('inbox.sent.you');
    const line = t('inbox.sent.recentLine', { answer: what, who, when: fTime(ans.at) });
    return a.state === 'read' ? `${line} · ${t('inbox.sent.readSuffix', { agent: a.author.name })}` : line;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
    const buttons = [...(e.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>('button.row-ask')];
    const i = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (i < 0) return;
    e.preventDefault();
    e.stopPropagation();
    const next = buttons[Math.max(0, Math.min(buttons.length - 1, i + (e.key === 'ArrowDown' ? 1 : -1)))];
    next.focus();
    next.click();
  }
</script>

{#snippet row(item: QueueItem)}
  {@const key = itemKey(item)}
  {#if item.kind === 'stopped'}
    {@const what = stoppedWhat(item.agent)}
    <!-- IN-3: InboxViewRow — the blocking requests become IN-3's views (answered in the box); until then, a row that leads to its agent. -->
    <button
      class="row-ask wt"
      class:on={inbox.selected === key}
      aria-current={inbox.selected === key}
      onclick={() => inbox.select(key)}
      data-testid="inbox-stopped"
    >
      <span class="ic"><Icon name={STOP_ICON[what]} /></span>
      <span class="t ell">{stoppedTitle(item.agent) || t('inbox.ask.stoppedTitle', { agent: item.agent.name })}</span>
      <span class="tm">{fSince(item.agent.lastActivity, now)}</span>
      <span class="s">
        <span class="pdot" style:background={colorOf(item.agent.projectId)}></span>
        <span class="ell">{item.agent.name}</span>
        <span aria-hidden="true">·</span>
        <b class="wk">{t(`inbox.queue.stoppedWhat.${what}`)}</b>
      </span>
    </button>
  {:else}
    {@const a = item.ask}
    {@const thumbs = a.kind === 'choice' ? a.options.map((o) => ({ o, f: imagesOf(a, o.id)[0] })).filter((x) => x.f) : []}
    <button
      class="row-ask"
      class:on={inbox.selected === key}
      class:unseen={!a.seen}
      aria-current={inbox.selected === key}
      onclick={() => inbox.select(key)}
      data-testid="inbox-row"
      data-ask={a.id}
    >
      <span class="ic"><Icon name={KIND_ICON[a.kind]} /></span>
      <span class="t ell">{a.title}</span>
      <span class="tm">{fSince(a.createdAt, now)}</span>
      <span class="s">
        <span class="pdot" style:background={colorOf(a.projectId)}></span>
        <span class="ell nm">{(a.author.agent && app.agents[a.author.agent]?.name) || a.author.name}</span>
        <span aria-hidden="true">·</span>
        {#if a.files.length}<Icon name="clip" size={11} />{/if}
        <span class="ell">{meta(a).join(' · ')}</span>
      </span>
      {#if thumbs.length && a.filesErasedAt === null}
        <span class="mini" aria-label={t('inbox.queue.thumbs')}>
          {#each thumbs as { o, f } (o.id)}
            <AskThumb ask={a} file={f} alt={`${letter(o.id)} · ${o.label}`} />
          {/each}
        </span>
      {/if}
    </button>
  {/if}
{/snippet}

<aside class="queue" aria-label={t('inbox.queue.label')} data-testid="inbox-queue">
  <div class="qh">
    <h1>{t('inbox.queue.title')} <span class="c" data-testid="inbox-scoped-count">{inbox.scopedCount.total}</span></h1>
    <div class="stabs" role="radiogroup" aria-label={t('inbox.queue.scope')}>
      <button role="radio" aria-checked={inbox.scope === 'all'} class:on={inbox.scope === 'all'} onclick={() => (inbox.scope = 'all')}
        >{t('inbox.queue.all')}</button
      >
      <button
        role="radio"
        aria-checked={inbox.scope === 'project'}
        class:on={inbox.scope === 'project'}
        disabled={!app.ui.activeProject}
        onclick={() => (inbox.scope = 'project')}>{t('inbox.queue.project')}</button
      >
    </div>
  </div>
  <div class="meter" role="img" aria-label={t('inbox.queue.meter', { done, left })}>
    {#each cells as c, i (i)}<i class={c}></i>{/each}
    {#each { length: Math.max(0, 12 - cells.length) } as _, i (i)}<i></i>{/each}
  </div>
  <p class="left">
    {t('inbox.queue.done', { count: done })}{#if left}{' · '}{t('inbox.queue.left', { count: left })}{/if}
  </p>
  <div class="scroll" role="presentation" onkeydown={onKey}>
    {#if g.stopped.length}
      <h2 class="grp wt"><Icon name="shield" size={12} />{t('inbox.queue.stopped')} <span class="c">{g.stopped.length}</span></h2>
      <div class="list">
        {#each g.stopped as it (itemKey(it))}{@render row(it)}{/each}
      </div>
    {/if}
    {#if g.today.length}
      <h2 class="grp">{t('inbox.queue.today')} <span class="c">{g.today.length}</span></h2>
      <div class="list">
        {#each g.today as it (itemKey(it))}{@render row(it)}{/each}
      </div>
    {/if}
    {#if g.earlier.length}
      <h2 class="grp">{t('inbox.queue.earlier')} <span class="c">{g.earlier.length}</span></h2>
      <div class="list">
        {#each g.earlier as it (itemKey(it))}{@render row(it)}{/each}
      </div>
    {/if}
    {#if g.later.length}
      <h2 class="grp">
        <button class="fold" aria-expanded={laterOpen} onclick={() => (laterOpen = !laterOpen)} data-testid="inbox-later">
          <Icon name="clock" size={12} />{t('inbox.queue.later')} <span class="c">{g.later.length}</span>
        </button>
      </h2>
      {#if laterOpen}
        <div class="list">
          {#each g.later as a (a.id)}
            <button
              class="row-ask"
              class:on={inbox.selected === a.id}
              aria-current={inbox.selected === a.id}
              onclick={() => inbox.select(a.id)}
              data-testid="inbox-row"
              data-ask={a.id}
            >
              <span class="ic"><Icon name={KIND_ICON[a.kind]} /></span>
              <span class="t ell">{a.title}</span>
              <span class="tm"></span>
              <span class="s">
                <span class="pdot" style:background={colorOf(a.projectId)}></span>
                <Icon name="clock" size={11} />
                <span class="ell">{t('inbox.queue.until', { when: a.snoozedUntil ? fLater(a.snoozedUntil, now) : '' })}</span>
              </span>
            </button>
          {/each}
        </div>
      {/if}
    {/if}
    {#if g.recent.length}
      <h2 class="grp"><Icon name="check" size={12} />{t('inbox.queue.recent')}</h2>
      <div class="list" data-testid="inbox-recent">
        {#each g.recent as a (a.id)}
          <button
            class="row-ask done"
            class:on={inbox.selected === a.id}
            aria-current={inbox.selected === a.id}
            onclick={() => inbox.select(a.id)}
            data-testid="inbox-recent-row"
            data-ask={a.id}
          >
            <span class="ic"><Icon name={a.answer ? 'check' : 'close'} /></span>
            <span class="t ell">{a.title}</span>
            <span class="tm"></span>
            <span class="s"><span class="ell">{answerLine(a)}</span></span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
</aside>

<style>
  .queue {
    width: 300px;
    flex: none;
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 4px 10px 10px 2px;
    gap: 2px;
  }
  .qh {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px 4px;
    flex: none;
  }
  .qh h1 {
    margin: 0;
    font-size: 15px;
    font-weight: 650;
    white-space: nowrap;
  }
  .qh h1 .c {
    margin-left: 4px;
    color: var(--muted);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .stabs {
    display: flex;
    gap: 2px;
    margin-left: auto;
    padding: 3px;
    border-radius: 10px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .stabs button {
    height: 24px;
    padding: 0 8px;
    border: none;
    border-radius: 7px;
    background: none;
    color: var(--muted);
    font: 550 12px/1 var(--font);
    white-space: nowrap;
    cursor: pointer;
  }
  .stabs button:hover:not(:disabled) {
    color: var(--text);
  }
  .stabs button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .stabs button.on {
    background: var(--panel);
    color: var(--text);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 1px 2px rgba(0, 0, 0, calc(0.18 * var(--shade, 1)));
  }
  .meter {
    display: flex;
    gap: 3px;
    padding: 6px 8px 8px;
    flex: none;
  }
  .meter i {
    height: 4px;
    flex: 1;
    border-radius: 2px;
    background: var(--line-2);
  }
  .meter i.d {
    background: var(--run);
  }
  .meter i.w {
    background: var(--wait);
  }
  .meter i.c {
    background: var(--text);
  }
  .left {
    margin: 0;
    padding: 0 8px 4px;
    color: var(--muted);
    font-size: 12px;
    flex: none;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding-bottom: 8px;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .grp {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 10px 8px 5px;
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 600;
    white-space: nowrap;
    flex: none;
  }
  .grp .c {
    color: var(--dim);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .grp.wt {
    color: var(--wait);
  }
  .fold {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 0;
    border: none;
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .fold:hover {
    color: var(--text);
  }
  .row-ask {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr) auto;
    gap: 2px 9px;
    align-items: center;
    width: 100%;
    padding: 8px 9px;
    border: none;
    border-radius: 9px;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
    flex: none;
    min-width: 0;
  }
  .row-ask:hover {
    background: var(--hover);
  }
  .row-ask:focus-visible {
    outline: none;
    box-shadow: inset 0 0 0 1.5px var(--p-ink);
  }
  .row-ask .ic {
    display: grid;
    place-items: center;
    color: var(--muted);
  }
  .row-ask .t {
    font-weight: 580;
    font-size: 12.5px;
    color: var(--text);
  }
  .row-ask.unseen .t::before {
    content: '';
    display: inline-block;
    width: 6px;
    height: 6px;
    margin: 0 6px 1px 0;
    border-radius: 50%;
    background: var(--brand);
    vertical-align: middle;
  }
  .row-ask .tm {
    color: var(--dim);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .row-ask .s {
    grid-column: 2 / 4;
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    color: var(--muted);
    font-size: 11.5px;
    white-space: nowrap;
  }
  .row-ask .s .nm {
    flex: none;
    max-width: 50%;
  }
  .row-ask .mini {
    grid-column: 2 / 4;
    display: flex;
    gap: 4px;
    margin-top: 4px;
  }
  .row-ask.on {
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .row-ask.wt {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .row-ask.wt.on {
    box-shadow: inset 0 0 0 1.5px var(--wait);
  }
  .row-ask.wt .ic,
  .row-ask.wt .wk {
    color: var(--wait);
  }
  .row-ask .wk {
    font-weight: 600;
  }
  .row-ask.done .t {
    color: var(--soft);
    font-weight: 520;
  }
  .row-ask.done .ic {
    color: var(--run);
  }
  .pdot {
    width: 8px;
    height: 8px;
    flex: none;
    border-radius: 2.5px;
  }
  .ell {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  @media (max-width: 1100px) {
    .queue {
      width: 236px;
    }
    .stabs button {
      padding: 0 6px;
    }
  }
</style>
