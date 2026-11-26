<script lang="ts">
  // The requests of this device's invitations (SR2-08): no one comes in on a link alone. Each request shows six words
  // drawn from the invitee's signed KeyPackage; the person who invites compares them with the invitee's screen, by
  // voice or face to face, then says « Les mots sont les mêmes » (the request is admitted, with the words shown) or
  // « Les mots diffèrent » (refused, the invitation cancelled, the administrators told).
  import { t } from '../../lib/i18n';
  import { errorText, teams, teamsApi, type AdmissionView, type TeamView } from '../../lib/teams.svelte';
  import SixWords from './SixWords.svelte';

  let { team }: { team: TeamView } = $props();

  let busy = $state<string | null>(null);
  let failure = $state('');

  async function act(a: AdmissionView, f: () => Promise<TeamView>) {
    busy = a.invitation;
    failure = '';
    try {
      teams.put(await f());
    } catch (e) {
      failure = errorText(e);
    } finally {
      busy = null;
    }
  }

  const same = (a: AdmissionView) => act(a, () => teamsApi.admit(team.id, a.invitation, a.words));
  const differ = (a: AdmissionView) => act(a, () => teamsApi.refuse(team.id, a.invitation));

  const what = (a: AdmissionView) =>
    a.kind === 'member'
      ? t('team.admit.member', { role: a.role === 'admin' ? t('team.card.admin') : t('team.card.member') })
      : t('team.admit.device');
</script>

{#if team.admissions?.length}
  <div class="admissions" role="group" aria-label={t('team.admit.title')}>
    <span class="lbl">{t('team.admit.title')}</span>
    <p class="lead">{t('team.admit.lead')}</p>
    <ul>
      {#each team.admissions as a (a.invitation + a.words.join(' '))}
        <li data-testid="team-admission">
          <span class="what">{what(a)}</span>
          <SixWords words={a.words} label={t('team.admit.words')} />
          <div class="acts">
            <button class="btn primary" disabled={busy !== null} onclick={() => same(a)}>{t('team.admit.same')}</button>
            <button class="btn danger" disabled={busy !== null} onclick={() => differ(a)}>{t('team.admit.differ')}</button>
          </div>
          <span class="hint">{t('team.admit.differHint')}</span>
        </li>
      {/each}
    </ul>
    {#if failure}<p class="error" role="alert">{failure}</p>{/if}
  </div>
{/if}

<style>
  .admissions {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .lbl {
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 560;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }
  .lead,
  .hint,
  .error {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
  }
  .lead {
    color: var(--text);
  }
  .hint {
    color: var(--dim);
    font-size: 12px;
  }
  .error {
    color: var(--del);
  }
  ul {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .what {
    color: var(--text);
    font-size: 13px;
    font-weight: 560;
  }
  .acts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
</style>
