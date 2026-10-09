<script lang="ts">
  import { keyLabel } from '../lib/platform';
  import { fAgo, fBytes, fCountdown, fPct } from '../lib/format';
  import { api } from '../lib/ipc';
  import { menu } from '../lib/menu.svelte';
  import { ESTIMATE_HINT, fSpentUsd } from '../lib/spend';
  import { app } from '../lib/state.svelte';

  const agents = $derived(Object.values(app.agents).filter((a) => !a.archived));
  const running = $derived(agents.filter((a) => a.status === 'running').length);
  const waiting = $derived(agents.filter((a) => a.status === 'waiting').length);
  const done = $derived(agents.filter((a) => a.status === 'done').length);
  // The Claude processes running, each with the tools and MCP servers it started.
  const procs = $derived(app.resources);
  const procsTitle = $derived(
    [
      'Processus Claude en cours (avec les outils et serveurs MCP qu’ils lancent)',
      ...[...procs.agents]
        .sort((a, b) => b.memory - a.memory)
        .map((r) => `${app.agents[r.id]?.name ?? '?'} : ${fBytes(r.memory)} · ${fPct(r.cpu)}`),
    ].join('\n'),
  );
  const five = $derived(app.usage.fiveHour);
  const week = $derived(app.usage.sevenDay);

  function toggleSound() {
    app.settings.sound = !app.settings.sound;
    app.run(api.saveSettings($state.snapshot(app.settings)));
  }

  /** Seconds left before the automatic restart for the update, when one is planned. */
  const restartIn = $derived(app.restartAt === null ? null : Math.max(0, Math.ceil((app.restartAt - app.now) / 1000)));

  function postpone() {
    app.restartAt = null;
    app.run(api.updatePostpone());
  }

  type SyncOp = 'pull' | 'push' | 'fetch';
  const SYNC: Record<SyncOp, { label: string; call: (projectId: string) => Promise<string> }> = {
    pull: { label: 'Pull', call: api.gitPull },
    push: { label: 'Push', call: api.gitPush },
    fetch: { label: 'Fetch', call: api.gitFetch },
  };

  /** The active project's checkout against its remote, when it can sync (a branch, a remote). */
  const sync = $derived.by(() => {
    const p = app.ui.view === 'project' ? app.project : null;
    const g = p ? app.git[p.id] : undefined;
    if (!p || !g?.isRepo || !g.hasRemote || !g.branch || g.branch === '(detached)') return null;
    // Pull and push need an upstream that still exists; otherwise the branch is (re)published.
    return { ...g, projectId: p.id, tracked: !!g.upstream && !g.upstreamGone };
  });
  /** Sync running, by project. */
  let syncing = $state<Record<string, SyncOp>>({});
  const busy = $derived(sync ? syncing[sync.projectId] : undefined);
  let syncButton = $state<HTMLButtonElement>();

  const syncTitle = $derived.by(() => {
    if (!sync) return '';
    const fetched = sync.lastFetch ? fAgo(sync.lastFetch / 1000, app.now) : 'jamais';
    return [
      sync.tracked
        ? `Suit ${sync.upstream} : ${sync.behind} à tirer, ${sync.ahead} à pousser`
        : sync.upstream
          ? `La branche suivie ${sync.upstream} n'existe plus sur le dépôt distant`
          : 'Branche pas encore publiée sur le dépôt distant',
      `Dernier fetch : ${fetched}`,
    ].join('\n');
  });

  async function runSync(op: SyncOp, projectId: string) {
    syncing[projectId] = op;
    const summary = await app.run(SYNC[op].call(projectId));
    delete syncing[projectId];
    if (summary) app.toast(summary, 'ok');
  }

  function syncMenu() {
    const s = sync;
    if (!s || !syncButton || busy) return;
    const go = (op: SyncOp) => () => runSync(op, s.projectId);
    menu.showAt(syncButton, [
      { label: 'Pull', hint: `↓${s.behind}`, disabled: !s.tracked || s.behind === 0, onClick: go('pull') },
      s.tracked
        ? { label: 'Push', hint: `↑${s.ahead}`, disabled: s.ahead === 0, onClick: go('push') }
        : { label: 'Publier la branche', onClick: go('push') },
      { label: '', separator: true },
      { label: 'Fetch', hint: 'maintenant', onClick: go('fetch') },
    ]);
  }
</script>

