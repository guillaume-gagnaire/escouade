<script lang="ts">
  import { t } from '../../lib/i18n';
  import { settingsForm } from '../../lib/settings.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';

  // What a project added starts with: the app's settings, under the project's own (also with no project open).
  const s = $derived(settingsForm.settings);
</script>

<Group title={t('settings.newProjects.title')}>
  <Row label={t('settings.newProjects.private')} desc={t('settings.newProjects.privateDesc')} row="privateProjects">
    <Switch label={t('settings.newProjects.private')} bind:on={s.privateProjects} />
  </Row>
</Group>
