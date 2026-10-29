<script lang="ts">
  import { t } from '../../lib/i18n';
  import { activeIn, type PluginView } from '../../lib/plugins';
  import { plugins } from '../../lib/plugins-view.svelte';
  import { app } from '../../lib/state.svelte';
  import ProjectChip from './ProjectChip.svelte';

  // A plugin's switch project by project (§3.3, choice 217): a line per open project, on where the user set it on and,
  // elsewhere, unless the project is private — a private project is off by default, and says that nothing reaches it,
  // or, switched on, what Escouade cannot keep a program from doing.
  let { plugin }: { plugin: PluginView } = $props();
</script>

<section class="projects">
  <p class="intro">{t('plugins.projects.intro')}</p>
  {#if !plugin.choices.enabled}<p class="off" role="status">{t('plugins.projects.appOff')}</p>{/if}
  {#each app.projects as project (project.id)}
    {@const on = activeIn(plugin, project)}
    <div class="prj">
      <ProjectChip {project} {on} size={22} />
      <span class="nm"
        ><b>{project.name}</b>{#if project.private}<span class="why">
            · {plugin.choices.projects[project.id] ? t('plugins.projects.privateOn') : t('plugins.projects.private')}</span
          >{/if}</span
      >
      <button
        class="switch"
        class:on
        role="switch"
        aria-checked={on}
        aria-label={t('plugins.projects.label', { project: project.name })}
        disabled={!plugin.choices.enabled}
        onclick={() => plugins.setProject(plugin.id, project.id, !on)}
      ></button>
    </div>
  {:else}
    <p class="intro">{t('plugins.projects.none')}</p>
  {/each}
</section>

<style>
  .projects {
    display: grid;
  }
  .intro {
    margin: 0 0 8px;
    color: var(--muted);
    font-size: 12px;
  }
  .off {
    margin: 0 0 8px;
    color: var(--warn-ink);
    font-size: 12px;
  }
  .prj {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
    padding: 8px 0;
    border-bottom: 1px solid var(--line);
    font-size: 12.5px;
  }
  .nm b {
    font-weight: 560;
  }
  .why {
    color: var(--muted);
    font-size: 11.5px;
  }
  .switch:disabled {
    opacity: 0.4;
  }
</style>
