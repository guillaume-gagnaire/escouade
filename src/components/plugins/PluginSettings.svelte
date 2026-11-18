<script lang="ts">
  import { onMount } from 'svelte';
  import { locale, t } from '../../lib/i18n';
  import { keyLockedBy, lockLine, SECRETS_FILE_KEY } from '../../lib/locks.svelte';
  import Icon from '../Icon.svelte';
  import {
    pluginsApi,
    refusedOf,
    textOf,
    type ManifestSetting,
    type PluginView,
    type SecretRefused,
    type SecretState,
    type SecretsFile,
  } from '../../lib/plugins';
  import { plugins } from '../../lib/plugins-view.svelte';

  // A plugin's settings and secrets (§3.5): the fields it declares (`contributes.settings`, `contributes.secrets`). A
  // setting is kept in the list of the installed plugins and given by `settings/get`; a secret goes to the system's
  // keychain under a name of its own, and only whether it is kept, and where, comes back here. « Garder » that kept nothing
  // says why beside its field (issue #37, ESC-499); where the keychain refused, the field offers the file readable by the user
  // alone, never taken by itself; a secret in the file, once a keychain is there, offers to move to it.
  let { plugin }: { plugin: PluginView } = $props();

  const lang = $derived(locale.ui);
  const fields = $derived((plugin.manifest?.contributes.settings as ManifestSetting[] | undefined) ?? []);
  const secretFields = $derived(
    plugin.secretKeys.map((key) => {
      const declared = (
        plugin.manifest?.contributes.secrets as { key: string; title?: string | Record<string, string> }[] | undefined
      )?.find((s) => s.key === key);
      return { key, title: textOf(declared?.title, lang) || key };
    }),
  );
  let kept = $state<Record<string, SecretState>>({});
  let typed = $state<Record<string, string>>({});
  /** Why the last gesture on a secret did nothing, by its key: shown beside its field. */
  let refused = $state<Record<string, SecretRefused>>({});
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
    try {
      await pluginsApi.setSecret(plugin.id, key, value, inFile);
      typed[key] = '';
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

  const statusOf = (key: string) =>
    !kept[key]?.kept
      ? t('plugins.settings.secretUnset')
      : kept[key].inFile
        ? t('plugins.settings.secretInFile')
        : t('plugins.settings.secretSet');

  const valueOf = (f: ManifestSetting) => plugin.choices.settings[f.key] ?? f.default ?? (f.type === 'boolean' ? false : '');
  const optionsOf = (f: ManifestSetting) =>
    (f.options ?? []).map((o) =>
      typeof o === 'string' ? { value: o, title: o } : { value: o.value, title: textOf(o.title, lang) || o.value },
    );

  onMount(() => {
    if (plugin.secretKeys.length) void readSecrets();
  });
</script>

{#if !fields.length && !secretFields.length}
  <p class="hint">{t('plugins.settings.none')}</p>
{/if}
{#if fields.length}
  <section>
    <h4>{t('plugins.settings.settings')}</h4>
    {#each fields as f (f.key)}
      {@const title = textOf(f.title, lang) || f.key}
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
    {/each}
  </section>
{/if}
{#if secretFields.length}
  <section>
    <h4>{t('plugins.settings.secrets')}</h4>
    <p class="hint">{t(file && !file.keychain ? 'plugins.settings.secretNoteNoKeychain' : 'plugins.settings.secretNote')}</p>
    {#each secretFields as s (s.key)}
      {@const why = refused[s.key]}
      <div class="row" role="group" aria-label={s.title}>
        <div class="lab">
          <b>{s.title}</b><span>{statusOf(s.key)}</span>
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
            aria-label={s.title}
            placeholder={t('plugins.settings.secretPlaceholder')}
            bind:value={typed[s.key]}
          />
          <button class="btn" type="submit" disabled={!typed[s.key]}>{t('plugins.settings.keep')}</button>
          {#if kept[s.key]?.kept}<button class="btn ghost" type="button" onclick={() => keep(s.key, '')}
              >{t('plugins.settings.forget')}</button
            >{/if}
        </form>
        {#if why}
          <div class="why">
            <p class="err" role="alert">{why.message}</p>
            {#if why.keychainRefused && typed[s.key] && fileLock}
              <p class="lock" data-testid="secrets-file-lock">
                <Icon name="lock" size={13} />{t('plugins.settings.keepInFileDenied', { by: lockLine(fileLock) })}
              </p>
            {:else if why.keychainRefused && typed[s.key]}
              <div class="offer">
                <button class="btn" type="button" onclick={() => keep(s.key, typed[s.key], true)}>{t('plugins.settings.keepInFile')}</button
                >
                <span>{t('plugins.settings.keepInFileNote', { path: file?.path ?? '' })}</span>
              </div>
            {/if}
          </div>
        {/if}
        {#if kept[s.key]?.inFile && file?.keychain}
          <div class="offer">
            <button class="btn" type="button" onclick={() => move(s.key)}>{t('plugins.settings.moveToKeychain')}</button>
            <span>{t('plugins.settings.moveNote')}</span>
          </div>
        {/if}
      </div>
    {/each}
  </section>
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
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px 20px;
    padding: 10px 0;
    border-bottom: 1px solid var(--line);
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
