<script lang="ts">
  // The band of a team's settings (TS-13, §6.5 point 2): « L'équipe Acme a changé 2 réglages · Claire Martin, il y a
  // 5 min · Voir », across the window while the member has not seen the version in force, or while a sensitive key or
  // a proposed command waits for this device's consent. « Voir » opens the team's settings, with the change.
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import { bandOf, bandText, teamSettingsApi, type Band } from '../../lib/team-settings';
  import { teams } from '../../lib/teams.svelte';
  import Icon from '../Icon.svelte';

  onMount(() => void teams.load());

  let tick = $state(Date.now());
  onMount(() => {
    const every = setInterval(() => (tick = Date.now()), 30_000);
    return () => clearInterval(every);
  });

  const band = $derived<Band | null>(
    Object.values(teams.settings)
      .map(bandOf)
      .find((b): b is Band => b !== null) ?? null,
  );

  async function view(b: Band) {
    app.modal = { kind: 'team', step: 'settings', teamId: b.team };
    if (b.changes > 0) {
      try {
        await teamSettingsApi.seen(b.team, b.number);
      } catch {
        // The band comes back at the next reading: nothing lost.
      }
    }
  }
</script>

{#if band}
  <div class="team-band" role="status" data-testid="team-settings-band">
    <Icon name="team" size={14} />
    <span class="msg">{bandText(band, tick)}</span>
    <button class="small" onclick={() => view(band)}>{t('team.settings.band.view')}</button>
  </div>
{/if}

<style>
  .team-band {
    display: flex;
    align-items: center;
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
  }
</style>
