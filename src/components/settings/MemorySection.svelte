<script lang="ts">
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { MEMORY_DIR } from '../../lib/memory';
  import { app } from '../../lib/state.svelte';
  import type { Memory, Project } from '../../lib/types';
  import { teamMemoryApi, type TeamMemoryView } from '../../lib/team-memory';
  import Group from './Group.svelte';
  import TeamMemory from './TeamMemory.svelte';

  // « Mémoire partagée » of a project's settings (ESC-255, choice of the chief): its memories read, each one forgotten
  // on a second click. Nothing else: the agents write it through Escouade's tools. Read again when it changes.
  let { project }: { project: Project } = $props();

  let memories = $state<Memory[] | null>(null);
  /** The team's memory (TS-10), when it is on the team server: shown in its place. */
  let team = $state<TeamMemoryView | null>(null);
  let error = $state<string | null>(null);
  /** The memory whose « Oublier » was clicked once: the next click forgets it. */
  let armed = $state<string | null>(null);
  let busy = $state(false);

  async function load(id: string) {
    try {
      const view = project.team && !project.private ? await teamMemoryApi.view(id) : null;
      if (id !== project.id) return;
      team = view?.on ? view : null;
      if (team) {
        memories = team.memories;
        error = null;
        return;
      }
      const got = await api.memoryList(id);
      if (id !== project.id) return;
      memories = got;
      error = null;
    } catch (e) {
      if (id !== project.id) return;
      memories = [];
      error = String(e);
    }
  }

  $effect(() => {
    // Read again at each change the backend tells (an agent wrote, another forgot).
    void app.memoryTicks[project.id];
    void load(project.id);
  });

  async function forget(name: string) {
    if (armed !== name) {
      armed = name;
      return;
    }
    busy = true;
    const done = await app.run(api.memoryForget(project.id, name).then(() => true));
    busy = false;
    armed = null;
    if (!done) return;
    memories = (memories ?? []).filter((m) => m.name !== name);
    app.toast(t('memory.settings.forgotten', { name }));
  }
</script>

<Group
  title={t('memory.settings.title')}
  anchor="memory"
  note={team ? t('memory.team.note') : t('memory.settings.note', { dir: MEMORY_DIR })}
>
  {#snippet state()}
    {#if memories?.length}<span class="count">{t('memory.settings.count', { count: memories.length })}</span>{/if}
  {/snippet}
  {#if team}
    <TeamMemory {project} bind:view={team} />
  {:else if error}
    <p class="empty error">{t('memory.settings.unreadable', { error })}</p>
  {:else if memories && !memories.length}
    <p class="empty">{t('memory.settings.empty')}</p>
  {:else if memories}
    <ul class="memories" aria-label={t('memory.settings.title')}>
      {#each memories as m (m.name)}
        <li>
          <details>
            <summary>
              <span class="name mono">{m.name}</span>
              <span class="kind">{t(`memory.kinds.${m.kind}`)}</span>
              <span class="desc">{m.description}</span>
            </summary>
            <pre class="body">{m.body}</pre>
          </details>
          <button
            class="btn ghost small"
            class:danger={armed === m.name}
            aria-label={armed === m.name ? t('memory.settings.confirm') : t('memory.settings.forgetLabel', { name: m.name })}
            disabled={busy}
            onclick={() => forget(m.name)}
            onblur={() => armed === m.name && (armed = null)}
            >{armed === m.name ? t('memory.settings.confirm') : t('memory.settings.forget')}</button
          >
        </li>
      {/each}
    </ul>
  {/if}
</Group>

<style>
  .count {
    color: var(--muted);
    font-size: 12px;
  }
  .empty {
    max-width: 64ch;
    margin: 4px 0 12px;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }
  .empty.error {
    color: var(--del);
  }
  .memories {
    display: grid;
    margin: 4px 0 12px;
    padding: 0;
    list-style: none;
  }
  .memories li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: start;
    gap: 10px;
    padding: 7px 0;
    border-top: 1px solid var(--line);
  }
  .memories li:first-child {
    border-top: none;
  }
  summary {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 8px;
    cursor: pointer;
    list-style: none;
  }
  summary::-webkit-details-marker {
    display: none;
  }
  .name {
    color: var(--text);
    font-size: 12.5px;
    font-weight: 550;
  }
  .kind {
    padding: 1px 6px;
    border-radius: 999px;
    background: rgba(var(--fg-rgb), 0.06);
    color: var(--muted);
    font-size: 11px;
  }
  .desc {
    flex-basis: 100%;
    color: var(--soft);
    font-size: 12px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  .body {
    max-height: 240px;
    margin: 8px 0 0;
    padding: 9px 11px;
    overflow: auto;
    border-radius: 8px;
    background: rgba(var(--fg-rgb), 0.035);
    color: var(--soft);
    font: 12px/1.5 var(--mono);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .danger {
    color: var(--del);
  }
</style>
