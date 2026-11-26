<script lang="ts">
  import { fDate, fSince, fTime } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import type { IconName } from '../../lib/icons';
  import { fLater, imagesOf, inbox, itemKey, letter, stoppedTitle, stoppedWhat, type QueueItem } from '../../lib/inbox/inbox.svelte';
  import { app } from '../../lib/state.svelte';
  import type { AskKind, WindowAsk } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import AskThumb from './AskThumb.svelte';
  import type { InboxItem, ViewSource } from '../../lib/inbox/views';
  import { viewTitle } from './view-title';
  import { handedMark } from '../../lib/inbox/handed';

  // The queue of the box « À toi » (direction B, `b-pleine.png`): « À toi » and its count, the scope (« Tous », « Ce
  // projet »), the meter (a cell per row: green handled, amber stopped, white the one open), « 3 traitées aujourd'hui · il
  // en reste 9 », then the groups: the agents stopped (amber), « Aujourd'hui », « Plus tôt », « Plus tard » (folded) and
  // « Répondu récemment ». A choice shows the renders of its options under its line (from direction D).

  const KIND_ICON: Record<AskKind, IconName> = { choice: 'choice', confirm: 'check-c', answer: 'edit', task: 'check' };
  const STOP_ICON: Record<string, IconName> = { permission: 'shield', question: 'question', plan: 'read', delete: 'trash' };
  /** What Escouade puts in the box of its own (ESC-513): an icon by source. */
  const VIEW_ICON: Record<ViewSource, IconName> = {
    permission: 'shield',
    question: 'question',
    plan: 'read',
    ticketReview: 'kanban',
    ticketBlocked: 'kanban',
    decision: 'check-c',
    docs: 'file',
    review: 'pr',
    handoff: 'team',
    teamAccept: 'team',
    scheduled: 'clock',
  };
  /** Who a view is about, on its line: its agent, else its ticket, else its project. */
  const whoOf = (v: InboxItem) => v.agentName ?? v.ticketKey ?? v.projectName;

  let now = $state(Date.now());
  $effect(() => {
    const id = setInterval(() => (now = Date.now()), 30_000);
    return () => clearInterval(id);
  });

  let laterOpen = $state(false);
  const g = $derived(inbox.groups);
  const left = $derived(inbox.allRows.length);
  const done = $derived(inbox.doneToday);
  /** The meter: one cell per row handled today and per row left, 24 at most. */
  const cells = $derived.by(() => {
    const out: ('d' | 'w' | 'c' | '')[] = [];
    for (let i = 0; i < done; i++) out.push('d');
    for (const r of inbox.allRows)
      out.push(itemKey(r) === inbox.selected ? 'c' : r.kind === 'stopped' || (r.kind === 'view' && r.view.group === 'stopped') ? 'w' : '');
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
    // It changed hands (ESC-585): said once, quietly, after the rest.
    if (a.handed) out.push(handedMark(a));
    return out;
  }

  /** A closed request's line: its answer, who, when, read or not. */
  function answerLine(a: WindowAsk): string {
    const ans = a.answer;
    if (a.state === 'withdrawn')
      return a.reason
        ? t('inbox.sent.withdrawn', { agent: a.author.name, reason: a.reason })
        : t('inbox.sent.withdrawnBare', { agent: a.author.name });
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

  /** The filter by type (ESC-578): ←, →, Début and Fin go from one of its buttons to another. */
  function filterKey(e: KeyboardEvent) {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(e.key)) return;
    const buttons = [...(e.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>('button')];
    const i = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (i < 0) return;
    e.preventDefault();
    e.stopPropagation();
    const to =
      e.key === 'Home'
        ? 0
        : e.key === 'End'
          ? buttons.length - 1
          : (i + (e.key === 'ArrowRight' ? 1 : -1) + buttons.length) % buttons.length;
    buttons[to].focus();
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

{#snippet viewRow(v: InboxItem, key: string)}
  <!-- A view of what waits elsewhere (ESC-513): its source, what, who, since when; its gestures in the panel. -->
  <button
    class="row-ask"
    class:wt={v.group === 'stopped' && !v.mark}
    class:done={v.mark?.kind === 'settled'}
    class:lead={v.mark?.kind === 'claimed'}
    class:on={inbox.selected === key}
    aria-current={inbox.selected === key}
    onclick={() => inbox.select(key)}
    data-testid="ivw-row"
    data-key={v.key}
    data-source={v.source}
    data-mark={v.mark?.kind ?? null}
  >
    <span class="ic"><Icon name={v.mark?.kind === 'settled' ? 'check' : VIEW_ICON[v.source]} /></span>
    <span class="t ell">{viewTitle(v)}</span>
    <span class="tm">{v.mark ? fSince(v.mark.at, now) : v.snoozedUntil ? '' : v.since ? fSince(v.since, now) : ''}</span>
    <span class="s">
      <span class="pdot" style:background={colorOf(v.projectId)}></span>
      {#if v.mark?.kind === 'settled'}
        <span class="ell" data-testid="ivw-settled-line"
          >{t('inboxViews.mark.settledLine', { name: v.mark.byName, note: v.mark.note })}</span
        >
      {:else if v.mark}
        <span class="ell nm">{v.mark.byName}</span>
        <span aria-hidden="true">·</span>
        <span class="ell" data-testid="ivw-claim-note">{v.mark.note}</span>
      {:else if v.snoozedUntil}
        <Icon name="clock" size={11} />
        <span class="ell">{t('inbox.queue.until', { when: fLater(v.snoozedUntil, now) })}</span>
      {:else}
        <span class="ell nm">{whoOf(v)}</span>
        <span aria-hidden="true">·</span>
        {#if v.group === 'stopped'}<b class="wk">{t(`inboxViews.kind.${v.source}`)}</b>{:else}<span class="ell" data-testid="ivw-kind"
            >{t(`inboxViews.kind.${v.source}`)}</span
          >{/if}
        {#if v.target}
          <!-- A ticket of its own target (ESC-552), not the Kanban's: where its validation goes. -->
          <span aria-hidden="true">·</span>
          <span class="ell mono" data-testid="ivw-target">{t('inboxViews.to', { branch: v.target })}</span>
        {/if}
      {/if}
    </span>
  </button>
{/snippet}

{#snippet row(item: QueueItem)}
  {@const key = itemKey(item)}
  {#if item.kind === 'view'}
    {@render viewRow(item.view, key)}
  {:else if item.kind === 'stopped'}
    {@const what = stoppedWhat(item.agent)}
    <!-- An agent stopped with no view of its request (a deletion it asks for, the Claude Code view): a row that leads to it. -->
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
  {#if inbox.types.length > 1 || inbox.filter.length}
    <!-- The filter by type (ESC-578): « Tout », then each type that waits and its number; several at once, kept on this machine. -->
    <div class="tf" role="toolbar" tabindex="-1" aria-label={t('inbox.filter.label')} data-testid="inbox-filter" onkeydown={filterKey}>
      <button
        class="chip"
        class:on={!inbox.filter.length}
        aria-pressed={!inbox.filter.length}
        onclick={() => inbox.setFilter([])}
        data-type="all">{t('inbox.filter.all')}</button
      >
      {#each inbox.types as c (c.type)}
        <button class="chip" class:on={c.on} aria-pressed={c.on} onclick={() => inbox.toggleType(c.type)} data-type={c.type}
          >{t(`inbox.filter.types.${c.type}`)}{' '}<span class="c">{c.count}</span></button
        >
      {/each}
      {#if inbox.filter.length}
        <button class="clr" aria-label={t('inbox.filter.clearAria')} onclick={() => inbox.setFilter([])} data-testid="inbox-filter-clear"
          ><Icon name="close" size={11} />{t('inbox.filter.clear')}</button
        >
      {/if}
    </div>
  {/if}
  <!-- What waits and the filter hides, said quietly, and to a screen reader when one more arrives. -->
  <p class="hid" role="status" data-testid="inbox-filter-hidden">
    {#if inbox.hidden}<Icon name="sliders" size={11} /><span class="hw">{t('inbox.filter.hidden', { count: inbox.hidden })}</span><button
        class="lnk"
        onclick={() => inbox.setFilter([])}>{t('inbox.filter.showAll')}</button
      >{/if}
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
    {#if g.claimed.length}
      <!-- What the squad lead took in hand (ESC-557): out of the counts, still there to read and to act on. -->
      <h2 class="grp" data-testid="ivw-claimed-group">
        <Icon name="agents" size={12} />{t('inboxViews.mark.group')} <span class="c">{g.claimed.length}</span>
      </h2>
      <div class="list">
        {#each g.claimed as v (v.key)}{@render viewRow(v, `view:${v.key}`)}{/each}
      </div>
    {/if}
    {#if g.later.length || g.laterViews.length}
      <h2 class="grp">
        <button class="fold" aria-expanded={laterOpen} onclick={() => (laterOpen = !laterOpen)} data-testid="inbox-later">
          <Icon name="clock" size={12} />{t('inbox.queue.later')} <span class="c">{g.later.length + g.laterViews.length}</span>
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
          {#each g.laterViews as v (v.key)}{@render viewRow(v, `view:${v.key}`)}{/each}
        </div>
      {/if}
    {/if}
    {#if g.recent.length || g.settled.length}
      <h2 class="grp"><Icon name="check" size={12} />{t('inbox.queue.recent')}</h2>
      <div class="list" data-testid="inbox-recent">
        <!-- The views the squad lead ticked as settled (ESC-557): their trace, whence the user puts them back. -->
        {#each g.settled as v (v.key)}{@render viewRow(v, `view:${v.key}`)}{/each}
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
  .tf {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 4px 8px 6px;
    flex: none;
  }
  .chip,
  .clr {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 8px;
    border: none;
    border-radius: 7px;
    background: none;
    color: var(--muted);
    font: 550 12px/1 var(--font);
    white-space: nowrap;
    cursor: pointer;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .chip .c {
    color: var(--dim);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .chip:hover,
  .clr:hover {
    color: var(--text);
  }
  .chip.on {
    background: var(--raise);
    color: var(--text);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 1px 2px rgba(0, 0, 0, calc(0.18 * var(--shade, 1)));
  }
  .chip.on .c {
    color: var(--muted);
  }
  .clr {
    box-shadow: none;
    padding: 0 6px;
  }
  .chip:focus-visible,
  .clr:focus-visible,
  .lnk:focus-visible {
    outline: none;
    box-shadow: inset 0 0 0 1.5px var(--p-ink);
  }
  .hid {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    margin: 0;
    padding: 0 8px;
    color: var(--dim);
    font-size: 11.5px;
    flex: none;
  }
  .hid .hw {
    min-width: 0;
    line-height: 1.35;
  }
  .hid:not(:empty) {
    padding-bottom: 4px;
  }
  .lnk {
    flex: none;
    margin-left: auto;
    padding: 1px 3px;
    border: none;
    border-radius: 5px;
    background: none;
    color: var(--muted);
    font: 550 11.5px/1.3 var(--font);
    cursor: pointer;
  }
  .lnk:hover {
    color: var(--text);
    text-decoration: underline;
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
  .row-ask.lead .ic {
    color: var(--teal-ink);
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
