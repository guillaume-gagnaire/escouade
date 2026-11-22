<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import { errorText } from '../../lib/refusal';
  import type { EngineId, LaunchAsk } from '../../lib/types';
  import Modal from '../modals/Modal.svelte';
  import Icon from '../Icon.svelte';

  // The agreement to a harness a plugin of a third party describes (audit SEC-01, ESC-589): it runs a program of this
  // machine out of the sandbox, so its first launch says its whole command (the program found, its arguments, those it
  // resumes a session with and those that read its version, the variables it adds and takes off, those Escouade sets).
  // « Autoriser et lancer » keeps the line by its fingerprint, read again by the core, and starts the agent; « Refuser »
  // keeps nothing, the agent does not start and its conversation says so. A line an update changes is asked again.
  let { engine, agentId }: { engine: EngineId; agentId: string | null } = $props();

  let ask = $state<LaunchAsk | null>(null);
  let loading = $state(true);
  let busy = $state(false);
  let error = $state<string | null>(null);
  const refused = $derived(ask?.held === 'refused');

  onMount(async () => {
    try {
      ask = await api.harnessLaunchAsk(engine, agentId);
    } catch (e) {
      error = errorText(e);
    }
    loading = false;
  });

  async function answer(allow: boolean) {
    if (!ask) return;
    busy = true;
    error = null;
    try {
      await api.harnessAgree(engine, agentId, ask.fingerprint, allow);
      await app.refreshHarnessLaunches();
      app.modal = null;
    } catch (e) {
      error = errorText(e);
      // The line changed meanwhile: read again, to be read before it is agreed to.
      ask = await api.harnessLaunchAsk(engine, agentId).catch(() => ask);
    }
    busy = false;
  }

  /** A word of the line as the shell would show it: quoted when it holds a space. */
  const word = (w: string) => (/[\s"]/.test(w) ? JSON.stringify(w) : w);
</script>

<Modal title={t('plugins.harness.launchTitle', { name: ask?.harness ?? engine })} width={600} onclose={() => (app.modal = null)}>
  {#snippet lead()}<Icon name="term" />{/snippet}
  <div class="body" data-testid="harness-launch">
    {#if loading}
      <p class="hint">{t('plugins.harness.loading')}</p>
    {:else if ask}
      <p class="p">
        {refused
          ? t('plugins.harness.launchRefusedBody')
          : t('plugins.harness.launchBody', { name: ask.harness, plugin: `${ask.pluginName} (${ask.plugin})` })}
      </p>
      <pre class="line" aria-label={t('plugins.harness.program')} data-testid="launch-line">{[ask.program, ...ask.args]
          .map(word)
          .join(' ')}</pre>
      <dl class="facts">
        <dt>{t('plugins.harness.program')}</dt>
        <dd class="mono" data-testid="launch-program">{ask.program}</dd>
        <dt>{t('plugins.harness.args')}</dt>
        <dd class="mono">{ask.args.length ? ask.args.map(word).join(' ') : t('plugins.harness.none')}</dd>
        {#if ask.resumeArgs.length}
          <dt>{t('plugins.harness.resumeArgs')}</dt>
          <dd class="mono">{ask.resumeArgs.map(word).join(' ')}</dd>
        {/if}
        <dt>{t('plugins.harness.versionArgs')}</dt>
        <dd class="mono">{ask.versionArgs.length ? ask.versionArgs.map(word).join(' ') : t('plugins.harness.none')}</dd>
        <dt>{t('plugins.harness.env')}</dt>
        <dd class="mono" data-testid="launch-env">
          {#if ask.env.length}
            {#each ask.env as v (v.name)}<span class="var">{v.name}={word(v.value)}</span>{/each}
          {:else}{t('plugins.harness.none')}{/if}
        </dd>
        <dt>{t('plugins.harness.removeEnv')}</dt>
        <dd class="mono">{ask.removeEnv.length ? ask.removeEnv.join(' ') : t('plugins.harness.none')}</dd>
        <dt>{t('plugins.harness.escouadeEnv')}</dt>
        <dd class="mono">{ask.escouadeEnv.length ? ask.escouadeEnv.join(' ') : t('plugins.harness.none')}</dd>
      </dl>
      {#if !refused}<p class="hint">{t('plugins.harness.launchAfter')}</p>{/if}
    {/if}
    {#if error}<p class="err" role="alert">{error}</p>{/if}
  </div>
  {#snippet footer()}
    {#if refused || !ask}
      <button class="btn" onclick={() => (app.modal = null)}>{t('common.close')}</button>
    {:else}
      <button class="btn" disabled={busy} onclick={() => answer(false)}>{t('plugins.harness.refuse')}</button>
      <button class="btn primary" disabled={busy} onclick={() => answer(true)}>{t('plugins.harness.allow')}</button>
    {/if}
  {/snippet}
</Modal>

<style>
  .body {
    display: grid;
    gap: 12px;
  }
  .p {
    margin: 0;
    color: var(--soft);
    line-height: 1.55;
  }
  .line {
    margin: 0;
    padding: 10px 12px;
    border-radius: 8px;
    background: rgba(var(--fg-rgb), 0.05);
    font-family: var(--mono);
    font-size: 12px;
    white-space: pre-wrap;
    word-break: break-all;
    color: var(--text);
  }
  .facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 14px;
    margin: 0;
    font-size: 12px;
  }
  .facts dt {
    color: var(--muted);
  }
  .facts dd {
    margin: 0;
    color: var(--text);
    word-break: break-all;
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
  }
  .mono {
    font-family: var(--mono);
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
