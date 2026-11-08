<script lang="ts">
  import { flushDrafts } from '../../lib/drafts';
  import { buffers } from '../../lib/editor/buffers.svelte';
  import { t } from '../../lib/i18n';
  import { shownVersion } from '../../lib/updater';
  import { api } from '../../lib/ipc';
  import type { SettingsPage } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import Markdown from '../conv/Markdown.svelte';
  import Icon from '../Icon.svelte';
  import Modal from './Modal.svelte';

  // The update downloaded, which a restart installs; or, `installed`, the one installed since the last start: its notes.
  // `back`: the settings' page it was opened from (Réglages › Général), which « Plus tard » goes back to.
  let { version, notes, installed = false, back }: { version: string; notes: string; installed?: boolean; back?: SettingsPage } = $props();

  // The files « Quitter » asks about, which a restart would lose: saved first.
  const unsaved = $derived(buffers.unsaved);
  // A restart stops every agent's process: a turn under way, or a question waiting, is cut.
  const working = $derived(
    Object.values(app.agents).filter((a) => !a.archived && (a.status === 'running' || a.status === 'waiting')).length,
  );
  let restarting = $state(false);

  async function restart() {
    restarting = true;
    // What is being typed in the composers is written before the app stops.
    flushDrafts();
    // Once it is in, the app goes: nothing comes back but a failure.
    await app.run(api.updateRestart());
    restarting = false;
  }

  const close = () => (app.modal = back ? { kind: 'settings', page: back, resume: true } : null);

  /** A notice's sentence, its subject in bold as the preview has it: what comes before its first colon. */
  function subject(text: string): [string, string] {
    const at = text.indexOf(':');
    if (at < 0) return ['', text];
    const head = text.slice(0, at).replace(/s+$/, '');
    return [head, text.slice(head.length)];
  }
</script>

<Modal
  title={installed
    ? t('shell.update.notes', { version: shownVersion(version) })
    : t('shell.update.ready', { version: shownVersion(version) })}
  width={560}
  onclose={close}
>
  {#snippet lead()}<span class="lead"><Icon name="download" /></span>{/snippet}
  {#if !installed && unsaved}
    {@const [b, rest] = subject(t('shell.update.saveFirst', { count: unsaved }))}
    <p class="w-notice warn"><Icon name="info" /><span><b>{b}</b>{rest}</span></p>
  {/if}
  {#if !installed && working}
    {@const [b, rest] = subject(t('shell.update.working', { count: working }))}
    <p class="w-notice"><Icon name="info" /><span><b>{b}</b>{rest}</span></p>
  {/if}
  {#if notes.trim()}
    <div class="notes"><Markdown text={notes} /></div>
  {/if}
  {#snippet footer()}
    {#if installed}
      <button class="btn primary" onclick={close}>{t('common.close')}</button>
    {:else}
      <button class="btn" onclick={close}>{t('common.later')}</button>
      <button class="btn primary" disabled={!!unsaved || restarting} onclick={restart}
        >{restarting ? t('shell.update.restarting') : t('shell.update.restartNow')}</button
      >
    {/if}
  {/snippet}
</Modal>

<style>
  .lead {
    display: inline-flex;
    color: var(--p-ink);
  }
  /* Unsaved files: the notice in amber, the restart closed until they are saved. */
  .warn {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  /* The notes, as the preview's list: soft, their items close together. */
  .notes {
    color: var(--soft);
  }
  .notes :global(.md) {
    color: var(--soft);
    font-size: 13px;
    line-height: 1.45;
  }
  .notes :global(.md li) {
    margin: 0 0 4px;
  }
  .notes :global(.md ul) {
    padding-left: 18px;
  }
</style>
