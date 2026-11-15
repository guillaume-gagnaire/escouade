<script lang="ts">
  // « Inviter dans l'équipe » (TS-3, §3.3, C6-A): a role, a life (72 h by default), one use; the link to send (its
  // secret after the #) and its QR code. An administrator online lets the invitee in.
  import { t } from '../../lib/i18n';
  import { fDateTime } from '../../lib/format';
  import { errorText, teamsApi, type InviteView } from '../../lib/teams.svelte';
  import InviteLink from './InviteLink.svelte';

  let { teamId, onback }: { teamId: string; onback: () => void } = $props();

  let role = $state<'member' | 'admin'>('member');
  let hours = $state(72);
  let invite = $state<InviteView | null>(null);
  let busy = $state(false);
  let failure = $state('');

  async function make() {
    busy = true;
    failure = '';
    try {
      invite = await teamsApi.invite(teamId, role, hours);
    } catch (e) {
      failure = errorText(e);
    } finally {
      busy = false;
    }
  }
</script>

{#if !invite}
  <div class="row">
    <span class="lbl" id="team-role">{t('team.invite.role')}</span>
    <div class="segmented" role="radiogroup" aria-labelledby="team-role">
      <button role="radio" aria-checked={role === 'member'} class:on={role === 'member'} onclick={() => (role = 'member')}
        >{t('team.invite.member')}</button
      >
      <button role="radio" aria-checked={role === 'admin'} class:on={role === 'admin'} onclick={() => (role = 'admin')}
        >{t('team.invite.admin')}</button
      >
    </div>
  </div>
  <div class="row">
    <span class="lbl" id="team-life">{t('team.invite.expires')}</span>
    <div class="segmented" role="radiogroup" aria-labelledby="team-life">
      {#each [24, 72, 168] as h (h)}
        <button role="radio" aria-checked={hours === h} class:on={hours === h} onclick={() => (hours = h)}
          >{h === 24 ? t('team.invite.hours24') : h === 72 ? t('team.invite.hours72') : t('team.invite.days7')}</button
        >
      {/each}
    </div>
  </div>
  <p class="hint">{t('team.invite.once')}</p>
  {#if failure}<p class="error" role="alert">{failure}</p>{/if}
  <div class="foot">
    <button class="btn ghost" onclick={onback}>{t('team.common.back')}</button>
    <button class="btn primary" disabled={busy} onclick={make}>{t('team.invite.make')}</button>
  </div>
{:else}
  <InviteLink {invite} label={t('team.invite.link')} qrLabel={t('team.invite.qr')} />
  <p class="hint">{t('team.invite.left', { date: fDateTime(invite.expiresAt * 1000) })}</p>
  <p class="state" class:ready={invite.ready}>{invite.ready ? t('team.invite.ready') : t('team.invite.notReady')}</p>
  <div class="foot">
    <button class="btn ghost" onclick={() => (invite = null)}>{t('team.invite.another')}</button>
    <button class="btn primary" onclick={onback}>{t('team.common.close')}</button>
  </div>
{/if}

<style>
  .row {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .lbl {
    color: var(--soft);
    font-size: 12.5px;
    font-weight: 560;
  }
  .segmented {
    align-self: flex-start;
  }
  .hint,
  .error,
  .state {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
  }
  .hint {
    color: var(--dim);
  }
  .error {
    color: var(--del);
  }
  .state {
    color: var(--wait);
  }
  .state.ready {
    color: var(--add);
  }
  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
