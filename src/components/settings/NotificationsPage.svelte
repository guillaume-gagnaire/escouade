<script lang="ts">
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { IS_MAC } from '../../lib/platform';
  import { settingsForm } from '../../lib/settings.svelte';
  import Icon from '../Icon.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';

  // The page « Notifications » (PROPOSITION.md §2, as the 2.0 tab): what alerts the user, the pull requests and the
  // deployments, the channels, the sound.
  const s = $derived(settingsForm.settings);
</script>

<Group
  title={t('settings.notifications.notifyFor')}
  note={IS_MAC ? t('settings.notifications.noteMac') : t('settings.notifications.noteOther')}
>
  <!-- The types, ticked (`.checks` of the 2.0 design, `.w-tick` of the windows). -->
  <div class="checks">
    {#each ['questions', 'done', 'errors', 'tickets'] as const as k (k)}
      <label class="w-tick check-row">
        <input type="checkbox" bind:checked={s.notifyFor[k]} />
        <span class="tick-box" aria-hidden="true"><Icon name="check" size={12} /></span>
        <span>{t(`settings.notifications.${k}`)}</span>
      </label>
    {/each}
  </div>
</Group>
<Group title={t('settings.notifications.hosts')} note={t('settings.notifications.hostsNote')}>
  <div class="checks">
    {#each ['prCiFailed', 'prChanges', 'prApproved', 'prMerged', 'prConflict', 'prReview', 'deployOk', 'deployFailed'] as const as k (k)}
      <label class="w-tick check-row">
        <input type="checkbox" bind:checked={s.notifyFor[k]} />
        <span class="tick-box" aria-hidden="true"><Icon name="check" size={12} /></span>
        <span>{t(`settings.notifications.${k}`)}</span>
      </label>
    {/each}
  </div>
</Group>
<Group title={t('settings.notifications.channels')}>
  <Row label={t('settings.notifications.system', { os: IS_MAC ? 'macOS' : 'Windows' })}>
    <Switch label={t('settings.notifications.systemLabel')} bind:on={s.osNotifications} />
  </Row>
</Group>
<Group title={t('settings.notifications.sound')}>
  <Row label={t('settings.notifications.soundOn')} desc={t('settings.notifications.soundOnDesc')}>
    <Switch label={t('settings.notifications.soundOn')} bind:on={s.sound} />
  </Row>
  <Row label={t('settings.notifications.soundTest')}>
    <button class="btn" onclick={() => api.playChime()}><Icon name="play" size={14} />{t('settings.notifications.soundTestButton')}</button>
  </Row>
</Group>

<style>
  .checks {
    display: grid;
    gap: 2px;
    margin: 8px 0 14px;
  }
  .check-row {
    height: 32px;
    margin: 0 -8px;
    padding: 0 8px;
    border-radius: 7px;
    color: var(--text);
  }
  .check-row > input {
    left: 8px;
  }
  .check-row:hover {
    background: var(--hover);
  }
</style>
