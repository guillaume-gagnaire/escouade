<script lang="ts">
  // The screen of C15 (TS-16): the team went « par membre », and this member is told so at its next opening, before
  // its totals carry its name. « J'ai compris » lets them go named; « Voir ce qui remonte » opens « Équipe › Coûts ».
  import { t } from '../../lib/i18n';
  import { teamCostsApi } from '../../lib/team-costs';
  import { errorText } from '../../lib/teams.svelte';

  let { teamId, name, ondone, onsee }: { teamId: string; name: string; ondone: () => void; onsee: () => void } = $props();

  let failure = $state('');

  async function ack(then: () => void) {
    try {
      await teamCostsApi.ack(teamId);
      then();
    } catch (e) {
      failure = errorText(e);
    }
  }
</script>

<div class="told" data-testid="team-costs-told">
  <p>{t('teamCosts.told.body', { team: name })}</p>
  <p class="strong">{t('teamCosts.told.never')}</p>
  <p class="muted">{t('teamCosts.told.where')}</p>
  {#if failure}<p class="error" role="alert">{failure}</p>{/if}
  <div class="acts">
    <button class="btn ghost" onclick={() => ack(onsee)}>{t('teamCosts.told.see')}</button>
    <button class="btn primary" onclick={() => ack(ondone)}>{t('teamCosts.told.ok')}</button>
  </div>
</div>

<style>
  .told {
    display: flex;
    flex-direction: column;
    gap: 10px;
    font-size: 13px;
  }
  p {
    margin: 0;
    line-height: 1.5;
  }
  .strong {
    font-weight: 560;
  }
  .muted {
    color: var(--muted);
  }
  .error {
    color: var(--del);
  }
  .acts {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
