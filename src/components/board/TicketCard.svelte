<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { APPROVE_LABEL, canStart, criteriaMet, doneMeta, launchAnyway, ticketSpent, waitingFor, waitLabel } from '../../lib/board';
  import { buffers, lossNotice } from '../../lib/editor/buffers.svelte';
  import { fWhen } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { SERVICES, shortName } from '../../lib/integrations';
  import { api } from '../../lib/ipc';
  import { menu, type MenuItem } from '../../lib/menu.svelte';
  import { openAddress } from '../../lib/recipe';
  import { ESTIMATE_HINT, fSpentUsd } from '../../lib/spend';
  import { app } from '../../lib/state.svelte';
  import { anyRunning, flows, stopTests, testAgent } from '../../lib/test-launch.svelte';
  import type { Project, Ticket } from '../../lib/types';
  import StatusDot from '../StatusDot.svelte';
  import RejectForm from './RejectForm.svelte';

  /** Items of the agent's progress a ticket under way shows: the last ones. */
  const SHOWN = 3;

  let {
    ticket,
    project,
    queueIndex,
    busyCount,
    quota,
    onedit,
  }: { ticket: Ticket; project: Project; queueIndex: number; busyCount: number; quota: number | null; onedit: () => void } = $props();

  const s = $derived(project.board);
  /** Why no ticket of the board starts now (its target branch), if so. */
  const issue = $derived(app.boardIssues[project.id] ?? null);
  const agent = $derived(ticket.agentId ? app.agents[ticket.agentId] : undefined);
  /** What every agent of the ticket used so far: the card of a finished ticket keeps the cost it was closed with. */
  const used = $derived(ticket.column === 'doing' || ticket.column === 'review' ? ticketSpent(ticket, app.agents) : null);
  const met = $derived(criteriaMet(ticket));
  const total = $derived(ticket.criteria.length);
  /** The foot's figures: what the ticket cost, the criteria met of a ticket to test, the loops of a finished one. */
  const figures = $derived(!!used || ticket.column === 'review' || ticket.column === 'done');
  const waiting = $derived(ticket.column === 'doing' && agent?.status === 'waiting');
  // Under way, the last items; to test, all of them.
  const steps = $derived(ticket.column === 'doing' ? ticket.progress.slice(-SHOWN) : ticket.progress);
  const hidden = $derived(ticket.progress.length - steps.length);
  /** The keys of the tickets it comes after that are not done yet: the autopilot leaves it until they are. */
  const awaited = $derived(ticket.column === 'todo' ? waitingFor(ticket, app.tickets) : []);
  let rejecting = $state(false);
  /** The ⚠ of a failed sync shows its reason and « Resynchroniser ». */
  let syncOpen = $state(false);
  let resyncing = $state(false);
  let card = $state<HTMLDivElement>();
  let warn = $state<HTMLButtonElement>();
  let panel = $state<HTMLDivElement>();
  // The error gone (a retry went through, by itself or not), the ⚠ and its panel go: before they do, the focus they
  // hold moves to the card, not to the window. A later failure starts closed again.
  $effect.pre(() => {
    if (ticket.external?.error) return;
    if ([warn, panel].some((el) => el?.contains(document.activeElement))) card?.focus();
    syncOpen = false;
  });

  /** « Resynchroniser »: the failed syncs go again at once; their success takes the ⚠ away (the ticket comes back without its error). */
  async function resync() {
    if (resyncing) return;
    resyncing = true;
    const ok = await app.run(api.integrationResync(ticket.id).then(() => true));
    resyncing = false;
    if (!ok) return;
    syncOpen = false;
    // Its button goes with the ⚠: the focus stays on the card.
    card?.focus();
  }

  /** "Lancer": a ticket that waits for others starts without them only once the user agrees. */
  function launch() {
    const start = () => app.run(api.ticketStart(ticket.id));
    if (!awaited.length) {
      start();
      return;
    }
    app.modal = {
      kind: 'confirm',
      title: t('board.card.launchTitle', { key: ticket.key }),
      body: launchAnyway(ticket.key, awaited),
      confirm: t('board.card.launchAnyway'),
      onConfirm: start,
    };
  }

  function open() {
    if (ticket.column === 'todo') onedit();
    else if (ticket.agentId) app.selectAgent(ticket.agentId);
  }

  async function remove() {
    // Its agent is archived with it, which stops its test launches: stopped first, their ends are no crashes.
    if (ticket.column !== 'todo' && ticket.agentId) stopTests(ticket.agentId);
    const gone = await app.run(api.ticketDelete(ticket.id).then(() => true));
    if (gone) delete app.tickets[ticket.id];
  }

  /** A ticket « À faire » goes with its description, and an imported one is not brought back by the next import. */
  function confirmRemove() {
    app.modal = {
      kind: 'confirm',
      title: t('board.card.removeTitle', { key: ticket.key }),
      body: ticket.external
        ? t('board.card.removeBodyImported', { service: shortName(ticket.external.service) })
        : t('board.card.removeBody'),
      confirm: t('common.delete'),
      danger: true,
      onConfirm: remove,
    };
  }

  function contextMenu(e: MouseEvent) {
    // A text field keeps the browser's menu (copy, paste), as in main.ts.
    if ((e.target as Element).closest('input, textarea')) return;
    const items: MenuItem[] =
      ticket.column === 'todo'
        ? [
            { label: t('common.edit'), onClick: onedit },
            { label: t('board.card.prioritize'), onClick: () => app.run(api.ticketPrioritize(ticket.id)) },
            { label: '', separator: true },
            { label: t('common.delete'), danger: true, onClick: confirmRemove },
          ]
        : [
            { label: t('board.card.openAgent'), onClick: open, disabled: !agent },
            { label: '', separator: true },
            {
              label: t('common.delete'),
              danger: true,
              onClick: () => {
                if (ticket.column === 'done') return remove();
                app.modal = {
                  kind: 'confirm',
                  title: t('board.card.removeRunningTitle', { key: ticket.key }),
                  body: t('board.card.removeRunningBody'),
                  confirm: t('common.delete'),
                  danger: true,
                  onConfirm: remove,
                };
              },
            },
          ];
    menu.show(e, items);
  }

  /** The card opens its agent from the keyboard when it has the focus itself: its buttons keep their keys. */
  function key(e: KeyboardEvent) {
    if (e.target !== e.currentTarget || (e.key !== 'Enter' && e.key !== ' ')) return;
    // Space would scroll the column.
    e.preventDefault();
    open();
  }

  /** The card opens its agent on a click: its buttons only act. */
  function act(e: Event, f: () => unknown) {
    e.stopPropagation();
    f();
  }

  function approve() {
    const send = () => {
      // The test launches stop first (the backend does it too): their ends are no crashes.
      if (ticket.agentId) stopTests(ticket.agentId);
      return app.run(api.ticketApprove(ticket.id));
    };
    // A merge that removes the worktree takes the files of the agent open in the editor with it.
    const lost = s.action === 'merge' && s.cleanup && ticket.agentId ? buffers.unsavedIn(project.id, ticket.agentId) : 0;
    if (!lost) {
      send();
      return;
    }
    app.modal = {
      kind: 'confirm',
      title: t('board.card.approveTitle', { key: ticket.key }),
      body: t('board.card.approveBody') + lossNotice(lost),
      confirm: APPROVE_LABEL.merge,
      danger: true,
      onConfirm: send,
    };
  }

  async function reject(comment: string) {
    const sent = await app.run(api.ticketReject(ticket.id, comment).then(() => true));
    if (sent) rejecting = false;
  }
