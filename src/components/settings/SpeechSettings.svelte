<script lang="ts">
  import { conversation } from '../../lib/conversation.svelte';
  import { onlineService } from '../../lib/dictation';
  import { fBytes } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { voiceServices } from '../../lib/installed';
  import { api } from '../../lib/ipc';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import { modelIn, onlineVoice } from '../../lib/online-voice.svelte';
  import type { SpeechInfo, SpeechVoice } from '../../lib/types';
  import Chips from './Chips.svelte';
  import Group from './Group.svelte';
  import NoPlugin from './NoPlugin.svelte';
  import OnlineKey from './OnlineKey.svelte';
  import OnlineModel from './OnlineModel.svelte';
  import OnlineVoiceField from './OnlineVoiceField.svelte';
  import Row from './Row.svelte';
  import Icon from '../Icon.svelte';

  // Réglages › Voix, the conversation's part (ESC-97): the voice of each language among those whose licence allows it,
  // « Écouter », its download (size, progress) and its deletion, its licences and the credit it asks for; the pace, the
  // volume, and the silence that sends what was said. Saved with the other settings. « Voix générée par » (ESC-132):
  // Piper, or a service on line with the key of its dictation, its model and its voice by language; Piper's voices are
  // then those that stand in when the service fails.
  const s = $derived(settingsForm.settings.voice.speech);
  const online = $derived(s.engine === 'online');
  const service = $derived(onlineService(s.onlineService));
  /** The services an installed plugin brings (ESC-407): the choice of one whose plugin is gone stays kept, hidden. */
  const services = $derived(voiceServices());
  const serviceIn = $derived(services.some((o) => o.id === service.id));
  /** On line with a service installed: its model, its key and its voices are shown. */
  const shown = $derived(online && serviceIn);
  const catalog = $derived(onlineVoice.catalogs[service.id]);
  const model = $derived(modelIn(catalog, 'speak', s.onlineModels[service.id]).id);
  $effect(() => {
    if (!shown) return;
    void onlineVoice.loadKeys();
    void onlineVoice.load(service.id);
  });
  /** Whether `model` of the service reads `lang`. */
  const reads = (lang: string) => !!catalog?.defaultVoices[`${model}|${lang}`];
  const rateDesc = $derived.by(() => {
    if (!shown) return t('voice.speech.rateDesc');
    const speed = catalog?.speed;
    if (!speed) return t('voice.speech.rateNone', { service: service.label });
    if (service.id === 'deepgram') return t('voice.speech.rateEnglish', { service: service.label, min: speed[0], max: speed[1] });
    return t('voice.speech.rateService', { service: service.label, min: speed[0], max: speed[1] });
  });
  let info = $state<SpeechInfo | null>(null);
  let playing = $state<string | null>(null);
  let downloading = $state<Record<string, boolean>>({});

  const DEFAULTS: Record<string, string> = { fr: 'fr_FR-siwis-medium', en: 'en_US-ljspeech-medium' };
  const LANGS = ['fr', 'en'] as const;

  async function load() {
    info = await api.speechInfo().catch(() => ({ available: false, voices: [] }));
  }
  $effect(() => {
    void load();
  });

  const voicesOf = (lang: string) => (info?.voices ?? []).filter((v) => v.lang === lang);
  const chosen = (lang: string): SpeechVoice | undefined => {
    const id = s.voices[lang] || DEFAULTS[lang];
    return voicesOf(lang).find((v) => v.id === id) ?? voicesOf(lang)[0];
  };
  /** On line: the local voices that read when the service fails, those chosen and downloaded (« Siwis (Français) »). */
  const fallbackVoices = $derived(
    LANGS.flatMap((lang) => {
      const v = chosen(lang);
      return v?.downloaded ? [`${v.name} (${t(`voice.languages.${lang}`)})`] : [];
    }),
  );
  const quality = (q: string) => {
    const k = q === 'x_low' ? 'xLow' : q;
    return ['low', 'medium', 'high', 'xLow'].includes(k) ? t(`voice.speech.quality.${k as 'low'}`) : q;
  };
  const label = (v: SpeechVoice) =>
    t('voice.speech.voiceLabel', {
      name: `${v.name}${v.speakers > 1 ? ` (${v.speakers})` : ''}`,
      quality: quality(v.quality),
      size: fBytes(v.onnxSize + v.jsonSize),
    });

  async function download(v: SpeechVoice) {
    downloading[v.id] = true;
    try {
      await api.speechDownload(v.id);
    } catch (e) {
      app.toast(t('voice.downloadFailed', { error: String(e) }), 'error');
    }
    downloading[v.id] = false;
    delete conversation.voiceDownloads[v.id];
    await load();
  }

  async function remove(v: SpeechVoice) {
    try {
      await api.speechDelete(v.id);
    } catch (e) {
      app.toast(t('voice.deleteFailed', { error: String(e) }), 'error');
    }
    await load();
  }

  async function listen(v: SpeechVoice) {
    playing = v.id;
    try {
      await api.speechPreview(v.id, s.speakers[v.id]);
    } catch (e) {
      app.toast(t('voice.speech.previewFailed', { error: String(e) }), 'error');
    }
    playing = null;
  }

  const pct = (id: string) => {
    const p = conversation.voiceDownloads[id];
    return p && p.total ? Math.round((p.done / p.total) * 100) : 0;
  };
