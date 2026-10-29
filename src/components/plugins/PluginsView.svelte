<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { categoriesOf, type PluginView } from '../../lib/plugins';
  import { plugins } from '../../lib/plugins-view.svelte';
  import Icon from '../Icon.svelte';
  import InstallModal from './InstallModal.svelte';
  import PluginPage from './PluginPage.svelte';
  import PluginRows from './PluginRows.svelte';
  import PluginTile from './PluginTile.svelte';

  // The view Plugins (PLG-4; choices 213 to 218, design/plugins/): opened by its icon of the projects' rail, in place of a
  // project, its sidebar beside it (PluginsSide). Its pages on the work's panel; the install window over it.
  onMount(() => {
    void plugins.load();
  });

  const page = $derived(plugins.page);
  const builtin = (p: PluginView) => p.id.startsWith('dev.escouade.');
  const added = $derived(plugins.list.filter((p) => !builtin(p)));
  const shipped = $derived(plugins.list.filter(builtin));
  const current = $derived(page.kind === 'plugin' ? plugins.byId(page.id) : null);
</script>

<div class="plugins-work">
  {#if page.kind === 'plugin'}
    {#if current}
      <PluginPage plugin={current} tab={page.tab} />
    {:else if plugins.loaded}
      <div class="empty"><p>{t('plugins.page.gone')}</p></div>
    {/if}
  {:else}
    <div class="pane">
      <div class="head">
        <div class="row">
          {#if page.kind === 'store'}
            <h2>{t('plugins.store.title')}</h2>
            <span class="sub">{t('plugins.store.sub')}</span>
          {:else if page.kind === 'installed'}
            <h2>{t('plugins.installed.title')}</h2>
            <span class="sub"
              >{shipped.length
                ? t('plugins.installed.sub', { count: plugins.list.length, added: String(added.length), builtin: String(shipped.length) })
                : t('plugins.installed.subAdded', { count: added.length })}</span
            >
          {:else if page.kind === 'updates'}
            <h2>{t('plugins.updates.title')}</h2>
            <span class="sub">{t('plugins.updates.sub')}</span>
          {:else if page.kind === 'dev'}
            <h2>{t('plugins.dev.title')}</h2>
            <span class="sub">{t('plugins.dev.sub')}</span>
          {:else}
            <h2>{t(`plugins.cat.${page.category}`)}</h2>
          {/if}
          <span class="grow"></span>
          {#if page.kind === 'installed' && plugins.updates.length}
            <button class="btn warn" onclick={() => plugins.updateAll()}
              >{t('plugins.installed.updateAll', { count: plugins.updates.length })}</button
            >
          {/if}
          {#if page.kind === 'updates'}
            <button class="btn" disabled={plugins.checking} onclick={() => plugins.checkUpdates()}
              >{plugins.checking ? t('plugins.updates.checking') : t('plugins.updates.check')}</button
            >
          {:else if page.kind === 'dev'}
            <button class="btn" onclick={() => plugins.fromFolder(true)}><Icon name="folder" size={14} />{t('plugins.loadDev')}</button>
          {:else}
            <button class="btn" onclick={() => plugins.fromFolder()}><Icon name="folder" size={14} />{t('plugins.fromFolder')}</button>
            <button class="btn" onclick={() => plugins.fromGit()}><Icon name="branch" size={14} />{t('plugins.fromGit')}</button>
          {/if}
        </div>
        {#if plugins.error}<p class="err" role="alert">{plugins.error}</p>{/if}
      </div>
      <div class="body">
        {#if page.kind === 'store'}
          <div class="soon">
            <Icon name="sparkle" size={20} />
            <div>
              <b>{t('plugins.store.soon')}</b>
              <p>{t('plugins.store.soonHint')}</p>
              <button class="btn" onclick={() => openUrl('https://store.escouade.dev')}
                ><Icon name="ext" size={14} />{t('plugins.store.open')}</button
              >
            </div>
          </div>
        {:else if page.kind === 'installed'}
          {#if !plugins.list.length && plugins.loaded}
            <div class="soon">
              <Icon name="puzzle" size={20} />
              <div>
                <b>{t('plugins.installed.empty')}</b>
                <p>{t('plugins.installed.emptyHint')}</p>
              </div>
            </div>
          {:else}
            {#if added.length}<PluginRows rows={added} group={t('plugins.installed.added')} />{/if}
            {#if shipped.length}<PluginRows
                rows={shipped}
                group={t('plugins.installed.builtin')}
                note={t('plugins.installed.builtinNote')}
              />{/if}
          {/if}
        {:else if page.kind === 'updates'}
          {#if plugins.updates.length}
            <div class="ups">
              {#each plugins.updates as p (p.id)}
                <div class="up">
                  <PluginTile id={p.id} name={p.name} installed />
                  <div>
                    <b>{p.name}</b><span class="mono">{t('plugins.updates.row', { from: p.version, to: p.update?.version ?? '' })}</span>
                  </div>
                  <button class="btn" onclick={() => plugins.openPlugin(p.id, 'versions')}>{t('plugins.page.tabs.versions')}</button>
                  <button class="btn primary" onclick={() => plugins.update(p)}>{t('plugins.updates.update')}</button>
                </div>
              {/each}
            </div>
          {:else}
            <p class="hint">{t('plugins.updates.empty')}</p>
          {/if}
        {:else if page.kind === 'dev'}
          {#if plugins.dev.length}
            <PluginRows rows={plugins.dev} />
          {:else}
            <div class="soon">
              <Icon name="code" size={20} />
              <div>
                <b>{t('plugins.dev.empty')}</b>
                <p>{t('plugins.dev.emptyHint')}</p>
              </div>
            </div>
          {/if}
        {:else}
          {@const rows = plugins.list.filter((p) => categoriesOf(p).includes(page.category))}
          {#if rows.length}<PluginRows {rows} />{:else}<p class="hint">{t('plugins.installed.categoryEmpty')}</p>{/if}
        {/if}
      </div>
    </div>
  {/if}
</div>

{#if plugins.install}<InstallModal step={plugins.install} />{/if}

<style>
  .plugins-work {
    flex: 1;
    min-width: 0;
    display: flex;
    container: plugins / inline-size;
  }
  .pane {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: auto;
  }
  .head {
    padding: 18px 26px 14px;
    border-bottom: 1px solid var(--line);
    display: grid;
    gap: 10px;
  }
  .row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px 12px;
  }
  h2 {
    margin: 0;
    font-size: 19px;
    font-weight: 650;
  }
  .sub {
    color: var(--muted);
    font-size: 12px;
  }
  .grow {
    flex: 1;
  }
  .btn.warn {
    color: var(--warn-ink);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .body {
    padding: 8px 26px 22px;
    display: grid;
    gap: 4px;
    align-content: start;
  }
  .soon {
    margin-top: 14px;
    display: grid;
    grid-template-columns: 24px minmax(0, 1fr);
    gap: 12px;
    max-width: 640px;
    padding: 16px 18px;
    border-radius: 11px;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .soon :global(.icon) {
    color: var(--muted);
  }
  .soon b {
    font-weight: 600;
  }
  .soon p {
    margin: 6px 0 12px;
    color: var(--soft);
    font-size: 12.5px;
  }
  .ups {
    display: grid;
  }
  .up {
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr) auto auto;
    gap: 12px;
    align-items: center;
    padding: 11px 0;
    border-bottom: 1px solid var(--line);
  }
  .up b {
    display: block;
    font-weight: 600;
  }
  .up .mono {
    color: var(--warn-ink);
    font-size: 11.5px;
  }
  .hint {
    margin: 14px 0 0;
    color: var(--muted);
    font-size: 12.5px;
  }
  .err {
    margin: 0;
    color: var(--del);
    font-size: 12px;
    white-space: pre-wrap;
  }
  .empty {
    flex: 1;
    display: grid;
    place-items: center;
    color: var(--muted);
  }
</style>
