<script lang="ts">
  import { fDate } from '../../lib/format';
  import { locale, t } from '../../lib/i18n';
  import { activeIn, asksOf, categoriesOf, contributionsOf, originOf, shortPrint, type PluginView } from '../../lib/plugins';
  import { plugins, type PluginTab } from '../../lib/plugins-view.svelte';
  import { app } from '../../lib/state.svelte';
  import Icon from '../Icon.svelte';
  import AskList from './AskList.svelte';
  import PluginJournal from './PluginJournal.svelte';
  import PluginProjects from './PluginProjects.svelte';
  import PluginSettings from './PluginSettings.svelte';
  import PluginTile from './PluginTile.svelte';
  import Seal from './Seal.svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { stateText } from './phase';
  import { tip } from '../../lib/tip';

  // A plugin's page (choice 216): its tile, its name and its tags, who made it and what it is, the update with what it is
  // worth; then its tabs; on the right, always, what it can do, where it comes from, « Désactiver » and « Désinstaller ».
  let { plugin, tab }: { plugin: PluginView; tab: PluginTab } = $props();

  const TABS: PluginTab[] = ['overview', 'permissions', 'versions', 'projects', 'settings', 'journal'];
  const lang = $derived(locale.ui);
  const m = $derived(plugin.manifest);
  const asks = $derived(asksOf(plugin.asks, m, lang));
  const adds = $derived(contributionsOf(m, lang));
  const category = $derived(categoriesOf(plugin)[0]);
  const onCount = $derived(app.projects.filter((p) => activeIn(plugin, p)).length);
  const dev = $derived(plugin.origin.kind === 'dev');
  const settingsCount = $derived(((m?.contributes.settings as unknown[] | undefined)?.length ?? 0) + plugin.secretKeys.length);
  const state = $derived(stateText(plugin));
  const reason = $derived(plugin.state.kind === 'ready' ? null : plugin.state.reason);
  // A plugin of the store (STORE-9): its chain checked at the install, the review's summary from the catalogue.
  const store = $derived(plugin.origin.kind === 'store' ? plugin.origin : null);
  const listed = $derived(plugins.catalogPlugin(plugin.id)?.versions.find((v) => v.version === plugin.version) ?? null);
  const summary = $derived(listed?.review?.summary ? (lang === 'fr' ? listed.review.summary.fr : listed.review.summary.en) : null);
  const unverified = $derived(plugin.withdrawn?.action === 'unverify' ? plugin.withdrawn : null);
  const storePage = $derived(
    `${plugins.store?.url ?? 'https://store.escouade.dev'}${lang === 'fr' ? '' : '/en'}/plugins/${encodeURIComponent(plugin.id)}`,
  );

  function counted(k: PluginTab): number | null {
    if (k === 'permissions') return plugin.asks.length + plugin.commands.length;
    if (k === 'settings') return settingsCount || null;
    return null;
  }

  function uninstall() {
    app.modal = {
      kind: 'confirm',
      title: t('plugins.uninstall.title', { name: plugin.name }),
      body: dev ? t('plugins.uninstall.bodyDev') : t('plugins.uninstall.body'),
      confirm: t('plugins.uninstall.confirm'),
      danger: true,
      option: { label: t('plugins.uninstall.keep'), value: false },
      onConfirm: (keep) => plugins.remove(plugin.id, keep),
    };
  }
</script>