</script>

<Group
  title={t('voice.speech.title')}
  note={shown
    ? t('voice.speech.onlineNote', { service: service.label, host: service.host })
    : online
      ? undefined
      : info && !info.available
        ? t('voice.speech.unavailable')
        : t('voice.speech.note')}
>
  <Row label={t('voice.speech.engine')} desc={t('voice.speech.engineDesc')}>
    <Chips
      label={t('voice.speech.engine')}
      options={[
        { value: 'local', label: t('voice.speech.local') },
        { value: 'online', label: t('voice.speech.online') },
      ]}
      bind:value={s.engine}
    />
  </Row>
  {#if online && !services.length}
    <NoPlugin text={t('settings.noPlugin.voice')} />
  {:else if online}
    <Row label={t('voice.settings.service')} desc={serviceIn ? t('voice.settings.serviceDesc', { service: service.label }) : undefined}>
      <select class="field input service" aria-label={t('voice.speech.serviceLabel')} bind:value={s.onlineService}>
        {#if !serviceIn}<option value={s.onlineService} disabled>{t('voice.settings.servicePick')}</option>{/if}
        {#each services as o (o.id)}
          <option value={o.id}>{o.label}</option>
        {/each}
      </select>
    </Row>
  {/if}
  {#if shown}
    <OnlineModel
      {service}
      purpose="speak"
      bind:models={s.onlineModels}
      desc={t('voice.speech.onlineModelDesc', { service: service.label, model: catalog?.defaultSpeak ?? '' })}
    />
    <OnlineKey {service} />
    {#if catalog}
      {#each LANGS as lang (lang)}
        {#if reads(lang)}
          <OnlineVoiceField
            {catalog}
            {model}
            {lang}
            label={t('voice.speech.voiceFor', { lang: t(`voice.languages.${lang}`) })}
            bind:voices={s.onlineVoices}
          />
        {:else}
          <Row label={t('voice.speech.voiceFor', { lang: t(`voice.languages.${lang}`) })}>
            <p class="meta not-spoken" data-not-spoken={lang}>
              {t('voice.speech.notSpoken', { service: service.label, model, lang: t(`voice.languages.${lang}`) })}
            </p>
          </Row>
        {/if}
      {/each}
    {/if}
  {/if}
  {#if shown}
    <!-- Piper's settings are those of « Local (Piper) »: on line, only what happens when the service fails. -->
    <Row
      label={t('voice.speech.fallbackLabel', { service: service.label })}
      desc={fallbackVoices.length ? t('voice.speech.fallbackReady', { voices: fallbackVoices.join(', ') }) : t('voice.speech.fallbackNone')}
    />
  {/if}
  {#each online ? [] : LANGS as lang (lang)}
    {@const v = chosen(lang)}
    <Row label={t('voice.speech.voiceFor', { lang: t(`voice.languages.${lang}`) })} desc={t('voice.speech.voiceDesc')}>
      <div class="voice" data-speech-lang={lang}>
        <select
          class="field input"
          aria-label={t('voice.speech.voiceFor', { lang: t(`voice.languages.${lang}`) })}
          value={v?.id ?? ''}
          onchange={(e) => (s.voices = { ...s.voices, [lang]: e.currentTarget.value })}
        >
          {#each voicesOf(lang) as o (o.id)}
            <option value={o.id}>{label(o)}</option>
          {/each}
        </select>
        {#if v}
          {#if v.speakers > 1}
            <label class="speaker">
              <span class="meta">{t('voice.speech.speaker')}</span>
              <select
                class="field small-select"
                value={s.speakers[v.id] ?? 0}
                onchange={(e) => (s.speakers = { ...s.speakers, [v.id]: Number(e.currentTarget.value) })}
              >
                {#each Array.from({ length: v.speakers }, (_, i) => i) as i (i)}
                  <option value={i}>{v.speakerNames[i] && !/^voice-\d+$/.test(v.speakerNames[i]) ? v.speakerNames[i] : `#${i + 1}`}</option>
                {/each}
              </select>
            </label>
          {/if}
          <div class="actions">
            {#if downloading[v.id]}
              <span class="meta mono">{pct(v.id)} %</span>
              <button class="btn ghost small" onclick={() => api.speechDownloadCancel(v.id)}>{t('voice.conversation.cancel')}</button>
            {:else if v.downloaded}
              <button class="btn small" disabled={!info?.available || playing !== null} onclick={() => listen(v)}
                >{#if playing !== v.id}<Icon name="play" size={14} />{/if}{playing === v.id
                  ? t('voice.speech.listening')
                  : t('voice.speech.listen')}</button
              >
              <button class="btn ghost small" onclick={() => remove(v)}>{t('voice.speech.delete')}</button>
            {:else}
              <button class="btn small" disabled={!info?.available} onclick={() => download(v)}
                >{t('voice.speech.download')} ({fBytes(v.onnxSize + v.jsonSize)})</button
              >
            {/if}
          </div>
          <p class="licences" data-speech-licence>
            {v.license ? t('voice.speech.licence', { license: v.license }) : t('voice.speech.licenceNone')} ·
            {v.datasetLicense && !/^see /i.test(v.datasetLicense)
              ? t('voice.speech.dataset', { license: v.datasetLicense })
              : t('voice.speech.datasetUnknown')}
            {#if v.attribution}<br />{t('voice.speech.attribution', { text: v.attribution })}{/if}
            {#if v.exception}<br /><span class="exc" data-speech-exception>{t('voice.speech.exception')}</span>{/if}
          </p>
        {/if}
      </div>
    </Row>
  {/each}
  <Row label={t('voice.speech.rate')} desc={rateDesc}>
    <div class="slider">
      <input type="range" min="0.5" max="2" step="0.1" aria-label={t('voice.speech.rate')} bind:value={s.rate} />
      <span class="meta mono">{t('voice.speech.rateValue', { n: s.rate.toFixed(1) })}</span>
    </div>
  </Row>
  <Row label={t('voice.speech.volume')} desc={online ? t('voice.speech.volumeDesc') : undefined}>
    <div class="slider">
      <input type="range" min="0" max="1" step="0.05" aria-label={t('voice.speech.volume')} bind:value={s.volume} />
      <span class="meta mono">{Math.round(s.volume * 100)} %</span>
    </div>
  </Row>
  <Row label={t('voice.speech.silence')} desc={t('voice.speech.silenceDesc')}>
    <div class="slider">
      <input type="range" min="500" max="3000" step="100" aria-label={t('voice.speech.silence')} bind:value={s.silenceMs} />
      <span class="meta mono">{t('voice.speech.seconds', { n: (s.silenceMs / 1000).toFixed(1) })}</span>
    </div>
  </Row>
</Group>

<style>
  .voice {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
    width: 360px;
    max-width: 100%;
  }
  .input {
    width: 100%;
  }
  .service {
    width: 320px;
    max-width: 100%;
  }
  .not-spoken {
    margin: 0;
    text-align: right;
  }
  .speaker {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .small-select {
    height: 26px;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .licences {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.45;
    text-align: right;
    overflow-wrap: anywhere;
  }
  /* Fuze's exception to the rule of the licences, said in amber (lot 0 of the 2.0 design). */
  .exc {
    color: var(--warn-ink);
  }
  .meta {
    color: var(--muted);
    font-size: 12px;
  }
  .small {
    height: 26px;
    padding: 0 9px;
    font-size: 12px;
  }
  .slider {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .slider input {
    width: 200px;
    max-width: 100%;
    accent-color: color-mix(in oklch, var(--p) 75%, black);
  }
  .slider .meta {
    min-width: 40px;
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
</style>
