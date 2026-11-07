<script lang="ts">
  import { accountEngines, accountName, engineOfAccount } from '../lib/accounts';
  import { engineName } from '../lib/engines';
  import Rich from '../lib/i18n/Rich.svelte';
  import EngineMark from './EngineMark.svelte';
  import { readPref, writePref } from '../lib/prefs';
  import { fDateShort, fInt, fPct, fTok } from '../lib/format';
  import { locale, t } from '../lib/i18n';
  import { api } from '../lib/ipc';
  import { displayModel } from '../lib/models';
  import { app } from '../lib/state.svelte';
  import {
    costHint,
    costKindOf,
    fCost,
    figureParts,
    kpis,
    modelColor,
    niceMax,
    ROWS,
    SERIES_COLORS,
    shown,
    ticks,
    type Range,
  } from '../lib/stats';
  import type { Bucket, EngineId, StatsView } from '../lib/types';
  import { tip } from '../lib/tip';

  // The series of the tokens, from the bottom of a column, in the palette validated in both themes.
  const SERIES = (['input', 'cache', 'output'] as const).map((key) => ({ key, color: SERIES_COLORS[key] }));
  /** The chart's height, its axis' room at the left, its labels' at the bottom, as the preview's. */
  const H = 280;
  const LEFT = 44;
  const BOTTOM = 26;
  const PLOT = H - BOTTOM - 8;
  const RANGES = ['day', 'week', 'month'] as const satisfies readonly Range[];

  let range = $state<Range>((readPref('statsRange') as Range) || 'day');
  let view = $state<StatsView | null>(null);
  let table = $state(false);
  let hover = $state<number | null>(null);
  let error = $state<string | null>(null);
  /** « Tout voir » opened the whole list of agents, of tickets (20 lines each until then). */
  let allAgents = $state(false);
  let allTickets = $state(false);
  /** The account chosen ('': every account). Not kept from one visit to the next: the totals are what the window opens on. */
  let account = $state('');
  /** The harness chosen ('': every harness), combined with the account. Not kept either. */
  let engine = $state('');
  let loadTimer: ReturnType<typeof setTimeout> | undefined;
  /** The last question asked: the answer to an earlier one (another period, another account) comes too late to be shown. */
  let asked = 0;

  /** The harnesses to choose from: those with accounts, then those only the statistics remember. */
  const engineChoices = $derived(
    [...new Set<string>([...accountEngines(), ...(view?.engines ?? [])])].map((id) => ({ id, name: engineName(id as EngineId) })),
  );
  /** The choice of a harness and « Par harness » only mean something with several harnesses, or turns of several. */
  const severalEngines = $derived(engineChoices.length > 1);
  /** The harness the views are for; none (every harness) unless one is chosen that can be. */
  const engineFilter = $derived(severalEngines && engineChoices.some((c) => c.id === engine) ? engine : null);

  /** The accounts of the harness chosen, or of every one. */
  const accounts = $derived((app.settings.accounts ?? []).filter((a) => !engineFilter || engineOfAccount(a) === engineFilter));
  /** What to choose from: the accounts of the settings, then those only the statistics remember (removed, their turns remain). */
  const choices = $derived([
    ...accounts.map((a) => ({ id: a.id, name: accountName(a) })),
    ...(view?.accounts ?? [])
      .filter((id) => !(app.settings.accounts ?? []).some((a) => a.id === id))
      .map((id) => ({ id, name: t('stats.account.removed', { id }) })),
  ]);
  /** The choice and « Par compte » only mean something with several accounts, or turns of several. */
  const several = $derived(accounts.length > 1 || (view?.accounts.length ?? 0) > 1);
  /** The account the views are for; none (every account) unless one is chosen that can be. */
  const filter = $derived(several && choices.some((c) => c.id === account) ? account : null);
  /**
   * How the cost of what the views add up is known: that of the harness chosen, else of every harness that ran a turn in the
   * period. Each line of « Par harness » has its own.
   */
  const costKind = $derived(costKindOf(engineFilter ? [engineFilter] : (view?.byEngine ?? []).map((e) => e.engine)));
  const money = (cost: number) => fCost(cost, costKind);

  $effect(() => {
    const r = range;
    const a = filter;
    const e = engineFilter;
    void app.usage.todayCost; // refresh when new turns are recorded
    void locale.ui; // the backend names the steps of the period (« 27/09 », “Sep 27”) in the interface's language
    clearTimeout(loadTimer);
    loadTimer = setTimeout(() => {
      const n = ++asked;
      api
        .stats(r, a, e)
        .then((v) => {
          if (n !== asked) return;
          view = v;
          error = null;
        })
        .catch((e) => {
          if (n === asked) error = String(e);
        });
    }, 50);
    writePref('statsRange', r);
  });

  const k = $derived(view ? kpis(view) : null);
  const max = $derived(view ? niceMax(Math.max(1, ...view.buckets.map((b) => b.input + b.cache + b.output))) : 1);
  const unit = $derived(t(`stats.unit.${range}`));
  const projectCount = $derived(app.projects.length);
  const agentCount = $derived(Object.keys(app.agents).length);

  function projectOf(key: string) {
    const p = app.projects.find((x) => x.id === key);
    return p ? { name: p.name, color: p.color } : { name: t('stats.closedProject'), color: 'var(--dim)' };
  }

  function accountLabel(id: string) {
    return choices.find((c) => c.id === id)?.name ?? t('stats.account.removed', { id });
  }

  function total(b: Bucket) {
    return b.input + b.cache + b.output;
  }

  /** The chart's width, as laid out (1 100 before it is). */
  let chartWidth = $state(0);
  const W = $derived(chartWidth || 1100);
  const band = $derived(view ? (W - LEFT) / Math.max(1, view.buckets.length) : 1);
  const y = (v: number) => 8 + PLOT - (v / max) * PLOT;

  /**
   * The segments of a column, from the bottom: 2 px of the surface between two of them, the top one rounded (a path).
   * A series at 0 draws nothing.
   */
  function segments(b: Bucket, i: number) {
    const bw = Math.min(24, band * 0.5);
    const x0 = LEFT + band * i + band / 2 - bw / 2;
    const parts = SERIES.filter((s) => b[s.key] > 0);
    let acc = 0;
    return parts.map((s, k) => {
      const yb = y(acc) - (k ? 2 : 0);
      acc += b[s.key];
      const yt = y(acc);
      const h = Math.max(0, yb - yt);
      const r = Math.min(4, h);
      const x1 = x0 + bw;
      const d =
        k === parts.length - 1 ? `M${x0} ${yb}V${yt + r}Q${x0} ${yt} ${x0 + r} ${yt}H${x1 - r}Q${x1} ${yt} ${x1} ${yt + r}V${yb}Z` : null;
      return { key: s.key, color: s.color, x: x0, y: yt, w: bw, h, d };
    });
  }

  /** The bars of a share: as long as its part of the first, its cost written, or its tokens when no cost is known. */
  function bars<T extends { tokens: number; cost: number }>(list: T[]) {
    const measure = (x: T) => (costKind === 'none' ? x.tokens : x.cost);
    const top = Math.max(...list.map(measure), Number.MIN_VALUE);
    return list.map((x) => ({
      item: x,
      width: `${Math.round((measure(x) / top) * 1000) / 10}%`,
      value: costKind === 'none' ? fTok(x.tokens) : money(x.cost),
      title: t('stats.shareTip', { tokens: fTok(x.tokens), cost: money(x.cost) }),
    }));
  }

  const sub = $derived(
    t('stats.subtitle', {
      projects: t('common.count.projects', { count: projectCount }),
      agents: t('common.count.agents', { count: agentCount }),
    }),
  );

  /** A change of the tokens as the header says it: the sign of a rise is written (« +49 % », « -50 % »). */
  const signed = (percent: number) => `${percent >= 0 ? '+' : ''}${fPct(percent)}`;
