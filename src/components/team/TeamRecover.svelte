<script lang="ts">
  // « Récupérer une équipe » (TS-3, §3.5, C7-B): when no administrator has a device left, the server's address, the
  // token its host gives (`escouade-team-server recovery-token`), the 24 words and the team's six words, both from the
  // rescue sheet, bring the team back on this device, an administrator; the team then gets a new recovery key, shown
  // once. The six words come from the sheet, never from the server (SR2-09); a sheet without them is told as too old.
  import { t } from '../../lib/i18n';
  import { SIX, wordCount, wordsTyped } from '../../lib/team-words';
  import { errorText, teamsApi, type Created } from '../../lib/teams.svelte';

  let { onmade, onback }: { onmade: (c: Created) => void; onback: () => void } = $props();

  let server = $state('');
  let token = $state('');
  let words = $state('');
  let teamWords = $state('');
  let noTeamWords = $state(false);
  let name = $state('');
  let busy = $state(false);
  let failure = $state('');

  const count = $derived(wordCount(words));
  const teamCount = $derived(wordCount(teamWords));

  async function go() {
    busy = true;
    failure = '';
    try {
      const made = await teamsApi.recover(server, token, wordsTyped(words), wordsTyped(teamWords), name);
      words = '';
      teamWords = '';
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
    <span class="lbl">{t('team.recover.team')} · {teamCount}/{SIX}</span>
    <input
      class="field mono"
      bind:value={teamWords}
      disabled={noTeamWords}
      autocomplete="off"
      spellcheck="false"
      placeholder={t('team.recover.teamPlaceholder')}
    />
    <span class="hint">{t('team.recover.teamHint')}</span>
  </label>
  <label class="check"><input type="checkbox" bind:checked={noTeamWords} /><span>{t('team.recover.noTeamWords')}</span></label>
  {#if noTeamWords}<p class="old" role="note">{t('team.recover.oldSheet')}</p>{/if}
  <label class="row">
    <span class="lbl">{t('team.recover.name')}</span>
    <input class="field" bind:value={name} maxlength="80" />
  </label>
  {#if failure}<p class="error" role="alert">{failure}</p>{/if}
  <div class="foot">
    <button type="button" class="btn ghost" onclick={onback}>{t('team.common.back')}</button>
    <button
      type="submit"
      class="btn primary"
      disabled={busy || !server.trim() || !token.trim() || count !== 24 || noTeamWords || teamCount !== SIX}
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
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--soft);
    font-size: 13px;
  }
  .old {
    margin: 0;
    padding: 10px 12px;
    border-radius: 8px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--soft);
    font-size: 13px;
    line-height: 1.5;
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