<footer class="bar mono">
  <span class="it"><span class="dot" style="width:7px;height:7px;background:var(--ok)"></span>{running} actif{running > 1 ? 's' : ''}</span>
  <button
    class="it link"
    style:color={waiting ? 'var(--wait)' : 'var(--muted)'}
    onclick={() => app.nextWaiting()}
    title={`Aller au prochain agent en attente ou à voir (${keyLabel('Ctrl+J')})`}
  >
    {#if waiting}<span class="pulse" style="width:7px;height:7px"></span>{:else}<span
        class="dot"
        style="width:7px;height:7px;background:var(--dim)"
      ></span>{/if}
    {waiting} en attente
  </button>
  <span class="it"><span style="color:var(--ok)">✓</span>{done} terminé{done > 1 ? 's' : ''}</span>
  {#if procs.instances}
    <span class="vsep"></span>
    <span class="it" title={procsTitle}
      >{procs.instances} Claude · <span class="v">{fBytes(procs.memory)}</span> · <span class="v">{fPct(procs.cpu)}</span> CPU</span
    >
  {/if}
  <span class="vsep"></span>
  <span
    class="it"
    title={five?.resetsAt ? `Réinitialisation : ${new Date(five.resetsAt).toLocaleString('fr-FR')}` : 'Quota de session indisponible'}
  >
    Session 5 h
    <span class="meter"
      ><span style:width="{Math.min(100, five?.pct ?? 0)}%" style:background={(five?.pct ?? 0) > 80 ? 'var(--wait)' : 'var(--accent)'}
      ></span></span
    >
    <span class="v">{five ? fPct(five.pct) : '—'}</span>
    {#if five?.resetsAt}<span class="d">reset {fCountdown(five.resetsAt, app.now)}</span>{/if}
  </span>
  <span
    class="it"
    title={week?.resetsAt ? `Réinitialisation : ${new Date(week.resetsAt).toLocaleString('fr-FR')}` : 'Quota hebdomadaire indisponible'}
  >
    Hebdo
    <span class="meter"
      ><span style:width="{Math.min(100, week?.pct ?? 0)}%" style:background={(week?.pct ?? 0) > 80 ? 'var(--wait)' : 'var(--accent)'}
      ></span></span
    >
    <span class="v">{week ? fPct(week.pct) : '—'}</span>
    {#if week?.resetsAt}<span class="d">reset {fCountdown(week.resetsAt, app.now)}</span>{/if}
  </span>
  <span class="vsep"></span>
  <span class="it" title={app.liveCost > 0 ? ESTIMATE_HINT : undefined}
    >Aujourd'hui <span class="v strong">{fSpentUsd({ cost: app.usage.todayCost + app.liveCost, estimated: app.liveCost > 0 })}</span></span
  >
  {#if sync}
    <span class="vsep"></span>
    <button class="it link sync" bind:this={syncButton} disabled={!!busy} onclick={syncMenu} title={syncTitle}>
      <span class="v">⎇ {sync.branch}</span>
      {#if busy}
        <span>{SYNC[busy].label}…</span>
      {:else if sync.tracked}
        <span style:color={sync.behind ? 'var(--wait)' : 'var(--dim)'}>↓{sync.behind}</span>
        <span style:color={sync.ahead ? 'var(--text)' : 'var(--dim)'}>↑{sync.ahead}</span>
      {:else}
        <span class="d">{sync.upstream ? 'distante supprimée' : 'non publiée'}</span>
      {/if}
    </button>
  {/if}
  <div style="flex:1"></div>
  {#if restartIn !== null}
    <span class="it v">Redémarrage dans {restartIn} s</span>
    <button class="small" onclick={postpone}>Plus tard</button>
  {:else if app.update?.ready}
    <button class="upd" onclick={() => (app.modal = { kind: 'update' })}>Mise à jour {app.update.version} prête · Redémarrer</button>
  {:else if app.update}
    <span class="it">Mise à jour {app.update.version}…</span>
  {/if}
  <button class="small" onclick={toggleSound} title="Son des notifications">♪ {app.settings.sound ? 'On' : 'Off'}</button>
  <button
    class="small"
    onclick={() => {
      app.modal = { kind: 'settings' };
    }}
    title={`Réglages (${keyLabel('Ctrl+,')})`}>⚙</button
  >
</footer>

<style>
  .bar {
    height: 30px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 0 8px 0 16px;
    background: var(--panel);
    border-top: 1px solid var(--line);
    font-size: 11px;
    color: var(--muted);
    user-select: none;
    white-space: nowrap;
    overflow: hidden;
  }
  .it {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .link {
    border: none;
    background: transparent;
    font: inherit;
    cursor: pointer;
    padding: 0;
  }
  .sync:hover:not(:disabled) .v {
    text-decoration: underline;
  }
  .sync:disabled {
    cursor: default;
  }
  .vsep {
    width: 1px;
    height: 14px;
    background: var(--line2);
  }
  .meter {
    width: 48px;
    height: 5px;
    border-radius: 3px;
    background: var(--elev2);
    overflow: hidden;
    margin-left: 2px;
  }
  .meter span {
    display: block;
    height: 100%;
    transition: width 0.4s;
  }
  .v {
    color: var(--text);
  }
  .strong {
    font-weight: 600;
  }
  .d {
    color: var(--dim);
  }
  .small {
    height: 22px;
    padding: 0 8px;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font: inherit;
    cursor: pointer;
  }
  .small:hover {
    background: var(--elev2);
    color: var(--text);
  }
  .upd {
    height: 22px;
    padding: 0 10px;
    border: none;
    border-radius: var(--r-sm);
    background: var(--accent);
    color: var(--accent-ink);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
</style>
