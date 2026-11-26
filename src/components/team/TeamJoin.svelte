<script lang="ts">
  // « Rejoindre une équipe » (TS-3, §3.3): the link pasted or opened (`escouade://team/join`), its server and the team's
  // six words to check with whoever invites, then the request; an administrator online lets the device in.
  import { untrack } from 'svelte';
  import { t } from '../../lib/i18n';
  import { errorText, teams, teamsApi, type JoinPreview } from '../../lib/teams.svelte';
  import SixWords from './SixWords.svelte';

  let {
    link: given = '',
    expected = '',
    onback,
    ondone,
  }: {
    link?: string;
    /** The host a repository names (TS-7, the band of the invitation): said, never reached. */
    expected?: string;
    onback: () => void;
    ondone: () => void;
  } = $props();

  let link = $state(untrack(() => given));
  let name = $state('');
  let preview = $state<JoinPreview | null>(null);
  let previewError = $state('');
  let busy = $state(false);
  let failure = $state('');
  /** The team asked to join, once the request went. */
  let joining = $state<string | null>(null);

  // What the link says, as it is typed.
  $effect(() => {
    const text = link.trim();
    if (!text) {
      preview = null;
      previewError = '';
      return;
    }
    let stale = false;
    teamsApi
      .joinPreview(text)
      .then((p) => {
        if (stale) return;
        preview = p;
        previewError = '';
      })
      .catch((e) => {
        if (stale) return;
        preview = null;
        previewError = errorText(e);
      });
    return () => (stale = true);
  });

  async function join() {
    busy = true;
    failure = '';
    try {
      const team = await teamsApi.join(link, name);
      teams.put(team);
      joining = team.id;
    } catch (e) {
      failure = errorText(e);
    } finally {
      busy = false;
    }
  }

  const team = $derived(teams.byId(joining));
</script>

{#if !joining}
  <p class="lead">{t('team.join.lead')}</p>
  {#if expected}<p class="hint" data-testid="team-join-expected">{t('team.join.expected', { host: expected })}</p>{/if}
  <label class="row">
    <span class="lbl">{t('team.join.link')}</span>
    <!-- svelte-ignore a11y_autofocus -->
    <textarea class="field mono" rows="3" bind:value={link} autofocus spellcheck="false" placeholder={t('team.join.linkPlaceholder')}
    ></textarea>
  </label>
  {#if previewError && link.trim().length > 20}<p class="hint">{previewError}</p>{/if}
  {#if preview}
    <div class="preview">
      <div><span class="lbl">{t('team.join.server')}</span> <span class="mono">{preview.server}</span></div>
      {#if expected && preview.server.toLowerCase() !== expected.toLowerCase()}
        <p class="hint" role="alert">{t('team.join.otherServer', { server: preview.server, host: expected })}</p>
      {/if}
      <SixWords words={preview.fingerprint} label={t('team.card.fingerprint')} hint={t('team.join.check')} />
      {#if preview.ownDevice}<p class="hint">{t('team.join.ownDevice')}</p>{/if}
    </div>
    <label class="row">
      <span class="lbl">{t('team.join.name')}</span>
      <input class="field" bind:value={name} maxlength="80" placeholder={preview.server} />
    </label>
  {/if}
  {#if failure}<p class="error" role="alert">{failure}</p>{/if}
  <div class="foot">
    <button class="btn ghost" onclick={onback}>{t('team.common.back')}</button>
    <button class="btn primary" disabled={!preview || busy} onclick={join}>{busy ? t('team.join.working') : t('team.join.submit')}</button>
  </div>
{:else}
  {#if team?.joined}
    <p class="ok" role="status">{t('team.join.joined', { name: team.name })}</p>
    <SixWords words={team.fingerprint} label={t('team.card.fingerprint')} />
    {#if team.escrow}<p class="escrow" role="note">{t('team.card.escrow', { holder: team.escrow })}</p>{/if}
  {:else}
    <p class="wait" role="status">{t('team.join.waiting')}</p>
    {#if team?.joinWords?.length}
      <SixWords words={team.joinWords} label={t('team.admit.joinWords')} hint={t('team.admit.joinWordsHint')} />
    {/if}
  {/if}
  <div class="foot">
    <button class="btn primary" onclick={ondone}>{t('team.common.close')}</button>
  </div>
{/if}

<style>
  .lead,
  .hint,
  .error,
  .ok,
  .wait,
  .escrow {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
  }
  .lead {
    color: var(--soft);
  }
  .hint {
    color: var(--dim);
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
  .escrow {
    padding: 9px 12px;
    border-radius: 8px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--text);
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
    font-size: 12px;
  }
  .preview {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    font-size: 13px;
  }
  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
