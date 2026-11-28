<script lang="ts">
  import { DEFAULT_SHORTCUT, duration, liveOn, onlineService, shortcutLabel } from '../lib/dictation';
  import { dictation } from '../lib/dictation.svelte';
  import { fBytes } from '../lib/format';
  import { t } from '../lib/i18n';
  import { keyLabel } from '../lib/platform';
  import { app } from '../lib/state.svelte';
  import Icon from './Icon.svelte';
  import Level from './Level.svelte';

  // The dictation of a field, in the field itself (2.0 design): the level, the time, what the service heard so far, the
  // keys; the first time, the offer of the model and its download, at the same place. `targetId`: the field's
  // (`DictationTarget.id`), whose microphone draws no panel of its own.
  let { targetId }: { targetId: string } = $props();

  const phase = $derived(dictation.targetId === targetId ? dictation.phase : 'idle');
  const key = $derived(shortcutLabel(app.settings.voice?.shortcut || DEFAULT_SHORTCUT));
  const online = $derived(app.settings.voice?.transcription === 'online');
  const service = $derived(onlineService(app.settings.voice?.onlineService));
  const live = $derived(online && liveOn(app.settings.voice, service));
  const progress = $derived(dictation.progress);
  const pct = $derived(progress && progress.total ? Math.min(100, (progress.done / progress.total) * 100) : 0);
</script>

{#if phase !== 'idle' && app.feature('voice')}
  <div class="dict" class:offer={phase === 'offer' || phase === 'downloading'} role="status" aria-live="polite" data-voice-phase={phase}>
    {#if phase === 'starting'}
      <span class="muted">{t('voice.panel.starting')}</span>
    {:else if phase === 'listening' || phase === 'transcribing'}
      {#if phase === 'listening'}
        <Level level={dictation.level} label={t('voice.panel.level')} />
        <b>{t('voice.panel.listening')}</b><span class="muted time">{duration(dictation.ms)}</span>
      {:else}
        <span class="muted">{online ? t('voice.panel.transcribingOnline', { service: service.label }) : t('voice.panel.transcribing')}</span
        >
      {/if}
      <!-- What the service understood so far (ESC-136), its end paler while it may still change. -->
      {#if dictation.partial}
        <span class="live-text" data-voice-partial>{dictation.partial}</span>
      {:else}
        <span class="grow"></span>
      {/if}
      {#if live}<span class="live-tag"><i></i>{t('voice.panel.live', { service: service.label })}</span>{/if}
      {#if phase === 'listening'}
        <span class="hint">{t('voice.panel.keys', { key, escape: keyLabel('Esc') })}</span>
        <span class="hint-short">{t('voice.panel.keysShort')}</span>
      {/if}
    {:else if phase === 'offer' && dictation.offer}
      <Icon name="mic" size={14} />
      <div class="what">
        <b>{t('voice.panel.offerTitle')}</b>
        <p>{t('voice.panel.offerBody', { model: dictation.offer.model, size: fBytes(dictation.offer.size) })}</p>
      </div>
      <div class="acts">
        <button class="btn" onclick={() => dictation.cancel()}>{t('voice.panel.cancel')}</button>
        <button class="btn primary" onclick={() => dictation.download()}>{t('voice.panel.download')}</button>
      </div>
    {:else if phase === 'downloading' && progress}
      <Icon name="mic" size={14} />
      <div class="what">
        <b>{t('voice.panel.offerTitle')}</b>
        <p>{t('voice.panel.downloading', { model: progress.model, done: fBytes(progress.done), total: fBytes(progress.total) })}</p>
      </div>
      <div class="dl">
        <span class="gauge" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(pct)}
          ><i class="fill" style:width="{pct}%"></i></span
        >
        <button class="btn" onclick={() => dictation.cancel()}>{t('voice.panel.cancel')}</button>
      </div>
    {/if}
  </div>
{/if}

<style>
  .dict {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    margin: 8px 8px 0;
    padding: 9px 12px;
    border-radius: 10px;
    background: color-mix(in oklch, var(--p) 10%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--p) 32%, transparent);
    font-size: 12.5px;
  }
  .dict b {
    flex: none;
    font-weight: 600;
  }
  .muted {
    flex: none;
    color: var(--muted);
  }
  .time {
    font-variant-numeric: tabular-nums;
  }
  .grow {
    flex: 1;
  }
  .live-text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    color: var(--soft);
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .live-tag {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--muted);
    font-size: 12px;
  }
  .live-tag i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--del);
  }
  .hint {
    flex: none;
    margin-left: auto;
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
  }
  .live-tag + .hint {
    margin-left: 0;
  }
  /* The short form, for a narrow composer: the second click named all the same (ESC-664). */
  .hint-short {
    display: none;
    flex: none;
    margin-left: auto;
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
  }
  /* The model to download, the first time: what it is, then its buttons or its gauge. */
  .dict.offer {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr) auto;
    align-items: start;
    background: rgba(var(--fg-rgb), 0.035);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .offer > :global(.icon) {
    margin-top: 2px;
    color: var(--p-ink);
  }
  .what p {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: 12px;
  }
  .acts {
    display: flex;
    gap: 6px;
  }
  .dl {
    display: grid;
    gap: 6px;
    justify-items: end;
    min-width: 180px;
  }
  .dl .gauge {
    width: 100%;
  }
  .dl .gauge .fill {
    background: var(--p-ink);
  }
  /* A narrow composer keeps the level, the words and the time; the keys go (the microphone's tooltip says them), not the
     second click on the microphone, which a user did not guess (ESC-664). */
  @container conv (max-width: 760px) {
    .hint {
      display: none;
    }
    .hint-short {
      display: inline;
    }
  }
</style>
