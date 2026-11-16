<script lang="ts">
  // The team's signed .escouade.yml on this device (TS-13), where TS-3 put the team (« Réglages › Équipe » is TS-4):
  // the version in force and who signed it, what waits for this device's consent (a sensitive key with what it allows,
  // a proposed command read at its fingerprint), what the team imposes and recommends, the history with each change,
  // the team's projects on this device; for an administrator, a version to write, read and sign, and the versions of
  // the others to cosign.
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { fAgo } from '../../lib/format';
  import { app } from '../../lib/state.svelte';
  import {
    allows,
    applied,
    changesOf,
    diff,
    shown,
    statusLabel,
    teamSettingsApi,
    type Change,
    type Pending,
    type SettingsDraft,
    type TeamSettingsView,
  } from '../../lib/team-settings';
  import { errorText, teams } from '../../lib/teams.svelte';
  import { teamBoardApi } from '../../lib/teamboard';
  import { listedHere, teamProjectsApi } from '../../lib/team-projects';

  let { teamId, onback }: { teamId: string; onback: () => void } = $props();

  /** The projects of this device whose repository is in the team's list (TS-8): attached in « Projets de l'équipe », not by a box here. */
  let listed = $state<Set<string>>(new Set());
  onMount(() => {
    teamProjectsApi
      .get(teamId)
      .then((v) => (listed = listedHere(v)))
      .catch(() => {});
  });
  const boxed = $derived(app.projects.filter((p) => !listed.has(p.id)));

  const s = $derived<TeamSettingsView | undefined>(teams.settings[teamId]);
  const now = $derived(s ? applied(s.view) : null);
  let busy = $state(false);
  let failure = $state('');
  /** The versions whose change is shown, by number. */
  let open = $state<Record<string, boolean>>({});

  let yaml = $state('');
  let note = $state('');
  let by = $state('');
  let draft = $state<SettingsDraft | null>(null);

  onMount(async () => {
    try {
      const v = await teamSettingsApi.get(teamId);
      teams.putSettings(v);
      yaml = v.yaml;
      by = v.signerName;
      const n = v.view.applied;
      if (n !== null && n > (v.seen ?? 0)) await teamSettingsApi.seen(teamId, n);
    } catch (e) {
      failure = errorText(e);
    }
  });

  async function act(f: () => Promise<TeamSettingsView>) {
    busy = true;
    failure = '';
    try {
      teams.putSettings(await f());
    } catch (e) {
      failure = errorText(e);
    } finally {
      busy = false;
    }
  }

  const consent = (p: Pending) => act(() => teamSettingsApi.consent(teamId, p.part, p.key, p.fingerprint));
  const withdraw = (key: string) => act(() => teamSettingsApi.withdraw(teamId, 'enforced', key));
  const cosign = (digest: string) => act(() => teamSettingsApi.cosign(teamId, digest));
  const retry = () => act(() => teamSettingsApi.retry(teamId));

  async function check() {
    failure = '';
    try {
      draft = await teamSettingsApi.check(teamId, yaml);
    } catch (e) {
      failure = errorText(e);
    }
  }

  async function sign() {
    await act(async () => {
      const v = await teamSettingsApi.propose(teamId, yaml, note, by);
      draft = null;
      note = '';
      yaml = v.yaml;
      return v;
    });
  }

  const draftChanges = $derived<Change[]>(draft?.file ? diff(now?.file ?? null, draft.file) : []);
  const sensitiveKeys = ['proxy.url', 'proxy.authorities', 'proxy.exclusions'];
  const enforced = $derived(Object.entries(s?.view.effective.enforced ?? {}));
  const recommended = $derived(Object.entries(s?.view.effective.defaults ?? {}));
  const accepted = $derived(Object.entries(s?.view.effective.accepted ?? {}));
  const quorumLine = $derived(
    !s ? '' : s.view.quorum ? t('team.settings.quorumOn') : s.view.admins < 2 ? t('team.settings.quorumOne') : t('team.settings.quorumOff'),
  );

  function changeLine(c: Change): string {
    const what = t(`team.settings.change.${c.kind}`);
    if (c.kind === 'changed') return `${what} : ${shown(c.before)} → ${shown(c.after)}`;
    return `${what} : ${shown(c.kind === 'added' ? c.after : c.before)}`;
  }

  /** The id typed to join a teammate's Kanban, by project. */
  let joining = $state<Record<string, string>>({});

  async function linkBoard(id: string, journal: string | null) {
    failure = '';
    try {
      await teamBoardApi.link(id, journal);
    } catch (e) {
      failure = errorText(e);
    }
  }

  async function toggleProject(id: string, on: boolean) {
    failure = '';
    try {
      await teamSettingsApi.setProjectTeam(id, on ? teamId : null);
    } catch (e) {
      failure = errorText(e);
    }
  }
