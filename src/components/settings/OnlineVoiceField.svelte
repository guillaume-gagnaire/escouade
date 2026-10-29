<script lang="ts">
  import Icon from '../Icon.svelte';
  import { openUrl } from '../../lib/opener';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { voiceKey, voicesFor } from '../../lib/online-voice.svelte';
  import { app } from '../../lib/state.svelte';
  import type { ServiceCatalog } from '../../lib/types';
  import Row from './Row.svelte';

  // « Voix en … » of a service on line (ESC-132): when its API lists its voices (the user's own among them), a field
  // that completes as one types (name, id, language, gender or description), picked with the arrows and Enter; else a
  // free field, for the voice's name or id as the service expects it, with an example and the page of its voices.
  // Empty: the service's default voice. « ▶ Écouter » reads a sentence with it, and says at once a voice it refuses.
  let {
    catalog,
    label,
    model,
    lang,
    voices = $bindable(),
  }: {
    catalog: ServiceCatalog;
    label: string;
    model: string;
    lang: string;
    voices: Record<string, string>;
  } = $props();

  const key = $derived(voiceKey(catalog, model, lang));
  const chosen = $derived((voices[key] ?? '').trim());
  const def = $derived(catalog.defaultVoices[`${model}|${lang}`] ?? '');
  const listed = $derived(catalog.voices !== null);
  const id = $derived(`voice-${catalog.service}-${lang}`);
  let query = $state('');
  let open = $state(false);
  let active = $state(0);
  let playing = $state(false);
  const matches = $derived(listed ? voicesFor(catalog, model, lang, query).slice(0, 50) : []);
  const nameOf = (v: string) => catalog.voices?.find((x) => x.id === v)?.name ?? v;

  function set(v: string) {
    const next = { ...voices };
    if (!v.trim() || v.trim() === def) delete next[key];
    else next[key] = v.trim();
    voices = next;
  }

  function pick(v: string) {
    set(v);
    open = false;
    query = '';
  }

  function keydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      open = true;
      active = Math.min(active + 1, matches.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      active = Math.max(active - 1, 0);
    } else if (e.key === 'Enter' && open && matches[active]) {
      e.preventDefault();
      pick(matches[active].id);
    } else if (e.key === 'Escape' && open) {
      e.preventDefault();
      e.stopPropagation();
      open = false;
      query = '';
    }
  }

  async function listen() {
    playing = true;
    try {
      await api.speechPreviewOnline(catalog.service, model, chosen || def, lang);
    } catch (e) {
      app.toast(t('voice.speech.previewFailed', { error: String(e) }), 'error');
    }
    playing = false;
  }

  const desc = $derived(
    listed
      ? t('voice.speech.onlineVoiceListed', { default: nameOf(def) })
      : t('voice.speech.onlineVoiceFree', { example: catalog.voiceExample, default: def }),
  );
</script>

<Row {label} {desc}>
  <div class="voice" data-online-voice={lang}>
    <div class="line">
      {#if listed}
        <div class="combo">
          <input
            class="field input"
            role="combobox"
            aria-label={label}
            aria-expanded={open}
            aria-controls={`${id}-list`}
            aria-autocomplete="list"
            aria-activedescendant={open && matches[active] ? `${id}-${active}` : undefined}
            placeholder={nameOf(def)}
            value={open ? query : chosen ? nameOf(chosen) : ''}
            onfocus={() => {
              open = true;
              query = '';
              active = 0;
            }}
            oninput={(e) => {
              query = e.currentTarget.value;
              open = true;
              active = 0;
            }}
            onblur={() => setTimeout(() => (open = false), 150)}
            onkeydown={keydown}
          />
          {#if open}
            <ul class="list" id={`${id}-list`} role="listbox">
              {#each matches as v, i (v.id)}
                <li
                  id={`${id}-${i}`}
                  role="option"
                  aria-selected={i === active}
                  class:active={i === active}
                  onmousedown={(e) => (e.preventDefault(), pick(v.id))}
                >
                  <span class="name">{v.name}</span>
                  {#if v.own}<span class="own">{t('voice.speech.own')}</span>{/if}
                  <span class="note">{[v.note, v.langs.join(', ')].filter(Boolean).join(' · ')}</span>
                </li>
              {:else}
                <li class="none">{t('voice.speech.noVoices')}</li>
              {/each}
            </ul>
          {/if}
        </div>
      {:else}
        <input
          class="field input mono"
          aria-label={label}
          placeholder={def}
          spellcheck="false"
          value={chosen}
          onchange={(e) => set(e.currentTarget.value)}
        />
      {/if}
      <button class="btn small" disabled={playing} onclick={listen}
        >{#if !playing}<Icon name="play" size={14} />{/if}{playing ? t('voice.speech.listening') : t('voice.speech.listen')}</button
      >
    </div>
    <button class="link" onclick={() => openUrl(catalog.voicesUrl).catch((e) => app.toast(String(e), 'error'))}
      >{t('voice.speech.voicesPage')}</button
    >
  </div>
</Row>

<style>
  .voice {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 4px;
    width: 360px;
    max-width: 100%;
  }
  .line {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
  }
  .combo {
    position: relative;
    flex: 1;
    min-width: 0;
  }
  .input {
    width: 100%;
    flex: 1;
    min-width: 0;
  }
  .list {
    position: absolute;
    z-index: 20;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    max-height: 240px;
    overflow: auto;
    margin: 0;
    padding: 4px;
    list-style: none;
    background: var(--well);
    border: 1px solid var(--line-2);
    border-radius: 7px;
  }
  .list li {
    display: flex;
    align-items: baseline;
    gap: 8px;
    padding: 5px 8px;
    border-radius: 7px;
    font-size: 12px;
    cursor: pointer;
  }
  .list li.active {
    background: var(--line);
  }
  .list .none {
    color: var(--muted);
    cursor: default;
  }
  .note {
    margin-left: auto;
    font-size: 11px;
    color: var(--muted);
    text-align: right;
  }
  .own {
    font-size: 10.5px;
    color: var(--p-ink);
  }
  .small {
    height: 24px;
    padding: 0 8px;
    font-size: 11.5px;
    flex: none;
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    font-size: 11.5px;
    color: var(--muted);
    text-decoration: underline;
    cursor: pointer;
  }
</style>
