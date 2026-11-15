<script lang="ts">
  // The teams of this device and what can be done (TS-3): create, join by a link, add this device by a code, recover;
  // and for each team: its role, its six words, the escrow key when it has one (always shown), the advice of a second
  // administrator, its invitations, the alarm of a fork.
  import { t } from '../../lib/i18n';
  import { fDateTime } from '../../lib/format';
  import { isOpen } from '../../lib/team-words';
  import { errorText, teams, teamsApi, type InvitationView, type TeamView } from '../../lib/teams.svelte';
  import Icon from '../Icon.svelte';
  import ForkAlarm from './ForkAlarm.svelte';
  import SixWords from './SixWords.svelte';
  import type { TeamStep } from './TeamModal.svelte';

  let { onstep, onwords }: { onstep: (step: TeamStep, team?: string) => void; onwords: (team: string, words: string[]) => void } = $props();

  let busy = $state<string | null>(null);
  let failure = $state<Record<string, string>>({});
  /** The team whose « Retirer de cet appareil » waits for its confirmation. */
  let forgetting = $state<string | null>(null);

  async function act(team: TeamView, f: () => Promise<void>) {
    busy = team.id;
    failure = { ...failure, [team.id]: '' };
    try {
      await f();
    } catch (e) {
      failure = { ...failure, [team.id]: errorText(e) };
    } finally {
      busy = null;
    }
  }

  const sync = (team: TeamView) => act(team, async () => teams.put(await teamsApi.sync(team.id)));
  const forget = (team: TeamView) =>
    act(team, async () => {
      await teamsApi.forget(team.id);
      forgetting = null;
      await teams.load();
    });
  const newRecovery = (team: TeamView) => act(team, async () => onwords(team.id, await teamsApi.newRecovery(team.id)));

  const roleLabel = (team: TeamView) =>
    !team.joined
      ? t('team.card.waiting')
      : team.role === 'admin'
        ? t('team.card.admin')
        : team.role === 'recovery'
          ? t('team.card.recovery')
          : t('team.card.member');

  function invitationState(i: InvitationView): string {
    if (i.used) return t('team.card.used');
    if (!isOpen(i.expiresAt)) return t('team.card.expired');
    if (!i.ready) return t('team.card.notReady');
    return t('team.card.open', { left: fDateTime(i.expiresAt * 1000) });
  }

  const entries: { step: TeamStep; icon: 'plus' | 'link' | 'phone' | 'lock'; title: string; detail: string }[] = $derived([
    { step: 'create', icon: 'plus', title: t('team.home.create'), detail: t('team.home.createDetail') },
    { step: 'join', icon: 'link', title: t('team.home.join'), detail: t('team.home.joinDetail') },
    { step: 'pairJoin', icon: 'phone', title: t('team.home.pairJoin'), detail: t('team.home.pairJoinDetail') },
    { step: 'recover', icon: 'lock', title: t('team.home.recover'), detail: t('team.home.recoverDetail') },
  ]);
</script>

<p class="lead">{t('team.home.lead')}</p>

