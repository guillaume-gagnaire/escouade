<script lang="ts">
  // A page the agent published, in its landing zone: a small browser window (three dots, the address), the title, its
  // version and when, « Ouvrir ↗ ». While it is being published, its file's name and no button.
  import { fAgo } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { openUrl } from '../../lib/opener';
  import { app } from '../../lib/state.svelte';
  import type { ArtifactLanding } from '../../lib/workspace-landing';

  let { item }: { item: ArtifactLanding } = $props();
</script>

<article class="card" class:coming={item.url === null} data-side="south" aria-label={item.title}>
  <div class="window">
    <span class="dots" aria-hidden="true"><i></i><i></i><i></i></span>
    <span class="addr mono">{item.url ? item.url.replace(/^https:\/\//, '') : item.title}</span>
  </div>
  <b class="title">{item.title}</b>
  {#if item.url}
    <footer>
      <span class="meta">{t('workspace.land.artifact', { n: item.versions, ago: fAgo(item.at / 1000, app.now) })}</span>
      <button
        type="button"
        class="btn"
        aria-label={t('workspace.land.openPage', { title: item.title })}
        onclick={() => item.url && void openUrl(item.url)}>{t('workspace.land.open')}</button
      >
    </footer>
  {/if}
</article>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 4px;
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    padding: 0 0 7px;
    overflow: hidden;
    border-radius: 9px;
    background: var(--ws-card);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font-size: 12.5px;
  }
  .window {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 5px 8px;
    border-bottom: 1px solid var(--line-2);
    background: var(--raise);
  }
  .dots {
    display: flex;
    gap: 3px;
  }
  .dots i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--line-3);
  }
  .addr {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font: 10.5px var(--mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .title {
    padding: 0 10px;
    overflow: hidden;
    font-weight: 590;
    letter-spacing: -0.01em;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: auto;
    padding: 0 10px;
  }
  .meta {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .btn {
    padding: 2px 8px;
    font-size: 11.5px;
  }
</style>
