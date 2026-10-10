<script lang="ts">
  import { LANG_NAMES, t, type Lang } from '../../lib/i18n';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';

  // The « Application » tab: the language of the interface and that of the texts Claude writes, saved with the other
  // settings and applied at once (the backend tells the window, the menus and its own texts).
  const s = $derived(settingsForm.settings);
  /** Each language named in itself, whatever the interface's: who does not read it still finds their own. */
  const own = (['en', 'fr'] as const).map((l: Lang) => ({ value: l, label: LANG_NAMES[l], lang: l }));
  /** The system's language is named in the interface's: « Système (Anglais) », « System (English) ». */
  const uiOptions = $derived([
    { value: 'system' as const, label: t('settings.app.system', { lang: t(`settings.app.langName.${app.lang.system}`) }) },
    ...own,
  ]);
  const claudeOptions = $derived([{ value: 'ui' as const, label: t('settings.app.sameAsUi') }, ...own]);
</script>

<Group title={t('settings.app.language')}>
  <Row label={t('settings.app.uiLanguage')}>
    <Chips label={t('settings.app.uiLanguage')} options={uiOptions} bind:value={s.language} />
  </Row>
  <Row label={t('settings.app.claudeLanguage')} desc={t('settings.app.claudeLanguageHelp')}>
    <Chips label={t('settings.app.claudeLanguage')} options={claudeOptions} bind:value={s.claudeLanguage} />
  </Row>
</Group>
