<script lang="ts">
  import { t } from '../../lib/i18n';
  import { activeIn, categoriesOf, originOf, type PluginView } from '../../lib/plugins';
  import { plugins } from '../../lib/plugins-view.svelte';
  import { app } from '../../lib/state.svelte';
  import PluginTile from './PluginTile.svelte';
  import ProjectChip from './ProjectChip.svelte';
  import { stateText } from './phase';
  import { tip } from '../../lib/tip';

  // The installed plugins, a line each (choice 218): the tile, the name and what it is; its version and its tags (« Non
  // vérifié », the update found); where it comes from; the projects where it is on, in pills (a lock for a private
  // project); its app's switch. A line opens the plugin's page.
  let { rows, group, note }: { rows: PluginView[]; group?: string; note?: string } = $props();

  const kindOf = (p: PluginView) => {
    const c = categoriesOf(p)[0];
    const label = c ? t(`plugins.cat.${c}`) : '';
    return [label, stateText(p)].filter(Boolean).join(' · ');
  };
  const themeOnly = (p: PluginView) => !p.manifest?.run && categoriesOf(p).every((c) => c === 'themes') && categoriesOf(p).length > 0;
</script>

<div class="tbl" role="table" aria-label={group ?? t('plugins.installed.title')}>
  <div class="tr h" role="row">
    <span></span><span role="columnheader">{t('plugins.installed.cols.plugin')}</span><span role="columnheader"
      >{t('plugins.installed.cols.version')}</span
    ><span role="columnheader">{t('plugins.installed.cols.origin')}</span><span role="columnheader"
      >{t('plugins.installed.cols.projects')}</span
    ><span></span>
  </div>
  {#if group}<div class="group">
      {group} · {rows.length}{#if note}<span>— {note}</span>{/if}
    </div>{/if}
  {#each rows as p (p.id)}
    <div class="tr" class:off={!p.choices.enabled || p.state.kind !== 'ready'} role="row">
      <PluginTile id={p.id} name={p.name} installed />
      <span role="cell" class="cell">
        <button class="nm" onclick={() => plugins.openPlugin(p.id)}>
          <b>{p.name}</b><span class="by">{kindOf(p) || p.id}</span>
        </button>
      </span>
      <span class="tags" role="cell">
        {#if p.version}<span class="tag mono">{t('plugins.version', { version: p.version })}</span>{/if}
        {#if p.origin.kind === 'dev'}<span class="tag">{t('plugins.tag.dev')}</span>{:else}<span class="tag"
            >{t('plugins.tag.unverified')}</span
          >{/if}
        {#if p.update}<span class="tag y">{t('plugins.tag.update', { version: p.update.version })}</span>{/if}
      </span>
      <span class="from mono" role="cell" use:tip={{ text: originOf(p.origin), aria: false }}
        >{p.origin.kind === 'dev' ? t('plugins.origin.devNote', { path: originOf(p.origin) }) : originOf(p.origin)}</span
      >
      <span class="chips" role="cell">
        {#if themeOnly(p)}
          <span class="all">{t('plugins.installed.wholeApp')}</span>
        {:else}
          {#each app.projects as project (project.id)}
            {@const on = activeIn(p, project)}
            <ProjectChip
              {project}
              {on}
              label={on
                ? t('plugins.installed.chipOn', { project: project.name })
                : t('plugins.installed.chipOff', { project: project.name })}
            />
          {:else}
            <span class="all">{t('plugins.installed.nowhere')}</span>
          {/each}
        {/if}
      </span>
      <span role="cell" class="sw">
        <button
          class="switch"
          class:on={p.choices.enabled}
          role="switch"
          aria-checked={p.choices.enabled}
          aria-label={t('plugins.installed.appSwitch', { name: p.name })}
          onclick={() => plugins.setEnabled(p.id, !p.choices.enabled)}
        ></button>
      </span>
    </div>
  {/each}
</div>

<style>
  .tbl {
    display: grid;
  }
  .tr {
    display: grid;
    grid-template-columns: 40px minmax(0, 1.5fr) minmax(0, 1.1fr) minmax(0, 1.4fr) 150px 44px;
    gap: 14px;
    align-items: center;
    padding: 11px 0;
    border-bottom: 1px solid var(--line);
  }
  .tr.h {
    color: var(--muted);
    font-size: 11.5px;
    padding: 6px 0;
  }
  @container plugins (max-width: 860px) {
    .tr {
      grid-template-columns: 40px minmax(0, 1.5fr) minmax(0, 1.1fr) 120px 44px;
    }
    .tr > .from,
    .tr.h > :nth-child(4) {
      display: none;
    }
  }
  .tr.off :global(.tile) {
    filter: grayscale(0.8);
    opacity: 0.6;
  }
  .group {
    margin: 14px 0 2px;
    font-size: 11px;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--muted);
    display: flex;
    gap: 8px;
  }
  .group span {
    text-transform: none;
    letter-spacing: 0;
  }
  .cell {
    min-width: 0;
  }
  .nm {
    width: 100%;
    display: grid;
    gap: 1px;
    min-width: 0;
    padding: 0;
    border: none;
    background: none;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .nm b {
    font-size: 13px;
    font-weight: 600;
  }
  .nm:hover b {
    text-decoration: underline;
    text-decoration-color: var(--line-3);
  }
  .by {
    color: var(--muted);
    font-size: 11.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tags {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .tag {
    font-size: 11px;
    padding: 1px 7px;
    border-radius: 5px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
    white-space: nowrap;
  }
  .tag.mono {
    color: var(--soft);
  }
  .tag.y {
    color: var(--warn-ink);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .from {
    color: var(--soft);
    font-size: 11.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .chips {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }
  .all {
    color: var(--soft);
    font-size: 12px;
  }
  .sw {
    display: flex;
    justify-content: flex-end;
  }
</style>
