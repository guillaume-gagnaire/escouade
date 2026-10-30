<script lang="ts">
  import { locale, t } from '../../lib/i18n';
  import { textOf } from '../../lib/plugins';
  import { plugins } from '../../lib/plugins-view.svelte';
  import { askedOf, bestOf, count, sealed, type CatalogPlugin } from '../../lib/store';
  import Seal from './Seal.svelte';
  import PluginTile from './PluginTile.svelte';

  // A plugin of the store, a line (choices 165, 166 and 175): its tile, its name, its seal or « Non vérifié », its author
  // and installations, its description on one line, what it asks for in one line; on the right « Installer », « Installé »
  // or the update (« v1.3.0 · non vérifiée »). The line opens its page.
  let { p }: { p: CatalogPlugin } = $props();

  const lang = $derived(locale.ui);
  const view = $derived(plugins.store);
  const best = $derived(view ? bestOf(view, p) : null);
  const mine = $derived(plugins.byId(p.id));
  const name = $derived(textOf(p.name, lang) || p.id);
  const asked = $derived(askedOf(best));
</script>

<div class="rowp">
  <!-- Its letters: the window loads no image from the store (its CSP); the logo comes with the archive, at the install. -->
  <PluginTile id={p.id} {name} src={null} />
  <div class="mid">
    <div class="l1">
      <button class="nm" onclick={() => plugins.openStorePlugin(p.id)}><b>{name}</b></button>
      {#if best && sealed(best)}<Seal version={best.version} />{:else if best}<span class="tag">{t('plugins.tag.unverified')}</span>{/if}
      <span class="by">{[p.author, count(p.installs, lang)].filter(Boolean).join(' · ')}</span>
    </div>
    <div class="d">{textOf(p.description, lang)}</div>
    <div class="asks">
      {#each asked as a, i (i)}<span class:s={a.kind === 'run'} class:r={a.kind === 'decide'}
          >{#if a.kind === 'host'}<code>{a.text}</code>{:else}{a.text}{/if}</span
        >{/each}
    </div>
  </div>
  {#if mine?.update}
    <button class="btn" onclick={() => plugins.update(mine)}>{t('plugins.updates.update')}</button>
  {:else if mine}
    <span class="note">✓ {t('plugins.store.installed')}</span>
  {:else if best}
    <button class="btn" onclick={() => plugins.fromStore(p.id)}>{t('plugins.store.install')}</button>
  {:else}
    <span class="note">{t('plugins.store.noCompatible')}</span>
  {/if}
</div>

<style>
  .rowp {
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr) auto;
    gap: 12px;
    align-items: center;
    padding: 11px 0;
    border-bottom: 1px solid var(--line);
  }
  .mid {
    min-width: 0;
  }
  .l1 {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .nm {
    padding: 0;
    border: none;
    background: none;
    color: var(--text);
    font: inherit;
    cursor: pointer;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .nm b {
    font-size: 13px;
    font-weight: 650;
  }
  .nm:hover b {
    text-decoration: underline;
  }
  .by {
    color: var(--muted);
    font-size: 11.5px;
    white-space: nowrap;
  }
  .d {
    color: var(--soft);
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .asks {
    display: flex;
    gap: 12px;
    color: var(--muted);
    font-size: 11.5px;
    flex-wrap: wrap;
  }
  .asks .s {
    color: var(--soft);
    font-weight: 550;
  }
  .asks .r {
    color: var(--del);
  }
  .asks code {
    font:
      11px ui-monospace,
      Consolas,
      monospace;
  }
  .tag {
    font-size: 11px;
    padding: 1px 7px;
    border-radius: 5px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
    white-space: nowrap;
  }
  .note {
    color: var(--muted);
    font-size: 11.5px;
    white-space: nowrap;
  }
</style>
