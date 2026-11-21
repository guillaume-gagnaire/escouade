<script lang="ts">
  import Icon from '../Icon.svelte';
  import { docsStore } from '../../lib/docs.svelte';
  import { cancelDocsConflict, resolveDocsConflict, syncDocs } from '../../lib/docs-share';
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';

  // The synchronisation of the docs (ESC-534), in the head of each space of the Docs tab, always there: the docs branch,
  // what is left to share, and « Synchroniser », which shares as « Partager » did and, with nothing to share, brings in
  // what the others pushed. A conflict takes its place, with « L'agent résout » and « Annuler ». The person never sees
  // git: the pages in conflict are named in its tooltip, never as lines of git. A project whose docs are on the team server
  // (TS-9) has nothing of that: its docs leave by themselves.
  let { projectId }: { projectId: string } = $props();

  const project = $derived(app.projects.find((p) => p.id === projectId));
  const shared = $derived(app.docsShare[projectId]);
  const pending = $derived(shared?.pending ?? 0);
  const conflict = $derived(shared?.conflict ?? null);
  const syncing = $derived(!!docsStore.syncing[projectId]);
  /** The fixed docs branch, else the branch the project's folder is on. */
  const onBranch = $derived((project?.docsBranch ?? '').trim() || app.git[projectId]?.branch || '');
</script>

{#if app.teamDocsOn[projectId]}
  <!-- On the team server (TS-9): they leave by themselves, no branch to share nor to bring in. -->
  <div class="sync" data-docs-sync data-state="team">
    <span class="where"><span class="what">{t('teamDocs.sync.synced')}</span></span>
  </div>
{:else if conflict}
  <div class="sync conflict" data-docs-sync data-state="conflict" role="status">
    <span class="c-title" use:tip={t('docs.share.conflict', { files: conflict.files.join(', ') })}
      ><span class="mark" aria-hidden="true">!</span>{t('docs.share.conflictTitle')}</span
    >
    {#if conflict.agentId}
      <span class="d">{t('docs.share.resolving')}</span>
    {:else}
      <button class="btn" onclick={() => resolveDocsConflict(projectId)}>{t('docs.share.resolve')}</button>
      <button class="btn ghost" onclick={() => cancelDocsConflict(projectId)}>{t('docs.share.cancel')}</button>
    {/if}
  </div>
{:else}
  <div class="sync" data-docs-sync data-state={syncing ? 'syncing' : pending ? 'pending' : 'up'}>
    <span class="where">
      {#if onBranch}<span class="branch" use:tip={t('docs.share.on', { branch: onBranch })}
          ><Icon name="branch" size={12} /><span class="mono">{onBranch}</span></span
        >{/if}
      <span class="what" class:some={pending > 0} data-pending={pending}
        >{pending ? t('docs.share.pendingShort', { count: pending }) : t('docs.share.upToDate')}</span
      >
    </span>
    <button
      class="btn"
      data-docs-sync-go
      disabled={syncing}
      aria-busy={syncing}
      use:tip={pending ? `${t('docs.share.pending', { count: pending })}. ${t('docs.share.syncTip')}` : t('docs.share.syncTip')}
      onclick={() => syncDocs(projectId)}
      ><span class="spin" class:on={syncing}><Icon name="sync" size={14} /></span>{syncing
        ? t('docs.share.syncing')
        : t('docs.share.sync')}</button
    >
  </div>
{/if}

<style>
  .sync {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    white-space: nowrap;
  }
  .where {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    color: var(--muted);
    font-size: 12.5px;
  }
  .branch {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    max-width: 16ch;
    color: var(--soft);
  }
  .branch .mono {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .what {
    font-variant-numeric: tabular-nums;
  }
  .what.some {
    color: var(--wait);
    font-weight: 550;
  }
  .spin {
    display: inline-flex;
  }
  .spin.on {
    animation: turn 1s linear infinite;
  }
  @keyframes turn {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spin.on {
      animation: none;
    }
  }
  .conflict .c-title {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--del);
    font-size: 12.5px;
    font-weight: 600;
  }
  .mark {
    width: 16px;
    height: 16px;
    border-radius: 8px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: var(--del);
    color: var(--on-wait);
    font-size: 10.5px;
    font-weight: 700;
  }
  .d {
    color: var(--dim);
    font-size: 12.5px;
  }
</style>
