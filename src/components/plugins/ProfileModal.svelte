<script lang="ts">
  import { locale, t } from '../../lib/i18n';
  import { fList } from '../../lib/format';
  import { acceptOf, keptLabel, readImport, tooBig, visibleLines } from '../../lib/plugin-fields';
  import { nameOf, profilesApi, titleOf, usedBy, type Profile, type ProfileField, type ProfileKind } from '../../lib/plugin-profiles';
  import { refusedOf, textOf, type PluginView, type SecretState } from '../../lib/plugins';
  import { plugins } from '../../lib/plugins-view.svelte';
  import { app } from '../../lib/state.svelte';
  import Icon from '../Icon.svelte';
  import Modal from '../modals/Modal.svelte';
  import LinedText from './LinedText.svelte';

  // The window of a profile (ESC-530, design/plugins-contract-2-3 « b-profil »): its simple fields, then its secrets. A
  // secret is never read back: kept, it says where (and its lines and size on several lines) and is replaced or forgotten;
  // typed, it is kept with « Enregistrer », under a name of this profile's own in the keychain. A new profile is given its
  // id by Escouade at its first « Enregistrer ». « Supprimer le profil » names the projects that took it: its secrets are
  // erased, those projects have none from then on.
  let {
    plugin,
    kind,
    profile,
    secrets,
    onclose,
  }: { plugin: PluginView; kind: ProfileKind; profile: Profile | null; secrets: SecretState[]; onclose: () => void } = $props();

  const lang = $derived(locale.ui);
  // The window's own copy of what it changes: the profile it opened on, or a new one at its first « Enregistrer ».
  // svelte-ignore state_referenced_locally
  let id = $state<string | null>(profile?.id ?? null);
  // svelte-ignore state_referenced_locally
  let values = $state<Record<string, unknown>>(initial());
  // svelte-ignore state_referenced_locally
  let kept = $state<Record<string, SecretState>>(Object.fromEntries(secrets.map((s) => [s.key, s])));
  let typed = $state<Record<string, string>>({});
  let replacing = $state<Record<string, boolean>>({});
  let refused = $state<Record<string, string>>({});
  let error = $state<string | null>(null);
  let asking = $state(false);
  let busy = $state(false);
  const pickers: Record<string, HTMLInputElement> = {};

  function initial(): Record<string, unknown> {
    const out: Record<string, unknown> = {};
    for (const f of kind.fields) {
      if (f.secret) continue;
      out[f.key] = profile?.values[f.key] ?? f.default ?? (f.type === 'boolean' ? false : f.type === 'number' ? '' : '');
    }
    return out;
  }

  const simple = $derived(kind.fields.filter((f) => !f.secret));
  const secretFields = $derived(kind.fields.filter((f) => f.secret));
  const name = $derived(String(values[kind.nameField] ?? '').trim());
  const title = $derived(
    id ? t('plugins.profiles.title', { name: name || id }) : t('plugins.profiles.newTitle', { kind: titleOf(kind.title, lang, kind.id) }),
  );
  const projects = $derived(
    id
      ? usedBy(plugin, kind.id, id)
          .map((p) => app.projects.find((x) => x.id === p)?.name)
          .filter((x): x is string => !!x)
      : [],
  );
  const fieldTitle = (f: ProfileField) => textOf(f.title, lang) || f.key;
  const optionsOf = (f: ProfileField) =>
    (f.options ?? []).map((o) =>
      typeof o === 'string' ? { value: o, title: o } : { value: o.value, title: textOf(o.title, lang) || o.value },
    );
  const imports = (f: ProfileField) => (Array.isArray(f.import) && f.import.length ? f.import : null);

  /** The simple fields as they are kept: a number from its text, an empty text left out. */
  function given(): Record<string, unknown> {
    const out: Record<string, unknown> = {};
    for (const f of simple) {
      const v = values[f.key];
      if (f.type === 'number') {
        if (v !== '' && v !== null && v !== undefined && Number.isFinite(Number(v))) out[f.key] = Number(v);
      } else if (v !== '' && v !== undefined && v !== null) out[f.key] = v;
    }
    return out;
  }

  async function save() {
    error = null;
    refused = {};
    for (const [key, v] of Object.entries(typed)) {
      const big = v ? tooBig(v) : null;
      if (big) refused[key] = big;
    }
    if (Object.keys(refused).length) return;
    busy = true;
    try {
      const [view, pid] = await profilesApi.save(plugin.id, kind.id, id, given());
      plugins.put(view);
      id = pid;
      for (const [key, v] of Object.entries(typed)) {
        if (!v) continue;
        try {
          await profilesApi.setSecret(plugin.id, kind.id, pid, key, v);
          typed[key] = '';
          replacing[key] = false;
          kept[key] = { key, kept: true, inFile: false };
        } catch (e) {
          refused[key] = refusedOf(e).message;
        }
      }
      if (!Object.keys(refused).length) {
        app.toast(t('plugins.profiles.saved', { name: name || pid }));
        onclose();
      }
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function forget(key: string) {
    if (!id) return;
    try {
      await profilesApi.setSecret(plugin.id, kind.id, id, key, '');
      kept[key] = { key, kept: false, inFile: false };
    } catch (e) {
      refused[key] = refusedOf(e).message;
    }
  }

  async function remove() {
    if (!id) return;
    busy = true;
    try {
      const [view] = await profilesApi.remove(plugin.id, kind.id, id);
      plugins.put(view);
      app.toast(t('plugins.profiles.deleted', { name: name || id }));
      onclose();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function imported(f: ProfileField, input: HTMLInputElement) {
    const chosen = input.files?.[0];
    input.value = '';
    if (!chosen) return;
    const read = await readImport(chosen, imports(f) ?? []);
    delete refused[f.key];
    if ('error' in read) refused[f.key] = read.error;
    else if (f.secret) {
      typed[f.key] = read.text;
      replacing[f.key] = true;
    } else values[f.key] = read.text;
  }
</script>

{#snippet picker(f: ProfileField, replace: boolean)}
  {@const exts = imports(f)}
  {#if exts}
    <input
      class="import-file"
      type="file"
      hidden
      accept={acceptOf(exts)}
      data-field={f.key}
      bind:this={pickers[f.key]}
      onchange={(e) => imported(f, e.currentTarget)}
    />
    <button class="btn sm" type="button" onclick={() => pickers[f.key]?.click()}
      ><Icon name="import" size={13} />{replace ? t('plugins.profiles.replaceFile') : t('plugins.settings.import')}</button
    >
  {/if}
{/snippet}

<Modal {title} width={680} {onclose}>
  {#snippet lead()}<span class="mini" aria-hidden="true"><Icon name="globe" size={15} /></span>{/snippet}
  <div class="body">
    <section aria-label={t('plugins.profiles.fields')}>
      <h4>{titleOf(kind.title, lang, kind.id)}</h4>
      {#each simple as f (f.key)}
        {@const label = fieldTitle(f)}
        <div class="row" class:top={f.multiline}>
          <div class="lab">
            <b>{label}</b>
            {#if f.description}<span>{textOf(f.description, lang)}</span>{:else if f.key === kind.nameField}<span
                >{t('plugins.profiles.nameDesc')}</span
              >{/if}
          </div>
          {#if f.type === 'boolean'}
            <button
              class="switch"
              class:on={!!values[f.key]}
              role="switch"
              aria-checked={!!values[f.key]}
              aria-label={label}
              onclick={() => (values[f.key] = !values[f.key])}
            ></button>
          {:else if f.type === 'select'}
            <select class="field" aria-label={label} bind:value={values[f.key]}>
              {#each optionsOf(f) as o (o.value)}<option value={o.value}>{o.title}</option>{/each}
            </select>
          {:else if f.multiline}
            <div class="ctl">{@render picker(f, false)}</div>
            <div class="wide">
              <LinedText {label} rows={visibleLines(f.lines)} value={String(values[f.key] ?? '')} onchange={(v) => (values[f.key] = v)} />
            </div>
          {:else}
            <input class="field" aria-label={label} type={f.type === 'number' ? 'number' : 'text'} bind:value={values[f.key]} />
          {/if}
          {#if refused[f.key]}<p class="err wide" role="alert">{refused[f.key]}</p>{/if}
        </div>
      {/each}
    </section>
    {#if secretFields.length}
      <section aria-label={t('plugins.profiles.secrets')}>
        <h4>{t('plugins.profiles.secrets')}</h4>
        <p class="note">{t('plugins.profiles.secretsNote')}</p>
        {#each secretFields as f (f.key)}
          {@const label = fieldTitle(f)}
          {@const k = kept[f.key]}
          {@const shown = k?.kept && !replacing[f.key]}
          <div class="row" class:top={f.multiline && !shown} role="group" aria-label={label}>
            <div class="lab">
              <b>{label}</b>
              {#if f.description}<span>{textOf(f.description, lang)}</span>{/if}
              {#if shown}
                <span class="kept ok"
                  >{k.size
                    ? keptLabel(k.size, k.inFile)
                    : k.inFile
                      ? t('plugins.profiles.secretInFile')
                      : t('plugins.profiles.secretKept')}</span
                >
              {:else if !f.multiline}<span>{typed[f.key] ? t('plugins.profiles.secretTyped') : t('plugins.profiles.secretUnset')}</span
                >{/if}
            </div>
            {#if shown}
              <div class="ctl">
                <button class="btn ghost sm" type="button" onclick={() => forget(f.key)}>{t('plugins.profiles.forget')}</button>
                {#if imports(f)}{@render picker(f, true)}{:else}<button
                    class="btn sm"
                    type="button"
                    onclick={() => (replacing[f.key] = true)}>{t('plugins.profiles.replace')}</button
                  >{/if}
              </div>
            {:else if f.multiline}
              <div class="ctl">{@render picker(f, false)}</div>
              <div class="wide"><LinedText {label} rows={visibleLines(f.lines)} bind:value={typed[f.key]} /></div>
            {:else}
              <input
                class="field"
                type="password"
                autocomplete="off"
                aria-label={label}
                placeholder={t('plugins.settings.secretPlaceholder')}
                bind:value={typed[f.key]}
              />
            {/if}
            {#if refused[f.key]}<p class="err wide" role="alert">{refused[f.key]}</p>{/if}
          </div>
        {/each}
      </section>
    {/if}
    {#if error}<p class="err" role="alert">{error}</p>{/if}
  </div>
  {#snippet footer()}
    {#if asking && id}
      <span class="ask" role="alert"
        >{t('plugins.profiles.deleteAsk', { name: name || id })}{#if projects.length}
          {' '}{t('plugins.profiles.deleteUsed', { count: projects.length, projects: fList(projects) })}{/if}</span
      >
      <button class="btn" type="button" onclick={() => (asking = false)}>{t('plugins.profiles.cancel')}</button>
      <button class="btn danger" type="button" disabled={busy} onclick={remove}>{t('plugins.profiles.deleteYes')}</button>
    {:else}
      {#if id}<button class="btn ghost del" type="button" onclick={() => (asking = true)}>{t('plugins.profiles.delete')}</button>{/if}
      <span class="used"
        >{#if id}{projects.length
            ? t('plugins.profiles.usedBy', { projects: fList(projects) })
            : t('plugins.profiles.usedByNone')}{/if}</span
      >
      <button class="btn" type="button" onclick={onclose}>{t('plugins.profiles.cancel')}</button>
      <button class="btn primary" type="button" disabled={busy || !name} onclick={save}>{t('plugins.profiles.save')}</button>
    {/if}
  {/snippet}
</Modal>

<style>
  .mini {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 8px;
    background: color-mix(in oklch, var(--p) 22%, transparent);
    color: var(--p-ink);
  }
  .body {
    display: grid;
    gap: 14px;
  }
  section {
    display: grid;
  }
  h4 {
    margin: 0;
    padding: 2px 0 6px;
    border-bottom: 1px solid var(--line);
    color: var(--text);
    font-size: 12.5px;
    font-weight: 600;
  }
  .note {
    margin: 6px 0 2px;
    color: var(--muted);
    font-size: 11.5px;
    line-height: 1.45;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 20px;
    padding: 10px 0;
    border-bottom: 1px solid var(--line);
  }
  .row.top {
    align-items: flex-start;
  }
  .lab {
    display: grid;
    flex: 1 1 180px;
    gap: 3px;
    justify-items: start;
  }
  .lab b {
    font-size: 12.5px;
    font-weight: 560;
  }
  .lab span {
    color: var(--muted);
    font-size: 11.5px;
  }
  .row > .field,
  .row > select {
    width: 260px;
    max-width: 100%;
  }
  .ctl {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .wide {
    flex: 1 0 100%;
  }
  .btn.sm {
    height: 26px;
    padding: 0 10px;
    font-size: 12px;
  }
  .lab .kept {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 20px;
    padding: 0 8px;
    border-radius: 6px;
    font-weight: 550;
  }
  .lab .kept.ok {
    background: color-mix(in oklch, var(--run) 10%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--run) 30%, transparent);
    color: var(--run);
  }
  .lab .kept.ok::before {
    content: '';
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: currentColor;
  }
  .err {
    margin: 0;
    color: var(--del);
    font-size: 12px;
  }
  .err.wide {
    flex: 1 0 100%;
  }
  .del {
    margin-right: auto;
    color: var(--del);
  }
  .used {
    color: var(--muted);
    font-size: 12px;
  }
  .ask {
    flex: 1;
    color: var(--soft);
    font-size: 12px;
    line-height: 1.45;
  }
</style>
