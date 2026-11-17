<script lang="ts">
  // The band of the team a repository names (TS-7, `docs/TEAM-SERVER.md` §2.7): its `.escouade.yml` has a key `team`.
  // A device of that team (the same id on the same server) attaches the project by its gesture; any other is offered
  // to ask for an invitation, which only opens the screen of the team's entry with the address filled in. Nothing goes
  // to the file's address before a gesture; its name and address come from the repository, and the band says so.
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import { teamSettingsApi } from '../../lib/team-settings';
  import { hintBand, teamBoardApi } from '../../lib/teamboard';
  import { teams } from '../../lib/teams.svelte';
  import Icon from '../Icon.svelte';

  onMount(() => void teams.load());

  const band = $derived(hintBand(app.project, teams.list));
  let busy = $state(false);

  async function attach() {
    if (!band?.teamId || !app.project) return;
    busy = true;
    try {
      await app.run(teamSettingsApi.setProjectTeam(app.project.id, band.teamId));
    } finally {
      busy = false;
    }
  }

  function invite() {
    if (!band) return;
    app.modal = { kind: 'team', step: 'join', expected: band.hint.host };
  }

  async function dismiss() {
    if (!app.project) return;
    await app.run(teamBoardApi.dismissHint(app.project.id));
  }
</script>

{#if band}
  <div class="hint-band" role="status" data-testid="team-hint-band" data-kind={band.kind}>
    <Icon name="team" size={14} />
    <span class="msg">
      {band.kind === 'invite'
        ? t('teamBoard.hint.invite', { name: band.hint.name, host: band.hint.host })
        : t('teamBoard.hint.attach', { name: band.hint.name })}
      <span class="from">{t('teamBoard.hint.fromRepo')}</span>
    </span>
    {#if band.kind === 'invite'}
      <button class="btn small" onclick={invite}>{t('teamBoard.hint.inviteAction')}</button>
    {:else}
      <button class="btn small" disabled={busy} onclick={attach}>{t('teamBoard.hint.attachAction')}</button>
    {/if}
    <button class="btn ghost small" onclick={dismiss}>{t('teamBoard.hint.dismiss')}</button>
  </div>
{/if}

<style>
  .hint-band {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    margin-bottom: 6px;
    padding: 6px 12px;
    border-radius: 8px;
    font-size: 12.5px;
    color: var(--p-ink);
    background: var(--p-soft);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--p-ink) 25%, transparent);
  }
  .msg {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .from {
    display: block;
    color: var(--muted);
    font-size: 11.5px;
  }
</style>
