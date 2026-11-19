<script lang="ts">
  // « Partout » (ESC-539, direction B, docs/SERVER-UX.md § 5 and 6): what waits in every space, each one under its
  // machine — its plate, its name, its address and its way there and back — and answered here, without changing space.
  // The three safeguards: the machine is written on each request and on the gesture that answers it (« Autoriser sur
  // Travail »), a space that is cut greys its rows and takes their gestures away; a space whose policy denies the
  // aggregate view gives its numbers alone; a request that is not read whole here (cut, refused by default, a question)
  // is answered in its conversation, « Ouvrir » takes there.
  import { connection } from '../../lib/connection.svelte';
  import { fTime } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import {
    callIn,
    countsOnly,
    goToSpace,
    pollAll,
    retrySpace,
    spaceList,
    spacesState,
    stateOf,
    type Glance,
    type GlanceAgent,
    type Space,
  } from '../../lib/spaces.svelte';
  import { app } from '../../lib/state.svelte';
  import type { PendingRequest } from '../../lib/types';
  import { tip } from '../../lib/tip';
  import SpacePlate from './SpacePlate.svelte';

  let { filter = 'all' }: { filter?: string } = $props();

  /** A row: an agent that waits or works, or a request of the box. */
  interface Row {
    key: string;
    agentId: string | null;
    projectId: string;
    project: string;
    color: string;
    name: string;
    line: string;
    waiting: boolean;
    request: PendingRequest | null;
  }

  const shown = $derived(spaceList().filter((s) => filter === 'all' || s.id === filter));
  /** The open space first, then the others in their order. */
  const ordered = $derived([...shown.filter((s) => s.current), ...shown.filter((s) => !s.current)]);
  let busy = $state<Record<string, boolean>>({});
  let failed = $state<Record<string, string>>({});

  /** What waits in the space the window is on: the window's own state, fresher than a glance. */
  function ownGlance(s: Space): Glance {
    const agents: GlanceAgent[] = Object.values(app.agents)
      .filter((a) => !a.archived && (a.status === 'waiting' || a.status === 'running'))
      .sort((a, b) =>
        a.status === b.status
          ? a.status === 'waiting'
            ? a.lastActivity - b.lastActivity
            : b.lastActivity - a.lastActivity
          : a.status === 'waiting'
            ? -1
            : 1,
      )
      .map((a) => ({
        id: a.id,
        projectId: a.projectId,
        name: a.name,
        status: a.status,
        activity: a.activity ?? null,
        lastActivity: a.lastActivity,
        request: a.requests[0] ?? null,
      }));
    const waiting = agents.filter((a) => a.status === 'waiting').length;
    const asks = spacesState.watched[s.id]?.glance?.asks ?? [];
    return {
      machine: connection.machine,
      version: '',
      waiting: waiting + asks.length,
      running: agents.length - waiting,
      countsOnly: false,
      contentless: false,
      colors: [],
      projects: app.projects.map((p) => ({ id: p.id, name: p.name, color: p.color })),
      agents,
      asks,
    };
  }

  function glanceOf(s: Space): Glance | null {
    return s.current ? ownGlance(s) : (spacesState.watched[s.id]?.glance ?? null);
  }

  /** What a row says on its second line: the request, what the agent does, or the request of the box. */
  function lineOf(a: GlanceAgent): string {
    const r = a.request;
    if (a.status === 'waiting' && r) {
      if (r.kind === 'question') return `${t('spaces.kind.question')} · ${r.questions[0]?.question ?? r.arg}`;
      const arg = r.arg.split('\n')[0];
      return `${t('spaces.kind.permission')} · ${r.tool}${arg ? ` : ${arg}` : ''}`;
    }
    if (a.status === 'waiting') return t('spaces.kind.waits');
    return a.activity || t('spaces.kind.working');
  }

  function rowsOf(g: Glance): Row[] {
    const proj = (id: string) => g.projects.find((p) => p.id === id);
    return [
      ...g.agents.map((a) => ({
        key: `a-${a.id}`,
        agentId: a.id,
        projectId: a.projectId,
        project: proj(a.projectId)?.name ?? '',
        color: proj(a.projectId)?.color ?? 'var(--muted)',
        name: a.name,
        line: lineOf(a),
        waiting: a.status === 'waiting',
        request: a.status === 'waiting' ? a.request : null,
      })),
      ...g.asks.map((k) => ({
        key: `k-${k.id}`,
        agentId: null,
        projectId: k.projectId,
        project: proj(k.projectId)?.name ?? '',
        color: proj(k.projectId)?.color ?? 'var(--muted)',
        name: proj(k.projectId)?.name ?? '',
        line: `${t('spaces.kind.ask')} · ${k.title}`,
        waiting: true,
        request: null,
      })),
    ];
  }

  /** A permission answered here: read whole, not refused by default (else it is answered in its conversation). */
  const answerable = (r: PendingRequest | null): r is PendingRequest =>
    !!r && r.kind === 'permission' && r.tool !== 'ExitPlanMode' && !r.defaultNo && !r.cut;

  async function allow(s: Space, row: Row) {
    if (!row.agentId || !row.request || busy[row.key]) return;
    busy[row.key] = true;
    delete failed[row.key];
    try {
      await callIn(s.id, 'answer_permission', { id: row.agentId, requestId: row.request.id, decision: 'allow', message: null });
      await pollAll();
    } catch (e) {
      failed[row.key] = String(e);
    } finally {
      busy[row.key] = false;
    }
  }

  const waitPill = (n: number) => (n === 1 ? t('spaces.waits.one') : t('spaces.waits.many', { n }));

  function hostLine(s: Space): string {
    if (s.here) return s.host;
    const latency = s.current ? connection.latency : spacesState.watched[s.id]?.conn.latency;
    return latency != null ? `${s.host} · ${latency} ms` : s.host;
  }

  /** When what a cut space shows was read. */
  function staleSince(s: Space): string {
    const w = spacesState.watched[s.id];
    const at = s.current ? connection.lostAt : (w?.conn.lostAt ?? (w?.at || null));
    return at ? fTime(at) : '';
  }
