<script lang="ts">
  // The band of the demonstration project (ESC-505, docs/ONBOARDING.md § 4.4, choice O3): at the top of the project, it
  // says what it is and where it lives, and « Supprimer » removes it, its agents and its folder, after a confirmation.
  // Never removed by itself: only this gesture does it.
  import { samePath } from '../../lib/first-project';
  import { tildify } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { onboarding } from '../../lib/onboarding.svelte';
  import { app } from '../../lib/state.svelte';
  import Icon from '../Icon.svelte';

  const demo = $derived(onboarding.view?.demo ?? '');
  // Not over the first launch itself: the band belongs to the project's window.
  const project = $derived(!onboarding.shown && app.project && samePath(app.project.path, demo) ? app.project : null);

  function remove() {
    const p = project;
    if (!p) return;
    app.modal = {
      kind: 'confirm',
      title: t('onboarding.demo.confirmTitle'),
      body: t('onboarding.demo.confirmBody', { path: tildify(p.path) }),
      confirm: t('onboarding.demo.remove'),
      danger: true,
      icon: 'trash',
      onConfirm: async () => {
        app.modal = null;
        const gone = await app.run(api.demoRemove(p.id).then(() => true));
        if (!gone) return;
        app.forgetProject(p.id);
        if (onboarding.project === p.id) onboarding.project = null;
        app.toast(t('onboarding.demo.removed'));
      },
    };
  }
</script>

{#if project}
  <div class="demo-band" role="status" data-testid="demo-band">
    <Icon name="sparkle" size={14} />
    <span class="msg"><b>{t('onboarding.demo.label')}</b> · {t('onboarding.demo.text', { path: tildify(project.path) })}</span>
    <button class="btn ghost small" onclick={remove}><Icon name="trash" size={13} />{t('onboarding.demo.remove')}</button>
  </div>
{/if}

<style>
  .demo-band {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    margin-bottom: 6px;
    padding: 6px 12px;
    border-radius: 8px;
    font-size: 12.5px;
    color: var(--teal-ink);
    background: color-mix(in oklch, var(--brand) 9%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--brand) 35%, transparent);
  }
  .msg {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
    color: var(--soft);
  }
  .msg b {
    color: var(--teal-ink);
    font-weight: 620;
  }
</style>
