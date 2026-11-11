<script lang="ts">
  import { DEFAULT_SHORTCUT, liveOn, onlineService, shortcutLabel, shortcutOf } from '../../lib/dictation';
  import { dictation } from '../../lib/dictation.svelte';
  import { fBytes } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { voiceServices } from '../../lib/installed';
  import { api } from '../../lib/ipc';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import type { VoiceInfo } from '../../lib/types';
  import Chips from './Chips.svelte';
  import { onlineVoice } from '../../lib/online-voice.svelte';
  import Group from './Group.svelte';
  import NoPlugin from './NoPlugin.svelte';
  import OnlineKey from './OnlineKey.svelte';
  import OnlineModel from './OnlineModel.svelte';
  import Row from './Row.svelte';
  import SpeechSettings from './SpeechSettings.svelte';
  import { tip } from '../../lib/tip';
  import Icon from '../Icon.svelte';

  // « Voix »: the microphone, who transcribes (Whisper and its model, downloaded or deleted at once; or a service on
  // line, its key kept in the keychain at once), the language spoken and the shortcut, saved with the other settings.
  const v = $derived(settingsForm.settings.voice);
  let info = $state<VoiceInfo | null>(null);
  let recording = $state(false);
  /** The model being downloaded from here, and how far. */
  const progress = $derived(dictation.progress);
  let busy = $state<string | null>(null);

  async function load() {
    info = await api.voiceInfo().catch(() => ({ devices: [], models: [] }));
  }
  $effect(() => {
    void load();
  });

  const LANGS = ['fr', 'en', 'de', 'es', 'it', 'pt', 'nl'] as const;
  const languageOptions: { value: string; label: string; lang?: string }[] = $derived([
    { value: 'ui', label: t('voice.settings.languageUi') },
    { value: 'auto', label: t('voice.settings.languageAuto') },
    ...LANGS.map((l) => ({ value: l, label: t(`voice.languages.${l}`), lang: l })),
  ]);
  const systemDevice = $derived(info?.devices.find((d) => d.default));

  async function download(model: string) {
    busy = model;
    dictation.offer = { model, size: info?.models.find((m) => m.id === model)?.size ?? 0 };
    dictation.phase = 'offer';
    await dictation.download();
    busy = null;
    await load();
  }

  async function remove(model: string) {
    try {
      await api.voiceDeleteModel(model);
    } catch (e) {
      app.toast(t('voice.deleteFailed', { error: String(e) }), 'error');
    }
    await load();
  }

  // « En ligne » (ESC-110): the service chosen with the draft, its key kept in the keychain at once (never shown back),
  // the same as the conversation's voice's; its model (ESC-132), its API's list when it gives one.
  const online = $derived(v.transcription === 'online');
  const service = $derived(onlineService(v.onlineService));
  /**
   * The services an installed plugin brings (ESC-407), and whether the one chosen is among them: the choice of one whose
   * plugin is gone stays kept (its key in the keychain too), and comes back with it.
   */
  const services = $derived(voiceServices());
  const serviceIn = $derived(services.some((s) => s.id === service.id));
  $effect(() => {
    void onlineVoice.loadKeys();
  });
  $effect(() => {
    if (online && serviceIn) void onlineVoice.load(service.id);
  });
  // « Transcription en temps réel » (ESC-136): offered by the service, on unless unticked, kept by service.
  const live = $derived(liveOn(v, service));
  const liveModel = $derived(service.live || v.onlineModels[service.id] || service.model);
  function setLive(on: boolean) {
    v.onlineLive = { ...(v.onlineLive ?? {}), [service.id]: on };
  }

  function record(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === 'Escape') return stopRecording();
    const s = shortcutOf(e);
    if (!s) return;
    v.shortcut = s;
    stopRecording();
  }

  function startRecording() {
    recording = true;
    dictation.recording = true;
  }

  function stopRecording() {
    recording = false;
    dictation.recording = false;
  }

  $effect(() => () => (dictation.recording = false));
