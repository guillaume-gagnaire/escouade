<script lang="ts">
  import { onMount } from 'svelte';
  import SecretsOnServer from '../servers/SecretsOnServer.svelte';
  import { locale, t } from '../../lib/i18n';
  import { keyLockedBy, lockLine, SECRETS_FILE_KEY } from '../../lib/locks.svelte';
  import Icon from '../Icon.svelte';
  import {
    acceptOf,
    keptLabel,
    layoutOf,
    readImport,
    tooBig,
    visibleLines,
    type FieldSetting,
    type ManifestGroup,
    type ManifestSecret,
  } from '../../lib/plugin-fields';
  import {
    pluginsApi,
    refusedOf,
    textOf,
    type PluginView,
    type SecretRefused,
    type SecretState,
    type SecretsFile,
  } from '../../lib/plugins';
  import { kindsOf, nameOf, profilesOf } from '../../lib/plugin-profiles';
  import { plugins } from '../../lib/plugins-view.svelte';
  import LinedText from './LinedText.svelte';
  import PluginProfiles from './PluginProfiles.svelte';

  // A plugin's settings and secrets (§3.5): the fields it declares (`contributes.settings`, `contributes.secrets`). A
  // setting is kept in the list of the installed plugins and given by `settings/get`; a secret goes to the system's
  // keychain under a name of its own, and only whether it is kept, and where, comes back here. « Garder » that kept nothing
  // says why beside its field (issue #37, ESC-499); where the keychain refused, the field offers the file readable by the user
  // alone, never taken by itself; a secret in the file, once a keychain is there, offers to move to it.
  //
  // Issue #40 (ESC-526): the groups of the manifest (`contributes.groups`) come first, each its title, its note and its rows,
  // settings and secrets mixed in the order of its `fields`; `folded` folds it as « Avancé » is elsewhere. A field in no group
  // comes after, under « Réglages » then « Secrets », as before. A field on several lines (`multiline`) is a numbered text in
  // the font of fixed width; a secret on several lines, kept, is never read back: it says its lines and its size, and is
  // replaced. « Importer un fichier… » reads the file the user chose once (its kinds, UTF-8, 64 KiB): its text fills the field,
  // where it was is never known here, and the plugin gains no right on the files.
  let { plugin }: { plugin: PluginView } = $props();

  const lang = $derived(locale.ui);
  const layout = $derived(layoutOf(plugin.manifest, plugin.secretKeys));
  const fields = $derived(layout.settings);
  const secretFields = $derived(layout.secrets);
  // Its profiles (ESC-530) come first: a setting of each project is not here, each project sets it in its own settings.
  const kinds = $derived(kindsOf(plugin.manifest));
  const anyField = $derived(kinds.length > 0 || layout.groups.length > 0 || fields.length > 0 || secretFields.length > 0);
  const groupedSecret = $derived(layout.groups.some((g) => g.fields.some((f) => f.kind === 'secret')));
  let kept = $state<Record<string, SecretState>>({});
  let typed = $state<Record<string, string>>({});
  /** Why the last gesture on a secret did nothing, by its key: shown beside its field. */
  let refused = $state<Record<string, SecretRefused>>({});
  /** Why the last import or text of a setting was not taken, by its key. */
  let settingRefused = $state<Record<string, string>>({});
  /** A secret on several lines kept, being replaced: its text shown. */
  let replacing = $state<Record<string, boolean>>({});
  /** The groups unfolded or folded by the user, by their id; else as the manifest says. */
  let unfolded = $state<Record<string, boolean>>({});
  /** The file inputs of the fields that import a file, by their key. */
  const pickers: Record<string, HTMLInputElement> = {};
  let file = $state<SecretsFile | null>(null);
  let error = $state<string | null>(null);
  /** The file denied by the policies of this device or of a team (`secrets.fileFallback`, ESC-520): never offered. */
  const fileLock = $derived(keyLockedBy(SECRETS_FILE_KEY));

  async function readSecrets() {
    try {
      const [states, f] = await Promise.all([pluginsApi.secrets(plugin.id), pluginsApi.secretsFile()]);
      kept = Object.fromEntries(states.map((s) => [s.key, s]));
      file = f;
    } catch (e) {
      error = String(e);
    }
  }

  /** « Garder » (`value`; empty: « Oublier »), or « Garder dans un fichier lisible de toi seul » (`inFile`). */
  async function keep(key: string, value: string, inFile = false) {
    delete refused[key];
    const big = tooBig(value);
    if (big) {
      refused[key] = { message: big, keychainRefused: false };
      return;
    }
    try {
      await pluginsApi.setSecret(plugin.id, key, value, inFile);
      typed[key] = '';
      replacing[key] = false;
    } catch (e) {
      refused[key] = refusedOf(e);
    }
    await readSecrets();
  }

  async function move(key: string) {
    delete refused[key];
    try {
      await pluginsApi.moveSecret(plugin.id, key);
    } catch (e) {
      refused[key] = { message: String(e), keychainRefused: false };
    }
    await readSecrets();
  }

  /** A setting's text on several lines, given: over 64 KiB, refused, nothing kept nor cut. */
  function setText(f: FieldSetting, v: string) {
    delete settingRefused[f.key];
    const big = tooBig(v);
    if (big) {
      settingRefused[f.key] = big;
      return;
    }
    void plugins.setSetting(plugin.id, f.key, v === '' ? null : v);
  }

  /** The file the user chose for the field `key`: read once, its text in the field; the input emptied at once. */
  async function imported(key: string, exts: string[], input: HTMLInputElement, secret: boolean) {
    const chosen = input.files?.[0];
    input.value = '';
    if (!chosen) return;
    const read = await readImport(chosen, exts);
    if (secret) {
      delete refused[key];
      if ('error' in read) refused[key] = { message: read.error, keychainRefused: false };
      else {
        typed[key] = read.text;
        replacing[key] = true;
      }
    } else if ('error' in read) settingRefused[key] = read.error;
    else {
      const f = fields.find((s) => s.key === key) ?? settingOf(key)?.setting;
      if (f) setText(f, read.text);
    }
  }

  const settingOf = (key: string) =>
    layout.groups.flatMap((g) => g.fields).find((f) => f.kind === 'setting' && f.key === key) as
      { kind: 'setting'; setting: FieldSetting } | undefined;

  const statusOf = (key: string) =>
    !kept[key]?.kept
      ? t('plugins.settings.secretUnset')
      : kept[key].inFile
        ? t('plugins.settings.secretInFile')
        : t('plugins.settings.secretSet');

  const valueOf = (f: FieldSetting) => plugin.choices.settings[f.key] ?? f.default ?? (f.type === 'boolean' ? false : '');
  /** The profiles a `profile` setting chooses among (« Profil par défaut »), « Aucun profil » first. */
  const profileOptions = (f: FieldSetting) => {
    const kind = kinds.find((k) => k.id === f.of);
    return [
      { value: '', title: t('plugins.projectPage.profileNone') },
      ...(kind ? profilesOf(plugin, kind.id).map((p) => ({ value: p.id, title: nameOf(kind, p) })) : []),
    ];
  };
  const optionsOf = (f: FieldSetting) =>
    (f.options ?? []).map((o) =>
      typeof o === 'string' ? { value: o, title: o } : { value: o.value, title: textOf(o.title, lang) || o.value },
    );
  const isOpen = (g: ManifestGroup) => unfolded[g.id] ?? !g.folded;
  const imports = (f: { import?: string[] }) => (Array.isArray(f.import) && f.import.length ? f.import : null);

  onMount(() => {
    if (plugin.secretKeys.length) void readSecrets();
  });
