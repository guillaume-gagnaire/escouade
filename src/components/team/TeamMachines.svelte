<script lang="ts">
  // « Machines de l'équipe » (OE-1): the machines of the team, ranked apart from its people. Tied to the team, neither a
  // member nor an administrator; what they can and cannot sign said; an administrator enrolls one (a short code typed on
  // the machine) and removes one as a device is revoked. No tool of the MCP server gives or takes the role.
  import { t } from '../../lib/i18n';
  import { errorText, teams, teamsApi, type MachineView, type TeamView } from '../../lib/teams.svelte';
  import Icon from '../Icon.svelte';

  let { team, onenroll }: { team: TeamView; onenroll: () => void } = $props();

  const machines = $derived(team.machines ?? []);
  const admin = $derived(team.role === 'admin');
  let confirming = $state<string | null>(null);
  let busy = $state(false);
  let failure = $state('');

  const short = (m: MachineView) => m.id.slice(0, 8);

  async function remove(m: MachineView) {
    busy = true;
    failure = '';
    try {
      teams.put(await teamsApi.removeMachine(team.id, m.id));
      confirming = null;
    } catch (e) {
      failure = errorText(e);
    } finally {
      busy = false;
    }
  }
</script>

{#if machines.length || admin}
  <div class="machines" role="group" aria-label={t('team.machines.title')} data-testid="team-machines">
    <span class="lbl">{t('team.machines.title')}</span>
    <p class="lead">{t('team.machines.lead')}</p>
    {#if machines.length}
      <ul>
        {#each machines as m (m.id)}
          <li data-testid="team-machine">
            <Icon name="server" size={13} />
            <span class="what mono">{t('team.machines.name', { id: short(m) })}</span>
            <span class="st">{t('team.machines.devices', { count: m.devices.length })}</span>
            {#if m.me}<span class="me">{t('team.machines.me')}</span>{/if}
            {#if admin}
              <button class="btn ghost" disabled={busy} onclick={() => (confirming = m.id)}>{t('team.machines.remove')}</button>
            {/if}
          </li>
          {#if confirming === m.id}
            <li class="confirm" role="alertdialog" aria-label={t('team.machines.remove')}>
              <p>{t('team.machines.removeConfirm', { id: short(m) })}</p>
              <div class="acts">
                <button class="btn ghost" onclick={() => (confirming = null)}>{t('common.cancel')}</button>
                <button class="btn danger" disabled={busy} onclick={() => remove(m)}>{t('team.machines.remove')}</button>
              </div>
            </li>
          {/if}
        {/each}
      </ul>
    {:else}
      <p class="none">{t('team.machines.none')}</p>
    {/if}
    <p class="can">{t('team.machines.can')}</p>
    <p class="cannot">{t('team.machines.cannot')}</p>
    {#if admin}
      <div><button class="btn" onclick={onenroll}>{t('team.machines.enroll')}</button></div>
    {/if}
    {#if failure}<p class="error" role="alert">{failure}</p>{/if}
  </div>
{/if}

<style>
  .machines {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 12px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    border-radius: 8px;
  }
  .lbl {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--muted);
  }
  .lead,
  .none,
  .can,
  .cannot {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--muted);
    max-width: 68ch;
  }
  .cannot {
    color: var(--text);
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    font-size: 13px;
  }
  .st,
  .me {
    font-size: 12px;
    color: var(--muted);
  }
  .me {
    padding: 0 6px;
    border-radius: 4px;
    background: var(--hover);
  }
  li .btn {
    margin-left: auto;
  }
  .confirm {
    flex-direction: column;
    align-items: flex-start;
    padding: 8px;
    border-radius: 6px;
    background: var(--hover);
  }
  .confirm p {
    margin: 0;
    font-size: 12px;
  }
  .acts {
    display: flex;
    gap: 6px;
  }
  .error {
    margin: 0;
    font-size: 12px;
    color: var(--del);
  }
</style>
