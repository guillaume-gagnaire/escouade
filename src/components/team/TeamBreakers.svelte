<script lang="ts">
  // The team's circuit breakers (TS-15, docs/TEAM-SERVER.md §9), where TS-3 put the team (« Réglages › Équipe » is TS-4):
  // those in force (what, scope, who and why, when), those lifted; for an administrator, a breaker to set (its scope,
  // what it takes out, its reason), confirmed with what will stop and on how many devices online, and « Lever ».
  // The MCP server has no tool for it: only this screen sets and lifts.
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { fDateTime } from '../../lib/format';
  import { engineName } from '../../lib/engines';
  import { app } from '../../lib/state.svelte';
  import { scopeText, stopsText, teamBreakersApi, whatText, type BreakersView, type Scope, type What } from '../../lib/team-breakers';
  import { errorText, teams } from '../../lib/teams.svelte';

  let { teamId, onback }: { teamId: string; onback: () => void } = $props();

  const v = $derived<BreakersView | undefined>(teams.breakers[teamId]);
  let busy = $state(false);
  let failure = $state('');

  const names = (e: string) => engineName(e) || e;

  /** The form. */
  let scopeKind = $state<'team' | 'project'>('team');
  let project = $state('');
  let kind = $state<What['kind']>('harness');
  let harness = $state('');
  let model = $state('');
  let plugin = $state('');
  let uninstall = $state(false);
  let reason = $state('');
  let by = $state('');
  /** The confirmation shown, with the devices online (null: not read). */
  let confirming = $state(false);
  let online = $state<number | null>(null);

  /** The projects of the team on this device that follow its Kanban (a breaker of one project names its journal). */
  const projects = $derived(app.projects.filter((p) => p.team === teamId && p.teamProject));
  const harnesses = $derived(app.engines.map((e) => ({ value: e.id as string, label: e.name })));

  onMount(async () => {
    try {
      const got = await teamBreakersApi.get(teamId);
      teams.putBreakers(got);
      by = got.signerName;
    } catch (e) {
      failure = errorText(e);
    }
  });

  $effect(() => {
    if (!harness && harnesses.length) harness = harnesses[0].value;
  });
  $effect(() => {
    if (!project && projects.length) project = projects[0].id;
  });

  const what = $derived<What>(
    kind === 'harness'
      ? { kind, id: harness }
      : kind === 'model'
        ? { kind, id: model.trim() }
        : kind === 'plugin'
          ? { kind, id: plugin.trim(), uninstall }
          : { kind: 'agents' },
  );
  const scope = $derived.by<Scope>(() => {
    if (scopeKind === 'team') return { kind: 'team' };
    const p = projects.find((x) => x.id === project);
    return { kind: 'project', project: p?.teamProject ?? '', name: p?.name ?? '' };
  });
  const ready = $derived(
    reason.trim().length > 0 &&
      by.trim().length > 0 &&
      (what.kind === 'agents' || what.id.length > 0) &&
      (scope.kind === 'team' || scope.project.length > 0),
  );

  async function next() {
    failure = '';
    confirming = true;
    online = null;
    try {
      online = await teamBreakersApi.online(teamId);
    } catch {
      online = null;
    }
  }

  async function act(f: () => Promise<BreakersView>) {
    busy = true;
    failure = '';
    try {
      teams.putBreakers(await f());
      return true;
    } catch (e) {
      failure = errorText(e);
      return false;
    } finally {
      busy = false;
    }
  }

  async function set() {
    const done = await act(() => teamBreakersApi.set(teamId, { scope, what, reason: reason.trim(), by: by.trim() }));
    if (done) {
      confirming = false;
      reason = '';
    }
  }

  const lift = (id: string) => act(() => teamBreakersApi.lift(teamId, id, by.trim() || (v?.signerName ?? '')));
  const retry = () => act(() => teamBreakersApi.retry(teamId));
</script>

