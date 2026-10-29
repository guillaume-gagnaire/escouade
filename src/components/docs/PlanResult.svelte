<script lang="ts">
  import Icon from '../Icon.svelte';
  import { docsStore } from '../../lib/docs.svelte';
  import { fAgo } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { planProgress } from '../../lib/planning';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { DocsPlan, Project } from '../../lib/types';

  // What a « Planifier » made (ESC-262), drawn as the subjects' cards of the Specification (ESC-291): its tickets, each
  // opening the Kanban with its column, and where they stand. The note and « Annuler » follow that state: « Annuler »
  // removes only the tickets not started yet, and is gone once none is left; all started, the list folds into its keys.
  // The card closes (`docsStore.closePlan`). Under a discussion, and on top of the Specification.
  let { project, plan }: { project: Project; plan: DocsPlan } = $props();

  const when = $derived(fAgo(plan.at / 1000, app.now));
  const p = $derived(planProgress(plan, app.tickets));
  const total = $derived(plan.made.length);
  const keys = (list: { key: string }[]) => list.map((m) => m.key).join(', ');
  /** The user's fold, if any; else folded once every ticket left has started. */
  let unfolded = $state<boolean | null>(null);
  const open = $derived(unfolded ?? p.state !== 'started');
  let cancelling = $state(false);

  const pill = $derived.by(() => {
    switch (p.state) {
      case 'todo':
        return t('docs.plan.states.todo', { count: total });
      case 'partial':
        return t('docs.plan.states.partial', { count: p.started.length, total });
      case 'started':
        return t('docs.plan.states.started', { count: p.left.length });
      case 'cancelled':
        return t('docs.plan.states.cancelled');
      case 'gone':
        return t('docs.plan.states.gone');
      default:
        return '';
    }
  });

  const note = $derived.by(() => {
    switch (p.state) {
      case 'none':
        return t('docs.plan.none');
      case 'todo':
        return t('docs.plan.notStarted', { count: p.toCancel.length });
      case 'partial':
        return p.toCancel.length > 3
          ? t('docs.plan.partlyMany', { count: p.toCancel.length })
          : t('docs.plan.partly', { count: p.toCancel.length, keys: keys(p.toCancel) });
      case 'cancelled':
        return p.left.length ? t('docs.plan.undoneKept', { count: p.left.length, keys: keys(p.left) }) : t('docs.plan.undone');
      default:
        return '';
    }
  });

  async function cancel() {
    cancelling = true;
    await docsStore.cancelPlan(project.id, plan.id);
    cancelling = false;
  }
</script>

