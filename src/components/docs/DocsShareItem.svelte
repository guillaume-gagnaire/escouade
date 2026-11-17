<script lang="ts">
  import Icon from '../Icon.svelte';
  import Rich from '../../lib/i18n/Rich.svelte';
  import { t } from '../../lib/i18n';
  import { cancelDocsConflict, resolveDocsConflict, shareDocs } from '../../lib/docs-share';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';

  // In the status bar (ESC-258, design/factory/2-specification.png): « Docs sur la branche X · N changements à
  // partager » and « Partager », shown on the Docs tab, and elsewhere while the docs have something to share. The person
  // never sees git: a conflict says which pages, and « L'agent résout » / « Annuler ».
  let { projectId }: { projectId: string } = $props();

  const project = $derived(app.projects.find((p) => p.id === projectId));
  const shared = $derived(app.docsShare[projectId]);
  const pending = $derived(shared?.pending ?? 0);
  const conflict = $derived(shared?.conflict ?? null);
  /** The fixed docs branch, else the branch the project's folder is on. */
  const onBranch = $derived((project?.docsBranch ?? '').trim() || app.git[projectId]?.branch || '');
  let busy = $state(false);

  async function share() {
    busy = true;
    try {
      await shareDocs(projectId);
    } finally {
      busy = false;
    }
  }
</script>

<span class="docs-share" data-docs-share use:tip={{ text: t('docs.share.tip'), aria: false }}>
  {#if app.teamDocsOn[projectId]}
    <!-- On the team server (TS-9): they leave by themselves, no branch to share. -->
    <span class="on">{t('teamDocs.sync.synced')}</span>
  {:else}
    {#if onBranch}<span class="on"><Rich k="docs.share.on">{#snippet branch()}<b class="mono">{onBranch}</b>{/snippet}</Rich></span>{/if}
    {#if conflict}
      <span class="sep" aria-hidden="true">·</span>
      <span class="conflict" role="status">{t('docs.share.conflict', { files: conflict.files.join(', ') })}</span>
      {#if conflict.agentId}
        <span class="d">{t('docs.share.resolving')}</span>
      {:else}
        <button class="small" onclick={() => resolveDocsConflict(projectId)}>{t('docs.share.resolve')}</button>
        <button class="small" onclick={() => cancelDocsConflict(projectId)}>{t('docs.share.cancel')}</button>
      {/if}
    {:else}
      <span class="sep" aria-hidden="true">·</span>
      <span class:some={pending > 0} data-pending={pending}
        >{pending ? t('docs.share.pending', { count: pending }) : t('docs.share.none')}</span
      >
      {#if pending}
        <button class="small share" disabled={busy} onclick={share}
          ><Icon name="push" size={12} />{busy ? t('docs.share.going') : t('docs.share.go')}</button
        >
      {/if}
    {/if}
  {/if}
</span>

<style>
  .docs-share {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    white-space: nowrap;
    color: var(--muted);
  }
  .on b {
    color: var(--text);
    font-weight: 600;
  }
  .sep {
    color: var(--dim);
  }
  .some,
  .conflict {
    color: var(--wait);
  }
  .conflict {
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 40ch;
  }
  .d {
    color: var(--dim);
  }
  .share {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
</style>