</script>

{#snippet changes(list: Change[])}
  {#if list.length === 0}
    <p class="muted">{t('team.settings.history.noChange')}</p>
  {:else}
    <ul class="diff" data-testid="team-settings-diff">
      {#each list as c (c.part + c.key)}
        <li class={c.kind}>
          <span class="part">{t(`team.settings.part.${c.part}`)}</span>
          <span class="mono key">{c.key}</span>
          <span class="mono">{changeLine(c)}</span>
        </li>
      {/each}
    </ul>
  {/if}
{/snippet}

<div class="settings" data-testid="team-settings">
  <p class="lead">{t('team.settings.lead')}</p>

  {#if !s}
    <p class="muted">{failure || t('team.settings.none')}</p>
  {:else}
    {#if s.view.alarm}
      <div class="alarm" role="alert">
        <p>{t(`team.settings.alarm.${s.view.alarm}`)}</p>
        <button class="btn" disabled={busy} onclick={retry}>{t('team.settings.alarm.retry')}</button>
      </div>
    {/if}

    <section aria-label={t('team.settings.open')}>
      {#if now}
        <p class="force" data-testid="team-settings-force">
          {t('team.settings.inForce', { number: now.number, by: now.by, ago: fAgo(now.at, Date.now()) })}
          · {t('team.settings.signatures', { count: now.signers.length })}
        </p>
        {#if now.note}<p class="note">{now.note}</p>{/if}
      {:else}
        <p class="muted">{t('team.settings.none')}</p>
      {/if}
      <p class="muted small">{quorumLine}</p>
    </section>

    {#if s.view.effective.pending.length}
      <section class="pending" aria-label={t('team.settings.pending.title')}>
        <h4>{t('team.settings.pending.title')}</h4>
        {#each s.view.effective.pending as p (p.part + p.key)}
          <div class="ask" data-testid="team-consent">
            <div class="row">
              <span class="tag"
                >{p.sort === 'executable' ? t('team.settings.pending.executable') : t('team.settings.pending.sensitive')}</span
              >
              <span class="mono key">{p.key}</span>
            </div>
            <pre class="mono value">{shown(p.value)}</pre>
            <p class="allows">{allows(p)}</p>
            {#if s.view.effective.reasons[p.key]}<p class="muted small">{s.view.effective.reasons[p.key]}</p>{/if}
            <div class="row">
              <span class="muted small mono">{t('team.settings.pending.fingerprint', { print: p.fingerprint.slice(0, 12) })}</span>
              <button class="btn primary" disabled={busy} onclick={() => consent(p)}
                >{p.sort === 'executable' ? t('team.settings.pending.acceptCommand') : t('team.settings.pending.accept')}</button
              >
            </div>
          </div>
        {/each}
      </section>
    {/if}

    <section aria-label={t('team.settings.imposed.title')}>
      <h4>{t('team.settings.imposed.title')}</h4>
      {#if enforced.length === 0}<p class="muted">{t('team.settings.imposed.none')}</p>{/if}
      <ul class="keys">
        {#each enforced as [key, value] (key)}
          <li>
            <span class="mono key">{key}</span><span class="mono">{shown(value)}</span>
            {#if s.view.effective.reasons[key]}<span class="muted small">{s.view.effective.reasons[key]}</span>{/if}
            {#if sensitiveKeys.includes(key)}
              <button class="link" disabled={busy} onclick={() => withdraw(key)}>{t('team.settings.imposed.withdraw')}</button>
            {/if}
          </li>
        {/each}
      </ul>
      {#if recommended.length}
        <h4>{t('team.settings.imposed.recommended')}</h4>
        <ul class="keys">
          {#each recommended as [key, value] (key)}<li>
              <span class="mono key">{key}</span><span class="mono">{shown(value)}</span>
            </li>{/each}
        </ul>
      {/if}
      {#if accepted.length}
        <h4>{t('team.settings.imposed.accepted')}</h4>
        <ul class="keys">
          {#each accepted as [key, value] (key)}<li><span class="mono key">{key}</span><span class="mono">{shown(value)}</span></li>{/each}
        </ul>
      {/if}
      {#if s.view.effective.unknown.length}
        <p class="muted small">{t('team.settings.imposed.unknown', { keys: s.view.effective.unknown.join(', ') })}</p>
      {/if}
    </section>

    <section aria-label={t('team.settings.projects.title')}>
      <h4>{t('team.settings.projects.title')}</h4>
      <p class="muted small">{t('team.settings.projects.lead')}</p>
      {#if listed.size}<p class="muted small">{t('teamProjects.settingsNote', { count: listed.size })}</p>{/if}
      {#if boxed.length === 0 && listed.size === 0}<p class="muted">{t('team.settings.projects.none')}</p>{/if}
      {#each boxed as p (p.id)}
        <label class="check">
          <input
            type="checkbox"
            checked={p.team === teamId}
            disabled={!!p.team && p.team !== teamId}
            onchange={(e) => toggleProject(p.id, e.currentTarget.checked)}
          />
          {p.name}
        </label>
        {#if p.team === teamId}
          <!-- TS-6: the project's Kanban made the team's, a new one or a teammate's by its id (TS-8 lists them). -->
          <div class="board-link" data-team-board-link={p.id}>
            {#if p.teamProject}
              <span class="muted small mono" data-team-board-id={p.teamProject}>{t('teamBoard.link.linked', { id: p.teamProject })}</span>
            {:else}
              <button class="btn small" onclick={() => linkBoard(p.id, null)}>{t('teamBoard.link.create')}</button>
              <input
                class="field mono"
                aria-label={t('teamBoard.link.idLabel')}
                placeholder={t('teamBoard.link.idPlaceholder')}
                bind:value={() => joining[p.id] ?? '', (v) => (joining[p.id] = v)}
              />
              <button
                class="btn small"
                disabled={!/^[0-9a-f]{32}$/i.test((joining[p.id] ?? '').trim())}
                onclick={() => linkBoard(p.id, (joining[p.id] ?? '').trim().toLowerCase())}>{t('teamBoard.link.join')}</button
              >
            {/if}
          </div>
        {/if}
      {/each}
    </section>

    <section aria-label={t('team.settings.history.title')}>
      <h4>{t('team.settings.history.title')}</h4>
      <ol class="history" data-testid="team-settings-history">
        {#each s.view.versions as v (v.digest)}
          <li class:refused={v.status.kind === 'refused'}>
            <div class="row">
              <span class="num">{v.number}</span>
              <span>{v.by}</span>
              <span class="muted small">{fAgo(v.at, Date.now())}</span>
              <span class="status">{statusLabel(v)}</span>
              {#if s.admin && v.status.kind === 'proposed' && !v.signedByMe}
                <button class="btn" disabled={busy} onclick={() => cosign(v.digest)}>{t('team.settings.history.cosign')}</button>
              {/if}
              <button class="link" onclick={() => (open = { ...open, [v.digest]: !open[v.digest] })}
                >{open[v.digest] ? t('team.settings.history.hide') : t('team.settings.history.show')}</button
              >
            </div>
            {#if v.note}<p class="note">{v.note}</p>{/if}
            {#if open[v.digest]}{@render changes(changesOf(s.view, v))}{/if}
          </li>
        {/each}
      </ol>
      {#if s.view.refusals.length}
        <p class="muted small">{t('team.settings.history.refusals', { count: s.view.refusals.length })}</p>
      {/if}
    </section>

    {#if s.admin}
      <section class="propose" aria-label={t('team.settings.propose.title')}>
        <h4>{t('team.settings.propose.title')}</h4>
        <p class="muted small">{t('team.settings.propose.lead')}</p>
        <textarea class="mono" rows="10" bind:value={yaml} oninput={() => (draft = null)} aria-label={t('team.settings.propose.title')}
        ></textarea>
        <label>{t('team.settings.propose.name')} <input bind:value={by} /></label>
        <label>{t('team.settings.propose.note')} <input bind:value={note} /></label>
        <div class="row">
          <button class="btn" disabled={busy} onclick={check}>{t('team.settings.propose.check')}</button>
          {#if draft && !draft.error}
            <button class="btn primary" disabled={busy || draftChanges.length === 0 || !by.trim()} onclick={sign}
              >{busy ? t('team.settings.propose.signing') : t('team.settings.propose.sign')}</button
            >
          {/if}
        </div>
        {#if draft}
          {#if draft.error}<p class="error" role="alert">{draft.error}</p>{/if}
          {#each draft.warnings as w (w)}<p class="advice">{w}</p>{/each}
          {#if !draft.error}
            {#if draftChanges.length === 0}
              <p class="muted">{t('team.settings.propose.noChange')}</p>
            {:else}
              {@render changes(draftChanges)}
              <p class="advice">{draft.need > 1 ? t('team.settings.propose.needTwo') : t('team.settings.propose.needOne')}</p>
            {/if}
          {/if}
        {/if}
      </section>
    {/if}
  {/if}

  {#if failure && s}<p class="error" role="alert">{failure}</p>{/if}
  <div class="acts"><button class="btn ghost" onclick={onback}>{t('team.common.back')}</button></div>
</div>

<style>
  .board-link {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin: 2px 0 8px 24px;
  }
  .board-link .field {
    width: 22em;
    max-width: 100%;
  }
  .settings {
    display: flex;
    flex-direction: column;
    gap: 14px;
    font-size: 13px;
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
  p {
    margin: 0;
    line-height: 1.5;
  }
  .lead {
    color: var(--soft);
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
  .force {
    color: var(--text);
    font-weight: 560;
  }
  .note {
    color: var(--soft);
    font-style: italic;
  }
  .alarm {
    display: flex;
    gap: 10px;
    align-items: center;
    padding: 10px 12px;
    border-radius: 8px;
    color: var(--del);
    background: color-mix(in oklch, var(--del) 10%, transparent);
  }
  .ask {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .allows {
    color: var(--text);
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
  }
  .row .btn:last-child {
    margin-left: auto;
  }
  .tag,
  .part {
    padding: 1px 7px;
    border-radius: 999px;
    background: var(--p-soft);
    color: var(--p-ink);
    font-size: 11px;
    font-weight: 600;
  }
  .mono {
    font-family: var(--mono);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  .key {
    color: var(--text);
    font-weight: 560;
  }
  .value {
    margin: 0;
    white-space: pre-wrap;
  }
  .keys,
  .diff,
  .history {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .keys li,
  .diff li {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: baseline;
  }
  .diff .added .mono:last-child {
    color: var(--add);
  }
  .diff .removed .mono:last-child {
    color: var(--del);
  }
  .history li {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 10px;
    border-radius: 8px;
    background: var(--panel);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .history li.refused {
    opacity: 0.75;
  }
  .num {
    font-weight: 650;
    color: var(--text);
  }
  .status {
    color: var(--soft);
  }
  .check {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  textarea {
    width: 100%;
    box-sizing: border-box;
    resize: vertical;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    color: var(--soft);
  }
  .check {
    flex-direction: row;
  }
  .link {
    border: 0;
    background: none;
    padding: 0;
    color: var(--p-ink);
    cursor: pointer;
    font-size: 12px;
  }
  .acts {
    display: flex;
    justify-content: flex-end;
  }
</style>
