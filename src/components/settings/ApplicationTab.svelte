<script lang="ts">
  import { onMount } from 'svelte';
  import { THEMES, themeChoice } from '../../lib/appearance';
  import { DEFAULT_FONT_SIZE, FONT_SIZE_MAX, FONT_SIZE_MIN, fontSize, setDisplay, ZOOM_LEVELS, zoomState } from '../../lib/display.svelte';
  import { writerOf, writerOptions } from '../../lib/engines';
  import { LANG_NAMES, t, type Lang } from '../../lib/i18n';
  import { keyLabel } from '../../lib/platform';
  import { settingsForm } from '../../lib/settings.svelte';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';

  // The « Application » tab: the language of the interface and that of the texts Claude writes, saved with the other
  // settings and applied at once (the backend tells the window, the menus and its own texts); and the display (the zoom,
  // the font sizes of the terminals and of the editor) and the theme, applied and saved as it changes, apart from the draft: its
  // shortcuts change it too, the modal open or not.
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
  /** « Modèle des textes rédigés »: the harnesses found. */
  const writers = $derived(writerOptions());
  /** The harness the texts go to, when it takes the model chosen here (OpenCode): its models, as its account listed them. */
  const writer = $derived(writerOf(s.writerEngine) ?? writers[0]?.value ?? null);
  const writerModels = $derived(
    writer && app.engines.find((e) => e.id === writer)?.writerTakesModel ? (app.engineModels[writer] ?? []) : null,
  );
  /** Its default model (« Par défaut »), named. */
  const writerDefault = $derived(writerModels?.find((m) => m.default));
  /** What the menu shows: the model chosen while it is one of them, else the default. */
  const writerModel = $derived(writerModels?.some((m) => m.id === s.writerModel) ? (s.writerModel ?? '') : '');
  /** « Thème »: Système, Clair, Sombre. */
  const themeOptions = $derived(
    THEMES.map((v) => ({
      value: v,
      label: { system: t('settings.app.themeSystem'), light: t('settings.app.themeLight'), dark: t('settings.app.themeDark') }[v],
    })),
  );
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
  const claudeOptions = $derived([{ value: 'ui' as const, label: t('settings.app.sameAsUi') }, ...own]);
</script>

<Group title={t('settings.app.display')}>
  <Row label={t('settings.app.theme')} desc={t('settings.app.themeDesc')}>
    <Chips
      label={t('settings.app.theme')}
      options={themeOptions}
      bind:value={() => themeChoice(app.settings.theme), (v) => setDisplay({ theme: v })}
    />
  </Row>
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
  {#each ['terminalFontSize', 'editorFontSize'] as const as key (key)}
    <Row
      label={t(`settings.app.${key}`)}
      hint={t('settings.app.fontSizeHint', { n: DEFAULT_FONT_SIZE })}
      desc={t('settings.app.fontSizeDesc')}
    >
      <input
        class="field input"
        type="number"
        min={FONT_SIZE_MIN}
        max={FONT_SIZE_MAX}
        step="0.5"
        aria-label={t(`settings.app.${key}`)}
        value={app.settings[key] ?? DEFAULT_FONT_SIZE}
        onchange={(e) => setSize(key, e.currentTarget)}
      />
    </Row>
  {/each}
</Group>

<!-- Only the harnesses found are offered; none found, nothing to choose. -->
{#if writers.length}
  <Group title={t('settings.app.writing')}>
    <Row label={t('settings.app.writer')} desc={t('settings.app.writerHelp')}>
      <Chips
        label={t('settings.app.writer')}
        options={writers}
        bind:value={() => writerOf(s.writerEngine) ?? writers[0].value, (v) => (s.writerEngine = v)}
      />
    </Row>
    {#if writerModels}
      <Row label={t('settings.app.writerModel')} desc={t('settings.app.writerModelHelp')}>
        <select
          class="field select"
          aria-label={t('settings.app.writerModel')}
          disabled={!writerModels.length}
          value={writerModel}
          onchange={(e) => (s.writerModel = e.currentTarget.value)}
        >
          <option value=""
            >{writerDefault
              ? t('settings.app.writerModelDefault', { model: writerDefault.label })
              : t('settings.app.writerModelNone')}</option
          >
          {#each writerModels as m (m.id)}
            <option value={m.id}>{m.provider ? `${m.label} · ${m.provider}` : m.label}</option>
          {/each}
        </select>
      </Row>
    {/if}
  </Group>
{/if}

<Group title={t('settings.app.language')}>
  <Row label={t('settings.app.uiLanguage')}>
    <Chips label={t('settings.app.uiLanguage')} options={uiOptions} bind:value={s.language} />
  </Row>
  <Row label={t('settings.app.claudeLanguage')} desc={t('settings.app.claudeLanguageHelp')}>
    <Chips label={t('settings.app.claudeLanguage')} options={claudeOptions} bind:value={s.claudeLanguage} />
  </Row>
</Group>

<!-- The user's name for the shared tickets (ESC-190, choice 113), and the id that tells a namesake apart. -->
<Group title={t('share.settings.group')}>
  <Row label={t('share.settings.name')} desc={t('share.settings.nameDesc')}>
    {#snippet extra()}
      {#if fromGit}<span class="git">{t('share.settings.fromGit')}</span>{/if}
    {/snippet}
    <input
      class="field input"
      aria-label={t('share.settings.name')}
      spellcheck="false"
      placeholder={t('share.settings.noName')}
      bind:value={() => s.userName ?? '', (v) => settingsForm.setUserName(v)}
    />
  </Row>
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
  /* The common field (src/app.css), a number's width. */
  .input {
    width: 64px;
    flex: none;
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
