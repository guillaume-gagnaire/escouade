<script lang="ts">
  import { t } from '../lib/i18n';
  import { api } from '../lib/ipc';
  import { menu } from '../lib/menu.svelte';
  import { nextLabel, rhythmLabel, runningTicket, tasksOf } from '../lib/scheduled';
  import { shortWindow } from '../lib/short-window.svelte';
  import { app } from '../lib/state.svelte';
  import { tip } from '../lib/tip';
  import type { Project, ScheduledTask, Ticket } from '../lib/types';
  import FoldTitle from './FoldTitle.svelte';
  import Icon from './Icon.svelte';
  import Switch from './settings/Switch.svelte';

  // « Programmées » in the sidebar (ESC-448, programmees-barre.png, place A): above the launches, without a frame, as
  // « Lancement » and « Terminaux ». Each row: the task's title and rhythm, its next due time (or « en cours » with the
  // link to its ticket), and the switch of its pause. Its rows have a height of their own and scroll: a dozen tasks do not
  // take the whole bar. « ＋ » opens the ticket's form in « Programmer »; a row opens it on the task.

  let { project }: { project: Project } = $props();

  let unfolded = $state(false);
  const folded = $derived(shortWindow.now && !unfolded);
  const tasks = $derived(tasksOf(project.id, app.scheduled));
  /** The minute that passes, for « ce soir » to become « demain » without an event. */
  let now = $state(Date.now());
  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), 60_000);
    return () => clearInterval(timer);
  });

  function add() {
    app.modal = { kind: 'schedule', projectId: project.id };
  }

  function edit(task: ScheduledTask) {
    app.modal = { kind: 'schedule', projectId: project.id, task };
  }

  function failed(e: unknown) {
    app.toast(e instanceof Error ? e.message : String(e), 'error');
  }

  /** The switch: on runs it, off pauses it. */
  function setOn(task: ScheduledTask, on: boolean) {
    api.scheduledPause(task.id, !on).catch(failed);
  }

  /** « Accepter » a task an agent scheduled (ESC-647), as the row shows it. */
  function accept(e: Event, task: ScheduledTask) {
    e.stopPropagation();
    api.scheduledAccept(task.id, task.print ?? null).catch(failed);
  }

  /** Who scheduled it, when an agent did. */
  function byLabel(task: ScheduledTask): string {
    const a = task.author;
    if (!a) return '';
    return a.ticketKey
      ? t('scheduled.section.byTicket', { name: a.agentName, key: a.ticketKey })
      : t('scheduled.section.by', { name: a.agentName });
  }

  function remove(task: ScheduledTask) {
    app.modal = {
      kind: 'confirm',
      title: t('scheduled.section.deleteTitle'),
      body: t('scheduled.section.deleteBody', { title: task.title }),
      confirm: t('scheduled.section.delete'),
      danger: true,
      onConfirm: () => api.scheduledDelete(task.id).catch(failed),
    };
  }

  function rowMenu(e: MouseEvent, task: ScheduledTask) {
    menu.show(e, [
      { label: t('scheduled.section.edit'), icon: 'edit', onClick: () => edit(task) },
      {
        label: task.paused ? t('scheduled.section.resume', { title: task.title }) : t('scheduled.section.pause', { title: task.title }),
        onClick: () => setOn(task, task.paused),
      },
      { label: '', separator: true },
      { label: t('scheduled.section.delete'), icon: 'trash', danger: true, onClick: () => remove(task) },
    ]);
  }

  /** Its ticket under way: its agent's conversation once it has one, else the Kanban. */
  function openTicket(e: Event, tk: Ticket) {
    e.stopPropagation();
    if (tk.agentId && app.agents[tk.agentId]) app.selectAgent(tk.agentId);
    else app.openBoard(project.id);
  }
</script>