</script>

<div
  bind:this={card}
  class="card"
  class:done={ticket.column === 'done'}
  class:waiting
  role="button"
  tabindex="0"
  aria-label={`${ticket.key} ${ticket.title}`}
  onclick={open}
  onkeydown={key}
  oncontextmenu={contextMenu}
>
  <div class="top">
    {#if ticket.external}
      {@const svc = SERVICES[ticket.external.service]}
      <button
        class="ext"
        title={t('board.card.openExternal', { key: ticket.external.key, service: svc.name })}
        aria-label={t('board.card.openExternal', { key: ticket.external.key, service: svc.name })}
        onclick={(e) => act(e, () => app.run(openUrl(ticket.external!.url)))}
        ><span class="svc" style:background={svc.color} style:color={svc.ink} aria-hidden="true">{svc.letter}</span><span class="mono"
          >{ticket.external.key}</span
        ></button
      >
      {#if ticket.external.error}<button
          bind:this={warn}
          class="sync-err"
          aria-label={t('board.card.syncError', { service: svc.name, error: ticket.external.error })}
          aria-expanded={syncOpen}
          title={ticket.external.error}
          onclick={(e) => act(e, () => (syncOpen = !syncOpen))}>⚠</button
        >{/if}
    {/if}
    <span class="key mono">{ticket.key}</span>
    <div style="flex:1"></div>
    {#if ticket.column === 'doing'}<span class="loop mono"
        >{t('board.card.loop', { iteration: ticket.iteration, max: ticket.maxLoops })}</span
      >{/if}
    {#if ticket.partial && (ticket.column === 'review' || ticket.column === 'done')}<span class="partial">{t('board.card.partial')}</span
      >{/if}
  </div>
  {#if ticket.external?.error && syncOpen}
    <div class="sync" bind:this={panel}>
      <span class="why">{ticket.external.error}</span>
      <div class="row">
        <!-- Not disabled while it runs: the focus stays on it (resync asks once). -->
        <button class="small" aria-disabled={resyncing} onclick={(e) => act(e, resync)}>{t('board.card.resync')}</button>
      </div>
    </div>
  {/if}
  <span class="title">{ticket.title}</span>

  {#if ticket.column === 'done' && ticket.outcome}
    {#if ticket.outcomeUrl}
      <button class="outcome mono link" onclick={(e) => act(e, () => app.run(openUrl(ticket.outcomeUrl!)))}>{ticket.outcome}</button>
    {:else}
      <span class="outcome mono">{ticket.outcome}</span>
    {/if}
  {/if}

  {#snippet stepList()}
    <ul class="steps" aria-label={t('board.card.progressLabel')}>
      {#each steps as p, i (i)}<li>{p}</li>{/each}
    </ul>
  {/snippet}

  {#if ticket.column === 'review' && steps.length}
    <div class="prog">
      <span class="h">{t('board.card.doneHeading')}</span>
      {@render stepList()}
    </div>
  {/if}

  {#if ticket.column === 'doing' || ticket.column === 'review'}
    <ul class="crit" aria-label={t('board.card.criteriaLabel')}>
      {#each ticket.criteria as c, i (i)}
        <li class:ok={c.ok} title={c.note || undefined}><span class="mark">{c.ok ? '✓' : '○'}</span><span>{c.text}</span></li>
      {/each}
    </ul>
  {/if}

  {#if ticket.column === 'doing'}
    <div
      class="bar"
      role="progressbar"
      aria-label={t('board.card.criteriaMetLabel')}
      aria-valuemin={0}
      aria-valuemax={total}
      aria-valuenow={met}
    >
      <div style:width={`${total ? (met / total) * 100 : 0}%`}></div>
    </div>
    {#if steps.length}
      <div class="prog">
        {@render stepList()}
        {#if hidden > 0}<span class="more mono" title={t('board.card.moreSteps', { count: hidden })}>+{hidden}</span>{/if}
      </div>
    {/if}
    {#if !ticket.blocked}
      {#if waiting}
        <div class="act wait"><span class="pulse" style="width:7px;height:7px"></span>{t('board.card.questionWaiting')}</div>
      {:else if agent?.resumeAt}
        <div class="act">{t('board.card.resumes', { when: fWhen(agent.resumeAt, app.now) })}</div>
      {:else if agent?.setup}
        <div class="act" title={agent.setup}>
          <span class="dots"><span></span><span></span><span></span></span>{t('board.card.settingUp', { step: agent.setup })}
        </div>
      {:else if agent?.status === 'running'}
        <div class="act"><span class="dots"><span></span><span></span><span></span></span>{agent.activity ?? t('board.card.thinking')}</div>
      {/if}
    {/if}
  {/if}

  {#if ticket.column === 'todo'}
    <span class="meta">{t('board.card.todoMeta', { count: total, max: ticket.maxLoops })}</span>
    <div class="row">
      <span class="k">{waitLabel(ticket, queueIndex, s, busyCount, issue, app.autopilotPause, awaited)}</span>
      <div style="flex:1"></div>
      {#if canStart(ticket, s, busyCount, quota, issue, app.autopilotPause)}
        <button class="small" onclick={(e) => act(e, launch)}>{t('board.card.launch')}</button>
      {/if}
    </div>
  {/if}

  {#if (agent?.recipe || agent?.isola) && (ticket.column === 'doing' || ticket.column === 'review')}
    {@const a = agent}
    {@const address = flows.all[a.id]?.opened ?? openAddress(a)}
    {#if anyRunning(a.id)}
      <div class="row actions">
        <button class="small" onclick={(e) => act(e, () => stopTests(a.id))}>■ {t('board.card.stopTests')}</button>
        {#if address}<button class="small" onclick={(e) => act(e, () => app.run(openUrl(address)))}>{t('common.open')}</button>{/if}
      </div>
    {:else if ticket.column === 'review' && !ticket.step}
      <div class="row"><button class="small" onclick={(e) => act(e, () => testAgent(a, project))}>▶ {t('board.card.test')}</button></div>
    {/if}
  {/if}

  {#if ticket.step}
    <div class="act"><span class="dots"><span></span><span></span><span></span></span>{ticket.step}</div>
  {/if}

  {#if ticket.blocked}
    <div class="blocked" class:conflict={ticket.conflict}>
      <span class="why">{ticket.blocked}</span>
      {#if ticket.column === 'doing'}
        <div class="row">
          <button class="small" onclick={(e) => act(e, () => app.run(api.ticketResume(ticket.id)))}>{t('board.card.resume')}</button>
        </div>
      {:else if ticket.conflict}
        <div class="row actions">
          <button class="small" onclick={(e) => act(e, () => app.run(api.ticketResolveConflict(ticket.id)))}
            >{t('board.card.agentResolves')}</button
          >
          <button class="small" onclick={(e) => act(e, () => app.run(api.ticketDismiss(ticket.id)))}>{t('common.cancel')}</button>
        </div>
      {/if}
    </div>
  {/if}

  {#if ticket.column !== 'todo' && (agent || ticket.column === 'done')}
    <div class="row foot">
      {#if agent}<span class="who mono"
          ><StatusDot status={agent.status} size={7} /><span class="name" title={agent.name}>{agent.name}</span></span
        >{/if}
      {#if figures}
        <div class="figures">
          {#if used}
            <span class="k mono" title={used.estimated ? ESTIMATE_HINT : undefined}
              ><span class="sr">{t('board.card.costLabel')}{' '}</span>{fSpentUsd(used)}</span
            >
          {/if}
          {#if ticket.column === 'review'}<span class="k mono">{t('board.card.criteriaMet', { count: total, met, total })}</span>{/if}
          {#if ticket.column === 'done'}<span class="k mono">{doneMeta(ticket)}</span>{/if}
        </div>
      {/if}
    </div>
  {/if}

  {#if ticket.column === 'review' && !ticket.step && !ticket.conflict}
    {#if rejecting}
      <RejectForm onsubmit={reject} oncancel={() => (rejecting = false)} />
    {:else}
      <div class="row actions">
        <button class="approve" onclick={(e) => act(e, approve)}>{ticket.blocked ? t('common.retry') : APPROVE_LABEL[s.action]}</button>
        <button class="small grow" onclick={(e) => act(e, () => (rejecting = true))}>{t('board.card.sendBack')}</button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .card {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 9px;
    padding: 11px 12px;
    border-radius: var(--r-sm);
    background: var(--elev);
    border: 1px solid var(--line);
    cursor: pointer;
    outline: none;
  }
  .card:hover,
  .card:focus-visible {
    border-color: var(--line2);
  }
  .card.waiting {
    border-color: var(--wait);
  }
  .card.done {
    opacity: 0.72;
  }
  .top,
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .key {
    font-size: 10.5px;
    color: var(--dim);
  }
  .ext {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 0;
    border: none;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 10.5px;
    cursor: pointer;
  }
  .ext:hover {
    color: var(--text);
  }
  .svc {
    min-width: 15px;
    height: 15px;
    padding: 0 2px;
    flex: none;
    border-radius: 4px;
    font-size: 8.5px;
    font-weight: 800;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .sync-err {
    padding: 0 2px;
    border: none;
    border-radius: 3px;
    background: none;
    font: inherit;
    font-size: 11px;
    color: var(--wait);
    cursor: pointer;
  }
  .sync-err[aria-expanded='true'] {
    background: var(--wait-soft);
  }
  .sync {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 7px 9px;
    border-radius: var(--r-sm);
    background: var(--wait-soft);
    color: var(--wait);
    font-size: 11.5px;
  }
  .sync .why {
    overflow-wrap: anywhere;
  }
  .small[aria-disabled='true'] {
    opacity: 0.5;
    cursor: default;
  }
  .loop {
    font-size: 10.5px;
    font-weight: 600;
    color: var(--accent);
  }
  .partial {
    font-size: 10.5px;
    font-weight: 700;
    padding: 2px 6px;
    border-radius: 3px;
    background: var(--wait-soft);
    color: var(--wait);
  }
  .title {
    font-size: 13px;
    font-weight: 600;
    line-height: 1.4;
    text-wrap: pretty;
  }
  .outcome {
    align-self: flex-start;
    font-size: 10.5px;
    padding: 3px 7px;
    border-radius: 3px;
    background: var(--elev2);
    color: var(--ok);
  }
  .outcome.link {
    border: none;
    cursor: pointer;
  }
  .outcome.link:hover {
    text-decoration: underline;
  }
  .crit {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .crit li {
    display: flex;
    gap: 7px;
    font-size: 11.5px;
    line-height: 1.4;
  }
  .crit li.ok {
    color: var(--muted);
  }
  .mark {
    width: 10px;
    flex: none;
    color: var(--dim);
  }
  .crit li.ok .mark {
    color: var(--ok);
  }
  .prog {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .h {
    font-size: 10.5px;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--dim);
  }
  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .steps li {
    display: flex;
    gap: 7px;
    font-size: 11.5px;
    line-height: 1.4;
    color: var(--muted);
  }
  .steps li::before {
    content: '·';
    width: 10px;
    flex: none;
    text-align: center;
    color: var(--dim);
  }
  .more {
    align-self: flex-start;
    font-size: 10.5px;
    color: var(--dim);
  }
  .bar {
    height: 4px;
    border-radius: 2px;
    background: var(--elev2);
    overflow: hidden;
  }
  .bar div {
    height: 100%;
    background: var(--accent);
    transition: width 0.9s linear;
  }
  .act {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 11.5px;
    color: var(--muted);
  }
  .act.wait {
    color: var(--wait);
  }
  .dots {
    display: inline-flex;
    gap: 3px;
    flex: none;
  }
  .dots span {
    width: 4px;
    height: 4px;
    border-radius: 50%;
    background: var(--accent);
    animation: ccBlink 1.2s infinite;
  }
  .dots span:nth-child(2) {
    animation-delay: 0.15s;
  }
  .dots span:nth-child(3) {
    animation-delay: 0.3s;
  }
  .meta {
    font-size: 11.5px;
    color: var(--muted);
  }
  .k {
    font-size: 11px;
    color: var(--dim);
  }
  .who {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    font-size: 11px;
  }
  /* A long agent name is cut, not broken over the card's lines. */
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /*
   * The agent's name keeps at least a dozen characters: where the figures (cost, criteria met, loops) would squeeze it
   * below that, they go under it, and break between them if they do not fit one line either. A column is a quarter of
   * the board, and nothing may stick out of its card.
   */
  .foot {
    flex-wrap: wrap;
    row-gap: 3px;
  }
  .foot .who {
    flex: 1 1 13ch;
  }
  .figures {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 3px 8px;
    margin-left: auto;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  .blocked {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 7px 9px;
    border-radius: var(--r-sm);
    background: color-mix(in oklch, var(--del) 12%, transparent);
    color: var(--del);
    font-size: 11.5px;
  }
  .blocked.conflict {
    background: var(--wait-soft);
    color: var(--wait);
  }
  .small {
    height: 24px;
    padding: 0 10px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 11.5px;
    font-weight: 600;
    cursor: pointer;
  }
  .small:hover {
    border-color: var(--accent);
  }
  /* In a narrow column the buttons of a row go under one another (the "Valider" and "Renvoyer" ones then fill their line). */
  .actions {
    flex-wrap: wrap;
  }
  .small.grow,
  .approve {
    flex: 1;
    height: 28px;
  }
  .approve {
    border: none;
    border-radius: var(--r-sm);
    background: var(--ok);
    color: #0f1a12;
    font: inherit;
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
    white-space: nowrap;
  }
</style>
