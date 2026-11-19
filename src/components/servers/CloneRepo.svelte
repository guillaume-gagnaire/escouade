<script lang="ts">
  // « Cloner un dépôt » (ESC-538, docs/SERVER-UX.md § 4): the usual way of a project on a server. The address (https or
  // ssh), the folder of the server it goes in (the browser's), the folder's name (the one git would make); the server
  // clones with its own keys and git credentials (`clone_repo`), then the assistant goes on with the folder made.
  import { connection } from '../../lib/connection.svelte';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { cloneFolder, joinServer } from '../../lib/server-browse';
  import { tip } from '../../lib/tip';
  import Icon from '../Icon.svelte';

  let { parent, oncloned }: { parent: string; oncloned: (path: string) => void } = $props();

  let url = $state('');
  let folder = $state('');
  let folderTouched = false;
  let busy = $state(false);
  let error = $state('');
  let done = $state('');
  const name = $derived(connection.server?.name ?? '');

  $effect(() => {
    const u = url;
    if (!folderTouched) folder = cloneFolder(u);
  });

  async function go(e: SubmitEvent) {
    e.preventDefault();
    if (!url.trim() || !parent || busy) return;
    busy = true;
    error = '';
    try {
      const path = await api.cloneRepo(url.trim(), parent, folder.trim());
      done = path;
      oncloned(path);
    } catch (err) {
      error = String(err);
    } finally {
      busy = false;
    }
  }
</script>

<form class="clone" onsubmit={go} data-testid="clone-repo">
  <label class="w-fld">
    <span class="lab">{t('moves.clone.url')}</span>
    <input class="field mono" bind:value={url} placeholder={t('moves.clone.urlPlaceholder')} spellcheck="false" autocomplete="off" />
  </label>
  <div class="row">
    <div class="w-fld into">
      <span class="lab">{t('moves.clone.into')}</span>
      <span class="field mono ro" use:tip={{ text: parent, cut: true }}
        >{parent ? joinServer(parent, folder || '…') : t('moves.browse.loading')}</span
      >
    </div>
    <label class="w-fld name">
      <span class="lab">{t('moves.clone.folder')}</span>
      <input class="field mono" bind:value={folder} oninput={() => (folderTouched = true)} spellcheck="false" />
    </label>
  </div>
  <div class="act">
    <span class="note"><Icon name="info" size={12} />{t('moves.clone.note')}</span>
    <button class="btn primary" type="submit" disabled={busy || !url.trim() || !parent}
      ><Icon name="download" size={14} />{busy ? t('moves.clone.busy', { name }) : t('moves.clone.go', { name })}</button
    >
  </div>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if done}<p class="ok" role="status"><Icon name="check" size={12} />{t('moves.clone.done', { path: done })}</p>{/if}
</form>

<style>
  .clone {
    display: grid;
    gap: 10px;
  }
  .row {
    display: grid;
    grid-template-columns: minmax(0, 2fr) minmax(0, 1fr);
    gap: 10px;
  }
  .ro {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
  }
  .act {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .note {
    flex: 1;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    color: var(--muted);
    font-size: 11.5px;
  }
  .err,
  .ok {
    display: flex;
    align-items: center;
    gap: 5px;
    margin: 0;
    font-size: 12px;
  }
  .err {
    color: var(--del);
  }
  .ok {
    color: var(--run);
  }
</style>