<section class="sec sched" class:folded aria-label={t('scheduled.section.title')} data-scheduled-section>
  <div class="sec-h">
    <Icon name="clock" size={13} />
    <FoldTitle title={t('scheduled.section.title')} short={shortWindow.now} {folded} ontoggle={() => (unfolded = !unfolded)} />
    {#if tasks.length}<span class="n">{tasks.length}</span>{/if}
    <span class="acts">
      <button
        class="icon-btn"
        use:tip={{ text: t('scheduled.section.add'), aria: false }}
        aria-label={t('scheduled.section.add')}
        onclick={add}><Icon name="plus" size={14} /></button
      >
    </span>
  </div>
  <div class="rows">
    {#each tasks as task (task.id)}
      {@const tk = runningTicket(task, app.tickets)}
      <div
        class="row-item"
        class:paused={task.paused || task.awaiting}
        class:awaiting={task.awaiting}
        role="button"
        tabindex="0"
        data-task={task.id}
        onclick={() => edit(task)}
        onkeydown={(e) => e.key === 'Enter' && e.target === e.currentTarget && edit(task)}
        oncontextmenu={(e) => rowMenu(e, task)}
      >
        <span class="txt">
          <span class="nm" use:tip={{ text: task.title, cut: true }}>{task.title}</span>
          <span class="when"
            ><span>{rhythmLabel(task.rhythm)}</span><span class="sep">{' · '}</span>{#if task.awaiting}<span class="wait"
                >{t('scheduled.section.awaiting')}</span
              >{:else if task.paused}<span>{t('scheduled.section.paused')}</span>{:else if tk}<button
                class="run"
                onclick={(e) => openTicket(e, tk)}>{t('scheduled.section.running', { key: tk.key })}</button
              >{:else if task.next}<span class="next" use:tip={task.lastNote ?? ''}>{nextLabel(task.next, now)}</span>{:else}<span
                >{t('scheduled.section.never')}</span
              >{/if}</span
          >
          {#if task.author}<span class="by" data-author use:tip={t('scheduled.section.byTip', { name: task.author.agentName })}
              >{byLabel(task)}</span
            >{/if}
        </span>
        {#if task.awaiting}
          <button
            class="accept"
            aria-label={t('scheduled.section.acceptLabel', { title: task.title, name: task.author?.agentName ?? '' })}
            onclick={(e) => accept(e, task)}>{t('scheduled.section.accept')}</button
          >
        {:else}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <span class="sw" onclick={(e) => e.stopPropagation()}>
            <Switch
              label={task.paused
                ? t('scheduled.section.resume', { title: task.title })
                : t('scheduled.section.pause', { title: task.title })}
              bind:on={() => !task.paused, (on) => setOn(task, on)}
            />
          </span>
        {/if}
      </div>
    {:else}
      <div class="none">{t('scheduled.section.none')}</div>
    {/each}
  </div>
</section>

<style>
  .sec {
    display: grid;
    gap: 1px;
    min-height: 0;
    margin: 6px -2px 0;
    padding-top: 8px;
    border-top: 1px solid var(--line);
  }
  .sec-h {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 2px 0 8px;
    color: var(--muted);
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
  }
  .sec-h .n {
    color: var(--dim);
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .sec-h .acts {
    display: flex;
    gap: 2px;
    margin-left: auto;
  }
  /* Of a height of its own, and scrolled: a dozen tasks never take the whole bar. */
  .rows {
    display: grid;
    gap: 1px;
    max-height: min(26vh, 220px);
    overflow: auto;
  }
  .row-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 6px 5px 9px;
    border-radius: 7px;
    cursor: pointer;
    outline: none;
  }
  .row-item:hover {
    background: var(--hover);
  }
  .row-item:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: -2px;
  }
  .txt {
    display: grid;
    flex: 1 1 auto;
    min-width: 0;
    gap: 1px;
  }
  .nm {
    overflow: hidden;
    color: var(--soft);
    font-size: 12.5px;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .when {
    overflow: hidden;
    color: var(--muted);
    font-size: 11.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .next {
    color: var(--p-ink);
  }
  .run {
    padding: 0;
    border: none;
    background: transparent;
    color: var(--run);
    font: inherit;
    cursor: pointer;
  }
  .run:hover {
    text-decoration: underline;
  }
  .paused .nm,
  .paused .when {
    color: var(--dim);
  }
  /* Scheduled by an agent, awaiting the user's go-ahead (ESC-647): said, and its author shown. */
  .awaiting .wait {
    color: var(--warn-ink);
  }
  .by {
    overflow: hidden;
    color: var(--dim);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .accept {
    flex: none;
    padding: 2px 8px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: transparent;
    color: var(--soft);
    font: inherit;
    font-size: 11.5px;
    cursor: pointer;
  }
  .accept:hover {
    background: var(--hover);
  }
  .sw {
    display: flex;
    flex: none;
  }
  .none {
    padding: 2px 9px 6px;
    color: var(--dim);
    font-size: 12px;
  }
  /* A short window: the section folds to its header, as « Lancement » (choice 9), until its title unfolds it. */
  .folded .rows {
    display: none;
  }
  .sec.folded {
    padding-bottom: 2px;
  }
</style>
