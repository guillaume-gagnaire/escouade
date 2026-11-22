<script lang="ts">
  import Icon from '../Icon.svelte';
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import {
    bands,
    bodyOf,
    fieldText,
    megabytes,
    someFiles,
    syncState,
    teamDocsApi,
    type Settle,
    type TeamDocsView,
  } from '../../lib/team-docs';
  import type { Project } from '../../lib/types';
  import { tip } from '../../lib/tip';

  // Above the Docs of a project whose docs are on the team server (TS-9): where they are and what waits to go, a page
  // written on both sides that did not merge (its two versions side by side on « Comparer »), a decision's field that
  // lost to a teammate's, the repository's files left after the import, the documents too heavy, the numbers that wait.
  let { project, onchange }: { project: Project; onchange?: () => void } = $props();

  let view = $state<TeamDocsView | null>(null);
  let busy = $state(false);
  let open = $state<string | null>(null);

  const tick = $derived(app.teamDocsTick[project.id] ?? 0);
  const shown = $derived(bands(view));
  const sync = $derived(syncState(view));

  $effect(() => {
    void tick;
    void project.team;
    void project.teamProject;
    void load(project.id);
  });

  async function load(id: string) {
    if (!project.team) {
      view = null;
      return;
    }
    const v = await app.run(teamDocsApi.status(id));
    if (v && id === project.id) {
      view = v;
      app.teamDocsOn[id] = v.onServer;
    }
  }

  async function act(p: Promise<TeamDocsView>) {
    busy = true;
    try {
      const v = await app.run(p);
      if (v) view = v;
      onchange?.();
    } finally {
      busy = false;
    }
  }

  const settle = (path: string, choice: Settle) => act(teamDocsApi.settle(project.id, path, choice));
</script>