<div class="page">
  <div class="phead">
    <button class="back" onclick={() => plugins.show({ kind: 'installed' })}
      ><Icon name="down" size={12} />{t('plugins.page.back')}{#if category}<span> · {t(`plugins.cat.${category}`)}</span>{/if}</button
    >
    <div class="top">
      <PluginTile id={plugin.id} name={plugin.name} installed size={64} />
      <div class="who">
        <h2>
          {plugin.name}
          {#if plugin.verified}
            <Seal version={plugin.version} long />
          {:else}
            <span class="ptag">{dev ? t('plugins.tag.dev') : t('plugins.tag.unverified')}</span>
            {#if plugin.version}<span class="ptag mono">{t('plugins.version', { version: plugin.version })}</span>{/if}
          {/if}
          {#if state}<span class="ptag" class:bad={plugin.state.kind !== 'ready'}>{state}</span>{/if}
        </h2>
        <div class="meta">
          {#if m?.author}<span class="soft">{m.author}</span>{/if}
          {#if category}<span>{t(`plugins.cat.${category}`)}</span>{/if}
          {#if m?.license}<span>{m.license}</span>{/if}
          <span class="mono">{plugin.id}</span>
        </div>
        <p class="desc">{plugin.description || t('plugins.page.noDescription')}</p>
      </div>
      <div class="act">
        {#if plugin.update}
          <button class="btn primary" onclick={() => plugins.update(plugin)}
            >{t('plugins.page.updateTo', { version: plugin.update.version })}</button
          >
          <small class="upd"
            >{plugin.verified
              ? t('plugins.page.installedVerified', { version: plugin.version })
              : t('plugins.page.installedNote', { version: plugin.version })}
            {plugin.update.verified === false
              ? t('plugins.page.updateUnverified', { version: plugin.update.version })
              : plugin.update.verified
                ? t('plugins.page.updateVerified', { version: plugin.update.version })
                : t('plugins.page.updateNote', { version: plugin.update.version })}</small
          >
        {:else if plugin.inStore}
          <button class="btn primary" onclick={() => plugins.fromStore(plugin.id)}
            >{t('plugins.page.toStore', { version: plugin.inStore.version })}</button
          >
          <small class="note"
            >{plugin.inStore.verified
              ? t('plugins.page.inStoreVerified', { version: plugin.inStore.version })
              : t('plugins.page.inStore', { version: plugin.inStore.version })}</small
          >
        {/if}
      </div>
    </div>
    {#if reason}<p class="reason" role="status">{reason}</p>{/if}
    {#if unverified}<p class="reason amber" role="status">
        {t('plugins.withdrawn.unverified', { version: unverified.version, date: unverified.date, reason: unverified.reason })}
      </p>{/if}
  </div>
  <div class="tabs" role="tablist" aria-label={t('plugins.page.tabs.label')}>
    {#each TABS as k (k)}
      <button role="tab" aria-selected={tab === k} class:on={tab === k} onclick={() => plugins.openPlugin(plugin.id, k)}
        >{t(`plugins.page.tabs.${k}`)}{#if counted(k)}<em>{counted(k)}</em>{/if}{#if k === 'projects'}<em
            >{onCount} / {app.projects.length}</em
          >{/if}</button
      >
    {/each}
  </div>
  <div class="pgrid">
    <div class="main" role="tabpanel" aria-label={t(`plugins.page.tabs.${tab}`)}>
      {#if tab === 'overview'}
        <section class="box">
          <h5>{t('plugins.page.adds')}</h5>
          {#if adds.length}
            <ul class="adds">
              {#each adds as line (line)}<li><Icon name="sparkle" size={14} /><span>{line}</span></li>{/each}
            </ul>
          {:else}
            <p class="hint">{t('plugins.page.addsNothing')}</p>
          {/if}
        </section>
        <section class="box">
          <h5>{t('plugins.page.tabs.projects')}</h5>
          <PluginProjects {plugin} />
        </section>
      {:else if tab === 'permissions'}
        <p class="hint">{t('plugins.perms.intro')}</p>
        {#if asks.length}
          <div class="box">
            <AskList {asks} revoked={plugin.choices.revoked} onToggle={(w, granted) => plugins.setPermission(plugin.id, w, granted)} />
          </div>
        {:else}
          <p class="hint">{t('plugins.perms.none')}</p>
        {/if}
        {#if plugin.commands.length}
          <!-- The commands it runs on facts (PLG-5): agreed to as they are, as the install window agrees to the plugin. -->
          <section class="box cmds" aria-label={t('plugins.cmds.title')}>
            <h5>{t('plugins.cmds.title')}</h5>
            <ul>
              {#each plugin.commands as line, i (i)}<li><Icon name="bash" size={14} /><code class="mono">{line}</code></li>{/each}
            </ul>
            {#if plugin.commandsApproved}
              <p class="hint">{t('plugins.cmds.agreed')}</p>
              <button class="btn" onclick={() => plugins.approveCommands(plugin.id, false)}>{t('plugins.cmds.takeBack')}</button>
            {:else}
              <p class="notice"><Icon name="info" size={14} /><span>{t('plugins.cmds.waiting')}</span></p>
              <button class="btn primary" onclick={() => plugins.approveCommands(plugin.id, true)}>{t('plugins.cmds.agree')}</button>
            {/if}
          </section>
        {/if}
      {:else if tab === 'versions'}
        <div class="vers">
          {#if plugin.update}
            <div>
              <span class="vn mono">{plugin.update.version}</span><span class="vd">{t('plugins.versions.available')}</span><button
                class="btn"
                onclick={() => plugins.update(plugin)}>{t('plugins.updates.update')}</button
              >
            </div>
          {/if}
          <div>
            <span class="vn mono">{plugin.version || '—'}</span><span class="vd"
              >{plugin.installedAt ? t('plugins.versions.installed', { date: fDate(Date.parse(plugin.installedAt)) }) : ''}</span
            ><span class="mono print">{plugin.fingerprint ? shortPrint(plugin.fingerprint) : t('plugins.origin.devPrint')}</span>
          </div>
        </div>
        {#if !dev}
          <button class="btn check" disabled={plugins.checking} onclick={() => plugins.checkUpdates()}
            >{plugins.checking ? t('plugins.updates.checking') : t('plugins.versions.check')}</button
          >
          {#if !plugin.update && !plugins.checking}<span class="hint">{t('plugins.versions.upToDate')}</span>{/if}
        {/if}
      {:else if tab === 'projects'}
        <PluginProjects {plugin} />
      {:else if tab === 'settings'}
        <PluginSettings {plugin} />
      {:else}
        <PluginJournal {plugin} />
      {/if}
      {#if plugins.error && tab !== 'settings'}<p class="err" role="alert">{plugins.error}</p>{/if}
    </div>
    <aside class="pp-aside">
      <section class="box">
        <h5>{t('plugins.page.canDo')}</h5>
        {#if asks.length}<AskList {asks} revoked={plugin.choices.revoked} />{:else}<p class="hint">{t('plugins.perms.none')}</p>{/if}
      </section>
      {#if plugin.verified}
        <section class="box review">
          <p>
            <b>{t('plugins.store.reviewed', { date: fDate(Date.parse(listed?.review?.signed_at ?? plugin.installedAt)) })}</b>
            {summary ?? ''}
          </p>
        </section>
      {/if}
      <section class="box">
        <h5>{t('plugins.page.from')}</h5>
        {#if store}
          <div class="chain">
            <span class="k">{t('plugins.origin.repo')}</span><span class="v mono" use:tip={{ text: store.repository, aria: false }}
              >{store.repository.replace(/^https:\/\/github\.com\//, '')}</span
            >
            <span class="k">{t('plugins.origin.tagLabel')}</span><span class="v mono"
              >{store.tag} → {store.commit.slice(0, 7)} · {t('plugins.origin.archiveSame')}</span
            >
            <span class="k">{t('plugins.origin.fingerprint')}</span><span class="v mono" use:tip={{ text: plugin.fingerprint, aria: false }}
              >{shortPrint(plugin.fingerprint)}</span
            >
            <span class="k">{t('plugins.origin.signature')}</span>{#if plugin.verified}<span class="v ok"
                >{t('plugins.origin.signedHere')}</span
              >{:else}<span class="v no">{t('plugins.origin.storeUnsigned')}</span>{/if}
          </div>
        {:else}
          <div class="chain">
            <span class="k">{t(`plugins.origin.${plugin.origin.kind}`)}</span><span
              class="v mono"
              use:tip={{ text: originOf(plugin.origin), aria: false }}>{originOf(plugin.origin)}</span
            >
            <span class="k">{t('plugins.origin.fingerprint')}</span><span
              class="v mono"
              use:tip={{ text: plugin.fingerprint || t('plugins.origin.devPrint'), aria: false }}
              >{plugin.fingerprint ? shortPrint(plugin.fingerprint) : t('plugins.origin.devPrint')}</span
            >
            <span class="k">{t('plugins.origin.signature')}</span><span class="v no">{t('plugins.origin.unsigned')}</span>
          </div>
        {/if}
      </section>
      {#if !plugin.choices.enabled}<p class="hint">{t('plugins.page.offNote')}</p>{/if}
      <div class="btns">
        <button class="btn" onclick={() => plugins.setEnabled(plugin.id, !plugin.choices.enabled)}
          >{plugin.choices.enabled ? t('plugins.page.disable') : t('plugins.page.enable')}</button
        >
        <button class="btn" onclick={uninstall}>{t('plugins.page.uninstall')}</button>
        {#if store || plugin.inStore}<button class="btn report" onclick={() => openUrl(storePage)}>{t('plugins.store.report')}</button>{/if}
      </div>
    </aside>
  </div>
</div>

<style>
  .page {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: auto;
  }
  .phead {
    padding: 16px 26px 12px;
    display: grid;
    gap: 10px;
  }
  .back {
    justify-self: start;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0;
    border: none;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .back :global(.icon) {
    transform: rotate(90deg);
  }
  .back:hover {
    color: var(--text);
  }
  .top {
    display: grid;
    grid-template-columns: 64px minmax(0, 1fr) auto;
    gap: 16px;
    align-items: start;
  }
  h2 {
    margin: 0;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    font-size: 19px;
    font-weight: 650;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    margin-top: 4px;
    color: var(--muted);
    font-size: 12px;
  }
  .meta .soft {
    color: var(--soft);
  }
  .meta .mono {
    font-size: 11px;
  }
  .desc {
    margin: 8px 0 0;
    color: var(--soft);
    font-size: 12.5px;
  }
  .act {
    display: grid;
    gap: 6px;
    justify-items: end;
  }
  .upd {
    max-width: 300px;
    text-align: right;
    color: var(--warn-ink);
    font-size: 11.5px;
  }
  /* A narrow panel: the action goes under the title, the description keeps its room. */
  @container plugins (max-width: 900px) {
    .top {
      grid-template-columns: 64px minmax(0, 1fr);
    }
    .act {
      grid-column: 2;
      justify-items: start;
    }
    .act:empty {
      display: none;
    }
    .upd {
      max-width: none;
      text-align: left;
    }
  }
  .reason {
    margin: 0;
    padding: 8px 10px;
    border-radius: 8px;
    background: color-mix(in oklch, var(--del) 8%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--del) 30%, transparent);
    color: var(--soft);
    font-size: 12px;
    white-space: pre-wrap;
  }
  .reason.amber {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .note {
    max-width: 300px;
    text-align: right;
    color: var(--muted);
    font-size: 11.5px;
  }
  .box.review {
    background: color-mix(in oklch, var(--brand) 7%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--brand) 35%, transparent);
  }
  .box.review p {
    margin: 0;
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .chain .v.ok {
    color: var(--teal-ink);
    font-size: 12px;
  }
  .btn.report {
    margin-left: auto;
  }
  .ptag {
    font-size: 11px;
    font-weight: 500;
    padding: 1px 7px;
    border-radius: 5px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
    white-space: nowrap;
  }
  .ptag.bad {
    color: var(--del);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--del) 45%, transparent);
  }
  .tabs {
    display: flex;
    gap: 2px;
    padding: 0 26px;
    border-bottom: 1px solid var(--line);
  }
  .tabs button {
    padding: 9px 12px;
    border: none;
    border-bottom: 2px solid transparent;
    background: none;
    color: var(--muted);
    font: inherit;
    font-weight: 550;
    cursor: pointer;
  }
  .tabs button:hover {
    color: var(--text);
  }
  .tabs button.on {
    color: var(--text);
    border-bottom-color: var(--p);
  }
  .tabs em {
    margin-left: 5px;
    font-style: normal;
    font-weight: 450;
    color: var(--muted);
  }
  .pgrid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 340px;
    gap: 26px;
    padding: 18px 26px 22px;
    align-items: start;
  }
  @container plugins (max-width: 900px) {
    .pgrid {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .main {
    display: grid;
    gap: 14px;
    align-content: start;
    min-width: 0;
  }
  .pp-aside {
    display: grid;
    gap: 12px;
  }
  .box {
    border-radius: 10px;
    box-shadow: inset 0 0 0 1px var(--line);
    padding: 12px 14px;
    display: grid;
    gap: 10px;
  }
  h5 {
    margin: 0;
    font-size: 10.5px;
    letter-spacing: 0.06em;
    color: var(--muted);
    text-transform: uppercase;
  }
  .adds {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 6px;
  }
  .adds li {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    gap: 8px;
    font-size: 12.5px;
    color: var(--soft);
  }
  .adds :global(.icon) {
    margin-top: 2px;
    color: var(--muted);
  }
  .cmds ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 6px;
  }
  .cmds li {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    gap: 8px;
    align-items: start;
  }
  .cmds li :global(.icon) {
    margin-top: 2px;
    color: var(--muted);
  }
  .cmds code {
    font-size: 11.5px;
    color: var(--text);
    overflow-wrap: anywhere;
  }
  .cmds .btn {
    justify-self: start;
  }
  .cmds .notice {
    margin: 0;
    display: flex;
    gap: 8px;
    align-items: flex-start;
    padding: 9px 11px;
    border-radius: 9px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--soft);
    font-size: 12px;
    line-height: 1.5;
  }
  .cmds .notice :global(.icon) {
    margin-top: 2px;
    color: var(--warn-ink);
  }
  .chain {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    gap: 6px 12px;
    font-size: 12px;
  }
  .chain .k {
    color: var(--dim);
  }
  .chain .v {
    color: var(--soft);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 11px;
  }
  .chain .v.no {
    color: var(--warn-ink);
    font-family: inherit;
    font-size: 12px;
  }
  .btns {
    display: flex;
    gap: 8px;
  }
  .vers {
    display: grid;
  }
  .vers > div {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 9px 0;
    border-bottom: 1px solid var(--line);
    font-size: 12.5px;
  }
  .vn {
    min-width: 60px;
    font-weight: 600;
  }
  .vd {
    color: var(--muted);
    flex: 1;
  }
  .print {
    color: var(--muted);
    font-size: 11px;
  }
  .check {
    justify-self: start;
  }
  .hint {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .err {
    margin: 0;
    color: var(--del);
    font-size: 12px;
  }
</style>
