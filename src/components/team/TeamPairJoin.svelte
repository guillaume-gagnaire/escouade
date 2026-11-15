<script lang="ts">
  // The new device of a short code (TS-3, §3.4): the server's address and the 8 characters shown on the device in the
  // team. One try: a wrong code spends it, and a new one is needed.
  import { t } from '../../lib/i18n';
  import { codeComplete, typedCode } from '../../lib/team-words';
  import { errorText, teams, teamsApi } from '../../lib/teams.svelte';

  let { onback, ondone }: { onback: () => void; ondone: () => void } = $props();

  let server = $state('');
  let code = $state('');
  let name = $state('');
  let busy = $state(false);
  let failure = $state('');
  let joined = $state<string | null>(null);

  async function go() {
    busy = true;
    failure = '';
    try {
      const team = await teamsApi.pairJoin(server, code, name);
      teams.put(team);
      joined = team.id;
    } catch (e) {
      failure = errorText(e);
    } finally {
      busy = false;
    }
  }

  const team = $derived(teams.byId(joined));
</script>

{#if !joined}
  <p class="lead">{t('team.pairJoin.lead')}</p>
  <form class="form" onsubmit={(e) => (e.preventDefault(), void go())}>
    <label class="row">
      <span class="lbl">{t('team.pairJoin.server')}</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="field mono"
        bind:value={server}
        autofocus
        autocomplete="off"
        spellcheck="false"
        placeholder={t('team.create.serverPlaceholder')}
      />
    </label>
    <label class="row">
      <span class="lbl">{t('team.pairJoin.code')}</span>
      <input
        class="field mono code"
        value={code}
        oninput={(e) => (code = typedCode(e.currentTarget.value))}
        autocomplete="off"
        spellcheck="false"
        placeholder={t('team.pairJoin.codePlaceholder')}
      />
    </label>
    <label class="row">
      <span class="lbl">{t('team.pairJoin.name')}</span>
      <input class="field" bind:value={name} maxlength="80" />
    </label>
    {#if failure}<p class="error" role="alert">{failure}</p>{/if}
    <div class="foot">
      <button type="button" class="btn ghost" onclick={onback}>{t('team.common.back')}</button>
      <button type="submit" class="btn primary" disabled={busy || !server.trim() || !codeComplete(code)}
        >{busy ? t('team.pairJoin.working') : t('team.pairJoin.submit')}</button
      >
    </div>
  </form>
{:else}
  <p class={team?.joined ? 'ok' : 'wait'} role="status">
    {team?.joined ? t('team.join.joined', { name: team.name }) : t('team.join.waiting')}
  </p>
  <div class="foot"><button class="btn primary" onclick={ondone}>{t('team.common.close')}</button></div>
{/if}

<style>
  .lead,
  .error,
  .ok,
  .wait {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
  }
  .lead {
    color: var(--soft);
  }
  .error {
    color: var(--del);
  }
  .ok {
    color: var(--add);
  }
  .wait {
    color: var(--wait);
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .lbl {
    color: var(--soft);
    font-size: 12.5px;
    font-weight: 560;
  }
  .code {
    font-size: 18px;
    letter-spacing: 0.1em;
  }
  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
