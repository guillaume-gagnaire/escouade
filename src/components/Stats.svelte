<script lang="ts">
  import { readPref, writePref } from '../lib/prefs';
  import { fDate, fInt, fTok, fUsd, plural } from '../lib/format';
  import { api } from '../lib/ipc';
  import { displayModel } from '../lib/models';
  import { app } from '../lib/state.svelte';
  import { kpis, niceMax, ROWS, shown, type Range } from '../lib/stats';
  import type { Bucket, StatsView } from '../lib/types';

  // Categorical palette validated for the dark surface (dataviz validator: all checks pass).
  const SERIES = [
    { key: 'input', label: 'Entrée', color: '#3987e5' },
    { key: 'cache', label: 'Cache', color: '#199e70' },
    { key: 'output', label: 'Sortie', color: '#d95926' },
  ] as const;

  let range = $state<Range>((readPref('statsRange') as Range) || 'day');
  let view = $state<StatsView | null>(null);
  let table = $state(false);
  let hover = $state<number | null>(null);
  let error = $state<string | null>(null);
  /** « Tout voir » opened the whole list of agents, of tickets (20 lines each until then). */
  let allAgents = $state(false);
  let allTickets = $state(false);
  let loadTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    const r = range;
    void app.usage.todayCost; // refresh when new turns are recorded
    clearTimeout(loadTimer);
    loadTimer = setTimeout(
      () =>
        api
          .stats(r)
          .then((v) => {
            view = v;
            error = null;
          })
          .catch((e) => (error = String(e))),
      50,
    );
    writePref('statsRange', r);
  });

  const k = $derived(view ? kpis(view) : null);
  const max = $derived(view ? niceMax(Math.max(1, ...view.buckets.map((b) => b.input + b.cache + b.output))) : 1);
  const unit = $derived({ day: 'jour', week: 'semaine', month: 'mois' }[range]);
  const projectCount = $derived(app.projects.length);
  const agentCount = $derived(Object.keys(app.agents).length);

  function projectOf(key: string) {
    const p = app.projects.find((x) => x.id === key);
    return p ? { name: p.name, color: p.color } : { name: 'Projet fermé', color: 'var(--dim)' };
  }

  function total(b: Bucket) {
    return b.input + b.cache + b.output;
  }
</script>

