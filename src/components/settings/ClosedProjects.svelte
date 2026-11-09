<script lang="ts">
  import { onMount } from 'svelte';
  import { fDate, fSize } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { ClosedProject } from '../../lib/types';
  import Group from './Group.svelte';

  // The projects closed with their history kept: what adding their folder again brings back, and where to erase it.
  let closed = $state<ClosedProject[]>([]);

  // Read each time the settings open (they open again after an erasing), and nothing is said of a list that cannot be read.
  onMount(async () => {
    closed = await api.closedProjects().catch(() => []);
  });

  /** The file an entry the app cannot read is: its id is the name of the file, without its extension. */
  const fileOf = (c: ClosedProject) => `${c.id}.json`;

  /** The window that adds a project, on this one's folder and on this history of it (the folder may have several). */
  function reopen(c: ClosedProject) {
    app.modal = { kind: 'newProject', path: c.path, historyId: c.id };
  }

  /**
   * Asks before erasing the history (an entry the app cannot read: its one file), then comes back to these settings,
   * their draft as it was.
   */
  function askErase(c: ClosedProject) {
    // Taken now: the settings are gone while the confirmation is open.
    const projectId = settingsForm.projectId ?? undefined;
    const back = () => {
      app.modal = { kind: 'settings', page: 'allProjects', projectId, resume: true };
    };
    const file = c.unreadable;
    app.modal = {
      kind: 'confirm',
      title: file ? t('settings.closed.eraseFileTitle', { file: fileOf(c) }) : t('settings.closed.eraseTitle', { name: c.name }),
      body: file ? t('settings.closed.eraseFileBody') : t('settings.closed.eraseBody'),
      confirm: file ? t('settings.closed.eraseFileConfirm') : t('settings.closed.eraseConfirm'),
      danger: true,
      onCancel: back,
      onConfirm: async () => {
        await app.run(api.eraseClosedProject(c.id));
        back();
      },
    };
  }
</script>

{#if closed.length}
  <Group title={t('settings.closed.title')} note={t('settings.closed.note')}>
    <ul class="list">
      {#each closed as c (c.id)}
        <li class="item">
          {#if c.unreadable}
            <!-- Nothing to reopen: only the file's name and size are known, and the file can only go. -->
            <span class="sw off"></span>
            <div class="txt">
              <span class="name">{t('settings.closed.unreadable')}</span>
              <span class="path mono" id="closed-{c.id}" use:tip={{ text: fileOf(c), cut: true }}>{fileOf(c)}</span>
              <span class="detail">{fSize(c.size)}</span>
            </div>
            <button class="btn danger" aria-describedby="closed-{c.id}" onclick={() => askErase(c)}>{t('settings.closed.eraseFile')}</button
            >
          {:else}
            <span class="sw" style:background={c.color}></span>
            <div class="txt">
              <span class="name" id="closed-{c.id}">{c.name}</span>
              <span class="path mono" use:tip={{ text: c.path, cut: true }}>{c.path}</span>
              <span class="detail">{t('settings.closed.detail', { size: fSize(c.size), date: fDate(c.lastActivity) })}</span>
            </div>
            <button class="btn" aria-describedby="closed-{c.id}" onclick={() => reopen(c)}>{t('settings.closed.reopen')}</button>
            <button class="btn danger" aria-describedby="closed-{c.id}" onclick={() => askErase(c)}>{t('settings.closed.erase')}</button>
          {/if}
        </li>
      {/each}
    </ul>
  </Group>
{/if}

<style>
  /* A closed project (`.closed-row` of the 2.0 design): its colour, its name and what is kept, Rouvrir, Effacer. */
  .list {
    margin: 8px 0 0;
    padding: 0;
    list-style: none;
  }
  .item {
    display: grid;
    grid-template-columns: 10px minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 12px;
    padding: 10px 0;
    border-bottom: 1px solid rgba(var(--fg-rgb), 0.05);
  }
  .item:has(.off) {
    grid-template-columns: 10px minmax(0, 1fr) auto;
  }
  .sw {
    width: 8px;
    height: 8px;
    border-radius: 3px;
  }
  .sw.off {
    background: var(--dim);
  }
  .txt {
    display: grid;
    min-width: 0;
  }
  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .path,
  .detail {
    color: var(--muted);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
