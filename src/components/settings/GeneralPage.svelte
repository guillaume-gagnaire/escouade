<script lang="ts">
  import { onMount } from 'svelte';
  import { THEMES, themeChoice } from '../../lib/appearance';
  import { DEFAULT_FONT_SIZE, FONT_SIZE_MAX, FONT_SIZE_MIN, fontSize, setDisplay, ZOOM_LEVELS, zoomState } from '../../lib/display.svelte';
  import { LANG_NAMES, t, type Lang } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { openUrl } from '../../lib/opener';
  import { keyLabel } from '../../lib/platform';
  import { pluginThemes } from '../../lib/plugin-theme.svelte';
  import { themeLabel } from '../../lib/plugin-themes';
  import { plugins } from '../../lib/plugins-view.svelte';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import { privacyUrl } from '../../lib/telemetry';
  import { checkForUpdate, postponeRestart, restartIn, shownVersion } from '../../lib/updater';
  import Markdown from '../conv/Markdown.svelte';
  import Icon from '../Icon.svelte';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';

  // The page « Général » (PROPOSITION.md §2, design/settings-2-2/index.html#a-general):
  // - « Apparence » (the theme, the colours, the zoom, the font size of the terminals), applied and saved as it changes,
  //   apart from the draft: its shortcuts change it too, the modal open or not;
  // - « Langue », the interface's and that of the texts the agents write, applied at once (the backend tells the window,
  //   the menus and its own texts);
  // - « Mises à jour » and « Toi » (the name, the statistics), which were in « À propos » and « Application »;
  // - « Avancé »: the editor's font size, the id.
  const s = $derived(settingsForm.settings);
  // « Ton nom » left empty is filled with git's (ESC-243), unless emptied on purpose.
  onMount(
    () =>
      void api.gitUserName().then(
        (git) => settingsForm.prefillName(git),
        () => {},
      ),
  );
  /** The field shows the name taken from git: its help says so. */
  const fromGit = $derived(!!settingsForm.gitName && s.userName?.trim() === settingsForm.gitName);
  /** Each language named in itself, whatever the interface's: who does not read it still finds their own. */
  const own = (['en', 'fr'] as const).map((l: Lang) => ({ value: l, label: LANG_NAMES[l], lang: l }));
  /** The system's language is named in the interface's: « Système (Anglais) », « System (English) ». */
  const uiOptions = $derived([
    { value: 'system' as const, label: t('settings.app.system', { lang: t(`settings.app.langName.${app.lang.system}`) }) },
    ...own,
  ]);
  const claudeOptions = $derived([{ value: 'ui' as const, label: t('settings.app.sameAsUi') }, ...own]);
  /** « Thème »: Système, Clair, Sombre. */
  const themeOptions = $derived(
    THEMES.map((v) => ({
      value: v,
      label: { system: t('settings.app.themeSystem'), light: t('settings.app.themeLight'), dark: t('settings.app.themeDark') }[v],
    })),
  );
  /** « Couleurs » (PLG-7): the themes of the plugins on, the one chosen while it is among them. */
  const themes = $derived(pluginThemes());
  const chosen = $derived(themes.find((th) => th.key === app.settings.pluginTheme) ?? null);
  /** Chosen, but its plugin off or gone: the window shows Escouade's colours, and says why. */
  const gone = $derived(plugins.loaded && !!app.settings.pluginTheme && !chosen ? app.settings.pluginTheme.split('/')[0] : null);
  /** « Voir les thèmes »: the view Plugins, on its category of themes. */
  function findThemes() {
    app.modal = null;
    plugins.page = { kind: 'category', category: 'themes' };
    app.openPlugins();
  }
  const SEGMENT_ZOOMS: number[] = [90, 100, 110, 125, 150];
  const percent = (n: number) => t('settings.app.percent', { n });
  /**
   * The usual steps in segments, 90 to 150 % (choice 35 of the 2.0 design), and the one chosen when it is another (80,
   * 175 or 200 %, which the keys reach); the steps the window cannot hold are not offered (`zoomState.fit`).
   */
  const zoomOptions = $derived(
    ZOOM_LEVELS.filter((z) => SEGMENT_ZOOMS.includes(z) || z === (app.settings.uiZoom ?? 100)).map((z) => ({
      value: z as number,
      label: percent(z),
      disabled: z > zoomState.fit,
      title: z > zoomState.fit ? t('settings.app.zoomTooBig') : undefined,
    })),
  );
  /** The zoom chosen, when the window is too small for it: what it is shown at meanwhile. */
  const held = $derived((app.settings.uiZoom ?? 100) > zoomState.fit);
  const zoomKeys = $derived(t('settings.app.zoomKeys', { in: keyLabel('Ctrl+='), out: keyLabel('Ctrl+-'), reset: keyLabel('Ctrl+0') }));

  /** A font size as typed, kept once the field is left (or its arrows used): held in range, shown as kept. */
  function setSize(key: 'terminalFontSize' | 'editorFontSize', input: HTMLInputElement) {
    const size = fontSize(input.value === '' ? NaN : Number(input.value));
    setDisplay({ [key]: size });
    input.value = String(size);
  }

  // The update found (ESC-250), by the button or the automatic check: downloading, then ready. A download that failed keeps
  // its own row while the same version is tried again (choice 13), never both; once ready, or a newer one found, the update's.
  const update = $derived(app.update && !app.update.ready && app.updateTrouble?.version === app.update.version ? null : app.update);
  const restartSeconds = $derived(restartIn());
  let notesOpen = $state(false);
  let checking = $state(false);
  /** A check by the button found nothing: said under the version. */
  let upToDate = $state(false);
  /** What a screen reader hears of the update as it comes: its region is there from the start, so it is told. */
  const updateLive = $derived(
    !update
      ? ''
      : update.ready
        ? t('settings.about.updateReadyLive', { version: shownVersion(update.version) })
        : t('settings.about.updateFoundLive', { version: shownVersion(update.version) }),
  );

  async function check() {
    checking = true;
    const found = await checkForUpdate(true);
    checking = false;
    upToDate = found === false;
    // A check that failed has told why: it is not "nothing new".
    if (found === false) app.toast(t('settings.about.noUpdate'), 'info');
  }

  /**
   * « Recevoir les versions bêta » (ESC-374): saved at once, as the accounts are, for the check that follows reads the
   * saved settings. Ticked, a check runs as with the button; unticked, a quiet one, which takes back a beta on offer.
   */
  async function setBetaUpdates(on: boolean) {
    app.settings.betaUpdates = on;
    const saved = await app.run(api.saveSettings($state.snapshot(app.settings)));
    if (saved === undefined) {
      app.settings.betaUpdates = !on;
      return;
    }
    if (on) await check();
    else await checkForUpdate(false);
  }
