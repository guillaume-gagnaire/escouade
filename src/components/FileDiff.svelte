<script lang="ts">
  import { fileLines, parseUnifiedDiff, type DiffFile } from '../lib/diff';
  import { t } from '../lib/i18n';
  import { api } from '../lib/ipc';
  import { newestReads } from '../lib/reads';
  import { app } from '../lib/state.svelte';
  import DiffView from './DiffView.svelte';
  import { tip } from '../lib/tip';

  // The diff of one uncommitted file, kept up to date while the agent edits it.
  let { projectId, agentId, path }: { projectId: string; agentId: string | null; path: string } = $props();

  let file = $state<DiffFile | null>(null);
  let loaded = $state(false);
  let error = $state<string | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let first = true;
  // Answers in any order: the newest one come is shown, a newer read still on its way (ESC-522).
  const reads = newestReads();
  /** Whether a read of `pid`, `aid`, `p` is still of the file shown. */
  const stillShown = (pid: string, aid: string | null, p: string) => pid === projectId && aid === agentId && p === path;

  $effect(() => {
    void app.gitTick;
    const [pid, aid, p] = [projectId, agentId, path];
    clearTimeout(timer);
    // Load at once, then debounce the refreshes that follow the agent's edits.
    timer = setTimeout(() => load(pid, aid, p), first ? 0 : 120);
    first = false;
    return () => clearTimeout(timer);
  });

  async function load(pid: string, aid: string | null, p: string) {
    const mine = reads.ask();
    try {
      const d = await api.gitDiff(pid, aid, [p]);
      if (!stillShown(pid, aid, p) || !reads.take(mine)) return;
      file = parseUnifiedDiff(d)[0] ?? null;
      error = null;
    } catch (e) {
      if (!stillShown(pid, aid, p) || !reads.take(mine)) return;
      error = String(e);
    }
    loaded = true;
  }
</script>

<!-- In a well, as the preview draws the diff beside the files: the file, its counts and the layout above its lines. -->
<div class="side-diff">
  <section class="code" aria-label={t('git.diff.ofFile', { path })}>
    <div class="code-head">
      <span class="p mono" use:tip={{ text: path, cut: true }}>{path}</span>
      {#if file && !file.tooLarge}<span class="stat mono"><span class="add">+{file.add}</span><span class="del">−{file.del}</span></span
        >{/if}
      <div class="segmented small">
        <button class:on={!app.diffSplit} onclick={() => app.setDiffSplit(false)}>{t('git.diff.unified')}</button>
        <button class:on={app.diffSplit} onclick={() => app.setDiffSplit(true)}>{t('git.diff.split')}</button>
      </div>
    </div>
    <div class="body">
      {#if error}
        <div class="msg">{error}</div>
      {:else if !loaded}
        <div class="msg">{t('common.loading')}</div>
      {:else if !file}
        <div class="msg">{t('git.diff.none')}</div>
      {:else if file.binary}
        <div class="msg">{t('git.diff.binary')}</div>
      {:else}
        <DiffView lines={fileLines(file)} split={app.diffSplit} tooLarge={file.tooLarge} />
      {/if}
    </div>
  </section>
</div>

<style>
  .side-diff {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 10px 14px 14px;
  }
  .code {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-radius: 10px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .code-head {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    padding: 0 6px 0 12px;
    border-bottom: 1px solid var(--line);
    color: var(--muted);
    font-size: 12px;
  }
  .p {
    min-width: 0;
    overflow: hidden;
    color: var(--soft);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .stat {
    flex: none;
    display: inline-flex;
    gap: 8px;
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
  }
  .add {
    color: var(--add);
  }
  .del {
    color: var(--del);
  }
  .segmented {
    margin-left: auto;
    flex: none;
  }
  .segmented button {
    height: 22px;
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .msg {
    padding: 40px;
    text-align: center;
    color: var(--muted);
    font-size: 12.5px;
  }
</style>
