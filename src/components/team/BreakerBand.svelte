<script lang="ts">
  // The band of a team's circuit breaker (TS-15, docs/TEAM-SERVER.md §9): « Arrêt demandé par Claire Martin : fuite
  // de clé chez le fournisseur X », with what is taken out and its scope, across the window while it holds; or, under
  // the policy « sans contact depuis N heures » of this device, that no new agent of the team starts, with the hour
  // of the last contact. « Voir » opens the team's circuit breakers.
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { engineName } from '../../lib/engines';
  import { locks } from '../../lib/locks.svelte';
  import { app } from '../../lib/state.svelte';
  import { bandOf, type Band } from '../../lib/team-breakers';
  import { teams } from '../../lib/teams.svelte';
  import Icon from '../Icon.svelte';

  onMount(() => void teams.load());

  let tick = $state(Date.now());
  onMount(() => {
    const every = setInterval(() => (tick = Date.now()), 60_000);
    return () => clearInterval(every);
  });

  /** The policy « sans contact depuis N heures » of this device (`team.maxOfflineHours`), when it is set. */
  const hours = $derived.by(() => {
    const v = locks.current.locks['team.maxOfflineHours']?.value;
    return typeof v === 'number' && v > 0 ? v : null;
  });

  const band = $derived<Band | null>(bandOf(Object.values(teams.breakers), hours, tick, (e) => engineName(e) || e));
</script>

{#if band}
  <div class="breaker-band" class:contact={band.kind === 'contact'} role="status" data-testid="breaker-band">
    <Icon name="shield" size={14} />
    <span class="msg">
      <strong>{band.text}</strong>
      <span class="detail">{band.detail}</span>
    </span>
    <button class="small" onclick={() => (app.modal = { kind: 'team', step: 'breakers', teamId: band.team })}
      >{t('teamBreakers.band.view')}</button
    >
  </div>
{/if}

<style>
  .breaker-band {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 6px;
    padding: 6px 12px;
    border-radius: 8px;
    font-size: 12.5px;
    color: var(--del);
    background: color-mix(in oklch, var(--del) 11%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--del) 35%, transparent);
  }
  .breaker-band.contact {
    color: var(--warn-ink);
    background: color-mix(in oklch, var(--warn-ink) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--warn-ink) 35%, transparent);
  }
  .msg {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 2px 10px;
    align-items: baseline;
  }
  strong {
    font-weight: 600;
  }
  .detail {
    color: var(--soft);
  }
</style>