</script>

<div class="everywhere" data-testid="everywhere">
  {#each ordered as s (s.id)}
    {@const g = glanceOf(s)}
    {@const cut = stateOf(s) !== 'open'}
    {@const counts = !s.current && countsOnly(s, g)}
    {@const rows = g && !counts ? rowsOf(g) : []}
    <section class="space" class:cut aria-label={s.name} data-testid="everywhere-space" data-space={s.id}>
      <h2 class="sh">
        <SpacePlate name={s.name} here={s.here} {cut} size={22} />
        <b>{s.name}</b>
        <span class="host">{hostLine(s)}</span>
        {#if cut}
          <span class="spill cut">{t('servers.reconnecting')}</span>
          {#if staleSince(s)}<span class="stale">{t('spaces.staleSince', { at: staleSince(s) })}</span>{/if}
          {#if !s.here}<button class="btn sm" onclick={() => void retrySpace(s.id)}>{t('servers.problem.retry')}</button>{/if}
        {:else if g && g.waiting > 0}
          <span class="spill wt">{waitPill(g.waiting)}</span>
        {/if}
      </h2>
      {#if !g}
        <p class="none">{t(cut ? 'spaces.notYet' : 'spaces.reading')}</p>
      {:else if counts}
        <div class="counts" data-testid="everywhere-counts">
          <span>{t('spaces.counts', { waiting: g.waiting, running: g.running })}</span>
          <span class="why">{t('spaces.countsWhy', { name: s.name })}</span>
          <button class="btn sm quiet" disabled={cut} onclick={() => void goToSpace(s.id)}>{t('servers.goTo', { name: s.name })}</button>
        </div>
      {:else if !rows.length}
        <p class="none">{t('spaces.nothing')}</p>
      {:else}
        <ul>
          {#each rows as row (row.key)}
            <li class="row" class:wt={row.waiting} data-cut={cut || undefined} data-testid="everywhere-row">
              <span class="t">
                <span class="dot" style:background={row.color}></span>
                <b>{row.name}</b>
                {#if row.agentId}<span class="muted">{row.project}</span>{/if}
                <!-- The machine, on each request (safeguard 1). -->
                <span class="on" data-testid="row-machine"><SpacePlate name={s.name} here={s.here} {cut} size={14} />{s.name}</span>
              </span>
              <span class="m" use:tip={{ text: row.line, cut: true }}>{row.line}</span>
              {#if failed[row.key]}<span class="err" role="alert">{failed[row.key]}</span>{/if}
              <span class="acts">
                {#if !cut && answerable(row.request)}
                  <button
                    class="btn sm"
                    disabled={busy[row.key]}
                    aria-label={t('spaces.allowOnLabel', { name: s.name, agent: row.name })}
                    onclick={() => void allow(s, row)}>{t('spaces.allowOn', { name: s.name })}</button
                  >
                {/if}
                {#if !cut}
                  <button
                    class="btn sm quiet"
                    onclick={() =>
                      void goToSpace(s.id, row.agentId ? { agentId: row.agentId, projectId: row.projectId } : { projectId: row.projectId })}
                    >{t('spaces.open')}</button
                  >
                {/if}
              </span>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  {/each}
</div>

<style>
  .everywhere {
    display: grid;
    gap: 18px;
    align-content: start;
    padding: 18px 24px 28px;
  }
  .space {
    display: grid;
    gap: 8px;
  }
  .sh {
    display: flex;
    align-items: center;
    gap: 9px;
    min-width: 0;
    margin: 0;
    padding: 2px;
    font-size: 13.5px;
    font-weight: 500;
  }
  .sh b {
    font-weight: 640;
  }
  .host {
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .spill {
    flex: none;
    padding: 2px 7px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 600;
  }
  .spill.wt,
  .spill.cut {
    color: var(--wait);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .stale {
    color: var(--muted);
    font-size: 12px;
  }
  ul {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 3px 14px;
    padding: 10px 14px;
    border-radius: 11px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .row.wt {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .cut .row {
    opacity: 0.55;
  }
  .t {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    font-size: 12.5px;
  }
  .t b {
    overflow: hidden;
    font-weight: 620;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .muted {
    color: var(--muted);
    white-space: nowrap;
  }
  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 2px;
  }
  .on {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 5px;
    margin-left: auto;
    color: var(--soft);
    font-size: 11.5px;
    font-weight: 600;
  }
  .m {
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .row.wt .m {
    color: var(--wait);
  }
  .acts {
    display: flex;
    grid-row: 1 / span 2;
    grid-column: 2;
    gap: 8px;
  }
  .err {
    color: var(--del);
    font-size: 12px;
  }
  .none,
  .counts {
    margin: 0;
    padding: 10px 14px;
    border-radius: 11px;
    color: var(--muted);
    font-size: 12.5px;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .counts {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 12px;
    color: var(--text);
  }
  .counts .why {
    color: var(--muted);
  }
  .counts .btn {
    margin-left: auto;
  }
</style>
