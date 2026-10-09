<script lang="ts">
  import { fileLines, parseUnifiedDiff, type DiffFile } from '../lib/diff';
  import { api } from '../lib/ipc';
  import { trapFocus } from '../lib/focus';
  import { app } from '../lib/state.svelte';
  import DiffView from './DiffView.svelte';

  // Uncommitted changes (`paths` of the agent's checkout), what `commit` changed, or, with
  // `wholeProject`, every file the project's list shows: the project's checkout and each agent's worktree.
  let {
    projectId,
    agentId,
    paths,
    title,
    commit = null,
    wholeProject = false,
  }: {
    projectId: string;
    agentId: string | null;
    paths: string[];
    title: string;
    commit?: string | null;
    wholeProject?: boolean;
  } = $props();

  // `agentId`: the agent a whole-project file is listed under, as in the files panel. `key`: the same
  // path can be modified in the project and in a worktree, so the path alone does not identify a file.
  type Shown = DiffFile & { key: string; agentId: string | null };
  const shown = (diff: string, owner: string | null, group: number): Shown[] =>
    parseUnifiedDiff(diff).map((f) => ({ ...f, key: `${group}:${f.path}`, agentId: owner }));

  let files = $state<Shown[]>([]);
  let current = $state(0);
  let loading = $state(true);
  let error = $state<string | null>(null);

  const agentName = (id: string) => app.agents[id]?.name ?? '?';

  $effect(() => {
    (commit
      ? api.gitShow(projectId, commit).then((d) => shown(d, null, 0))
      : wholeProject
        ? api.gitProjectDiff(projectId).then((parts) => parts.flatMap((p, i) => shown(p.diff, p.agentId, i)))
        : api.gitDiff(projectId, agentId, paths).then((d) => shown(d, null, 0))
    )
      .then((list) => {
        files = list;
        loading = false;
      })
      .catch((e) => {
        error = String(e);
        loading = false;
      });
  });

  const file = $derived(files[current]);
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && (app.modal = null)} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="overlay" onclick={() => (app.modal = null)}>
  <div class="win" use:trapFocus onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1" aria-label={title}>
    <div class="head">
      <span class="t">{title}</span>
      <span class="n mono">{files.length} fichier{files.length > 1 ? 's' : ''}</span>
      <div style="flex:1"></div>
      <div class="segmented">
        <button class:on={!app.diffSplit} onclick={() => app.setDiffSplit(false)}>Unifié</button>
        <button class:on={app.diffSplit} onclick={() => app.setDiffSplit(true)}>Côte à côte</button>
      </div>
      <button class="icon-btn" title="Fermer (Échap)" onclick={() => (app.modal = null)}>×</button>
    </div>
    <div class="body">
      {#if files.length > 1}
        <nav class="files">
          {#each files as f, i (f.key)}
            <button class:on={i === current} onclick={() => (current = i)}>
              <span class="st" class:a={f.status === 'A'} class:d={f.status === 'D'}>{f.status}</span>
              <span class="p mono">{f.path}</span>
              {#if f.agentId}<span class="tag mono">{agentName(f.agentId)}</span>{/if}
              <span class="add mono">+{f.add}</span><span class="del mono">−{f.del}</span>
            </button>
          {/each}
        </nav>
      {/if}
      <div class="diff">
        {#if loading}
          <div class="msg">Chargement…</div>
        {:else if error}
          <div class="msg">{error}</div>
        {:else if !file}
          <div class="msg">Aucune différence.</div>
        {:else}
          <div class="fhead mono">
            {file.path}
            {#if file.agentId}<span class="tag">{agentName(file.agentId)}</span>{/if}
            <span class="add">+{file.add}</span> <span class="del">−{file.del}</span>
          </div>
          {#if file.binary}
            <div class="msg">Fichier binaire.</div>
          {:else}
            <DiffView lines={fileLines(file)} split={app.diffSplit} />
          {/if}
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 60;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(12, 10, 8, 0.6);
    backdrop-filter: blur(3px);
  }
  .win {
    width: calc(100vw - 80px);
    height: calc(100vh - 80px);
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--line2);
    border-radius: 14px;
    box-shadow: 0 30px 80px rgba(0, 0, 0, 0.55);
    overflow: hidden;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px 12px 20px;
    border-bottom: 1px solid var(--line);
  }
  .t {
    font-size: 15px;
    font-weight: 700;
  }
  .n {
    font-size: 11px;
    color: var(--dim);
  }
  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .files {
    width: 300px;
    flex: none;
    overflow: auto;
    border-right: 1px solid var(--line);
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .files button {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    text-align: left;
    cursor: pointer;
    min-width: 0;
  }
  .files button:hover,
  .files button.on {
    background: var(--elev);
  }
  .st {
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 700;
    color: var(--wait);
    flex: none;
  }
  .st.a {
    color: var(--add);
  }
  .st.d {
    color: var(--del);
  }
  .p {
    flex: 1;
    min-width: 0;
    font-size: 11.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }
  /* The agent a file is listed under, as in the files panel. */
  .tag {
    flex: none;
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 3px;
    background: var(--elev2);
    color: var(--muted);
    white-space: nowrap;
    max-width: 90px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .fhead .tag {
    display: inline-block;
    vertical-align: bottom;
  }
  .add {
    color: var(--add);
    font-size: 11px;
  }
  .del {
    color: var(--del);
    font-size: 11px;
  }
  .diff {
    flex: 1;
    overflow: auto;
    background: var(--term);
    min-width: 0;
  }
  .msg {
    padding: 40px;
    text-align: center;
    color: var(--muted);
  }
  .fhead {
    position: sticky;
    top: 0;
    z-index: 1;
    padding: 8px 14px;
    font-size: 12px;
    background: var(--panel);
    border-bottom: 1px solid var(--line);
  }
</style>