</script>

<!-- « Transcription » (choice 34 of the 2.0 design): local or online first, what goes online said under it, then the
     microphone, the shortcut, the language and Whisper's models. -->
<Group title={t('voice.settings.transcription')}>
  <Row label={t('voice.settings.engine')} desc={t('voice.settings.engineDesc')}>
    <Chips
      label={t('voice.settings.engine')}
      options={[
        { value: 'local', label: t('voice.settings.local') },
        { value: 'online', label: t('voice.settings.online') },
      ]}
      bind:value={v.transcription}
    />
  </Row>
  <p class="vnote">
    {online
      ? serviceIn
        ? t(live ? 'voice.settings.onlineNoteLive' : 'voice.settings.onlineNote', { service: service.label, host: service.host })
        : ''
      : t('voice.settings.transcriptionNote')}
  </p>
  {#if online && !services.length}
    <NoPlugin text={t('settings.noPlugin.voice')} />
  {:else if online}
    <Row label={t('voice.settings.service')} desc={serviceIn ? t('voice.settings.serviceDesc', { service: service.label }) : undefined}>
      <select class="field input" aria-label={t('voice.settings.service')} bind:value={v.onlineService}>
        {#if !serviceIn}<option value={v.onlineService} disabled>{t('voice.settings.servicePick')}</option>{/if}
        {#each services as s (s.id)}
          <option value={s.id}>{s.label}</option>
        {/each}
      </select>
    </Row>
  {/if}
  {#if online && serviceIn}
    <OnlineModel
      {service}
      purpose="transcribe"
      bind:models={v.onlineModels}
      desc={t('voice.settings.onlineModelDesc', { service: service.label, model: service.model })}
    />
    {#if service.live !== null}
      <Row
        label={t('voice.settings.live')}
        desc={`${t('voice.settings.liveDesc', { service: service.label, model: liveModel })} ${t('voice.settings.livePrice', { service: service.label })}`}
      >
        <input
          class="switch"
          type="checkbox"
          role="switch"
          data-voice-live
          aria-label={t('voice.settings.live')}
          checked={live}
          onchange={(e) => setLive(e.currentTarget.checked)}
        />
      </Row>
    {/if}
    <OnlineKey {service} />
  {/if}
  <Row label={t('voice.settings.device')} desc={info && !info.devices.length ? t('voice.settings.noDevice') : undefined}>
    <select class="field input" aria-label={t('voice.settings.device')} bind:value={v.device}>
      <option value=""
        >{systemDevice ? t('voice.settings.deviceSystemNamed', { name: systemDevice.name }) : t('voice.settings.deviceSystem')}</option
      >
      {#each info?.devices ?? [] as d (d.id)}
        <option value={d.id}>{d.name}</option>
      {/each}
      {#if v.device && info && !info.devices.some((d) => d.id === v.device)}
        <option value={v.device}>{v.device}</option>
      {/if}
    </select>
  </Row>
  <Row label={t('voice.settings.shortcut')} desc={t('voice.settings.shortcutDesc')}>
    {#if recording}
      <!-- svelte-ignore a11y_autofocus -->
      <button class="btn recording" autofocus onkeydown={record} onblur={stopRecording}>{t('voice.settings.recording')}</button>
      <span class="meta">{t('voice.settings.recordingHint')}</span>
    {:else}
      <kbd class="key">{shortcutLabel(v.shortcut || DEFAULT_SHORTCUT)}</kbd>
      <button class="btn" onclick={startRecording}>{t('voice.settings.change')}</button>
      {#if v.shortcut !== DEFAULT_SHORTCUT}
        <button class="btn ghost" onclick={() => (v.shortcut = DEFAULT_SHORTCUT)}>{t('voice.settings.restore')}</button>
      {/if}
    {/if}
  </Row>
  <Row label={t('voice.settings.language')} desc={t('voice.settings.languageDesc')}>
    <!-- Nine of them: a menu, as the preview has it. Each language named in itself. -->
    <select class="field" aria-label={t('voice.settings.language')} bind:value={v.language}>
      {#each languageOptions as o (o.value)}<option value={o.value} lang={o.lang}>{o.label}</option>{/each}
    </select>
  </Row>
  {#if !online}
    <!-- The list's own title: its note is not « Langue parlée »'s (ESC-84). -->
    <Row label={t('voice.settings.model')} desc={t('voice.settings.modelDesc')} wide />
    <div class="models model-list" role="radiogroup" aria-label={t('voice.settings.model')}>
      {#each info?.models ?? [] as m (m.id)}
        {@const on = v.model === m.id}
        <div class="model mrow" class:on>
          <label class="pick">
            <input class="rd" type="radio" name="voice-model" value={m.id} checked={on} onchange={() => (v.model = m.id)} />
            <span class="mn"
              ><b>{m.id}</b><span>{fBytes(m.size)} · {t('voice.settings.modelSpeed', { speed: m.speed, quality: m.quality })}</span></span
            >
          </label>
          {#if progress && progress.model === m.id}
            {@const pct = Math.round((progress.done / Math.max(1, progress.total)) * 100)}
            <span class="dl"><span class="gauge"><span class="fill" style:width="{pct}%"></span></span>{pct} %</span>
            <button class="btn" onclick={() => dictation.cancel()}>{t('voice.panel.cancel')}</button>
          {:else if m.downloaded}
            <span class="ok"><Icon name="check" size={14} />{t('voice.settings.downloaded')}</span>
            <button
              class="btn"
              use:tip={t('voice.settings.deleteTitle', { model: m.id, size: fBytes(m.size) })}
              onclick={() => remove(m.id)}>{t('voice.settings.delete')}</button
            >
          {:else}
            <span></span>
            <button class="btn" disabled={!!busy || dictation.phase !== 'idle'} onclick={() => download(m.id)}
              >{t('voice.settings.download')}</button
            >
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</Group>

<SpeechSettings />

<style>
  .vnote {
    max-width: 64ch;
    margin: 0 0 6px;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }
  .input {
    width: 320px;
    max-width: 100%;
  }
  /* A switch that is a box (the e2e tick it): drawn as the others. */
  input.switch {
    appearance: none;
    margin: 0;
  }
  input.switch:checked {
    background: color-mix(in oklch, var(--p) 75%, black);
  }
  input.switch:checked::after {
    left: 16px;
  }
  .meta {
    color: var(--muted);
    font-size: 12px;
  }
  .key {
    height: 26px;
    padding: 0 8px;
    font-size: 12px;
  }
  .recording {
    box-shadow: inset 0 0 0 1px var(--p-ink);
  }
  /* Whisper's models, a line each under a hairline (`.model-list` of the 2.0 design). */
  .model-list {
    display: grid;
    margin-top: 6px;
    border-top: 1px solid rgba(var(--fg-rgb), 0.05);
  }
  .mrow {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 12px;
    padding: 10px 0;
    border-bottom: 1px solid rgba(var(--fg-rgb), 0.05);
  }
  .pick {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    cursor: pointer;
  }
  .rd {
    flex: none;
    width: 16px;
    height: 16px;
    margin: 0 1px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1.5px var(--ring);
    appearance: none;
    cursor: pointer;
  }
  .rd:checked {
    box-shadow: inset 0 0 0 5px color-mix(in oklch, var(--p) 75%, white);
  }
  .mn {
    display: grid;
    gap: 1px;
    min-width: 0;
  }
  .mn b {
    font-weight: 550;
  }
  .mn span {
    color: var(--muted);
    font-size: 12px;
  }
  .ok {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--add);
    font-size: 12px;
  }
  .dl {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 150px;
    color: var(--muted);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .dl .gauge {
    flex: 1;
  }
</style>