</script>

{#snippet size(key: 'terminalFontSize' | 'editorFontSize')}
  <Row
    label={t(`settings.app.${key}`)}
    hint={t('settings.app.fontSizeHint', { n: DEFAULT_FONT_SIZE })}
    desc={t('settings.app.fontSizeDesc')}
  >
    <input
      class="field input num"
      type="number"
      min={FONT_SIZE_MIN}
      max={FONT_SIZE_MAX}
      step="0.5"
      aria-label={t(`settings.app.${key}`)}
      value={app.settings[key] ?? DEFAULT_FONT_SIZE}
      onchange={(e) => setSize(key, e.currentTarget)}
    />
  </Row>
{/snippet}

<Group title={t('settings.groups.appearance')}>
  <Row label={t('settings.app.theme')} desc={t('settings.app.themeDesc')}>
    <Chips
      label={t('settings.app.theme')}
      options={themeOptions}
      bind:value={() => themeChoice(app.settings.theme), (v) => setDisplay({ theme: v })}
    />
  </Row>
  <Row label={t('settings.app.colors')} desc={themes.length ? t('settings.app.colorsDesc') : t('settings.app.colorsNone')}>
    {#if themes.length}
      <select
        class="field select"
        aria-label={t('settings.app.colors')}
        value={chosen?.key ?? ''}
        onchange={(e) => setDisplay({ pluginTheme: e.currentTarget.value })}
      >
        <option value="">{t('settings.app.colorsEscouade')}</option>
        {#each themes as th (th.key)}
          <option value={th.key}>{themeLabel(th, app.lang.ui)}</option>
        {/each}
      </select>
    {:else}
      <button class="btn" onclick={findThemes}>{t('settings.app.colorsFind')}</button>
    {/if}
  </Row>
  {#if gone}
    <p class="held" role="status">{t('settings.app.colorsGone', { plugin: gone })}</p>
  {:else if chosen?.dropped.length}
    <p class="held" role="status">{t('settings.app.colorsDropped', { count: chosen.dropped.length })}</p>
  {/if}
  <Row label={t('settings.app.zoom')} desc={`${t('settings.app.zoomDesc')} ${zoomKeys}`}>
    <Chips
      label={t('settings.app.zoom')}
      options={zoomOptions}
      bind:value={() => app.settings.uiZoom ?? 100, (z) => setDisplay({ uiZoom: z })}
    />
  </Row>
  {#if held}
    <p class="held" role="status">
      {t('settings.app.zoomHeld', { zoom: percent(app.settings.uiZoom), shown: percent(zoomState.shown) })}
    </p>
  {/if}
  {@render size('terminalFontSize')}
</Group>

<Group title={t('settings.app.language')}>
  <Row label={t('settings.app.uiLanguage')}>
    <Chips label={t('settings.app.uiLanguage')} options={uiOptions} bind:value={s.language} />
  </Row>
  <Row label={t('settings.app.claudeLanguage')} desc={t('settings.app.claudeLanguageHelp')}>
    <Chips label={t('settings.app.claudeLanguage')} options={claudeOptions} bind:value={s.claudeLanguage} />
  </Row>
</Group>

<!-- « Mises à jour » (from « À propos »): the version and its check, the automatic ones, the betas (ESC-374), and the
     update found as the status bar says it (ESC-250): downloading, then ready with its restart and its notes. -->
<Group title={t('settings.groups.updates')}>
  <Row label="Escouade {shownVersion(app.version)}" desc={upToDate && !update ? t('settings.about.noUpdate') : ''}>
    <button class="btn" disabled={checking} onclick={check}>{checking ? t('settings.about.checking') : t('settings.about.check')}</button>
    <span class="live" role="status" aria-live="polite" data-testid="update-live">{updateLive}</span>
  </Row>
  {#if update?.ready}
    {@const notes = update.notes.trim()}
    <Row label={t('settings.about.updateReady', { version: shownVersion(update.version) })} desc={t('settings.about.updateReadyDesc')}>
      {#snippet extra()}
        <div class="upd" data-update="ready">
          {#if restartSeconds !== null}
            <b class="restart-in">{t('shell.status.restartIn', { seconds: restartSeconds })}</b>
          {/if}
          {#if notes}
            <button class="link" aria-expanded={notesOpen} aria-controls="update-notes" onclick={() => (notesOpen = !notesOpen)}
              >{notesOpen ? t('settings.about.hideNotes') : t('settings.about.seeNotes')}</button
            >
            {#if notesOpen}<div class="notes" id="update-notes"><Markdown text={notes} /></div>{/if}
          {/if}
        </div>
      {/snippet}
      {#if restartSeconds !== null}
        <button class="btn" onclick={postponeRestart}>{t('common.later')}</button>
      {/if}
      <!-- The update's window, as the status bar's: it says what a restart cuts, and « Plus tard » comes back here. -->
      <button class="btn primary" onclick={() => (app.modal = { kind: 'update', back: 'general' })}>{t('shell.update.restartNow')}</button>
    </Row>
  {:else if update}
    <Row label={t('settings.about.updateFound', { version: shownVersion(update.version) })}>
      {#snippet extra()}<span class="more upd" data-update="downloading">{t('settings.about.updateDownloading')}</span>{/snippet}
    </Row>
  {/if}
  <!-- An update that does not download, said here while it lasts (choice 13 of the 2.0 design). -->
  {#if app.updateTrouble && !update}
    {@const trouble = app.updateTrouble}
    <Row label={t('settings.about.updateTrouble', { version: shownVersion(trouble.version) })}>
      {#snippet extra()}
        <span class="warn">{t('settings.about.updateTroubleDesc', { error: trouble.message })}</span>
      {/snippet}
      {#if trouble.page}
        <button class="btn" onclick={() => openUrl(trouble.page!).catch((e) => app.toast(String(e), 'error'))}
          ><Icon name="ext" size={14} />{t('shell.update.openInBrowser')}</button
        >
      {/if}
    </Row>
  {/if}
  <Row label={t('settings.about.autoUpdate')} desc={t('settings.about.autoUpdateDesc')}>
    <Switch label={t('settings.about.autoUpdate')} bind:on={s.autoUpdate} />
  </Row>
  <Row label={t('settings.about.betaUpdates')} desc={t('settings.about.betaUpdatesDesc')}>
    <Switch label={t('settings.about.betaUpdates')} bind:on={() => app.settings.betaUpdates === true, (on) => void setBetaUpdates(on)} />
  </Row>
</Group>

<!-- « Toi »: the name for the shared tickets (ESC-190, choice 113), and the statistics (from « À propos »). -->
<Group title={t('settings.groups.you')}>
  <Row label={t('share.settings.name')} desc={t('share.settings.nameDesc')}>
    {#snippet extra()}
      {#if fromGit}<span class="git">{t('share.settings.fromGit')}</span>{/if}
    {/snippet}
    <input
      class="field input name"
      aria-label={t('share.settings.name')}
      spellcheck="false"
      placeholder={t('share.settings.noName')}
      bind:value={() => s.userName ?? '', (v) => settingsForm.setUserName(v)}
    />
  </Row>
  <Row label={t('settings.about.telemetry')} desc={t('settings.about.telemetryDesc')}>
    {#snippet extra()}
      <button class="link" onclick={() => openUrl(privacyUrl()).catch((e) => app.toast(String(e), 'error'))}
        >{t('settings.about.telemetryMore')}</button
      >
    {/snippet}
    <Switch label={t('settings.about.telemetry')} bind:on={() => s.telemetry === true, (on) => (s.telemetry = on)} />
  </Row>
</Group>

<!-- The editor's font size, and the id that tells a namesake apart. -->
<Group advanced="general">
  {@render size('editorFontSize')}
  {#if s.userId}
    <Row label={t('share.settings.id')} desc={t('share.settings.idDesc')}>
      <span class="chip mono">{s.userId}</span>
    </Row>
  {/if}
</Group>

<style>
  .git {
    color: var(--p-ink);
    font-size: 11.5px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    height: 22px;
    padding: 0 8px;
    border-radius: 6px;
    background: rgba(var(--fg-rgb), 0.06);
    color: var(--soft);
    font-size: 11.5px;
  }
  .held {
    margin: 0 0 12px;
    font-size: 12px;
    color: var(--warn-ink);
  }
  .select {
    width: 260px;
    max-width: 100%;
  }
  /* The common field (src/app.css): a number's width, a name's. */
  .num {
    width: 64px;
    flex: none;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .name {
    width: 200px;
    max-width: 100%;
    flex: 0 1 auto;
  }
  /* Heard, not seen: the update found, told to a screen reader. */
  .live {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
  .upd {
    display: grid;
    justify-items: start;
    gap: 4px;
    margin-top: 2px;
  }
  .restart-in {
    color: var(--text);
    font-size: 12px;
    font-weight: 600;
  }
  /* The notes, as the update's window has them: soft, their items close together. */
  .notes {
    max-width: 520px;
    color: var(--soft);
  }
  .notes :global(.md) {
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .notes :global(.md li) {
    margin: 0 0 3px;
  }
  .notes :global(.md ul) {
    margin: 0;
    padding-left: 18px;
  }
  .link {
    justify-self: start;
    padding: 0;
    border: none;
    background: none;
    color: var(--p-ink);
    font-size: 12px;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
  }
</style>
