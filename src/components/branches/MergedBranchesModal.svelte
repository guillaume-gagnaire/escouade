<script lang="ts">
  import { onMount } from 'svelte';
  import { refusalText } from '../../lib/branch-actions';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import Modal from '../modals/Modal.svelte';

  // « Branches mergées »: the local branches already in the project's base (those of validated tickets, mostly), to
  // delete in one go. Each is deleted as git deletes a merged branch, never forced: one that is not merged any more by
  // now stays, with why.
  let { projectId }: { projectId: string } = $props();

  /** The merged branches; null while they are read. */
  let merged = $state<string[] | null>(null);
  let selected = $state<Record<string, boolean>>({});
  /** Why the list could not be read. */
  let failure = $state<string | null>(null);
  /** The branches that could not be deleted, each with why. */
  let left = $state<string[]>([]);
  let busy = $state(false);
  let gone = false;

  const chosen = $derived((merged ?? []).filter((n) => selected[n]));

  async function load() {
    try {
      const names = await api.branchesMerged(projectId);
      if (gone) return;
      merged = names;
      selected = Object.fromEntries(names.map((n) => [n, true]));
      failure = null;
    } catch (e) {
      if (!gone) failure = String(e);
    }
  }
  onMount(() => {
    void load();
    return () => {
      gone = true;
    };
  });

  function close() {
    app.modal = null;
  }

  async function remove() {
    if (busy || !chosen.length) return;
    busy = true;
    left = [];
    let deleted = 0;
    for (const branch of chosen) {
      try {
        await api.branchDelete(projectId, branch, false, false);
        deleted++;
      } catch (e) {
        left.push(t('branches.merged.failed', { branch, reason: refusalText(e, branch) }));
      }
    }
    if (deleted) app.toast(t('branches.merged.done', { count: deleted }), 'ok');
    if (!left.length) {
      if (!gone) close();
    } else if (!gone) {
      await load();
    }
    busy = false;
  }
</script>

<Modal title={t('branches.merged.title')} width={460} onclose={close}>
  {#if failure}
    <p class="error" role="alert">{failure}</p>
  {:else if !merged}
    <p class="note">{t('common.loading')}</p>
  {:else if !merged.length}
    <p class="note">{t('branches.merged.empty')}</p>
  {:else}
    <p class="note">{t('branches.merged.intro')}</p>
    <div class="list" role="group" aria-label={t('branches.merged.list')}>
      {#each merged as name (name)}
        <label class="item">
          <input type="checkbox" bind:checked={selected[name]} disabled={busy} />
          <span class="name">{name}</span>
        </label>
      {/each}
    </div>
  {/if}
  {#if left.length}
    <div class="error" role="alert">
      {#each left as line (line)}<p>{line}</p>{/each}
    </div>
  {/if}
  {#snippet footer()}
    {#if merged?.length}
      <button class="btn ghost" disabled={busy} onclick={close}>{t('common.cancel')}</button>
      <button class="btn danger" disabled={busy || !chosen.length} onclick={remove}
        >{busy ? t('branches.merged.removing') : t('branches.merged.remove', { count: chosen.length })}</button
      >
    {:else}
      <button class="btn ghost" onclick={close}>{t('common.close')}</button>
    {/if}
  {/snippet}
</Modal>

<style>
  .note {
    margin: 0;
    color: var(--muted);
    line-height: 1.55;
  }
  .list {
    display: flex;
    flex-direction: column;
    max-height: 280px;
    overflow: auto;
    padding: 4px;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--bg);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 5px 8px;
    border-radius: var(--r-sm);
    cursor: pointer;
  }
  .item input {
    accent-color: var(--accent);
  }
  .item:hover {
    background: var(--elev2);
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: var(--mono);
    font-size: 12.5px;
  }
  .error {
    margin: 0;
    font-size: 12.5px;
    color: var(--del);
    overflow-wrap: anywhere;
  }
  .error p {
    margin: 0 0 4px;
  }
</style>
