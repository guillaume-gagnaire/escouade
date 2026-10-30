<script lang="ts">
  import Icon from '../Icon.svelte';
  import PluginTile from '../plugins/PluginTile.svelte';
  import { t } from '../../lib/i18n';
  import { textOf } from '../../lib/plugins';
  import { plugins } from '../../lib/plugins-view.svelte';
  import { proposedPlugins } from '../../lib/proposed-plugins';
  import { app } from '../../lib/state.svelte';
  import { askedOf, bestOf } from '../../lib/store';
  import type { Project } from '../../lib/types';

  // The plugins the project proposes (PLG-14): those of `plugins.recommended` of its .escouade.yml not installed here,
  // each with its page in the store, where the user installs it, its permissions shown. Nothing is installed nor turned
  // on from here; « Pas pour ce projet » silences the band for what it names.
  let { project }: { project: Project } = $props();

  // The installed plugins and the catalogue, read once: until then the band waits (it would name installed ones).
  $effect(() => {
    if (!plugins.loaded) void plugins.load();
    if (!plugins.store) void plugins.loadStore();
  });

  const lang = $derived(app.settings.language);
  const ids = $derived(plugins.loaded ? proposedPlugins(project, (id) => !!plugins.byId(id), plugins.notForProject[project.id] ?? []) : []);
  const rows = $derived(
    ids.map((id) => {
      const p = plugins.catalogPlugin(id);
      const best = p && plugins.store ? bestOf(plugins.store, p) : null;
      return {
        id,
        name: (p && textOf(p.name, lang)) || id,
        about: p ? textOf(p.description, lang) : '',
        asked: askedOf(best)
          .map((a) => a.text)
          .join(' · '),
        inStore: !!p,
      };
    }),
  );
</script>

{#if rows.length}
  <section class="proposed" aria-label={t('plugins.proposed.label')}>
    <div class="head">
      <Icon name="puzzle" size={16} />
      <div class="say">
        <b>{t('plugins.proposed.title', { count: rows.length })}</b>
        <p>{t('plugins.proposed.body')}</p>
        {#if project.private}<p class="private">{t('plugins.proposed.private')}</p>{/if}
      </div>
      <button class="btn ghost" onclick={() => plugins.setAside(project.id, ids)}>{t('plugins.proposed.notHere')}</button>
    </div>
    <ul>
      {#each rows as r (r.id)}
        <li>
          <PluginTile id={r.id} name={r.name} src={null} size={32} />
          <div class="what">
            <b>{r.name}</b>
            <span class="id mono">{r.id}</span>
            {#if r.about}<p>{r.about}</p>{/if}
            {#if r.asked}<p class="asks">{r.asked}</p>{/if}
            {#if !r.inStore && plugins.store?.catalog}<p class="asks">{t('plugins.proposed.notInStore')}</p>{/if}
          </div>
          <button class="btn" onclick={() => plugins.showFromSettings(r.id)}
            ><Icon name="puzzle" size={14} />{t('plugins.proposed.see')}</button
          >
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .proposed {
    display: grid;
    gap: 10px;
    margin: 0 0 18px;
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--p-soft);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .head {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr) auto;
    gap: 10px;
    align-items: start;
  }
  .head > :global(svg) {
    margin-top: 2px;
  }
  .say b {
    font-weight: 600;
  }
  .say p {
    margin: 3px 0 0;
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .say .private {
    color: var(--warn-ink);
  }
  ul {
    display: grid;
    gap: 8px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: grid;
    grid-template-columns: 32px minmax(0, 1fr) auto;
    gap: 10px;
    align-items: center;
    padding: 8px 10px;
    border-radius: 8px;
    background: var(--panel);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .what {
    min-width: 0;
  }
  .what b {
    font-weight: 600;
    margin-right: 6px;
  }
  .id {
    color: var(--muted);
    font-size: 11.5px;
    overflow-wrap: anywhere;
  }
  .what p {
    margin: 2px 0 0;
    color: var(--soft);
    font-size: 12px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  .what .asks {
    color: var(--muted);
  }
  @media (max-width: 560px) {
    .head,
    li {
      grid-template-columns: minmax(0, 1fr);
    }
    .head > :global(svg),
    li > :global(.tile) {
      display: none;
    }
  }
</style>
