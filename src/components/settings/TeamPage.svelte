<script lang="ts">
  import { t } from '../../lib/i18n';
  import { settingsForm } from '../../lib/settings.svelte';
  import { gitNameOf } from '../../lib/share-actions';
  import { SHARED_DIR } from '../../lib/sharing';
  import { app } from '../../lib/state.svelte';
  import type { Project } from '../../lib/types';
  import Group from './Group.svelte';
  import MemorySection from './MemorySection.svelte';
  import Row from './Row.svelte';
  import SharedSection from './SharedSection.svelte';
  import Switch from './Switch.svelte';

  // Projets › <projet> › Équipe: what the project shares with those who work on it: the shared Kanban (ESC-190), its
  // settings shared in .escouade.yml (ESC-146), its shared memory.
  let { project }: { project: Project } = $props();
  // The name git knows in its folder (ESC-243): what « Ton nom dans ce projet » falls back on after the app's.
  $effect(() => void gitNameOf(project.id));

  const draft = $derived(settingsForm.project!);
  /** Its shared tickets (ESC-190): all of them, and those of this machine's user. */
  const sharedCount = $derived.by(() => {
    const list = Object.values(app.tickets).filter((x) => x.projectId === project.id && x.shared && !x.shared.gone);
    return { all: list.length, mine: list.filter((x) => x.shared?.mine).length };
  });
  /**
   * « Chef d’escouade » (ESC-423): one of the project's agents (archived ones apart, unless it is the one chosen), or none.
   * Set here alone: no tool of the MCP server names it.
   */
  const chefChoices = $derived(
    Object.values(app.agents)
      .filter((a) => a.projectId === project.id && (!a.archived || a.id === draft.chef))
      .sort((a, b) => a.name.localeCompare(b.name)),
  );
  /** The chef saved is an agent that went: shown as none. */
  const chefValue = $derived(chefChoices.some((a) => a.id === draft.chef) ? draft.chef : '');
  const chefInForce = $derived((settingsForm.settings.agentsAnswer ?? 'nobody') === 'chef');
</script>

<!-- The shared Kanban (ESC-190, choice 113): the name in this project, its shared tickets, « Partager chaque nouveau ticket ». -->
<Group title={t('share.settings.projectGroup')} anchor="share">
  <Row
    label={t('share.settings.projectName')}
    desc={settingsForm.ownName || !app.gitNames[project.id]
      ? t('share.settings.projectNameDesc', { name: settingsForm.ownName || t('share.settings.noName') })
      : t('share.settings.projectNameGit', { name: app.gitNames[project.id] })}
  >
    <input
      class="field input"
      aria-label={t('share.settings.projectName')}
      spellcheck="false"
      placeholder={settingsForm.ownName || app.gitNames[project.id] || ''}
      bind:value={draft.userName}
    />
  </Row>
  <Row
    label={t('share.settings.files')}
    desc={sharedCount.all
      ? t('share.settings.filesDesc', { count: sharedCount.all, dir: SHARED_DIR, mine: sharedCount.mine })
      : t('share.settings.filesNone', { dir: SHARED_DIR })}
  />
  <Row label={t('share.settings.shareNew')} desc={t('share.settings.shareNewDesc')}>
    <Switch label={t('share.settings.shareNew')} bind:on={draft.shareNew} />
  </Row>
</Group>

<!-- « Chef d’escouade » (ESC-423, audit SEC-03): the agent that answers the other agents' requests, with
     Réglages › Agents › « Les agents peuvent répondre… » on « Seulement le chef d’escouade ». Of this machine. -->
<Group title={t('mcp.section.chef')} anchor="chef">
  <Row label={t('mcp.section.chef')} desc={t('mcp.section.chefDesc')}>
    <select
      class="field select"
      aria-label={t('mcp.section.chef')}
      value={chefValue}
      onchange={(e) => (draft.chef = e.currentTarget.value)}
    >
      <option value="">{t('mcp.section.chefNone')}</option>
      {#each chefChoices as a (a.id)}
        <option value={a.id}>{a.name}</option>
      {/each}
    </select>
    {#snippet extra()}
      {#if chefValue && !chefInForce}
        <p class="chef-off" data-testid="chef-off">{t('mcp.section.chefOff')}</p>
      {/if}
    {/snippet}
  </Row>
</Group>

<!-- « Réglages partagés », .escouade.yml under its title: its commands that wait are also said on the Kanban. -->
<SharedSection {project} />

<MemorySection {project} />

<style>
  .input {
    width: 220px;
    max-width: 100%;
  }
  .chef-off {
    max-width: 64ch;
    margin: 6px 0 0;
    font-size: 12px;
    line-height: 1.5;
    color: var(--muted);
  }
</style>