{#if teams.loaded && teams.list.length === 0}
  <p class="none">{t('team.home.none')}</p>
{/if}

{#each teams.list as team (team.id)}
  <section class="team" aria-label={team.name}>
    <header>
      <Icon name="team" size={16} />
      <h3>{team.name}</h3>
      <span class="server mono">{team.server}</span>
      <span class="role" class:waiting={!team.joined}>{roleLabel(team)}</span>
    </header>

    {#if team.alarm}
      <ForkAlarm {team} busy={busy === team.id} onretry={() => sync(team)} onforget={() => (forgetting = team.id)} />
    {:else if !team.joined}
      <p class="note">{t('team.card.waitingLead')}</p>
    {/if}

    {#if team.escrow}
      <p class="escrow" role="note"><Icon name="shield" size={13} />{t('team.card.escrow', { holder: team.escrow })}</p>
    {/if}

    {#if team.joined && !team.alarm}
      <div class="facts">
        <span>{t('team.card.members', { count: team.members })}</span>
      </div>
      <SixWords words={team.fingerprint} label={t('team.card.fingerprint')} hint={t('team.card.fingerprintHint')} />
      {#if team.role === 'admin' && team.admins < 2}
        <p class="advice">{t('team.card.secondAdmin')}</p>
      {/if}
      {#if team.keysInFile}
        <p class="advice">{t('team.card.keysInFile')}</p>
      {/if}
      {#if team.role === 'admin' && !team.recoveryConfirmed}
        <p class="advice">{t('team.card.wordsLost')}</p>
      {/if}
      {#if team.invitations.length}
        <div class="invs">
          <span class="lbl">{t('team.card.invitations')}</span>
          <ul>
            {#each team.invitations.slice(0, 5) as i (i.id)}
              <li>
                <span
                  >{i.kind === 'member'
                    ? t('team.card.invitationMember', { role: i.role === 'admin' ? t('team.card.admin') : t('team.card.member') })
                    : t('team.card.invitationDevice')}</span
                >
                <span class="st">{invitationState(i)}</span>
                {#if i.reusedAt}<span class="reused" role="alert">{t('team.card.reused', { date: fDateTime(i.reusedAt * 1000) })}</span
                  >{/if}
              </li>
            {/each}
          </ul>
        </div>
      {/if}
      <div class="acts">
        {#if team.role === 'admin' || team.role === 'recovery'}
          <button class="btn" onclick={() => onstep('invite', team.id)}>{t('team.card.invite')}</button>
        {/if}
        <button class="btn" onclick={() => onstep('device', team.id)}>{t('team.card.device')}</button>
        {#if team.role === 'admin'}
          <button class="btn ghost" disabled={busy === team.id} onclick={() => newRecovery(team)}>{t('team.card.newRecovery')}</button>
        {/if}
        <button class="btn ghost" disabled={busy === team.id} onclick={() => sync(team)}>{t('team.card.sync')}</button>
        <button class="btn ghost" onclick={() => (forgetting = team.id)}>{t('team.card.forget')}</button>
      </div>
    {:else if !team.alarm}
      <SixWords words={team.fingerprint} label={t('team.card.fingerprint')} />
      <div class="acts">
        <button class="btn ghost" disabled={busy === team.id} onclick={() => sync(team)}>{t('team.card.sync')}</button>
        <button class="btn ghost" onclick={() => (forgetting = team.id)}>{t('team.card.forget')}</button>
      </div>
    {/if}

    {#if forgetting === team.id}
      <div class="confirm" role="alertdialog" aria-label={t('team.card.forget')}>
        <p>{t('team.card.forgetConfirm', { name: team.name })}</p>
        <div class="acts">
          <button class="btn ghost" onclick={() => (forgetting = null)}>{t('common.cancel')}</button>
          <button class="btn danger" disabled={busy === team.id} onclick={() => forget(team)}>{t('team.card.forget')}</button>
        </div>
      </div>
    {/if}

    {#if failure[team.id]}<p class="error" role="alert">{failure[team.id]}</p>{/if}
    {#if team.lastError && !team.alarm}<p class="last">{t('team.card.error', { error: team.lastError })}</p>{/if}
  </section>
{/each}

<div class="entries">
  {#each entries as e (e.step)}
    <button class="entry" onclick={() => onstep(e.step)}>
      <Icon name={e.icon} size={15} />
      <span class="tx"><span class="ti">{e.title}</span><span class="de">{e.detail}</span></span>
      <Icon name="right" size={12} />
    </button>
  {/each}
</div>

<style>
  .lead,
  .none,
  .note,
  .advice,
  .escrow,
  .last,
  .error {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
  }
  .lead {
    color: var(--soft);
  }
  .none,
  .note,
  .last {
    color: var(--muted);
  }
  .advice {
    color: var(--warn-ink);
  }
  .error {
    color: var(--del);
  }
  .escrow {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    padding: 9px 12px;
    border-radius: 8px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--text);
  }
  .escrow :global(.icon) {
    flex: none;
    margin-top: 2px;
    color: var(--wait);
  }
  .team {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px 16px;
    border-radius: 12px;
    background: var(--panel);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--soft);
  }
  h3 {
    margin: 0;
    color: var(--text);
    font-size: 14px;
    font-weight: 620;
  }
  .server {
    color: var(--muted);
    font-size: 12px;
  }
  .role {
    margin-left: auto;
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--p-soft);
    color: var(--p-ink);
    font-size: 11.5px;
    font-weight: 600;
  }
  .role.waiting {
    background: var(--wait-bg);
    color: var(--wait);
  }
  .facts {
    color: var(--muted);
    font-size: 12.5px;
  }
  .invs {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .lbl {
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 560;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }
  .invs ul {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 12.5px;
    color: var(--soft);
  }
  .invs li {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
  }
  .st {
    color: var(--muted);
  }
  .reused {
    flex-basis: 100%;
    color: var(--del);
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .confirm {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    border-radius: 8px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line-2);
    font-size: 13px;
    color: var(--soft);
  }
  .confirm p {
    margin: 0;
  }
  .entries {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .entry {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 11px 14px;
    border: 0;
    border-radius: 10px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .entry:hover {
    background: var(--hover);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .entry:focus-visible {
    outline: 2px solid var(--ring);
    outline-offset: 2px;
  }
  .tx {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 2px;
  }
  .ti {
    color: var(--text);
    font-size: 13.5px;
    font-weight: 580;
  }
  .de {
    color: var(--muted);
    font-size: 12px;
  }
</style>
