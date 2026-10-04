<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { APPROVE_LABEL, canStart, criteriaMet, doneMeta, waitLabel } from '../../lib/board';
  import { buffers, lossNotice } from '../../lib/editor/buffers.svelte';
  import { fWhen, plural } from '../../lib/format';
  import { api } from '../../lib/ipc';
  import { menu, type MenuItem } from '../../lib/menu.svelte';
  import { openAddress } from '../../lib/recipe';
  import { app } from '../../lib/state.svelte';
  import { anyRunning, stopTests, testAgent } from '../../lib/test-launch.svelte';
  import type { Project, Ticket } from '../../lib/types';
  import StatusDot from '../StatusDot.svelte';
  import RejectForm from './RejectForm.svelte';

  /** Items of the agent's progress a ticket under way shows: the last ones. */
  const SHOWN = 3;

  let {
    ticket: t,
    project,
    queueIndex,
    busyCount,
    quota,
    onedit,
  }: { ticket: Ticket; project: Project; queueIndex: number; busyCount: number; quota: number | null; onedit: () => void } = $props();

  const s = $derived(project.board);
  /** Why no ticket of the board starts now (its target branch), if so. */
  const issue = $derived(app.boardIssues[project.id] ?? null);
  const agent = $derived(t.agentId ? app.agents[t.agentId] : undefined);
  const met = $derived(criteriaMet(t));
  const total = $derived(t.criteria.length);
  const waiting = $derived(t.column === 'doing' && agent?.status === 'waiting');
  // Under way, the last items; to test, all of them.
  const steps = $derived(t.column === 'doing' ? t.progress.slice(-SHOWN) : t.progress);
  const hidden = $derived(t.progress.length - steps.length);
  let rejecting = $state(false);

  function open() {
    if (t.column === 'todo') onedit();
    else if (t.agentId) app.selectAgent(t.agentId);
  }

  async function remove() {
    // Its agent is archived with it, which stops its test launches: stopped first, their ends are no crashes.
    if (t.column !== 'todo' && t.agentId) stopTests(t.agentId);
    const gone = await app.run(api.ticketDelete(t.id).then(() => true));
    if (gone) delete app.tickets[t.id];
  }

  function contextMenu(e: MouseEvent) {
    // A text field keeps the browser's menu (copy, paste), as in main.ts.
    if ((e.target as Element).closest('input, textarea')) return;
    const items: MenuItem[] =
      t.column === 'todo'
        ? [
            { label: 'Modifier', onClick: onedit },
            { label: 'Passer en tête', onClick: () => app.run(api.ticketPrioritize(t.id)) },
            { label: '', separator: true },
            { label: 'Supprimer', danger: true, onClick: remove },
          ]
        : [
            { label: "Ouvrir l'agent", onClick: open, disabled: !agent },
            { label: '', separator: true },
            {
              label: 'Supprimer',
              danger: true,
              onClick: () => {
                if (t.column === 'done') return remove();
                app.modal = {
                  kind: 'confirm',
                  title: `Supprimer le ticket ${t.key} ?`,
                  body: 'Son agent est archivé, avec son worktree.',
                  confirm: 'Supprimer',
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
      if (t.agentId) stopTests(t.agentId);
      return app.run(api.ticketApprove(t.id));
    };
    // A merge that removes the worktree takes the files of the agent open in the editor with it.
    const lost = s.action === 'merge' && s.cleanup && t.agentId ? buffers.unsavedIn(project.id, t.agentId) : 0;
    if (!lost) {
      send();
      return;
    }
    app.modal = {
      kind: 'confirm',
      title: `Valider ${t.key} ?`,
      body: 'Le worktree de son agent est supprimé après le merge.' + lossNotice(lost),
      confirm: APPROVE_LABEL.merge,
      danger: true,
      onConfirm: send,
    };
  }

  async function reject(comment: string) {
    const sent = await app.run(api.ticketReject(t.id, comment).then(() => true));
    if (sent) rejecting = false;
  }
</script>

<div
  class="card"
  class:done={t.column === 'done'}
  class:waiting
  role="button"
  tabindex="0"
  aria-label={`${t.key} ${t.title}`}
  onclick={open}
  onkeydown={key}
  oncontextmenu={contextMenu}
>
  <div class="top">
    <span class="key mono">{t.key}</span>
    <div style="flex:1"></div>
    {#if t.column === 'doing'}<span class="loop mono">Boucle {t.iteration}/{t.maxLoops}</span>{/if}
    {#if t.partial && (t.column === 'review' || t.column === 'done')}<span class="partial">Objectif partiel</span>{/if}
  </div>
  <span class="title">{t.title}</span>

  {#if t.column === 'done' && t.outcome}
    {#if t.outcomeUrl}
      <button class="outcome mono link" onclick={(e) => act(e, () => app.run(openUrl(t.outcomeUrl!)))}>{t.outcome}</button>
    {:else}
      <span class="outcome mono">{t.outcome}</span>
    {/if}
  {/if}

  {#snippet stepList()}
    <ul class="steps" aria-label="Avancement">
      {#each steps as p, i (i)}<li>{p}</li>{/each}
    </ul>
  {/snippet}

  {#if t.column === 'review' && steps.length}
    <div class="prog">
      <span class="h">Ce qui a été fait</span>
      {@render stepList()}
    </div>
  {/if}

  {#if t.column === 'doing' || t.column === 'review'}
    <ul class="crit" aria-label="Critères">
      {#each t.criteria as c, i (i)}
        <li class:ok={c.ok} title={c.note || undefined}><span class="mark">{c.ok ? '✓' : '○'}</span><span>{c.text}</span></li>
      {/each}
    </ul>
  {/if}

  {#if t.column === 'doing'}
    <div class="bar" role="progressbar" aria-label="Critères atteints" aria-valuemin={0} aria-valuemax={total} aria-valuenow={met}>
      <div style:width={`${total ? (met / total) * 100 : 0}%`}></div>
    </div>
    {#if steps.length}
      <div class="prog">
        {@render stepList()}
        {#if hidden > 0}<span class="more mono" title={plural(hidden, 'autre élément', 'autres éléments')}>+{hidden}</span>{/if}
      </div>
    {/if}
    {#if !t.blocked}
      {#if waiting}
        <div class="act wait"><span class="pulse" style="width:7px;height:7px"></span>Question en attente de ta réponse</div>
      {:else if agent?.resumeAt}
        <div class="act">Reprise {fWhen(agent.resumeAt, app.now)}</div>
      {:else if agent?.status === 'running'}
        <div class="act"><span class="dots"><span></span><span></span><span></span></span>{agent.activity ?? 'Réfléchit'}</div>
      {/if}
    {/if}
  {/if}

  {#if t.column === 'todo'}
    <span class="meta">{plural(total, 'critère', 'critères')} · max {t.maxLoops} boucles</span>
    <div class="row">
      <span class="k">{waitLabel(t, queueIndex, s, busyCount, issue)}</span>
      <div style="flex:1"></div>
      {#if canStart(t, s, busyCount, quota, issue)}
        <button class="small" onclick={(e) => act(e, () => app.run(api.ticketStart(t.id)))}>Lancer</button>
      {/if}
    </div>
  {/if}

  {#if agent?.recipe && (t.column === 'doing' || t.column === 'review')}
    {@const a = agent}
    {@const address = openAddress(a)}
    {#if anyRunning(a.id)}
      <div class="row actions">
        <button class="small" onclick={(e) => act(e, () => stopTests(a.id))}>■ Arrêter</button>
        {#if address}<button class="small" onclick={(e) => act(e, () => app.run(openUrl(address)))}>Ouvrir</button>{/if}
      </div>
    {:else if t.column === 'review' && !t.step}
      <div class="row"><button class="small" onclick={(e) => act(e, () => testAgent(a, project))}>▶ Tester</button></div>
    {/if}
  {/if}

  {#if t.step}
    <div class="act"><span class="dots"><span></span><span></span><span></span></span>{t.step}</div>
  {/if}

  {#if t.blocked}
    <div class="blocked" class:conflict={t.conflict}>
      <span class="why">{t.blocked}</span>
      {#if t.column === 'doing'}
        <div class="row"><button class="small" onclick={(e) => act(e, () => app.run(api.ticketResume(t.id)))}>Reprendre</button></div>
      {:else if t.conflict}
        <div class="row actions">
          <button class="small" onclick={(e) => act(e, () => app.run(api.ticketResolveConflict(t.id)))}>L'agent résout</button>
          <button class="small" onclick={(e) => act(e, () => app.run(api.ticketDismiss(t.id)))}>Annuler</button>
        </div>
      {/if}
    </div>
  {/if}

  {#if t.column !== 'todo' && (agent || t.column === 'done')}
    <div class="row foot">
      {#if agent}<span class="who mono"
          ><StatusDot status={agent.status} size={7} /><span class="name" title={agent.name}>{agent.name}</span></span
        >{/if}
      <div style="flex:1"></div>
      {#if t.column === 'review'}<span class="k mono">{met}/{total} critères</span>{/if}
      {#if t.column === 'done'}<span class="k mono">{doneMeta(t)}</span>{/if}
    </div>
  {/if}

  {#if t.column === 'review' && !t.step && !t.conflict}
    {#if rejecting}
      <RejectForm onsubmit={reject} oncancel={() => (rejecting = false)} />
    {:else}
      <div class="row actions">
        <button class="approve" onclick={(e) => act(e, approve)}>{t.blocked ? 'Réessayer' : APPROVE_LABEL[s.action]}</button>
        <button class="small grow" onclick={(e) => act(e, () => (rejecting = true))}>Renvoyer</button>
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
  .foot .k {
    flex: none;
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