<section class="plan {p.state}" data-plan={plan.id} data-state={p.state} aria-label={t('docs.plan.heading')}>
  <header class="head">
    <span class="ico" aria-hidden="true"><Icon name="kanban" size={14} /></span>
    <span class="title">{total ? t('docs.plan.made', { count: total, when }) : t('docs.plan.planned', { when })}</span>
    {#if pill}<span class="badge {p.state}" data-plan-state>{pill}</span>{/if}
    {#if total && p.state === 'started'}
      <button
        class="x"
        aria-expanded={open}
        aria-label={open ? t('docs.plan.fold') : t('docs.plan.unfold')}
        use:tip={open ? t('docs.plan.fold') : t('docs.plan.unfold')}
        onclick={() => (unfolded = !open)}><Icon name={open ? 'down' : 'right'} size={12} /></button
      >
    {/if}
    <button
      class="x"
      data-plan-close
      aria-label={t('docs.plan.close')}
      use:tip={t('docs.plan.closeTip')}
      onclick={() => docsStore.closePlan(plan.id)}><Icon name="close" size={12} /></button
    >
  </header>

  {#if total && open}
    <ul class="made">
      {#each plan.made as m (m.id)}
        {@const ticket = app.tickets[m.id]}
        <li data-made={m.key}>
          <button
            class="tk"
            disabled={!ticket}
            aria-label={t('docs.plan.open', { key: m.key })}
            use:tip={`${m.key} · ${m.title}`}
            onclick={() => app.openBoard(project.id)}
            ><span class="key">{m.key}</span><span class="label">{m.title}</span><span class="col {ticket?.column ?? 'gone'}"
              >{ticket ? t(`board.columns.${ticket.column}`) : t('docs.plan.removed')}</span
            ></button
          >
        </li>
      {/each}
    </ul>
  {:else if total}
    <!-- Folded: the keys, colored by their column, as on the subjects' cards. -->
    <div class="keys" data-plan-keys>
      {#each p.left as m (m.id)}
        {@const ticket = app.tickets[m.id]}
        <button
          class="tkey {ticket.column}"
          aria-label={t('docs.plan.open', { key: m.key })}
          use:tip={`${m.key} · ${m.title} · ${t(`board.columns.${ticket.column}`)}`}
          onclick={() => app.openBoard(project.id)}>{m.key}</button
        >
      {/each}
    </div>
  {/if}

  {#if note || p.toCancel.length || plan.skipped.length}
    <footer class="foot">
      <span class="notes">
        {#if note}<span class="note" data-plan-note>{note}</span>{/if}
        {#if plan.skipped.length}
          <span class="note" data-skipped
            >{t('docs.plan.skipped', { count: plan.skipped.length, keys: [...new Set(plan.skipped.map((s) => s.key))].join(', ') })}</span
          >
        {/if}
      </span>
      {#if p.toCancel.length}
        <button
          class="btn ghost"
          data-plan-cancel
          disabled={cancelling}
          use:tip={t('docs.plan.cancelTip', { count: p.toCancel.length, keys: keys(p.toCancel) })}
          onclick={cancel}>{t('docs.plan.cancel')}</button
        >
      {/if}
    </footer>
  {/if}
</section>

<style>
  /* The language of the subjects' cards (SpecSpace `.card`): radius, border, padding, title and pill. */
  .plan {
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
    padding: 14px 12px 12px 16px;
    border: 1px solid var(--line-2);
    border-radius: 12px;
    background: var(--raise);
    color: var(--soft);
    font-size: 13px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .ico {
    flex: none;
    display: grid;
    color: var(--muted);
  }
  .title {
    flex: 1;
    min-width: 0;
    color: var(--text);
    font-size: 14.5px;
    font-weight: 640;
    overflow-wrap: anywhere;
  }
  .badge {
    flex: none;
    padding: 2px 9px;
    border-radius: 999px;
    background: var(--hover);
    color: var(--muted);
    font-size: 12px;
    font-weight: 560;
    white-space: nowrap;
  }
  .badge.partial {
    background: oklch(from var(--p-ink) l c h / 0.14);
    color: var(--p-ink);
  }
  .badge.started {
    background: oklch(from var(--run) l c h / 0.12);
    color: var(--run);
  }
  .x {
    flex: none;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: none;
    color: var(--dim);
    cursor: pointer;
  }
  .x:hover {
    background: var(--hover);
    color: var(--text);
  }

  /* The tickets, their keys, titles and columns lined up (subgrid): one column in a narrow card (a discussion's side),
     two in a wide one. */
  .made {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    gap: 2px 0;
    margin: 0 4px 0 -8px;
    padding: 0;
    list-style: none;
  }
  @container (width >= 760px) {
    .made {
      grid-template-columns: auto minmax(0, 1fr) auto auto minmax(0, 1fr) auto;
      column-gap: 0;
    }
    .made li:nth-child(even) {
      margin-left: 14px;
    }
  }
  .made li {
    grid-column: span 3;
    display: grid;
    grid-template-columns: subgrid;
    min-width: 0;
  }
  .tk {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: subgrid;
    align-items: baseline;
    width: 100%;
    min-width: 0;
    padding: 5px 8px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--soft);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .tk:hover:not(:disabled) {
    background: var(--hover);
    color: var(--text);
  }
  .tk:disabled {
    color: var(--dim);
    cursor: default;
  }
  .tk:disabled .label {
    text-decoration: line-through;
  }
  .key {
    padding-right: 10px;
    color: var(--muted);
    font-family: var(--mono);
    font-size: 12px;
  }
  .label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .col {
    padding-left: 10px;
    color: var(--dim);
    font-size: 12px;
    white-space: nowrap;
  }
  .col.doing,
  .col.review {
    color: var(--p-ink);
  }
  .col.done {
    color: var(--run);
  }

  .keys {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .tkey {
    padding: 0 6px;
    border: none;
    border-radius: 5px;
    background: var(--hover);
    color: var(--muted);
    font-family: var(--mono);
    font-size: 11px;
    line-height: 18px;
    cursor: pointer;
  }
  .tkey:hover {
    background: var(--line-2);
  }
  .tkey.done {
    color: var(--run);
  }
  .tkey.doing,
  .tkey.review {
    color: var(--p-ink);
  }

  /* The note and « Annuler » on one line, the button at the end of what it speaks of. */
  .foot {
    display: flex;
    align-items: center;
    gap: 6px 14px;
    padding-top: 10px;
    border-top: 1px solid var(--line);
  }
  .notes {
    flex: 1 1 0;
    display: grid;
    gap: 2px;
    min-width: 0;
  }
  .note {
    color: var(--dim);
    font-size: 12.5px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  .foot .btn {
    flex: none;
    margin-left: auto;
  }
</style>
