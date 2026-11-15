<script lang="ts">
  // The alarm of a team (TS-3, §3.7): the server showed two versions of the team, went back, or rewrote what was read.
  // The device stopped reading and sends nothing; the user compares the head's six words with an administrator, tries
  // again (a reading that passes lifts it), or removes the team from this device.
  import { t } from '../../lib/i18n';
  import type { TeamView } from '../../lib/teams.svelte';
  import { tip } from '../../lib/tip';
  import Icon from '../Icon.svelte';
  import SixWords from './SixWords.svelte';

  let { team, busy = false, onretry, onforget }: { team: TeamView; busy?: boolean; onretry: () => void; onforget: () => void } = $props();

  const alarm = $derived(team.alarm);
  const why = $derived(
    alarm?.kind === 'rollback'
      ? t('team.alarm.rollback')
      : alarm?.kind === 'rewritten'
        ? t('team.alarm.rewritten')
        : t('team.alarm.fork', { epoch: alarm?.epoch ?? '?' }),
  );
</script>

{#if alarm}
  <div class="alarm" role="alert">
    <div class="head"><Icon name="shield" size={15} /><strong>{t('team.alarm.title')}</strong></div>
    <p>{why}</p>
    <p>{t('team.alarm.blocked')}</p>
    {#if team.head.length}
      <SixWords words={team.head} label={t('team.card.head')} hint={t('team.alarm.compare')} />
    {/if}
    <div class="acts">
      <button class="btn" disabled={busy} onclick={onretry} use:tip={{ title: t('team.alarm.retryHint') }}>{t('team.alarm.retry')}</button>
      <button class="btn ghost danger" disabled={busy} onclick={onforget}>{t('team.alarm.forget')}</button>
    </div>
  </div>
{/if}

<style>
  .alarm {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px 16px;
    border-radius: 10px;
    background: oklch(from var(--del) l c h / 0.08);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.4);
    color: var(--soft);
    font-size: 13px;
    line-height: 1.5;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--del);
  }
  .head strong {
    color: var(--text);
    font-weight: 620;
  }
  p {
    margin: 0;
  }
  .acts {
    display: flex;
    gap: 8px;
  }
</style>
