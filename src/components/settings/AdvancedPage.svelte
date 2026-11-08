<script lang="ts">
  import { tildify } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { openPath } from '../../lib/opener';
  import { IS_MAC } from '../../lib/platform';
  import { settingsForm, type NumberSetting } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import Icon from '../Icon.svelte';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import NumberField from './NumberField.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';
  import VectorsSection from './VectorsSection.svelte';

  // The page « Avancé » (PROPOSITION.md §2): « Réseau » and its certificates (the 2.0 tab), « Terminaux » (the 2.0 tab),
  // « Pièces jointes » (from « Harness »), « Recherche par le sens » (from « Projets › Tous les projets ») and « Données
  // locales » (from « À propos »).
  const s = $derived(settingsForm.settings);
  /** Why a number typed is not saved (ESC-380), the one saved staying. */
  const numberError = (key: NumberSetting) =>
    settingsForm.badNumbers.has(key) ? t('settings.problem.number', { value: app.settings[key] }) : '';
  const bigTextOptions = $derived([
    { value: 'path' as const, label: t('settings.claude.attachBigTextPath') },
    { value: 'refuse' as const, label: t('settings.claude.attachBigTextRefuse') },
  ]);

  // The folder of Escouade's data (another one for a development build or a sandbox).
  let dataDir = $state<string | null>(null);
  $effect(() => {
    if (dataDir) return;
    api
      .dataDir()
      .then((d) => (dataDir = d))
      .catch(() => {});
  });
</script>

{#snippet text(label: string, hint: string, value: string, set: (v: string) => void, placeholder = '')}
  <Row {label} {hint}>
    <input
      class="field mono input"
      aria-label={label}
      spellcheck="false"
      {placeholder}
      {value}
      oninput={(e) => set(e.currentTarget.value)}
    />
  </Row>
{/snippet}

<Group title={t('settings.groups.network')} note={t('settings.network.proxyNote')}>
  {@render text(
    t('settings.network.proxyUrl'),
    t('settings.network.proxyUrlHint'),
    s.proxyUrl,
    (v) => (s.proxyUrl = v),
    t('settings.network.proxyUrlNone'),
  )}
  {@render text(t('settings.network.exclusions'), t('settings.network.exclusionsHint'), s.noProxy, (v) => (s.noProxy = v))}
  <Row label={t('settings.network.proxyTerminals')}>
    <Switch label={t('settings.network.proxyTerminals')} bind:on={s.proxyTerminals} />
  </Row>
  <Row
    label={t('settings.network.insecureTls')}
    desc={`${t('settings.network.insecureTlsDesc')} ${t('settings.network.certificatesNote')}`}
  >
    <Switch label={t('settings.network.insecureTls')} bind:on={s.insecureTls} />
  </Row>
</Group>

<Group
  title={t('settings.groups.terminals')}
  note={t('settings.terminals.detected', { list: app.shells.map((x) => x.label).join(', ') || t('settings.terminals.none') })}
>
  {#if IS_MAC}
    {@render text('bash', t('settings.terminals.auto'), s.bashPath, (v) => (s.bashPath = v))}
  {:else}
    {@render text('PowerShell 7', t('settings.terminals.auto'), s.pwshPath, (v) => (s.pwshPath = v))}
    {@render text('Git Bash', t('settings.terminals.auto'), s.bashPath, (v) => (s.bashPath = v))}
    {@render text(t('settings.terminals.wsl'), t('settings.terminals.wslHint'), s.wslDistro, (v) => (s.wslDistro = v), 'Ubuntu')}
  {/if}
</Group>

<Group title={t('settings.claude.attachments')} note={t('settings.claude.attachmentsNote')}>
  <Row label={t('settings.claude.attachMax')} error={numberError('attachMaxMb')} errorId="attachMaxMb-error">
    <NumberField key="attachMaxMb" label={t('settings.claude.attachMaxLabel')} hint={t('settings.claude.attachMaxHint')} />
  </Row>
  <Row label={t('settings.claude.attachWarn')} error={numberError('attachWarnMb')} errorId="attachWarnMb-error">
    <NumberField key="attachWarnMb" label={t('settings.claude.attachWarnLabel')} hint={t('settings.claude.attachWarnHint')} />
  </Row>
  <Row label={t('settings.claude.attachOrigin')} desc={t('settings.claude.attachOriginDesc')}>
    <Switch label={t('settings.claude.attachOrigin')} bind:on={s.attachOrigin} />
  </Row>
  <Row label={t('settings.claude.attachBigText')} desc={t('settings.claude.attachBigTextDesc')}>
    <Chips label={t('settings.claude.attachBigText')} options={bigTextOptions} bind:value={s.attachBigText} />
  </Row>
  <Row label={t('settings.claude.attachKeep')} error={numberError('attachKeepDays')} errorId="attachKeepDays-error">
    <NumberField key="attachKeepDays" label={t('settings.claude.attachKeepLabel')} hint={t('settings.claude.attachKeepHint')} />
  </Row>
</Group>

<!-- Also with no project open: that is when a folder's old history is most worth finding. -->
<VectorsSection />

<Group title={t('settings.groups.localData')}>
  <Row label={t('common.folder')}>
    {#snippet extra()}<span class="more mono data">{dataDir ? tildify(dataDir) : '~/.escouade'}</span>{/snippet}
    {#if dataDir}
      <button class="btn" onclick={() => openPath(dataDir!).catch((e) => app.toast(String(e), 'error'))}
        ><Icon name="folder" size={14} />{t('nav.titleBar.menu.openFolder')}</button
      >
    {/if}
  </Row>
</Group>

<style>
  .input {
    width: 320px;
    max-width: 100%;
    flex: 0 1 auto;
  }
  .data {
    color: var(--soft);
  }
</style>