<div class="stats">
  <div class="inner">
    <div class="top">
      <div class="title">
        <span class="h">Statistiques</span>
        <span class="s"
          >Agents lancés depuis l'app · {projectCount} projet{projectCount > 1 ? 's' : ''}, {agentCount} agent{agentCount > 1
            ? 's'
            : ''}</span
        >
      </div>
      <div style="flex:1"></div>
      <div class="ranges">
        {#each [['day', 'Jour'], ['week', 'Semaine'], ['month', 'Mois']] as [r, l] (r)}
          <button class:on={range === r} onclick={() => (range = r as Range)}>{l}</button>
        {/each}
      </div>
    </div>

    {#if view && k}
      <div class="kpis">
        <div class="kpi">
          <span class="kl">Tokens</span>
          <span class="kv mono">{fTok(view.tokens)}</span>
          <span class="ks">{k.span}{k.delta !== null ? ` · ${k.delta >= 0 ? '+' : ''}${k.delta} %` : ''}</span>
        </div>
        <div class="kpi">
          <span class="kl">Coût global</span>
          <span class="kv mono">{fUsd(view.cost)}</span>
          <span class="ks">{fUsd(view.costAll)} {view.firstTs ? `depuis le ${fDate(view.firstTs)}` : 'au total'}</span>
        </div>
        <div class="kpi">
          <span class="kl">Coût moyen / prompt</span>
          <span class="kv mono">{k.costPerPrompt !== null ? fUsd(k.costPerPrompt) : '—'}</span>
          <span class="ks"
            >{k.tokensPerPrompt !== null ? `≈ ${fTok(k.tokensPerPrompt)} tokens / prompt` : 'aucun prompt sur la période'}</span
          >
        </div>
        <div class="kpi">
          <span class="kl">Prompts</span>
          <span class="kv mono">{fInt(view.prompts)}</span>
          <span class="ks">{fInt(k.promptsPerBucket)} par {unit} en moyenne</span>
        </div>
      </div>

      <section class="card">
        <div class="chead">
          <span class="ct">Tokens par {unit}</span>
          <div style="flex:1"></div>
          {#each SERIES as s (s.key)}
            <span class="legend"><span class="sw" style:background={s.color}></span>{s.label}</span>
          {/each}
          <button class="btn ghost small" onclick={() => (table = !table)}>{table ? 'Graphique' : 'Tableau'}</button>
        </div>
        {#if table}
          <table class="tbl mono">
            <thead><tr><th>Période</th><th>Entrée</th><th>Cache</th><th>Sortie</th><th>Total</th><th>Coût</th><th>Prompts</th></tr></thead>
            <tbody>
              {#each view.buckets as b (b.start)}
                <tr>
                  <td>{b.label}</td><td>{fTok(b.input)}</td><td>{fTok(b.cache)}</td><td>{fTok(b.output)}</td><td>{fTok(total(b))}</td><td
                    >{fUsd(b.cost)}</td
                  ><td>{b.prompts}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}
          <div class="chart" role="img" aria-label="Tokens par {unit}, empilés entrée, cache et sortie">
            <div class="grid">
              <div class="gl" style="top:0"><span>{fTok(max)}</span></div>
              <div class="gl" style="top:50%"><span>{fTok(max / 2)}</span></div>
              <div class="gl base"><span>0</span></div>
            </div>
            <div class="bars">
              {#each view.buckets as b, i (b.start)}
                <div
                  class="col"
                  role="presentation"
                  onmouseenter={() => (hover = i)}
                  onmouseleave={() => (hover = null)}
                  class:dim={hover !== null && hover !== i}
                >
                  <div class="stack" style:height="{(total(b) / max) * 100}%">
                    {#each [...SERIES].reverse() as s (s.key)}
                      {@const v = b[s.key]}
                      {#if v > 0}
                        <div class="seg" style:flex-grow={v} style:background={s.color}></div>
                      {/if}
                    {/each}
                  </div>
                  {#if hover === i}
                    <div class="tip" class:left={i > view.buckets.length / 2}>
                      <div class="tt">{b.label}</div>
                      {#each SERIES as s (s.key)}
                        <div class="tr">
                          <span class="sw" style:background={s.color}></span>{s.label}<span class="tv mono">{fTok(b[s.key])}</span>
                        </div>
                      {/each}
                      <div class="tr total">Total<span class="tv mono">{fTok(total(b))}</span></div>
                      <div class="tr">Coût<span class="tv mono">{fUsd(b.cost)}</span></div>
                      <div class="tr">Prompts<span class="tv mono">{b.prompts}</span></div>
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
          <div class="labels">
            {#each view.buckets as b (b.start)}<span class="mono">{b.label}</span>{/each}
          </div>
        {/if}
      </section>

      <div class="split">
        <section class="card">
          <span class="ct">Par projet</span>
          {#each view.byProject as s (s.key)}
            {@const p = projectOf(s.key)}
            <div class="share">
              <span class="sname"><span class="psw" style:background={p.color}></span>{p.name}</span>
              <div class="track">
                <div
                  class="fill"
                  style:width="{(s.tokens / Math.max(1, view.byProject[0].tokens)) * 100}%"
                  style:background={p.color}
                ></div>
              </div>
              <span class="stok mono">{fTok(s.tokens)}</span>
              <span class="scost mono">{fUsd(s.cost)}</span>
            </div>
          {:else}
            <span class="none">Aucune donnée sur la période.</span>
          {/each}
        </section>
        <section class="card">
          <span class="ct">Par modèle</span>
          {#each view.byModel as s (s.key)}
            <div class="share">
              <span class="sname">{displayModel(s.key)}</span>
              <div class="track"><div class="fill" style:width="{(s.tokens / Math.max(1, view.byModel[0].tokens)) * 100}%"></div></div>
              <span class="stok mono">{fTok(s.tokens)}</span>
              <span class="scost mono">{fUsd(s.cost)}</span>
            </div>
          {:else}
            <span class="none">Aucune donnée sur la période.</span>
          {/each}
        </section>
      </div>

      <section class="card">
        <span class="ct" id="stats-agents">Par agent</span>
        {#if view.byAgent.length}
          <table class="list" aria-labelledby="stats-agents">
            <thead>
              <tr><th class="w-name">Agent</th><th class="w-proj">Projet</th><th class="num">Tokens</th><th class="num">Coût</th></tr>
            </thead>
            <tbody>
              {#each shown(view.byAgent, allAgents) as a (a.agentId)}
                {@const p = projectOf(a.projectId)}
                <tr>
                  <td class="cut strong" title={a.name ?? undefined}>{a.name ?? 'Agent supprimé'}</td>
                  <td class="cut"><span class="psw" style:background={p.color}></span>{p.name}</td>
                  <td class="num mono">{fTok(a.tokens)}</td>
                  <td class="num mono">{fUsd(a.cost)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
          {#if view.byAgent.length > ROWS}
            <div class="more">
              <button class="btn ghost small" aria-expanded={allAgents} onclick={() => (allAgents = !allAgents)}
                >{allAgents ? 'Réduire' : 'Tout voir'}</button
              >
              {#if !allAgents}<span class="none">{ROWS} sur {view.byAgent.length}</span>{/if}
            </div>
          {/if}
        {:else}
          <span class="none">Aucune donnée sur la période.</span>
        {/if}
      </section>

      <section class="card">
        <span class="ct" id="stats-tickets">Par ticket</span>
        {#if view.byTicket.length}
          <table class="list" aria-labelledby="stats-tickets">
            <thead>
              <tr
                ><th class="w-key">Ticket</th><th>Titre</th><th class="num w-loops">Boucles</th><th class="num w-period"
                  >Coût sur la période</th
                ></tr
              >
            </thead>
            <tbody>
              {#each shown(view.byTicket, allTickets) as t (t.id)}
                <tr>
                  <td class="mono">{t.key}</td>
                  <td class="cut strong" title={t.title}>{t.title}</td>
                  <td class="num">{plural(t.loops, 'boucle', 'boucles')}</td>
                  <td class="num mono">{fUsd(t.cost)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
          {#if view.byTicket.length > ROWS}
            <div class="more">
              <button class="btn ghost small" aria-expanded={allTickets} onclick={() => (allTickets = !allTickets)}
                >{allTickets ? 'Réduire' : 'Tout voir'}</button
              >
              {#if !allTickets}<span class="none">{ROWS} sur {view.byTicket.length}</span>{/if}
            </div>
          {/if}
        {:else}
          <span class="none">Aucun ticket sur la période.</span>
        {/if}
      </section>
    {:else if error}
      <div class="loading">Statistiques indisponibles : {error}</div>
    {:else}
      <div class="loading">Chargement…</div>
    {/if}
  </div>
</div>

<style>
  .stats {
    flex: 1;
    overflow: auto;
  }
  .inner {
    max-width: 1180px;
    margin: 0 auto;
    padding: 30px 36px 40px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .top {
    display: flex;
    align-items: flex-end;
    gap: 16px;
  }
  .title {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .h {
    font-size: 24px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .s {
    font-size: 12.5px;
    color: var(--muted);
  }
  .ranges {
    display: flex;
    gap: 2px;
    padding: 3px;
    border-radius: var(--r);
    background: var(--panel);
    border: 1px solid var(--line);
  }
  .ranges button {
    height: 30px;
    padding: 0 16px;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .ranges button.on {
    background: var(--elev2);
    color: var(--text);
  }
  .kpis {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 12px;
  }
  .kpi {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 18px 18px 16px;
    border-radius: var(--r);
    background: var(--panel);
    border: 1px solid var(--line);
  }
  .kl {
    font-size: 12px;
    color: var(--muted);
  }
  .kv {
    font-size: 26px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }
  .ks {
    font-size: 11.5px;
    color: var(--dim);
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 20px 22px;
    border-radius: var(--r);
    background: var(--panel);
    border: 1px solid var(--line);
  }
  .chead {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .ct {
    font-size: 14px;
    font-weight: 700;
  }
  .legend {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--muted);
  }
  .sw {
    width: 10px;
    height: 10px;
    border-radius: 2px;
    flex: none;
  }
  .small {
    height: 26px;
    font-size: 11.5px;
    padding: 0 10px;
  }
  .chart {
    position: relative;
    height: 240px;
    margin-top: 4px;
    padding-left: 56px;
  }
  .grid {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .gl {
    position: absolute;
    left: 0;
    right: 0;
    border-top: 1px dashed var(--line);
  }
  .gl.base {
    bottom: 0;
    border-top: 1px solid var(--line2);
  }
  .gl span {
    position: absolute;
    left: 0;
    top: -8px;
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--dim);
  }
  .gl.base span {
    top: -16px;
  }
  .bars {
    position: relative;
    height: 100%;
    display: flex;
    align-items: flex-end;
    gap: 8px;
  }
  .col {
    position: relative;
    flex: 1;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    transition: opacity 0.1s;
  }
  .col.dim {
    opacity: 0.55;
  }
  .stack {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-height: 0;
    border-radius: 4px 4px 0 0;
    overflow: hidden;
  }
  .seg {
    flex-basis: 0;
    min-height: 1px;
  }
  .tip {
    position: absolute;
    top: 0;
    left: calc(50% + 10px);
    z-index: 5;
    min-width: 180px;
    padding: 10px 12px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line2);
    background: var(--elev);
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.45);
    font-size: 12px;
    pointer-events: none;
  }
  .tip.left {
    left: auto;
    right: calc(50% + 10px);
  }
  .tt {
    font-weight: 700;
    margin-bottom: 6px;
  }
  .tr {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    line-height: 1.7;
  }
  .tr.total {
    color: var(--text);
    border-top: 1px solid var(--line);
    margin-top: 2px;
    padding-top: 2px;
  }
  .tv {
    margin-left: auto;
    color: var(--text);
  }
  .labels {
    display: flex;
    gap: 8px;
    padding-left: 56px;
    margin-top: -6px;
  }
  .labels span {
    flex: 1;
    text-align: center;
    font-size: 10.5px;
    color: var(--dim);
  }
  .tbl {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }
  .tbl th,
  .tbl td {
    padding: 6px 10px;
    text-align: right;
    border-bottom: 1px solid var(--line);
  }
  .tbl th:first-child,
  .tbl td:first-child {
    text-align: left;
  }
  .tbl th {
    color: var(--muted);
    font-weight: 600;
  }
  /* The lists of agents and of tickets: names are cut, never wrapped, so that a line stays a line. */
  .list {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
    font-size: 12.5px;
  }
  .list th,
  .list td {
    padding: 6px 10px;
    text-align: left;
    border-bottom: 1px solid var(--line);
  }
  .list th {
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
  .list .num {
    width: 90px;
    text-align: right;
  }
  .list .w-name {
    width: 34%;
  }
  .list .w-proj {
    width: 26%;
  }
  .list .w-key {
    width: 84px;
  }
  .list .w-loops {
    width: 110px;
  }
  .list .w-period {
    width: 150px;
  }
  .list td.num {
    font-size: 11.5px;
  }
  .list .cut {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .list .strong {
    font-weight: 600;
  }
  .list .psw {
    display: inline-block;
    margin-right: 8px;
  }
  .more {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .split {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }
  .share {
    display: grid;
    grid-template-columns: 150px minmax(0, 1fr) 70px 70px;
    align-items: center;
    gap: 12px;
    font-size: 12.5px;
  }
  .sname {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .psw {
    width: 9px;
    height: 9px;
    border-radius: 3px;
    flex: none;
  }
  .track {
    height: 6px;
    border-radius: 3px;
    background: var(--elev2);
  }
  .fill {
    height: 100%;
    border-radius: 3px;
    background: var(--accent);
  }
  .stok {
    font-size: 11.5px;
    color: var(--muted);
    text-align: right;
  }
  .scost {
    font-size: 11.5px;
    text-align: right;
  }
  .none,
  .loading {
    color: var(--dim);
    font-size: 12.5px;
  }
</style>
