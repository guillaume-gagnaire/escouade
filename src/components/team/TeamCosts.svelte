<script lang="ts">
  // « Équipe › Coûts » (TS-16, docs/TEAM-SERVER.md §10): the totals of the team's devices by day, encrypted for the
  // administrators alone. An administrator sees the table by member (when the team chose it), project, ticket, harness
  // and model, over the week, the month or a chosen period; a member sees itself and the total of each project. A cost on
  // a plan is an equivalent in API, marked. Then the budgets of the month, and what goes up of this member, exactly.
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { fPct, fTok, fUsd } from '../../lib/format';
  import {
    dimensions,
    fCostOf,
    grainLine,
    lines,
    periodBounds,
    planMark,
    projectName,
    teamCostsApi,
    total,
    type By,
    type Period,
    type TeamCostsView,
  } from '../../lib/team-costs';
  import { errorText } from '../../lib/teams.svelte';
  import { tip } from '../../lib/tip';

  let { teamId, onback, today = new Date() }: { teamId: string; onback: () => void; today?: Date } = $props();

  let v = $state<TeamCostsView | null>(null);
  let failure = $state('');
  let period = $state<Period>('month');
  let from = $state('');
  let to = $state('');
  let by = $state<By>('project');
  let showSent = $state(false);
  let sending = $state(false);

  async function load() {
    try {
      v = await teamCostsApi.get(teamId);
      failure = '';
    } catch (e) {
      failure = errorText(e);
    }
  }
  onMount(() => void load());

  async function sendNow() {
    sending = true;
    try {
      await teamCostsApi.send();
      await load();
    } catch (e) {
      failure = errorText(e);
    } finally {
      sending = false;
    }
  }

  const bounds = $derived(periodBounds(period, today, from, to));
  const dims = $derived(v ? dimensions(v) : (['project', 'ticket', 'model'] as By[]));
  const shownBy = $derived(dims.includes(by) ? by : 'project');
  const rows = $derived(v ? lines(v, shownBy, bounds) : []);
  const sum = $derived(total(rows));
  const summaryRows = $derived(v?.summary ? Object.entries(v.summary.projects).sort((a, b) => b[1].cost - a[1].cost) : []);
</script>

