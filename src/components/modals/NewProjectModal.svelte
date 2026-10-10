<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { modelOptions } from '../../lib/models';
  import { app } from '../../lib/state.svelte';
  import { PROJECT_COLORS } from '../../lib/theme';
  import type { FolderInfo } from '../../lib/types';
  import Modal from './Modal.svelte';

  const used = new Set(app.projects.map((p) => p.color));
  let path = $state('');
  let name = $state('');
  let nameTouched = false;
  let color = $state(PROJECT_COLORS.find((c) => !used.has(c)) ?? PROJECT_COLORS[0]);
  let firstAgent = $state(true);
  let model = $state(app.settings.defaultModel || 'sonnet');
  let worktree = $state(false);
  let info = $state<FolderInfo | null>(null);
  let busy = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    const p = path.trim();
    clearTimeout(timer);
    if (!p) {
      info = null;
      return;
    }
    timer = setTimeout(async () => {
      const i = await api.inspectFolder(p).catch(() => null);
      if (path.trim() !== p) return;
      info = i;
      if (i && !nameTouched) name = i.name;
    }, 200);
  });

  const gitLine = $derived(
    !info
      ? { text: t('shell.newProject.git.choose'), color: 'var(--dim)' }
      : !info.exists
        ? { text: t('shell.newProject.git.missing'), color: 'var(--del)' }
        : info.isRepo
          ? {
              text: info.dirty
                ? t('shell.newProject.git.repoDirty', { branch: info.branch || '—', count: info.dirty })
                : t('shell.newProject.git.repoClean', { branch: info.branch || '—' }),
              color: 'var(--ok)',
            }
          : { text: t('shell.newProject.git.none'), color: 'var(--muted)' },
  );

  async function browse() {
    const dir = await open({ directory: true, multiple: false, title: t('shell.newProject.folderTitle') });
    if (typeof dir === 'string') path = dir;
  }

  async function create() {
    if (!info?.exists || busy) return;
    busy = true;
    const p = await app.run(
      api.createProject({ path: path.trim(), name: name.trim(), color, worktreePerAgent: worktree, firstAgent: firstAgent ? model : null }),
    );
    busy = false;
    if (!p) return;
    app.projects.push(p);
    app.selectProject(p.id);
    app.modal = null;
  }
</script>

<Modal title={t('shell.newProject.title')} onclose={() => (app.modal = null)}>
  <div class="grp">
    <span class="lab">{t('common.folder')}</span>
    <div class="row">
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="field mono"
        style="flex:1;font-size:12.5px"
        bind:value={path}
        placeholder={t('shell.newProject.pathPlaceholder')}
        autofocus
      />
      <button class="btn" style="height:36px" onclick={browse}>{t('shell.newProject.browse')}</button>
    </div>
    <span class="git mono" style:color={gitLine.color}><span class="d" style:background={gitLine.color}></span>{gitLine.text}</span>
  </div>
  <div class="two">
    <div class="grp">
      <span class="lab">{t('common.name')}</span>
      <input class="field" style="font-weight:600;font-size:13.5px" bind:value={name} oninput={() => (nameTouched = true)} />
    </div>
    <div class="grp">
      <span class="lab">{t('shell.newProject.tabPreview')}</span>
      <div class="preview" style:border-bottom-color={color}>
        <span class="sw" style:background={color}></span>{name || t('shell.newProject.untitled')}
      </div>
    </div>
  </div>
  <div class="grp">
    <span class="lab">{t('common.color')}</span>
    <div class="colors">
      {#each PROJECT_COLORS as c, i (c)}
        <button
          class="swatch"
          aria-label={t('shell.newProject.colorN', { n: i + 1 })}
          aria-pressed={color === c}
          style:background={c}
          style:box-shadow={color === c ? '0 0 0 2px var(--panel), 0 0 0 4px var(--text)' : 'none'}
          onclick={() => (color = c)}
        ></button>
      {/each}
    </div>
  </div>
  <div class="hr"></div>
  <div class="grp" style="gap:14px">
    <div class="toggle">
      <div class="tx">
        <span class="tt">{t('shell.newProject.firstAgent')}</span>
        <span class="ts">{t('shell.newProject.firstAgentDesc')}</span>
      </div>
      <button
        class="switch"
        role="switch"
        aria-checked={firstAgent}
        class:on={firstAgent}
        aria-label={t('shell.newProject.firstAgent')}
        onclick={() => (firstAgent = !firstAgent)}
      ></button>
    </div>
    {#if firstAgent}
      <div class="models">
        <span class="ts">{t('common.model')}</span>
        {#each modelOptions(app.models) as m (m.value)}
          <button class="mbtn mono" class:on={model === m.value} onclick={() => (model = m.value)}>{m.label}</button>
        {/each}
      </div>
    {/if}
    <div class="toggle">
      <div class="tx">
        <span class="tt">{t('shell.newProject.worktree')}</span>
        <span class="ts">{t('shell.newProject.worktreeDesc')}</span>
      </div>
      <button
        class="switch"
        role="switch"
        aria-checked={worktree}
        class:on={worktree}
        aria-label={t('shell.newProject.worktree')}
        onclick={() => (worktree = !worktree)}
      ></button>
    </div>
  </div>
  {#snippet footer()}
    <button class="btn ghost" onclick={() => (app.modal = null)}>{t('common.cancel')}</button>
    <button class="btn primary" style:background={color} disabled={!info?.exists || busy} onclick={create}>
      {busy ? t('shell.newProject.creating') : t('shell.newProject.create')}
    </button>
  {/snippet}
</Modal>

<style>
  .grp {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .lab {
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
  .row {
    display: flex;
    gap: 8px;
  }
  .git {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 11px;
  }
  .d {
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }
  .two {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 16px;
    align-items: end;
  }
  .preview {
    height: 36px;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 0 14px;
    border: 1px solid var(--line2);
    border-bottom: 2px solid;
    border-radius: var(--r-sm);
    background: var(--bg);
    font-weight: 600;
    white-space: nowrap;
    max-width: 220px;
    overflow: hidden;
  }
  .sw {
    width: 10px;
    height: 10px;
    border-radius: 3px;
    flex: none;
  }
  .colors {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
  }
  .swatch {
    width: 30px;
    height: 30px;
    border-radius: 9px;
    border: none;
    padding: 0;
    cursor: pointer;
  }
  .hr {
    height: 1px;
    background: var(--line);
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .tx {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .tt {
    font-size: 13.5px;
    font-weight: 600;
  }
  .ts {
    font-size: 12px;
    color: var(--muted);
    text-wrap: pretty;
  }
  .models {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-left: 2px;
  }
  .mbtn {
    height: 26px;
    padding: 0 11px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line2);
    background: transparent;
    color: var(--muted);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }
  .mbtn.on {
    background: var(--elev2);
    color: var(--text);
  }
</style>