</script>

<!-- A figure of the header: its unit smaller than its number, as the preview writes it. -->
{#snippet figure(text: string)}
  {@const f = figureParts(text)}
  {#if f.before}<small>{f.before}</small>{/if}{f.number}{#if f.after}<small>{f.after}</small>{/if}
{/snippet}

<!-- « Par projet », « Par modèle »: a bar per share, in its colour. -->
{#snippet shares(
  id: string,
  title: string,
  list: { key: string; tokens: number; cost: number }[],
  name: (key: string) => string,
  color: (key: string, i: number) => string,
)}
  <section aria-labelledby={id}>
    <div class="sec-h"><h2 {id}>{title}</h2></div>
    {#if list.length}
      <ul class="hbars">
        {#each bars(list) as b, i (b.item.key)}
          {@const n = name(b.item.key)}
          <li class="hbar" use:tip={b.title}>
            <span class="nm">{n}</span>
            <span class="tr"><i style:width={b.width} style:background={color(b.item.key, i)}></i></span>
            <span class="vl">{b.value}</span>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="none">{t('stats.noData')}</p>
    {/if}
  </section>
{/snippet}

<main class="stats" aria-labelledby="st-title">
  <header class="head">
    <h1 id="st-title">{t('stats.title')}</h1>
    <span class="sub" use:tip={{ text: sub, cut: true }}>{sub}</span>
    <div class="tools">
      {#if severalEngines}
        <select
          class="field picker"
          aria-label={t('stats.engine.label')}
          value={engineFilter ?? ''}
          onchange={(e) => (engine = e.currentTarget.value)}
        >
          <option value="">{t('stats.engine.all')}</option>
          {#each engineChoices as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
        </select>
      {/if}
      {#if several}
        <select
          class="field picker"
          aria-label={t('stats.account.label')}
          value={filter ?? ''}
          onchange={(e) => (account = e.currentTarget.value)}
        >
          <option value="">{t('stats.account.all')}</option>
          {#each choices as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
        </select>
      {/if}
      <div class="segmented ranges" role="tablist" aria-label={t('stats.range.label')}>
        {#each RANGES as r (r)}
          <button role="tab" aria-selected={range === r} onclick={() => (range = r)}>{t(`stats.range.${r}`)}</button>
        {/each}
      </div>
    </div>
  </header>

  <div class="scroll">
    <div class="stw">
      {#if view && k}
        <div class="kpis">
          <div class="kpi">
            <span class="lb">{t('stats.kpi.tokens')}</span>
            <span class="vl">{@render figure(fTok(view.tokens))}</span>
            <span class="kd"
              >{#if k.delta !== null}<Rich k="stats.kpi.spanDelta" span={k.span}
                  >{#snippet delta()}<span class="up">{signed(k?.delta ?? 0)}</span>{/snippet}</Rich
                >{:else}{k.span}{/if}</span
            >
          </div>
          <div class="kpi">
            <span class="lb">{t('stats.kpi.totalCost')}</span>
            <span class="vl" use:tip={costHint(costKind)}
              >{@render figure(money(view.cost))}{#if costKind === 'estimated'}<span class="est">{t('stats.cost.estimated')}</span
                >{/if}</span
            >
            <span class="kd"
              >{costKind === 'none'
                ? '—'
                : view.firstTs
                  ? t('stats.kpi.costSince', { amount: money(view.costAll), date: fDateShort(view.firstTs, app.now) })
                  : t('stats.kpi.costAllTime', { amount: money(view.costAll) })}</span
            >
          </div>
          <div class="kpi">
            <span class="lb">{t('stats.kpi.costPerPrompt')}</span>
            <span class="vl">{@render figure(k.costPerPrompt !== null ? money(k.costPerPrompt) : '—')}</span>
            <span class="kd"
              >{k.tokensPerPrompt !== null
                ? t('stats.kpi.tokensPerPrompt', { tokens: fTok(k.tokensPerPrompt) })
                : t('stats.kpi.noPrompt')}</span
            >
          </div>
          <div class="kpi">
            <span class="lb">{t('stats.kpi.prompts')}</span>
            <span class="vl">{@render figure(fInt(view.prompts))}</span>
            <span class="kd">{t('stats.kpi.perUnit', { n: fInt(k.promptsPerBucket), unit })}</span>
          </div>
        </div>

        <section aria-labelledby="st-chart">
          <div class="sec-h">
            <h2 id="st-chart">{t('stats.chart.title', { unit })}</h2>
            <div class="legend" aria-hidden="true">
              {#each SERIES as s (s.key)}<span><i style:background={s.color}></i>{t(`stats.series.${s.key}`)}</span>{/each}
            </div>
            <button class="btn" onclick={() => (table = !table)}>{table ? t('stats.chart.showChart') : t('stats.chart.showTable')}</button>
          </div>
          {#if table}
            <table class="tbl">
              <thead>
                <tr>
                  <th>{t('stats.table.period')}</th>
                  <th class="r">{t('stats.series.input')}</th>
                  <th class="r">{t('stats.series.cache')}</th>
                  <th class="r">{t('stats.series.output')}</th>
                  <th class="r">{t('stats.table.total')}</th>
                  <th class="r">{t('common.cost')}</th>
                  <th class="r">{t('stats.table.prompts')}</th>
                </tr>
              </thead>
              <tbody>
                {#each view.buckets as b (b.start)}
                  <tr>
                    <td class="ttl">{b.label}</td>
                    <td class="r">{fTok(b.input)}</td>
                    <td class="r">{fTok(b.cache)}</td>
                    <td class="r">{fTok(b.output)}</td>
                    <td class="r">{fTok(total(b))}</td>
                    <td class="r">{money(b.cost)}</td>
                    <td class="r">{b.prompts}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {:else}
            <div class="chart" bind:clientWidth={chartWidth}>
              <svg viewBox="0 0 {W} {H}" role="img" aria-label={t('stats.chart.label', { unit })}>
                {#each ticks(max) as v (v)}
                  <line class="gl" x1={LEFT} x2={W} y1={y(v)} y2={y(v)} />
                  <text class="ax" x={LEFT - 10} y={y(v) + 4} text-anchor="end">{v ? fTok(v) : '0'}</text>
                {/each}
                {#each view.buckets as b, i (b.start)}
                  {@const cx = LEFT + band * i + band / 2}
                  <g
                    class="col"
                    class:on={hover === i}
                    role="presentation"
                    onmouseenter={() => (hover = i)}
                    onmouseleave={() => (hover = null)}
                  >
                    <rect class="col-hit" x={cx - band / 2 + 2} y="8" width={Math.max(0, band - 4)} height={PLOT} rx="6" />
                    {#each segments(b, i) as s (s.key)}
                      {#if s.d}
                        <path class="seg" d={s.d} fill={s.color} />
                      {:else}
                        <rect class="seg" x={s.x} y={s.y} width={s.w} height={s.h} fill={s.color} />
                      {/if}
                    {/each}
                    <text class="ax x" x={cx} y={H - 6} text-anchor="middle">{b.label}</text>
                  </g>
                {/each}
              </svg>
              {#if hover !== null && view.buckets[hover]}
                {@const b = view.buckets[hover]}
                <div
                  class="tip"
                  style:left="{Math.max(0, Math.min(W - 190, LEFT + band * hover + band / 2 + 18))}px"
                  style:top="{Math.max(0, y(total(b)) - 20)}px"
                >
                  <b>{b.label}</b>
                  {#each SERIES as s (s.key)}
                    <span><i style:background={s.color}></i>{t(`stats.series.${s.key}`)}<em>{fTok(b[s.key])}</em></span>
                  {/each}
                  <span class="tot">{t('stats.table.total')}<em>{fTok(total(b))}</em></span>
                  <span>{t('common.cost')}<em>{money(b.cost)}</em></span>
                  <span>{t('stats.table.prompts')}<em>{b.prompts}</em></span>
                </div>
              {/if}
            </div>
          {/if}
        </section>

        <div class="two">
          {@render shares(
            'st-projects',
            t('stats.byProject.title'),
            view.byProject,
            (key) => projectOf(key).name,
            (key) => projectOf(key).color,
          )}
          {@render shares(
            'st-models',
            t('stats.byModel.title'),
            view.byModel,
            (key) => displayModel(key),
            (_, i) => modelColor(i),
          )}
        </div>

        {#if severalEngines}
          <!-- The harnesses side by side: the choice of a harness does not narrow them, it marks its own line. Each one's cost is
               told as it gives it: an estimate is marked so, none is « — ». -->
          <section aria-labelledby="stats-engines">
            <div class="sec-h"><h2 id="stats-engines">{t('stats.byEngine.title')}</h2></div>
            {#if view.byEngine.length}
              <table class="tbl" aria-labelledby="stats-engines">
                <thead>
                  <tr>
                    <th class="w-name">{t('stats.byEngine.engine')}</th>
                    <th class="r">{t('stats.byEngine.tokens')}</th>
                    <th class="r w-cost">{t('common.cost')}</th>
                    <th class="r">{t('stats.byEngine.turns')}</th>
                    <th class="r">{t('stats.byEngine.agents')}</th>
                  </tr>
                </thead>
                <tbody>
                  {#each view.byEngine as e (e.engine)}
                    {@const kind = costKindOf([e.engine])}
                    <tr class:on={e.engine === engineFilter} aria-current={e.engine === engineFilter ? 'true' : undefined}>
                      <td class="ttl"><EngineMark engine={e.engine as EngineId} always full /></td>
                      <td class="r">{fTok(e.tokens)}</td>
                      <td class="r" use:tip={costHint(kind)}
                        >{fCost(e.cost, kind)}{#if kind === 'estimated'}<span class="est">{t('stats.cost.estimated')}</span>{/if}</td
                      >
                      <td class="r">{fInt(e.turns)}</td>
                      <td class="r">{fInt(e.agents)}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {:else}
              <p class="none">{t('stats.noData')}</p>
            {/if}
          </section>
        {/if}

        {#if several}
          <!-- The accounts side by side: the choice above does not narrow them, it marks its own line. -->
          <section aria-labelledby="stats-accounts">
            <div class="sec-h"><h2 id="stats-accounts">{t('stats.byAccount.title')}</h2></div>
            {#if view.byAccount.length}
              <table class="tbl" aria-labelledby="stats-accounts">
                <thead>
                  <tr>
                    <th class="w-name">{t('stats.byAccount.account')}</th>
                    <th class="r">{t('stats.byAccount.tokens')}</th>
                    <th class="r">{t('common.cost')}</th>
                    <th class="r">{t('stats.byAccount.turns')}</th>
                    <th class="r">{t('stats.byAccount.agents')}</th>
                  </tr>
                </thead>
                <tbody>
                  {#each view.byAccount as a (a.account)}
                    {@const name = accountLabel(a.account)}
                    <tr class:on={a.account === filter} aria-current={a.account === filter ? 'true' : undefined}>
                      <td class="ttl cut" use:tip={{ text: name, cut: true }}>{name}</td>
                      <td class="r">{fTok(a.tokens)}</td>
                      <td class="r">{money(a.cost)}</td>
                      <td class="r">{fInt(a.turns)}</td>
                      <td class="r">{fInt(a.agents)}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            {:else}
              <p class="none">{t('stats.noData')}</p>
            {/if}
          </section>
        {/if}

        <section aria-labelledby="stats-agents">
          <div class="sec-h"><h2 id="stats-agents">{t('stats.byAgent.title')}</h2></div>
          {#if view.byAgent.length}
            <table class="tbl" aria-labelledby="stats-agents">
              <thead>
                <tr>
                  <th class="w-name">{t('common.agent')}</th>
                  <th class="w-proj">{t('common.project')}</th>
                  <th class="r">{t('stats.byAgent.tokens')}</th>
                  <th class="r">{t('common.cost')}</th>
                </tr>
              </thead>
              <tbody>
                {#each shown(view.byAgent, allAgents) as a (a.agentId)}
                  {@const p = projectOf(a.projectId)}
                  <tr>
                    <td class="ttl cut" use:tip={{ text: a.name, cut: true }}>{a.name ?? t('stats.byAgent.deleted')}</td>
                    <td class="cut"><span class="psw" style:background={p.color}></span>{p.name}</td>
                    <td class="r">{fTok(a.tokens)}</td>
                    <td class="r">{money(a.cost)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
            {#if view.byAgent.length > ROWS}
              <div class="more">
                <button class="btn ghost" aria-expanded={allAgents} onclick={() => (allAgents = !allAgents)}
                  >{allAgents ? t('stats.showLess') : t('stats.showAll')}</button
                >
                {#if !allAgents}<span class="none">{t('stats.shownOf', { shown: ROWS, total: fInt(view.byAgent.length) })}</span>{/if}
              </div>
            {/if}
          {:else}
            <p class="none">{t('stats.noData')}</p>
          {/if}
        </section>

        <section aria-labelledby="stats-tickets">
          <div class="sec-h"><h2 id="stats-tickets">{t('stats.byTicket.title')}</h2></div>
          {#if view.byTicket.length}
            <table class="tbl" aria-labelledby="stats-tickets">
              <thead>
                <tr>
                  <th class="w-key">{t('stats.byTicket.ticket')}</th>
                  <th>{t('stats.byTicket.name')}</th>
                  <th class="r w-loops">{t('stats.byTicket.loops')}</th>
                  <th class="r w-period">{t('stats.byTicket.periodCost')}</th>
                </tr>
              </thead>
              <tbody>
                {#each shown(view.byTicket, allTickets) as x (x.id)}
                  <tr>
                    <td class="key">{x.key}</td>
                    <td class="ttl cut" use:tip={{ text: x.title, cut: true }}>{x.title}</td>
                    <td class="r">{t('common.count.loops', { count: x.loops })}</td>
                    <td class="r">{money(x.cost)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
            {#if view.byTicket.length > ROWS}
              <div class="more">
                <button class="btn ghost" aria-expanded={allTickets} onclick={() => (allTickets = !allTickets)}
                  >{allTickets ? t('stats.showLess') : t('stats.showAll')}</button
                >
                {#if !allTickets}<span class="none">{t('stats.shownOf', { shown: ROWS, total: fInt(view.byTicket.length) })}</span>{/if}
              </div>
            {/if}
          {:else}
            <p class="none">{t('stats.byTicket.none')}</p>
          {/if}
        </section>
      {:else if error}
        <p class="none">{t('stats.unavailable', { error })}</p>
      {:else}
        <p class="none">{t('common.loading')}</p>
      {/if}
    </div>
  </div>
</main>

<style>
  /* Full frame, as the preview's `#stats`: its parts fold by its own width, not the window's. */
  .stats {
    container: st / inline-size;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .head {
    min-height: 64px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 16px;
    min-width: 0;
    padding: 0 24px 0 28px;
    border-bottom: 1px solid var(--line);
    white-space: nowrap;
  }
  h1 {
    flex: none;
    margin: 0;
    font-size: 17px;
    font-weight: 640;
    letter-spacing: -0.02em;
  }
  .sub {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--muted);
    font-size: 12.5px;
  }
  .tools {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    margin-left: auto;
  }
  .picker {
    max-width: 200px;
  }
  .ranges {
    display: grid;
    gap: 0;
    grid-template-columns: repeat(3, 1fr);
    width: 240px;
  }
  .ranges button {
    padding: 0;
    font-size: 13px;
  }
  .scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .stw {
    display: grid;
    gap: 34px;
    width: min(1180px, 100% - 56px);
    margin: 0 auto;
    padding: 24px 0 40px;
  }
  .stw > * {
    min-width: 0;
  }
  .kpis {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    border-radius: 12px;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .kpi {
    display: grid;
    gap: 4px;
    min-width: 0;
    padding: 16px 20px 15px;
  }
  .kpi + .kpi {
    border-left: 1px solid var(--line);
  }
  .lb {
    color: var(--muted);
    font-size: 12px;
    font-weight: 500;
  }
  .vl {
    font-size: 26px;
    font-weight: 620;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.025em;
    line-height: 1.15;
    white-space: nowrap;
  }
  .vl small {
    color: var(--muted);
    font-size: 15px;
    font-weight: 500;
    letter-spacing: 0;
  }
  .kd {
    min-width: 0;
    color: var(--muted);
    font-size: 12px;
    text-wrap: pretty;
  }
  .kd .up {
    color: var(--soft);
    font-weight: 550;
  }
  .est {
    margin-left: 8px;
    padding: 1px 6px;
    border-radius: 5px;
    background: rgba(var(--fg-rgb), 0.06);
    color: var(--muted);
    font-size: 11px;
    font-weight: 550;
    letter-spacing: 0;
    vertical-align: middle;
  }
  .sec-h {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 28px;
    margin-bottom: 14px;
  }
  h2 {
    margin: 0;
    font-size: 14px;
    font-weight: 620;
    letter-spacing: -0.01em;
    white-space: nowrap;
  }
  .legend {
    display: flex;
    gap: 14px;
    margin-left: auto;
    color: var(--soft);
    font-size: 12px;
    white-space: nowrap;
  }
  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .legend i {
    width: 10px;
    height: 10px;
    border-radius: 3px;
  }
  .sec-h .btn {
    height: 28px;
  }
  .chart {
    position: relative;
  }
  .chart svg {
    display: block;
    width: 100%;
    height: 280px;
    overflow: visible;
  }
  .gl {
    stroke: rgba(var(--fg-rgb), 0.07);
    stroke-width: 1;
  }
  .ax {
    fill: var(--muted);
    font: 11px var(--font);
    font-variant-numeric: tabular-nums;
  }
  .col-hit {
    fill: transparent;
  }
  /* The column takes the pointer, not its segments. */
  .seg {
    pointer-events: none;
  }
  .col:hover .col-hit,
  .col.on .col-hit {
    fill: rgba(var(--fg-rgb), 0.04);
  }
  .tip {
    position: absolute;
    z-index: 3;
    display: grid;
    gap: 4px;
    min-width: 170px;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--raise-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 12px 32px -8px rgba(0, 0, 0, calc(0.7 * var(--shade)));
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    pointer-events: none;
    animation: tip-in 0.15s;
  }
  @keyframes tip-in {
    from {
      opacity: 0;
    }
  }
  .tip b {
    margin-bottom: 2px;
    font-weight: 600;
  }
  .tip span {
    display: flex;
    align-items: center;
    gap: 7px;
    color: var(--soft);
  }
  .tip em {
    margin-left: auto;
    padding-left: 14px;
    color: var(--text);
    font-style: normal;
  }
  .tip i {
    width: 8px;
    height: 8px;
    border-radius: 2px;
  }
  .tip .tot {
    margin-top: 3px;
    padding-top: 6px;
    border-top: 1px solid var(--line);
  }
  .two {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 48px;
  }
  .two > section {
    min-width: 0;
  }
  .hbars {
    display: grid;
    gap: 12px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .hbar {
    display: grid;
    grid-template-columns: 112px minmax(0, 1fr) 68px;
    align-items: center;
    gap: 14px;
    font-size: 12.5px;
  }
  .hbar .nm {
    min-width: 0;
    overflow: hidden;
    color: var(--soft);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hbar .tr {
    height: 10px;
  }
  .hbar .tr i {
    display: block;
    height: 100%;
    border-radius: 0 4px 4px 0;
  }
  .hbar .vl {
    color: var(--text);
    font-size: inherit;
    font-weight: 550;
    letter-spacing: 0;
    line-height: inherit;
    text-align: right;
  }
  .tbl {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
    font-size: 12.5px;
  }
  .tbl th {
    height: 34px;
    padding: 0 10px;
    border-bottom: 1px solid var(--line);
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 550;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tbl td {
    height: 38px;
    padding: 0 10px;
    border-bottom: 1px solid var(--line);
    color: var(--soft);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .tbl .r {
    text-align: right;
  }
  .tbl td.key {
    color: var(--muted);
    font-weight: 550;
  }
  .tbl td.ttl {
    color: var(--text);
  }
  .tbl .cut {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* The account, the harness the other views are for. */
  .tbl tr.on td.ttl {
    color: var(--p-ink);
    font-weight: 590;
  }
  .tbl .w-name {
    width: 34%;
  }
  .tbl .w-proj {
    width: 26%;
  }
  .tbl .w-key {
    width: 84px;
  }
  .tbl .w-loops {
    width: 110px;
  }
  .tbl .w-period {
    width: 170px;
  }
  .tbl .w-cost {
    width: 130px;
  }
  .psw {
    display: inline-block;
    width: 9px;
    height: 9px;
    margin-right: 8px;
    border-radius: 3px;
  }
  .more {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 10px;
  }
  .none {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
  }
  /* As the preview under 1 240 px of window: smaller figures. */
  @container st (max-width: 1180px) {
    .kpi .vl {
      font-size: 22px;
    }
  }
  /* Narrower: the figures two by two, the bars one above the other; then the figures one by one. */
  @container st (max-width: 900px) {
    .kpis {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
    .kpi:nth-child(3) {
      border-left: none;
    }
    .kpi:nth-child(n + 3) {
      border-top: 1px solid var(--line);
    }
    .two {
      grid-template-columns: minmax(0, 1fr);
      gap: 34px;
    }
    .sub {
      display: none;
    }
  }
  @container st (max-width: 620px) {
    .kpis {
      grid-template-columns: minmax(0, 1fr);
    }
    .kpi + .kpi {
      border-left: none;
      border-top: 1px solid var(--line);
    }
    .legend {
      display: none;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .tip {
      animation: none;
    }
  }
</style>