<div class="costs" data-testid="team-costs">
  {#if !v && !failure}
    <p class="muted">{t('teamCosts.loading')}</p>
  {:else if v}
    <p class="lead">{v.admin ? t('teamCosts.leadAdmin') : t('teamCosts.leadMember')}</p>

    <div class="bar">
      <div class="segmented small" role="group" aria-label={t('teamCosts.period.label')}>
        {#each ['week', 'month', 'custom'] as const as p (p)}
          <button class:on={period === p} aria-pressed={period === p} onclick={() => (period = p)}>{t(`teamCosts.period.${p}`)}</button>
        {/each}
      </div>
      {#if period === 'custom'}
        <label class="date">{t('teamCosts.period.from')} <input type="date" class="field" bind:value={from} /></label>
        <label class="date">{t('teamCosts.period.to')} <input type="date" class="field" bind:value={to} /></label>
      {/if}
    </div>

    <div class="segmented small" role="group" aria-label={t('teamCosts.by.label')}>
      {#each dims as d (d)}
        <button class:on={shownBy === d} aria-pressed={shownBy === d} onclick={() => (by = d)}>{t(`teamCosts.by.${d}`)}</button>
      {/each}
    </div>

    {#if rows.length === 0}
      <p class="muted">{t('teamCosts.empty')}</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th scope="col">{t('teamCosts.col.name')}</th>
            <th scope="col" class="n">{t('teamCosts.col.tokens')}</th>
            <th scope="col" class="n">{t('teamCosts.col.cost')}</th>
            {#if shownBy !== 'ticket'}<th scope="col" class="n">{t('teamCosts.col.quota')}</th>{/if}
          </tr>
        </thead>
        <tbody>
          {#each rows as r (r.key)}
            <tr class:mine={r.mine}>
              <th scope="row">{r.label}</th>
              <td class="n">{fTok(r.tokens)}</td>
              <td class="n">
                {fCostOf(r.cost, r.plan)}
                {#if planMark(r)}<span class="plan" use:tip={t('teamCosts.plan.hint')}>{planMark(r)}</span>{/if}
              </td>
              {#if shownBy !== 'ticket'}<td class="n">{r.quota === null ? '—' : fPct(r.quota)}</td>{/if}
            </tr>
          {/each}
        </tbody>
        <tfoot>
          <tr>
            <th scope="row">{t('teamCosts.total')}</th>
            <td class="n">{fTok(sum.tokens)}</td>
            <td class="n">{fCostOf(sum.cost, sum.plan)}</td>
            {#if shownBy !== 'ticket'}<td></td>{/if}
          </tr>
        </tfoot>
      </table>
    {/if}

    {#if !v.admin && summaryRows.length}
      <section>
        <h4>{t('teamCosts.summary.title')}</h4>
        <ul class="plain">
          {#each summaryRows as [journal, spent] (journal)}
            <li><span>{projectName(v, journal)}</span><span class="n">{fUsd(spent.cost)}</span></li>
          {/each}
          <li class="strong"><span>{t('teamCosts.summary.team')}</span><span class="n">{fUsd(v.summary!.team.cost)}</span></li>
        </ul>
      </section>
    {/if}

    <section>
      <h4>{t('teamCosts.budgets.title')}</h4>
      {#if v.budgets.length === 0}
        <p class="muted small">{t('teamCosts.budgets.none')}</p>
      {:else}
        <ul class="budgets">
          {#each v.budgets as b (b.scope + b.of)}
            <li class:warn={b.pct >= 80 && b.pct < 100} class:over={b.pct >= 100}>
              <span class="what">{b.text}</span>
              <span class="meter" aria-hidden="true"><span style:width="{Math.min(100, b.pct)}%"></span></span>
              <span class="n small">{t('teamCosts.budgets.spent', { spent: fUsd(b.spent), budget: fUsd(b.budget) })}</span>
            </li>
          {/each}
        </ul>
        {#if v.rules.stop}<p class="muted small">{t('teamCosts.budgets.stop')}</p>{/if}
      {/if}
    </section>

    <section class="mine-box" aria-labelledby="costs-mine">
      <h4 id="costs-mine">{t('teamCosts.mine.title')}</h4>
      <p>{t('teamCosts.mine.lead')}</p>
      <p class="grain">{grainLine(v)}</p>
      {#if v.epoch === null}
        <p class="advice small">{t('teamCosts.key.none')}</p>
      {:else if v.stale}
        <p class="advice small">{t('teamCosts.key.stale')}</p>
      {/if}
      {#if v.pending.length}
        <p class="small">{t('teamCosts.mine.pending', { count: v.pending.length })}</p>
      {/if}
      <div class="row">
        {#if v.sent}
          <button class="btn ghost" aria-expanded={showSent} onclick={() => (showSent = !showSent)}
            >{showSent ? t('teamCosts.mine.hide') : t('teamCosts.mine.show')}</button
          >
        {:else}
          <span class="muted small">{t('teamCosts.mine.none')}</span>
        {/if}
        <button class="btn ghost" disabled={sending} onclick={sendNow}>{t('teamCosts.mine.send')}</button>
      </div>
      {#if showSent && v.sent}
        <pre class="sent" data-testid="team-costs-sent">{JSON.stringify(v.sent, null, 2)}</pre>
      {/if}
    </section>
  {/if}

  {#if failure}<p class="error" role="alert">{failure}</p>{/if}
  <div class="acts"><button class="btn ghost" onclick={onback}>{t('team.common.back')}</button></div>
</div>

<style>
  .costs {
    display: flex;
    flex-direction: column;
    gap: 12px;
    font-size: 13px;
  }
  .lead {
    color: var(--soft);
    margin: 0;
    line-height: 1.5;
  }
  p {
    margin: 0;
    line-height: 1.5;
  }
  .muted {
    color: var(--muted);
  }
  .small {
    font-size: 12px;
  }
  .advice {
    color: var(--warn-ink);
  }
  .error {
    color: var(--del);
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .date {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--soft);
    font-size: 12px;
  }
  .date .field {
    width: 10.5em;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th,
  td {
    padding: 6px 8px;
    border-bottom: 1px solid var(--line);
    text-align: left;
    font-weight: 400;
  }
  thead th {
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 560;
  }
  tbody th {
    color: var(--text);
    overflow-wrap: anywhere;
  }
  tfoot th,
  tfoot td {
    font-weight: 600;
    border-bottom: none;
  }
  .n {
    text-align: right;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  tr.mine th {
    color: var(--p-ink);
  }
  .plan {
    display: inline-block;
    margin-left: 6px;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--p-soft);
    color: var(--p-ink);
    font-size: 11px;
    line-height: 18px;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  h4 {
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 560;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }
  .plain,
  .budgets {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .plain li {
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }
  .plain li.strong {
    font-weight: 600;
  }
  .budgets li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 120px auto;
    align-items: center;
    gap: 10px;
  }
  .meter {
    height: 6px;
    border-radius: 3px;
    background: var(--well);
    overflow: hidden;
  }
  .meter span {
    display: block;
    height: 100%;
    background: var(--p);
  }
  .budgets li.warn .meter span {
    background: var(--wait);
  }
  .budgets li.over .meter span {
    background: var(--del);
  }
  .budgets li.over .what {
    color: var(--del);
  }
  .mine-box {
    padding: 10px 12px;
    border: 1px solid var(--line);
    border-radius: var(--r);
    background: var(--panel);
  }
  .grain {
    color: var(--text);
    font-weight: 560;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .sent {
    margin: 0;
    max-height: 220px;
    overflow: auto;
    padding: 8px;
    border-radius: var(--r-sm);
    background: var(--well);
    font-family: var(--mono);
    font-size: 11.5px;
  }
  .acts {
    display: flex;
    justify-content: flex-end;
  }
</style>
