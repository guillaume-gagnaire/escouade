<script lang="ts">
  import { onMount } from 'svelte';
  import { locale, t } from '../../lib/i18n';
  import { pluginsApi, textOf, type ManifestSetting, type PluginView } from '../../lib/plugins';
  import { plugins } from '../../lib/plugins-view.svelte';

  // A plugin's settings and secrets (§3.5): the fields it declares (`contributes.settings`, `contributes.secrets`). A
  // setting is kept in the list of the installed plugins and given by `settings/get`; a secret goes to the system's
  // keychain under a name of its own, and only whether it is kept comes back here.
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
  let kept = $state<Record<string, boolean>>({});
  let typed = $state<Record<string, string>>({});
  let error = $state<string | null>(null);

  async function readSecrets() {
    try {
      kept = Object.fromEntries(await pluginsApi.secrets(plugin.id));
    } catch (e) {
      error = String(e);
    }
  }

  async function keep(key: string, value: string) {
    error = null;
    try {
      await pluginsApi.setSecret(plugin.id, key, value);
      typed[key] = '';
      await readSecrets();
    } catch (e) {
      error = String(e);
    }
  }

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
    <p class="hint">{t('plugins.settings.secretNote')}</p>
    {#each secretFields as s (s.key)}
      <div class="row">
        <div class="lab">
          <b>{s.title}</b><span>{kept[s.key] ? t('plugins.settings.secretSet') : t('plugins.settings.secretUnset')}</span>
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
          {#if kept[s.key]}<button class="btn ghost" type="button" onclick={() => keep(s.key, '')}>{t('plugins.settings.forget')}</button
            >{/if}
        </form>
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
</style>