<div class="breakers" data-testid="team-breakers">
  <p class="lead">{v?.admin ? t('teamBreakers.lead') : t('teamBreakers.leadMember')}</p>

  {#if v?.view.alarm}
    <div class="alarm" role="alert">
      <p>{t(`teamBreakers.alarm.${v.view.alarm}`)}</p>
      <button class="btn" disabled={busy} onclick={retry}>{t('teamBreakers.alarm.retry')}</button>
    </div>
  {/if}

  <section aria-label={t('teamBreakers.active')}>
    <h4>{t('teamBreakers.active')}</h4>
    {#if !v || v.view.active.length === 0}
      <p class="muted">{t('teamBreakers.none')}</p>
    {:else}
      <ul class="list">
        {#each v.view.active as b (b.id)}
          <li class="cut" data-testid="breaker-active">
            <p class="asked">{t('teamBreakers.band.asked', { by: b.by, reason: b.reason })}</p>
            <p class="what">{t('teamBreakers.band.cut', { what: whatText(b.what, names) })} · {scopeText(b.scope, v.name)}</p>
            <div class="row">
              <span class="muted small">{t('teamBreakers.setAt', { by: b.by, at: fDateTime(b.at * 1000) })}</span>
              {#if v.admin}
                <button class="btn" disabled={busy} onclick={() => lift(b.id)}>{t('teamBreakers.lift')}</button>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
    {#if v}
      <p class="muted small">
        {v.lastContact ? t('teamBreakers.lastContact', { at: fDateTime(v.lastContact * 1000) }) : t('teamBreakers.neverContact')}
      </p>
      {#if v.view.refusals.length}
        <p class="muted small">{t('teamBreakers.refused', { count: v.view.refusals.length })}</p>
      {/if}
    {/if}
  </section>

  {#if v?.admin}
    <section class="form" aria-label={t('teamBreakers.form.title')}>
      <h4>{t('teamBreakers.form.title')}</h4>
      {#if !confirming}
        <fieldset>
          <legend>{t('teamBreakers.form.scope')}</legend>
          <label class="radio"><input type="radio" bind:group={scopeKind} value="team" />{t('teamBreakers.form.scopeTeam')}</label>
          <label class="radio"
            ><input type="radio" bind:group={scopeKind} value="project" disabled={projects.length === 0} />{t(
              'teamBreakers.form.scopeProject',
            )}</label
          >
          {#if scopeKind === 'project'}
            <select class="field select" aria-label={t('teamBreakers.form.project')} bind:value={project}>
              {#each projects as p (p.id)}
                <option value={p.id}>{p.name}</option>
              {/each}
            </select>
          {/if}
          {#if projects.length === 0}<p class="muted small">{t('teamBreakers.form.noProject')}</p>{/if}
        </fieldset>
        <fieldset>
          <legend>{t('teamBreakers.form.what')}</legend>
          <div class="kinds">
            {#each ['harness', 'model', 'plugin', 'agents'] as const as k (k)}
              <label class="radio"><input type="radio" bind:group={kind} value={k} />{t(`teamBreakers.form.${k}`)}</label>
            {/each}
          </div>
          {#if kind === 'harness'}
            <select class="field select" aria-label={t('teamBreakers.form.harness')} bind:value={harness}>
              {#each harnesses as h (h.value)}
                <option value={h.value}>{h.label}</option>
              {/each}
            </select>
          {:else if kind === 'model'}
            <input class="field input mono" aria-label={t('teamBreakers.form.modelId')} spellcheck="false" bind:value={model} />
          {:else if kind === 'plugin'}
            <input class="field input mono" aria-label={t('teamBreakers.form.pluginId')} spellcheck="false" bind:value={plugin} />
            <label class="radio"><input type="checkbox" bind:checked={uninstall} />{t('teamBreakers.form.uninstall')}</label>
            <p class="muted small">{t('teamBreakers.form.pluginHint')}</p>
          {/if}
        </fieldset>
        <label>
          {t('teamBreakers.form.reason')}
          <textarea class="field" rows="2" aria-label={t('teamBreakers.form.reason')} bind:value={reason}></textarea>
          <span class="muted small">{t('teamBreakers.form.reasonHint')}</span>
        </label>
        <label>
          {t('teamBreakers.form.by')}
          <input class="field input" aria-label={t('teamBreakers.form.by')} bind:value={by} />
        </label>
        <div class="acts">
          <button class="btn primary" disabled={!ready || busy} onclick={next}>{t('teamBreakers.form.next')}</button>
        </div>
      {:else}
        <div class="confirm" role="alertdialog" aria-label={t('teamBreakers.confirm.title')} data-testid="breaker-confirm">
          <p class="asked">{t('teamBreakers.band.asked', { by: by.trim(), reason: reason.trim() })}</p>
          <p>{stopsText(what, names)}</p>
          <p class="muted">{scopeText(scope, v.name)}</p>
          <p class="force">
            {online === null
              ? t('teamBreakers.confirm.onlineUnknown')
              : online === 0
                ? t('teamBreakers.confirm.noneOnline')
                : t('teamBreakers.confirm.online', { count: online })}
          </p>
          <div class="acts">
            <button class="btn ghost" disabled={busy} onclick={() => (confirming = false)}>{t('teamBreakers.confirm.back')}</button>
            <button class="btn danger" disabled={busy} onclick={set}>{t('teamBreakers.confirm.set')}</button>
          </div>
        </div>
      {/if}
    </section>
  {/if}

  {#if v?.withdrawn?.length}
    <section aria-label={t('teamBreakers.withdrawn')} data-testid="breaker-withdrawn">
      <h4>{t('teamBreakers.withdrawn')}</h4>
      <p class="muted small">{t('teamBreakers.withdrawnLead')}</p>
      <ul class="list">
        {#each v.withdrawn as w (w.id)}
          <li class="cut">
            <p class="asked">{t('teamBreakers.withdrawnOne', { id: w.id, reason: w.reason })}</p>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if v && v.view.lifted.length}
    <section aria-label={t('teamBreakers.lifted')}>
      <h4>{t('teamBreakers.lifted')}</h4>
      <ul class="list">
        {#each v.view.lifted as l (l.breaker.id + l.at)}
          <li class="gone">
            <p>{t('teamBreakers.band.cut', { what: whatText(l.breaker.what, names) })} · {scopeText(l.breaker.scope, v.name)}</p>
            <p class="muted small">
              {t('teamBreakers.band.asked', { by: l.breaker.by, reason: l.breaker.reason })} · {t('teamBreakers.liftedAt', {
                by: l.by,
                at: fDateTime(l.at * 1000),
              })}
            </p>
          </li>
        {/each}
      </ul>
    </section>
  {/if}

  {#if failure}<p class="error" role="alert">{failure}</p>{/if}

  <div class="acts">
    <button class="btn ghost" onclick={onback}>{t('team.common.back')}</button>
  </div>
</div>

<style>
  .breakers {
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
  .error {
    color: var(--del);
  }
  .force {
    color: var(--text);
    font-weight: 560;
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
  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .list li {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px 10px;
    border-radius: 8px;
    background: var(--panel);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .list li.cut {
    box-shadow:
      inset 3px 0 0 var(--del),
      inset 0 0 0 1px var(--line-2);
  }
  .list li.gone {
    opacity: 0.8;
  }
  .asked {
    color: var(--text);
    font-weight: 600;
  }
  .what {
    color: var(--soft);
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
  fieldset {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin: 0;
    padding: 0;
    border: 0;
  }
  legend {
    margin-bottom: 4px;
    color: var(--soft);
  }
  .kinds {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
  }
  .radio {
    display: flex;
    flex-direction: row;
    gap: 6px;
    align-items: center;
    color: var(--text);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    color: var(--soft);
  }
  .field.select,
  .field.input {
    width: 22em;
    max-width: 100%;
  }
  .mono {
    font-family: var(--mono);
    font-size: 12px;
  }
  textarea {
    width: 100%;
    box-sizing: border-box;
    resize: vertical;
  }
  .confirm {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    border-radius: 10px;
    background: color-mix(in oklch, var(--del) 8%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--del) 30%, transparent);
  }
  .acts {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
</style>
