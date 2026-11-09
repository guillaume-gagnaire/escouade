<script lang="ts">
  import { onMount } from 'svelte';
  import { fTime } from '../../lib/format';
  import { locale, t } from '../../lib/i18n';
  import { markOf, type PluginView, type SeedOffer } from '../../lib/plugins';
  import { addsOf, hasSettings, reachOf } from '../../lib/plugins-settings';
  import { plugins } from '../../lib/plugins-view.svelte';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import { stateText } from '../plugins/phase';
  import Icon from '../Icon.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';

  // Réglages › Plugins (direction A, design/settings-2-2/index.html#a-plugins): the installed plugins, each with what it
  // adds and where that is set, on how many projects it is on, its seal, « Réglages » (its page in the view, when it has
  // settings or secrets) and its switch; the plugins of the app's seed not installed, each installed in a click from the
  // seed, with no network and no window (ESC-373); « N'installer que des versions vérifiées ». The view Plugins stays
  // where one installs from a folder or a repository, takes a permission back and reads a journal.
  onMount(() => {
    void plugins.load();
    if (!plugins.store) void plugins.loadStore();
  });

  const lang = $derived(locale.ui);
  const list = $derived(plugins.store?.revocations ?? null);

  /** Its page in the view Plugins, the settings window closed. */
  function openInView(id?: string) {
    app.modal = null;
    app.openPlugins();
    if (id) plugins.openPlugin(id, 'settings');
    else plugins.show({ kind: 'installed' });
  }

  /** What it adds and where, how far it reaches, and what stops it (disabled by Escouade, its manifest broken). */
  function lineOf(p: PluginView): string {
    const broken = p.state.kind !== 'ready' ? stateText(p) : null;
    return [addsOf(p, lang), reachOf(p, app.projects), broken].filter(Boolean).join(' · ');
  }

  const why = (o: SeedOffer) =>
    o.refused
      ? t('plugins.installed.seedRefused', { reason: o.refused })
      : o.removed !== null
        ? t('plugins.installed.seedRemoved', { version: o.removed })
        : t('plugins.installed.seedMissing');

  let installing = $state<string | null>(null);
  async function install(o: SeedOffer) {
    installing = o.id;
    await plugins.installSeed(o.id);
    installing = null;
  }

  const revocations = $derived(
    list?.serial
      ? t('plugins.installed.revocations', { time: list.readAt ? fTime(Date.parse(list.readAt)) : '—', serial: String(list.serial) })
      : t('plugins.installed.revocationsNone'),
  );
</script>

<Group title={t('plugins.installed.title')} aria-label={t('plugins.installed.title')}>
  {#snippet action()}
    <button class="btn" onclick={() => openInView()}><Icon name="puzzle" size={14} />{t('plugins.settingsPage.open')}</button>
  {/snippet}
  {#if plugins.error}<p class="err" role="alert">{plugins.error}</p>{/if}
  {#if plugins.loaded && !plugins.list.length}
    <p class="note">{t('plugins.settingsPage.none')}</p>
  {/if}
  {#each plugins.list as p (p.id)}
    <div class="item" class:off={!p.choices.enabled || p.state.kind !== 'ready'} role="group" aria-label={p.name}>
      <span class="mark" aria-hidden="true">{markOf(p.name)}</span>
      <div class="who">
        <b
          >{p.name}{#if p.version}<span class="v num">{t('plugins.version', { version: p.version })}</span>{/if}</b
        >
        <span use:tip={{ text: lineOf(p), aria: false }}>{lineOf(p)}</span>
      </div>
      <div class="side">
        {#if p.verified}
          <span class="seal" use:tip={t('plugins.settingsPage.verifiedTip', { version: p.version })}
            ><Icon name="shield" size={13} />{t('plugins.settingsPage.verified')}</span
          >
        {/if}
        {#if hasSettings(p)}
          <button class="btn small" aria-label={t('plugins.settingsPage.settingsOf', { name: p.name })} onclick={() => openInView(p.id)}
            >{t('plugins.settingsPage.settings')}</button
          >
        {/if}
        <button
          class="switch"
          class:on={p.choices.enabled}
          role="switch"
          aria-checked={p.choices.enabled}
          aria-label={t('plugins.installed.appSwitch', { name: p.name })}
          onclick={() => plugins.setEnabled(p.id, !p.choices.enabled)}
        ></button>
      </div>
    </div>
  {/each}
</Group>

{#if plugins.offers.length}
  <Group title={t('plugins.installed.seedTitle')} note={t('plugins.settingsPage.seedNote')} aria-label={t('plugins.installed.seedTitle')}>
    {#each plugins.offers as o (o.id)}
      <div class="item" role="group" aria-label={o.name}>
        <span class="mark" aria-hidden="true">{markOf(o.name)}</span>
        <div class="who">
          <b>{o.name}</b>
          <span class:bad={!!o.refused}>{why(o)}</span>
        </div>
        <div class="side">
          <button
            class="btn small"
            aria-label={t('plugins.settingsPage.installOf', { name: o.name })}
            disabled={installing !== null}
            onclick={() => install(o)}>{t('plugins.settingsPage.install')}</button
          >
        </div>
      </div>
    {/each}
  </Group>
{/if}

<Group title={t('plugins.settingsPage.security')} aria-label={t('plugins.settingsPage.security')}>
  <Row label={t('plugins.installed.verifiedOnly')} desc={revocations}>
    <button
      class="switch"
      class:on={!!settingsForm.settings.pluginsVerifiedOnly}
      role="switch"
      aria-checked={!!settingsForm.settings.pluginsVerifiedOnly}
      aria-label={t('plugins.installed.verifiedOnly')}
      onclick={() => (settingsForm.settings.pluginsVerifiedOnly = !settingsForm.settings.pluginsVerifiedOnly)}
    ></button>
  </Row>
</Group>

<style>
  /* A plugin: a row with its mark in front (`.item` of the preview), what it adds under its name, its actions on the right. */
  .item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 11px 0;
  }
  .item + .item {
    border-top: 1px solid rgba(var(--fg-rgb), 0.05);
  }
  .note + .item,
  .err + .item {
    margin-top: 4px;
  }
  .mark {
    display: inline-grid;
    flex: none;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 7px;
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    font-size: 11px;
    font-weight: 650;
  }
  .item.off .mark,
  .item.off .who {
    opacity: 0.55;
  }
  .who {
    display: grid;
    flex: 1;
    gap: 2px;
    min-width: 0;
  }
  .who b {
    font-weight: 550;
  }
  .who .v {
    margin-left: 5px;
    color: var(--muted);
    font-weight: 400;
  }
  .who > span {
    overflow: hidden;
    color: var(--muted);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .who > span.bad {
    color: var(--del);
  }
  .side {
    display: flex;
    flex: none;
    align-items: center;
    gap: 10px;
  }
  .seal {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--brand);
    font-size: 11.5px;
    font-weight: 550;
  }
  .note {
    max-width: 64ch;
    margin: 4px 0 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }
  .err {
    margin: 6px 0 0;
    color: var(--del);
    font-size: 12px;
  }
</style>
