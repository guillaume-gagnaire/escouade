<script lang="ts">
  // « Créer une équipe » (TS-3, §3.5, C7-B): the server's address and the token of `escouade-team-server init`, a name,
  // and, in option, the organisation's escrow key (pasted, or made here: its secret is shown once, for its holder).
  import { t } from '../../lib/i18n';
  import { errorText, teamsApi, type Created } from '../../lib/teams.svelte';

  let { onmade, onback }: { onmade: (c: Created) => void; onback: () => void } = $props();

  let server = $state('');
  let token = $state('');
  let name = $state('');
  let escrow = $state(false);
  let holder = $state('');
  let publicKey = $state('');
  /** The secret of a pair made here, shown once. */
  let secret = $state('');
  let copied = $state(false);
  let busy = $state(false);
  let failure = $state('');

  const ready = $derived(!!server.trim() && !!token.trim() && !busy && (!escrow || (!!holder.trim() && !!publicKey.trim())));

  async function makePair() {
    failure = '';
    try {
      const pair = await teamsApi.escrowNew(holder);
      publicKey = pair.public;
      secret = pair.secret;
      copied = false;
    } catch (e) {
      failure = errorText(e);
    }
  }

  async function copySecret() {
    try {
      await navigator.clipboard.writeText(secret);
      copied = true;
    } catch {
      copied = false;
    }
  }

  async function create() {
    busy = true;
    failure = '';
    try {
      const made = await teamsApi.create(server, token, name, escrow ? publicKey : null, escrow ? holder : null);
      secret = '';
      onmade(made);
    } catch (e) {
      failure = errorText(e);
    } finally {
      busy = false;
    }
  }
</script>

<form class="form" onsubmit={(e) => (e.preventDefault(), ready && create())}>
  <label class="row">
    <span class="lbl">{t('team.create.server')}</span>
    <!-- svelte-ignore a11y_autofocus -->
    <input
      class="field mono"
      bind:value={server}
      autofocus
      autocomplete="off"
      spellcheck="false"
      placeholder={t('team.create.serverPlaceholder')}
    />
    <span class="hint">{t('team.create.serverHint')}</span>
  </label>
  <label class="row">
    <span class="lbl">{t('team.create.token')}</span>
    <input class="field mono" type="password" bind:value={token} autocomplete="off" spellcheck="false" />
    <span class="hint">{t('team.create.tokenHint')}</span>
  </label>
  <label class="row">
    <span class="lbl">{t('team.create.name')}</span>
    <input class="field" bind:value={name} maxlength="80" placeholder={t('team.create.namePlaceholder')} />
  </label>

  <label class="check">
    <input type="checkbox" bind:checked={escrow} />
    <span>{t('team.create.escrow')}</span>
  </label>
  {#if escrow}
    <div class="escrow">
      <p class="hint">{t('team.create.escrowHint')}</p>
      <label class="row">
        <span class="lbl">{t('team.create.escrowHolder')}</span>
        <input class="field" bind:value={holder} maxlength="120" placeholder={t('team.create.escrowHolderPlaceholder')} />
      </label>
      <label class="row">
        <span class="lbl">{t('team.create.escrowPublic')}</span>
        <input class="field mono" bind:value={publicKey} spellcheck="false" placeholder={t('team.create.escrowPublicPlaceholder')} />
      </label>
      <div><button type="button" class="btn" disabled={!holder.trim()} onclick={makePair}>{t('team.create.escrowMake')}</button></div>
      {#if secret}
        <div class="secret" role="note">
          <p>{t('team.create.escrowSecret')}</p>
          <code class="mono">{secret}</code>
          <div>
            <button type="button" class="btn" onclick={copySecret}>{copied ? t('team.create.copied') : t('team.create.copy')}</button>
          </div>
        </div>
      {/if}
    </div>
  {/if}

  {#if failure}<p class="error" role="alert">{failure}</p>{/if}

  <div class="foot">
    <button type="button" class="btn ghost" onclick={onback}>{t('team.common.back')}</button>
    <button type="submit" class="btn primary" disabled={!ready}>{busy ? t('team.create.working') : t('team.create.submit')}</button>
  </div>
</form>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 14px;
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
  .hint {
    margin: 0;
    color: var(--dim);
    font-size: 12px;
    line-height: 1.45;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--soft);
    font-size: 13px;
  }
  .escrow {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .secret {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    border-radius: 8px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--text);
    font-size: 12.5px;
  }
  .secret p {
    margin: 0;
  }
  .secret code {
    word-break: break-all;
    font-size: 12px;
  }
  .error {
    margin: 0;
    color: var(--del);
    font-size: 13px;
  }
  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 4px;
  }
</style>
