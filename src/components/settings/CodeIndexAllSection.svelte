<script lang="ts">
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { codeIndexFollowers } from '../../lib/codeindex';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';

  // « Index du code sur tous les projets » (ESC-293), under « Projets » with the app's settings that hold for each project
  // (« Nouveaux projets », « Projets fermés »), below the project's own « Index du code » that follows it. Saved at once,
  // not through the draft: turning it off erases indexes, which « Annuler » could not take back. On, the projects that
  // follow it are read one by one by the backend (each section says where it is); off, their index is erased after a
  // confirmation that says how many. The model of the search by meaning is the project section's, never downloaded here.

  const on = $derived(!!app.settings?.codeIndexAll);
  const followers = $derived(codeIndexFollowers(app.projects).length);
  let busy = $state(false);

  async function apply(next: boolean): Promise<boolean> {
    busy = true;
    const count = await app.run(api.setCodeIndexAll(next));
    busy = false;
    if (count == null) return false;
    app.settings.codeIndexAll = next;
    if (next) app.toast(count ? t('codeindex.all.turnedOn', { count }) : t('codeindex.all.turnedOnNone'));
    else app.toast(count ? t('codeindex.all.turnedOff', { count }) : t('codeindex.all.turnedOffNone'));
    return true;
  }

  function toggle(next: boolean) {
    if (busy || next === on) return;
    if (next || !followers) return void apply(next);
    // Taken now: the settings are gone while the confirmation is open, and come back with their draft.
    const projectId = settingsForm.projectId ?? undefined;
    const back = () => {
      app.modal = { kind: 'settings', tab: 'projects', projectId, resume: true };
    };
    app.modal = {
      kind: 'confirm',
      title: t('codeindex.all.confirmTitle', { count: followers }),
      body: t('codeindex.all.confirmBody', { count: followers }),
      confirm: t('codeindex.all.confirm'),
      danger: true,
      onCancel: back,
      onConfirm: async () => {
        await apply(false);
        back();
      },
    };
  }
</script>

<Group title={t('codeindex.all.title')} anchor="code-index-all" note={`${t('codeindex.what')} ${t('codeindex.local')}`}>
  <Row label={t('codeindex.all.on')} desc={t('codeindex.all.onDesc')}>
    {#snippet extra()}
      <p class="more">{t('codeindex.all.fence')}</p>
      {#if app.projects.length}
        <p class="more count" data-code-index-followers>
          {followers ? t('codeindex.all.followers', { count: followers, total: app.projects.length }) : t('codeindex.all.followersNone')}
        </p>
      {/if}
    {/snippet}
    <Switch label={t('codeindex.all.on')} disabled={busy} bind:on={() => on, (next) => toggle(next)} />
  </Row>
</Group>

<style>
  .count {
    font-variant-numeric: tabular-nums;
  }
</style>
