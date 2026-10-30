<script lang="ts">
  import { t } from '../../lib/i18n';
  import { CATEGORIES, categoriesOf, type Category } from '../../lib/plugins';
  import { plugins, type PluginsPage } from '../../lib/plugins-view.svelte';
  import type { IconName } from '../../lib/icons';
  import Icon from '../Icon.svelte';
  import { categoriesOfCatalog } from '../../lib/store';

  // The sidebar of the view Plugins (choice 214): Store, Installés, Mises à jour (their count in amber), En développement,
  // then the categories with their count.
  const CAT_ICON: Record<Category, IconName> = {
    harnesses: 'spark',
    kanban: 'kanban',
    forges: 'branch',
    themes: 'circle',
    notifications: 'bell',
    permissions: 'shield',
    tools: 'sliders',
    languages: 'lang',
  };
  const page = $derived(plugins.page);
  // A category counts the store's plugins once the catalogue is read (STORE-9), else the installed ones.
  const counts = $derived(
    Object.fromEntries(
      CATEGORIES.map((c) => [
        c,
        plugins.store?.catalog
          ? plugins.catalog.filter((p) => categoriesOfCatalog(p).includes(c)).length
          : plugins.list.filter((p) => categoriesOf(p).includes(c)).length,
      ]),
    ) as Record<Category, number>,
  );
  /** A plugin's page is one of the installed ones'. */
  const on = (p: PluginsPage) =>
    JSON.stringify(p) ===
    JSON.stringify(page.kind === 'plugin' ? { kind: 'installed' } : page.kind === 'storePlugin' ? { kind: 'store' } : page);
</script>

<aside class="side plugins-side" aria-label={t('plugins.nav.label')}>
  <h5>{t('plugins.nav.label')}</h5>
  {#each [{ kind: 'store', icon: 'sparkle', n: plugins.store?.catalog ? plugins.catalog.length : null }, { kind: 'installed', icon: 'agents', n: plugins.list.length }, { kind: 'updates', icon: 'up', n: plugins.updates.length }, { kind: 'dev', icon: 'code', n: plugins.dev.length }] as const as item (item.kind)}
    <button
      class="it"
      class:on={on({ kind: item.kind })}
      class:upd={item.kind === 'updates' && plugins.updates.length > 0}
      aria-current={on({ kind: item.kind }) ? 'page' : undefined}
      onclick={() => plugins.show({ kind: item.kind })}
      ><Icon name={item.icon} size={14} /><span>{t(`plugins.nav.${item.kind}`)}</span>{#if item.n !== null}<span class="n">{item.n}</span
        >{/if}</button
    >
  {/each}
  <h5>{t('plugins.nav.categories')}</h5>
  {#each CATEGORIES as c (c)}
    <button
      class="it"
      class:on={on({ kind: 'category', category: c })}
      aria-current={on({ kind: 'category', category: c }) ? 'page' : undefined}
      onclick={() => plugins.show({ kind: 'category', category: c })}
      ><Icon name={CAT_ICON[c]} size={14} /><span>{t(`plugins.cat.${c}`)}</span><span class="n">{counts[c]}</span></button
    >
  {/each}
</aside>

<style>
  .plugins-side {
    width: var(--side);
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 10px 10px 4px;
    overflow: auto;
  }
  h5 {
    margin: 14px 8px 6px;
    font-size: 10.5px;
    letter-spacing: 0.06em;
    color: var(--muted);
    font-weight: 600;
    text-transform: uppercase;
  }
  .it {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 6px 10px;
    border: none;
    border-radius: 7px;
    background: none;
    color: var(--soft);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .it :global(.icon) {
    color: var(--muted);
  }
  .it:hover {
    background: var(--hover);
  }
  .it.on {
    background: rgba(var(--fg-rgb), 0.07);
    color: var(--text);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .it.upd,
  .it.upd .n,
  .it.upd :global(.icon) {
    color: var(--warn-ink);
  }
  .it .n {
    margin-left: auto;
    color: var(--muted);
    font-size: 11.5px;
  }
</style>
