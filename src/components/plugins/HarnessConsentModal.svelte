<script lang="ts">
  import { onMount } from 'svelte';
  import { locale, t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { asksOf, pluginsApi, type PluginView } from '../../lib/plugins';
  import { engineName } from '../../lib/engines';
  import { app } from '../../lib/state.svelte';
  import { errorText } from '../../lib/refusal';
  import type { EngineId } from '../../lib/types';
  import Modal from '../modals/Modal.svelte';
  import Icon from '../Icon.svelte';
  import AskList from './AskList.svelte';

  // The agreement to a harness in a project (ESC-404), as the install window asks it: a private project has its plugins
  // off by default, and a harness's plugin gets all its agent does. Asked once, before the first agent of the harness in a
  // private project, and from the card of an agent or a ticket whose harness is off for its project. « Activer » turns the
  // plugin on for this project alone, then what was asked goes on; « Annuler », nothing goes.
  let {
    projectId,
    engine,
    onEnabled,
    onCancel,
  }: {
    projectId: string;
    engine: EngineId;
    onEnabled: () => void | Promise<void>;
    onCancel?: () => void;
  } = $props();

  const lang = $derived(locale.ui);
  const project = $derived(app.projects.find((p) => p.id === projectId));
  const name = $derived(engineName(engine));
  const asksFirst = $derived(!!project?.private && !app.harnessScope[projectId]?.off.includes(engine));
  let plugin = $state<PluginView | null>(null);
  let loading = $state(true);
  let busy = $state(false);
  let error = $state<string | null>(null);
  const asks = $derived(plugin ? asksOf(plugin.asks, plugin.manifest, lang) : []);

  onMount(async () => {
    try {
      const id = await api.harnessPluginOf(engine);
      if (id) plugin = (await pluginsApi.list()).find((p) => p.id === id) ?? null;
    } catch {
      // Its permissions unread: the question still says what it gets.
    }
    loading = false;
  });

  async function enable() {
    busy = true;
    error = null;
    // Read before closing: the props are the modal's, gone once it is closed.
    const then = onEnabled;
    try {
      await api.harnessEnableIn(projectId, engine);
      await app.refreshEngines();
      app.modal = null;
      await then();
    } catch (e) {
      error = errorText(e);
    }
    busy = false;
  }

  function cancel() {
    const back = onCancel;
    app.modal = null;
    back?.();
  }
</script>

<Modal
  title={asksFirst ? t('plugins.harness.askTitle', { name }) : t('plugins.harness.offTitle', { name, project: project?.name ?? '' })}
  width={540}
  onclose={cancel}
>
  {#snippet lead()}<Icon name="puzzle" />{/snippet}
  <div class="body">
    <p class="p">
      {asksFirst
        ? t('plugins.harness.askBody', { name, project: project?.name ?? '', plugin: plugin?.name ?? name })
        : t('plugins.harness.offBody', { name, project: project?.name ?? '', plugin: plugin?.name ?? name })}
    </p>
    <section>
      <h4>{t('plugins.harness.gets')}</h4>
      <p class="p">{t('plugins.harness.getsWhat')}</p>
    </section>
    <section>
      <h4>{t('plugins.harness.perms')}</h4>
      {#if loading}
        <p class="hint">{t('plugins.harness.loading')}</p>
      {:else if asks.length}
        <AskList {asks} />
      {:else}
        <p class="hint">{t('plugins.perms.none')}</p>
      {/if}
    </section>
    <p class="hint">{t('plugins.harness.after')}</p>
    {#if error}<p class="err" role="alert">{error}</p>{/if}
  </div>
  {#snippet footer()}
    <button class="btn" onclick={cancel}>{t('common.cancel')}</button>
    <button class="btn primary" disabled={busy} onclick={enable}>{t('plugins.harness.enable')}</button>
  {/snippet}
</Modal>

<style>
  .body {
    display: grid;
    gap: 14px;
  }
  .p {
    margin: 0;
    color: var(--soft);
    line-height: 1.55;
  }
  h4 {
    margin: 0 0 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
  .hint {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .err {
    margin: 0;
    color: var(--danger-ink, var(--danger));
    font-size: 12px;
  }
</style>
