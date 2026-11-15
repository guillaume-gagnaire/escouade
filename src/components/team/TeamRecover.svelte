<script lang="ts">
  // « Récupérer une équipe » (TS-3, §3.5, C7-B): when no administrator has a device left, the server's address, the
  // token its host gives (`escouade-team-server recovery-token`) and the 24 words bring the team back on this device,
  // an administrator; the team then gets a new recovery key, shown once.
  import { t } from '../../lib/i18n';
  import { wordCount, wordsTyped } from '../../lib/team-words';
  import { errorText, teamsApi, type Created } from '../../lib/teams.svelte';

  let { onmade, onback }: { onmade: (c: Created) => void; onback: () => void } = $props();

  let server = $state('');
  let token = $state('');
  let words = $state('');
  let name = $state('');
  let busy = $state(false);
  let failure = $state('');

  const count = $derived(wordCount(words));

  async function go() {
    busy = true;
    failure = '';
    try {
      const made = await teamsApi.recover(server, token, wordsTyped(words), name);
      words = '';
      onmade(made);
    } catch (e) {
      failure = errorText(e);
    } finally {
      busy = false;
    }
  }
</script>

<p class="lead">{t('team.recover.lead')}</p>
<form class="form" onsubmit={(e) => (e.preventDefault(), void go())}>
  <label class="row">
    <span class="lbl">{t('team.recover.server')}</span>
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
    <span class="lbl">{t('team.recover.token')}</span>
    <input class="field mono" type="password" bind:value={token} autocomplete="off" spellcheck="false" />
    <span class="hint">{t('team.recover.tokenHint')}</span>
  </label>
  <label class="row">
    <span class="lbl">{t('team.recover.words')} · {count}/24</span>
    <textarea
      class="field mono"
      rows="4"
      bind:value={words}
      autocomplete="off"
      spellcheck="false"
      placeholder={t('team.recover.wordsPlaceholder')}></textarea>
  </label>
  <label class="row">
    <span class="lbl">{t('team.recover.name')}</span>
    <input class="field" bind:value={name} maxlength="80" />
  </label>
  {#if failure}<p class="error" role="alert">{failure}</p>{/if}
  <div class="foot">
    <button type="button" class="btn ghost" onclick={onback}>{t('team.common.back')}</button>
    <button type="submit" class="btn primary" disabled={busy || !server.trim() || !token.trim() || count !== 24}
      >{busy ? t('team.recover.working') : t('team.recover.submit')}</button
    >
  </div>
</form>

<style>
  .lead,
  .hint,
  .error {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
  }
  .lead {
    color: var(--soft);
  }
  .hint {
    color: var(--dim);
    font-size: 12px;
  }
  .error {
    color: var(--del);
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
  textarea {
    resize: vertical;
  }
  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
