<script lang="ts">
  import { fileLines, parseUnifiedDiff, type DiffFile } from '../lib/diff';
  import { t } from '../lib/i18n';
  import { api } from '../lib/ipc';
  import { app } from '../lib/state.svelte';
  import DiffView from './DiffView.svelte';

  // The diff of one uncommitted file, kept up to date while the agent edits it.
  let { projectId, agentId, path }: { projectId: string; agentId: string | null; path: string } = $props();

  let file = $state<DiffFile | null>(null);
  let loaded = $state(false);
  let error = $state<string | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let first = true;
  let seq = 0;

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
    const mine = ++seq;
    try {
      const d = await api.gitDiff(pid, aid, [p]);
      if (mine !== seq) return; // superseded by a newer request
      file = parseUnifiedDiff(d)[0] ?? null;
      error = null;
    } catch (e) {
      if (mine !== seq) return;
      error = String(e);
    }
    loaded = true;
  }
</script>

<section class="fdiff" aria-label={t('git.diff.ofFile', { path })}>
  <div class="bar">
    <span class="p mono" title={path}>{path}</span>
    {#if file && !file.tooLarge}<span class="add mono">+{file.add}</span><span class="del mono">−{file.del}</span>{/if}
    <div style="flex:1"></div>
    <div class="segmented">
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

<style>
  .fdiff {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--line);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px 8px 18px;
    min-width: 0;
  }
  .p {
    min-width: 0;
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .add {
    flex: none;
    font-size: 11px;
    color: var(--add);
  }
  .del {
    flex: none;
    font-size: 11px;
    color: var(--del);
  }
  .body {
    flex: 1;
    min-height: 0;
    overflow: auto;
    background: var(--term);
    border-top: 1px solid var(--line);
  }
  .msg {
    padding: 40px;
    text-align: center;
    color: var(--muted);
  }
</style>
