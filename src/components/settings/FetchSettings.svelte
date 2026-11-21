<script lang="ts">
  import { EVERY_MAX, EVERY_MIN, EVERY_PRESETS, TIMEOUT_MAX, TIMEOUT_MIN, TIMEOUT_PRESETS, fMinutes, fSeconds } from '../../lib/fetch';
  import { t } from '../../lib/i18n';
  import { settingsForm } from '../../lib/settings.svelte';
  import BoundedField from './BoundedField.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';

  // Tous les projets › Git (ESC-561): the background fetch of every project, on or not, its interval and its time
  // limit; each project may say its own in its Git page.
  const s = $derived(settingsForm.settings);
</script>

<Group title={t('fetch.settings.group')} anchor="fetch">
  <Row label={t('fetch.settings.auto')} desc={t('fetch.settings.autoDesc')}>
    <Switch label={t('fetch.settings.auto')} bind:on={s.gitFetch.auto} />
  </Row>
  <Row label={t('fetch.settings.every')} desc={t('fetch.settings.everyDesc')} wide>
    <BoundedField
      id="fetch-every"
      label={t('fetch.settings.every')}
      bind:value={s.gitFetch.everyMin}
      min={EVERY_MIN}
      max={EVERY_MAX}
      presets={EVERY_PRESETS}
      format={fMinutes}
      unit={t('fetch.settings.unitMin')}
    />
  </Row>
  <Row label={t('fetch.settings.timeout')} desc={t('fetch.settings.timeoutDesc')} wide>
    <BoundedField
      id="fetch-timeout"
      label={t('fetch.settings.timeout')}
      bind:value={s.gitFetch.timeoutSecs}
      min={TIMEOUT_MIN}
      max={TIMEOUT_MAX}
      presets={TIMEOUT_PRESETS}
      format={fSeconds}
      unit={t('fetch.settings.unitSec')}
    />
  </Row>
</Group>
