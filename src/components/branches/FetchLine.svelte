<script lang="ts">
  import { failureLine, fetchedLine } from '../../lib/fetch';
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { FetchFailure } from '../../lib/types';

  // Under « Fetch » in the branch picker (ESC-561): when the repository was last fetched (the time on hover), and, said
  // quietly, why its last background fetch failed, with what to do; the ssh of a Git for Windows that stalls, to update.
  // It guides; it never alerts.
  let {
    projectId,
    lastFetch,
    failure = null,
    sshNote = false,
    onsettings,
  }: {
    projectId: string;
    lastFetch: number | null;
    failure?: FetchFailure | null;
    sshNote?: boolean;
    /** The settings opened: the picker closes. */
    onsettings?: () => void;
  } = $props();

  const fetched = $derived(fetchedLine(lastFetch, app.now));
  const failed = $derived(failure ? failureLine(failure) : null);

  /** Its time limit is set where it is: the project's own, else the general one. */
  function settings() {
    const own = app.projects.find((p) => p.id === projectId)?.gitFetch?.timeoutSecs;
    app.modal = own ? { kind: 'settings', page: 'git', projectId } : { kind: 'settings', page: 'allProjects', projectId };
    onsettings?.();
  }
</script>

<div class="fetch-line" data-testid="fetch-line">
  <p class="fetched" use:tip={fetched.title || undefined}>{fetched.text}</p>
  {#if failed && failure}
    <p class="failed" data-kind={failure.kind} use:tip={failed.title}>
      {failed.text}
      {#if failure.kind === 'timeout'}<button class="link" onclick={settings}>{t('fetch.panel.openSettings')}</button>{/if}
    </p>
  {/if}
  {#if sshNote}<p class="note">{t('fetch.panel.sshNote')}</p>{/if}
</div>

<style>
  .fetch-line {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 2px 9px 4px;
    font-size: 11.5px;
    line-height: 1.4;
    color: var(--muted);
  }
  p {
    margin: 0;
  }
  .fetched {
    font-variant-numeric: tabular-nums;
  }
  .failed,
  .note {
    color: var(--muted);
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--p-ink);
    font: inherit;
    text-decoration: underline;
    text-underline-offset: 2px;
    cursor: pointer;
  }
</style>