</script>

{#snippet picker(key: string, exts: string[], secret: boolean)}
  <input
    class="import-file"
    type="file"
    hidden
    accept={acceptOf(exts)}
    data-field={key}
    bind:this={pickers[key]}
    onchange={(e) => imported(key, exts, e.currentTarget, secret)}
  />
  <button class="btn sm" type="button" onclick={() => pickers[key]?.click()}
    ><Icon name="import" size={13} />{t('plugins.settings.import')}</button
  >
{/snippet}

{#snippet settingRow(f: FieldSetting)}
  {@const title = textOf(f.title, lang) || f.key}
  {#if f.multiline && f.type === 'string'}
    {@const exts = imports(f)}
    <div class="row top" role="group" aria-label={title}>
      <div class="lab">
        <b>{title}</b>{#if f.description}<span>{textOf(f.description, lang)}</span>{/if}
      </div>
      {#if exts}<div class="ctl">{@render picker(f.key, exts, false)}</div>{/if}
      <LinedText label={title} rows={visibleLines(f.lines)} value={String(valueOf(f))} onchange={(v) => setText(f, v)} />
      {#if settingRefused[f.key]}<p class="err wide" role="alert">{settingRefused[f.key]}</p>{/if}
    </div>
  {:else}
    <div class="row">
      <div class="lab">
        <b>{title}</b>{#if f.description}<span>{textOf(f.description, lang)}</span>{/if}
      </div>
      <div class="ctl">
        {#if f.type === 'boolean'}
          <button
            class="switch"
            class:on={!!valueOf(f)}
            role="switch"
            aria-checked={!!valueOf(f)}
            aria-label={title}
            onclick={() => plugins.setSetting(plugin.id, f.key, !valueOf(f))}
          ></button>
        {:else if f.type === 'profile'}
          <select
            class="field"
            aria-label={title}
            value={String(valueOf(f) ?? '')}
            onchange={(e) => plugins.setSetting(plugin.id, f.key, e.currentTarget.value || null)}
          >
            {#each profileOptions(f) as o (o.value)}<option value={o.value}>{o.title}</option>{/each}
          </select>
        {:else if f.type === 'select'}
          <select
            class="field"
            aria-label={title}
            value={String(valueOf(f))}
            onchange={(e) => plugins.setSetting(plugin.id, f.key, e.currentTarget.value)}
          >
            {#each optionsOf(f) as o (o.value)}<option value={o.value}>{o.title}</option>{/each}
          </select>
        {:else}
          <input
            class="field"
            aria-label={title}
            type={f.type === 'number' ? 'number' : 'text'}
            value={String(valueOf(f))}
            onchange={(e) => {
              const v = e.currentTarget.value;
              void plugins.setSetting(plugin.id, f.key, v === '' ? null : f.type === 'number' ? Number(v) : v);
            }}
          />
        {/if}
      </div>
    </div>
  {/if}
{/snippet}

{#snippet whyOf(key: string)}
  {@const why = refused[key]}
  {#if why}
    <div class="why">
      <p class="err" role="alert">{why.message}</p>
      {#if why.keychainRefused && typed[key] && fileLock}
        <p class="lock" data-testid="secrets-file-lock">
          <Icon name="lock" size={13} />{t('plugins.settings.keepInFileDenied', { by: lockLine(fileLock) })}
        </p>
      {:else if why.keychainRefused && typed[key]}
        <div class="offer">
          <button class="btn" type="button" onclick={() => keep(key, typed[key], true)}>{t('plugins.settings.keepInFile')}</button>
          <span>{t('plugins.settings.keepInFileNote', { path: file?.path ?? '' })}</span>
        </div>
      {/if}
    </div>
  {/if}
  {#if kept[key]?.inFile && file?.keychain}
    <div class="offer">
      <button class="btn" type="button" onclick={() => move(key)}>{t('plugins.settings.moveToKeychain')}</button>
      <span>{t('plugins.settings.moveNote')}</span>
    </div>
  {/if}
{/snippet}

{#snippet secretRow(s: ManifestSecret)}
  {@const title = textOf(s.title, lang) || s.key}
  {#if s.multiline}
    {@const exts = imports(s)}
    {@const size = kept[s.key]?.kept && !replacing[s.key] ? kept[s.key].size : undefined}
    <div class="row top" role="group" aria-label={title}>
      <div class="lab">
        <b>{title}</b>{#if s.description}<span>{textOf(s.description, lang)}</span>{/if}
        {#if !size}<span>{statusOf(s.key)}</span>{/if}
      </div>
      {#if size}
        <div class="ctl">
          <span class="kept">{keptLabel(size, kept[s.key].inFile)}</span>
          <button class="btn ghost" type="button" onclick={() => keep(s.key, '')}>{t('plugins.settings.forget')}</button>
          <button class="btn sm" type="button" onclick={() => (replacing[s.key] = true)}>{t('plugins.settings.replace')}</button>
        </div>
      {:else}
        <div class="ctl">
          {#if exts}{@render picker(s.key, exts, true)}{/if}
          {#if replacing[s.key] && kept[s.key]?.kept}<button
              class="btn ghost"
              type="button"
              onclick={() => {
                replacing[s.key] = false;
                typed[s.key] = '';
                delete refused[s.key];
              }}>{t('plugins.settings.cancel')}</button
            >{/if}
          <button class="btn sm primary" type="button" disabled={!typed[s.key]} onclick={() => keep(s.key, typed[s.key])}
            >{t('plugins.settings.keep')}</button
          >
        </div>
        <LinedText label={title} rows={visibleLines(s.lines)} bind:value={typed[s.key]} />
      {/if}
      {@render whyOf(s.key)}
    </div>
  {:else}
    <div class="row" role="group" aria-label={title}>
      <div class="lab">
        <b>{title}</b>{#if s.description}<span>{textOf(s.description, lang)}</span>{/if}<span>{statusOf(s.key)}</span>
      </div>
      <form
        class="ctl"
        onsubmit={(e) => {
          e.preventDefault();
          if (typed[s.key]) void keep(s.key, typed[s.key]);
        }}
      >
        <input
          class="field"
          type="password"
          autocomplete="off"
          aria-label={title}
          placeholder={t('plugins.settings.secretPlaceholder')}
          bind:value={typed[s.key]}
        />
        <button class="btn" type="submit" disabled={!typed[s.key]}>{t('plugins.settings.keep')}</button>
        {#if kept[s.key]?.kept}<button class="btn ghost" type="button" onclick={() => keep(s.key, '')}
            >{t('plugins.settings.forget')}</button
          >{/if}
      </form>
      {@render whyOf(s.key)}
    </div>
  {/if}
{/snippet}

{#if !anyField}
  <p class="hint">{t('plugins.settings.none')}</p>
{/if}
{#if kinds.length}<PluginProfiles {plugin} />{/if}
{#each layout.groups as { group: g, fields: inside } (g.id)}
  {@const title = textOf(g.title, lang) || g.id}
  {@const open = isOpen(g)}
  <section class="grp" class:folded={g.folded} aria-label={title}>
    {#if g.folded}
      <button class="fold" type="button" aria-expanded={open} onclick={() => (unfolded[g.id] = !open)}
        ><Icon name={open ? 'down' : 'right'} size={14} /><span class="ft">{title}</span>{#if !open}<span class="cnt"
            >{t('plugins.settings.folded', { count: inside.length })}</span
          >{/if}</button
      >
    {:else}
      <h4 class="gh">{title}</h4>
    {/if}
    {#if open}
      {#if g.description}<p class="gsub">{textOf(g.description, lang)}</p>{/if}
      {#each inside as f (f.key)}
        {#if f.kind === 'setting'}{@render settingRow(f.setting)}{:else}{@render secretRow(f.secret)}{/if}
      {/each}
    {/if}
  </section>
{/each}
{#if fields.length}
  <section>
    <h4>{t('plugins.settings.settings')}</h4>
    {#each fields as f (f.key)}
      {@render settingRow(f)}
    {/each}
  </section>
{/if}
{#if secretFields.length}
  <section>
    <h4>{t('plugins.settings.secrets')}</h4>
    <p class="hint">{t(file && !file.keychain ? 'plugins.settings.secretNoteNoKeychain' : 'plugins.settings.secretNote')}</p>
    <SecretsOnServer />
    {#each secretFields as s (s.key)}
      {@render secretRow(s)}
    {/each}
  </section>
{:else if groupedSecret}
  <p class="hint">{t(file && !file.keychain ? 'plugins.settings.secretNoteNoKeychain' : 'plugins.settings.secretNote')}</p>
{/if}
{#if error || plugins.error}<p class="err" role="alert">{error ?? plugins.error}</p>{/if}

<style>
  section {
    display: grid;
    margin-bottom: 18px;
  }
  h4 {
    margin: 0 0 6px;
    color: var(--dim);
    font-size: 11.5px;
    font-weight: 600;
  }
  /* A group of the manifest (issue #40): its title and its note, no box, its rows under a hairline. */
  h4.gh {
    margin: 0;
    padding: 2px 0 6px;
    color: var(--text);
    font-size: 12.5px;
  }
  .gsub {
    max-width: 62ch;
    margin: -2px 0 6px;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }
  .grp.folded {
    margin-bottom: 8px;
  }
  .fold {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-width: 0;
    padding: 11px 0;
    border: none;
    border-top: 1px solid var(--line);
    background: transparent;
    color: var(--soft);
    font-size: 12.5px;
    font-weight: 550;
    text-align: left;
    cursor: pointer;
  }
  .fold:hover {
    color: var(--text);
  }
  .fold :global(.icon) {
    flex: none;
    color: var(--muted);
  }
  .fold .cnt {
    color: var(--dim);
    font-size: 12px;
    font-weight: 450;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px 20px;
    padding: 10px 0;
    border-bottom: 1px solid var(--line);
  }
  .row.top {
    align-items: flex-start;
  }
  .lab {
    flex: 1 1 180px;
    display: grid;
    gap: 2px;
  }
  .lab b {
    font-weight: 560;
    font-size: 12.5px;
  }
  .lab span {
    color: var(--muted);
    font-size: 11.5px;
  }
  .ctl {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .ctl .field {
    width: 240px;
  }
  .btn.sm {
    height: 26px;
    padding: 0 10px;
    font-size: 12px;
  }
  /* A secret on several lines, kept: its lines and its size, never its text. */
  .kept {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 22px;
    padding: 0 8px;
    border-radius: 6px;
    background: color-mix(in oklch, var(--run) 10%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--run) 30%, transparent);
    color: var(--run);
    font-size: 11.5px;
    font-weight: 550;
    white-space: nowrap;
  }
  .kept::before {
    content: '';
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }
  .hint {
    margin: 0 0 6px;
    color: var(--muted);
    font-size: 12px;
  }
  .err {
    margin: 8px 0 0;
    color: var(--del);
    font-size: 12px;
  }
  .err.wide {
    flex: 1 0 100%;
    margin: 0;
  }
  /* Beside the field: why it kept nothing, and what can be done. */
  .why,
  .offer {
    flex: 1 0 100%;
  }
  .why .err {
    margin: 0;
  }
  .offer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
    margin-top: 6px;
  }
  /* Imposed (ESC-520): the padlock and who denies the file, in place of its offer. */
  .lock {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin: 6px 0 0;
    color: var(--muted);
    font-size: 12px;
    font-weight: 550;
  }
  .offer span {
    flex: 1 1 260px;
    color: var(--muted);
    font-size: 11.5px;
    line-height: 1.45;
  }
</style>