{#if view?.onServer}
  <div class="team-docs" data-team-docs={sync}>
    <div class="line" role="status" data-sync={sync}>
      <Icon name={sync === 'offline' ? 'pause' : 'team'} size={13} />
      <span
        >{sync === 'offline'
          ? view.pending
            ? t('teamDocs.sync.offlinePending', { count: view.pending })
            : t('teamDocs.sync.offline')
          : sync === 'pending'
            ? t('teamDocs.sync.pending', { count: view.pending })
            : t('teamDocs.sync.synced')}</span
      >
    </div>
    {#each shown as b, i (i)}
      {#if b.kind === 'refused'}
        {@const text = [
          t(b.refused.kind === 'page' ? 'teamDocs.refused.page' : 'teamDocs.refused.decision', { key: b.refused.key }),
          b.refused.mine ? t('teamDocs.refused.mine') : t('teamDocs.refused.by', { device: b.refused.device.slice(0, 8) }),
        ].join(' ')}
        <div class="band refused" role="alert" data-refused={b.refused.kind}>
          <Icon name="shield" size={14} />
          <span class="why" use:tip={{ text, cut: true }}>{text}</span>
        </div>
      {:else if b.kind === 'conflict'}
        <div class="band conflict" role="alert" data-conflict={b.conflict.path}>
          <Icon name="merge" size={14} />
          <span class="why">{t('teamDocs.conflict.text', { title: b.conflict.title || b.conflict.path })}</span>
          {#if b.conflict.agentId}
            <span class="d">{t('teamDocs.conflict.agentBusy')}</span>
          {:else}
            <button
              class="btn"
              aria-expanded={open === b.conflict.path}
              onclick={() => (open = open === b.conflict.path ? null : b.conflict.path)}>{t('teamDocs.conflict.compare')}</button
            >
            <button class="btn" disabled={busy} onclick={() => settle(b.conflict.path, 'mine')}>{t('teamDocs.conflict.mine')}</button>
            <button class="btn" disabled={busy} onclick={() => settle(b.conflict.path, 'theirs')}>{t('teamDocs.conflict.theirs')}</button>
            <button class="btn" disabled={busy} onclick={() => settle(b.conflict.path, 'agent')}
              ><Icon name="sparkle" size={13} />{t('teamDocs.conflict.agent')}</button
            >
          {/if}
        </div>
        {#if open === b.conflict.path}
          <div class="sides" data-sides={b.conflict.path}>
            <section>
              <h4>{t('teamDocs.conflict.mineTitle')}</h4>
              <pre class="mono">{bodyOf(b.conflict.mine)}</pre>
            </section>
            <section>
              <h4>{t('teamDocs.conflict.theirsTitle')}</h4>
              <pre class="mono">{bodyOf(b.conflict.theirs)}</pre>
            </section>
          </div>
        {/if}
      {:else if b.kind === 'lost'}
        {@const text = t('teamDocs.lost.text', { title: b.lost.title, field: b.lost.field, mine: fieldText(b.lost.mine) })}
        <div class="band" role="status" data-lost={b.lost.id}>
          <Icon name="info" size={14} />
          <span class="why" use:tip={{ text, cut: true }}>{text}</span>
          <button class="btn" disabled={busy} onclick={() => act(teamDocsApi.dismiss(project.id, b.lost.stamp))}
            >{t('teamDocs.lost.ok')}</button
          >
        </div>
      {:else if b.kind === 'repoFiles'}
        {@const some = someFiles(b.files)}
        {@const text = t('teamDocs.repoFiles.text', {
          count: b.files.length,
          files: some.list.join(', '),
          more: some.more ? t('teamDocs.repoFiles.more', { count: some.more }) : '',
        })}
        <div class="band quiet" role="status" data-repo-files={b.files.length}>
          <Icon name="folder" size={14} />
          <span class="why" use:tip={{ text: [text, ...b.files].join('\n'), cut: true }}>{text}</span>
          <button class="btn" disabled={busy} onclick={() => act(teamDocsApi.removeRepoFiles(project.id))}
            >{t('teamDocs.repoFiles.remove')}</button
          >
        </div>
      {:else if b.kind === 'tooLarge'}
        {@const text = t('teamDocs.tooLarge.text', { count: b.files.length, max: megabytes(b.max), files: b.files.join(', ') })}
        <div class="band" role="status" data-too-large={b.files.length}>
          <Icon name="file" size={14} />
          <span class="why" use:tip={{ text, cut: true }}>{text}</span>
        </div>
      {:else}
        <div class="band quiet" role="status" data-waiting={b.ids.length}>
          <Icon name="clock" size={14} />
          <span class="why">{t('teamDocs.waiting.text', { count: b.ids.length })}</span>
        </div>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .team-docs {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 22px 0;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 12px;
  }
  .line[data-sync='offline'] {
    color: var(--wait);
  }
  .band {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px 8px 12px;
    border-radius: 10px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--soft);
    font-size: 12.5px;
  }
  .band > :global(.icon) {
    flex: none;
    color: var(--wait);
  }
  .band.quiet {
    background: rgba(var(--fg-rgb), 0.035);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .band.quiet > :global(.icon) {
    color: var(--muted);
  }
  .band.refused {
    background: oklch(from var(--del) l c h / 0.08);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.34);
  }
  .band.refused > :global(.icon) {
    color: var(--del);
  }
  .band .btn {
    flex: none;
    height: 28px;
    font-size: 12.5px;
  }
  .why {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .conflict .why {
    white-space: normal;
  }
  .d {
    color: var(--muted);
  }
  .sides {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .sides section {
    min-width: 0;
    border-radius: 10px;
    box-shadow: inset 0 0 0 1px var(--line);
    padding: 8px 10px;
  }
  .sides h4 {
    margin: 0 0 6px;
    font-size: 12px;
    color: var(--muted);
    font-weight: 600;
  }
  .sides pre {
    margin: 0;
    max-height: 220px;
    overflow: auto;
    white-space: pre-wrap;
    font-size: 11.5px;
    color: var(--soft);
  }
  @media (max-width: 720px) {
    .band {
      flex-wrap: wrap;
    }
    .sides {
      grid-template-columns: 1fr;
    }
  }
</style>
