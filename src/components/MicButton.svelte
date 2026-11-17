<script lang="ts">
  import { DEFAULT_SHORTCUT, duration, onlineService, shortcutLabel } from '../lib/dictation';
  import { dictation, type DictationTarget } from '../lib/dictation.svelte';
  import { fBytes } from '../lib/format';
  import { t } from '../lib/i18n';
  import { app } from '../lib/state.svelte';
  import { tip } from '../lib/tip';
  import { HOVER_REST_MS, offer } from '../lib/discover.svelte';
  import Icon from './Icon.svelte';
  import Level from './Level.svelte';

  // The microphone beside a field: a click starts the dictation and another ends it, a press held dictates until it is
  // let go. While it listens, its panel shows the level and the time; the first time, it offers the model's download.
  // `panel`: false where the field draws the dictation itself (the composer's band, DictationBand).
  let { target, below = false, panel = true }: { target: DictationTarget; below?: boolean; panel?: boolean } = $props();

  $effect(() => dictation.register(target));

  const mine = $derived(dictation.targetId === target.id);
  const phase = $derived(mine ? dictation.phase : 'idle');
  const listening = $derived(phase === 'listening');
  const key = $derived(shortcutLabel(app.settings.voice?.shortcut || DEFAULT_SHORTCUT));
  const progress = $derived(dictation.progress);
  const pct = $derived(progress && progress.total ? Math.min(100, (progress.done / progress.total) * 100) : 0);

  function down(e: PointerEvent) {
    // Only the main button (a right click is a menu).
    if (e.button > 0) return;
    // The field keeps the focus, and its cursor: the text goes there.
    e.preventDefault();
    (e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId);
    dictation.press(target);
  }

  // The dictation offered (ESC-506) on a hover that rests on the microphone, never on one that goes on to a press:
  // the bubble would cover the panel the press opens.
  let rest: ReturnType<typeof setTimeout> | undefined;
  function hovered(el: HTMLElement) {
    clearTimeout(rest);
    if (phase !== 'idle') return;
    rest = setTimeout(() => {
      if (phase === 'idle') offer('dictation', el);
    }, HOVER_REST_MS);
  }
  function unhovered() {
    clearTimeout(rest);
  }

  function key_(e: KeyboardEvent, isDown: boolean) {
    if (e.key !== 'Enter' && e.key !== ' ') return;
    e.preventDefault();
    if (isDown && !e.repeat) dictation.press(target);
    if (!isDown) dictation.release();
  }
</script>

<div class="mic-wrap">
  {#if panel && phase !== 'idle'}
    <div class="panel" class:below role="status" aria-live="polite" data-voice-phase={phase}>
      {#if phase === 'starting'}
        <span class="muted">{t('voice.panel.starting')}</span>
      {:else if phase === 'listening'}
        <div class="row">
          <span class="dot" aria-hidden="true"></span>
          <span class="strong">{t('voice.panel.listening')}</span>
          <span class="time mono">{duration(dictation.ms)}</span>
        </div>
        <Level level={dictation.level} label={t('voice.panel.level')} />
        {#if dictation.partial}
          <p class="partial" data-voice-partial><span>{dictation.partial}</span></p>
        {/if}
        <span class="muted small">{t('voice.panel.hint', { key })}</span>
      {:else if phase === 'transcribing'}
        <span class="muted"
          >{app.settings.voice?.transcription === 'online'
            ? t('voice.panel.transcribingOnline', { service: onlineService(app.settings.voice.onlineService).label })
            : t('voice.panel.transcribing')}</span
        >
        {#if dictation.partial}
          <p class="partial" data-voice-partial><span>{dictation.partial}</span></p>
        {/if}
      {:else if phase === 'offer' && dictation.offer}
        <span class="strong">{t('voice.panel.offerTitle')}</span>
        <span class="muted small">{t('voice.panel.offerBody', { model: dictation.offer.model, size: fBytes(dictation.offer.size) })}</span>
        <div class="actions">
          <button class="btn ghost" onclick={() => dictation.cancel()}>{t('voice.panel.cancel')}</button>
          <button class="btn primary" onclick={() => dictation.download()}>{t('voice.panel.download')}</button>
        </div>
      {:else if phase === 'downloading' && progress}
        <span class="muted small"
          >{t('voice.panel.downloading', { model: progress.model, done: fBytes(progress.done), total: fBytes(progress.total) })}</span
        >
        <div class="gauge dl" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={Math.round(pct)}>
          <div class="fill" style:width="{pct}%"></div>
        </div>
        <div class="actions">
          <button class="btn ghost" onclick={() => dictation.cancel()}>{t('voice.panel.cancel')}</button>
        </div>
      {/if}
    </div>
  {/if}
  <!-- Hovered the first time (ESC-506): the dictation offered, in a bubble on it, once. -->
  <button
    class="icon-btn mic"
    data-discover="dictation"
    onmouseenter={(e) => hovered(e.currentTarget)}
    onmouseleave={unhovered}
    class:on={listening}
    class:busy={phase === 'transcribing' || phase === 'starting'}
    aria-pressed={listening}
    aria-label={listening ? t('voice.button.stop') : t('voice.button.label')}
    aria-keyshortcuts={(app.settings.voice?.shortcut || DEFAULT_SHORTCUT).replace('Mod', 'Control')}
    use:tip={t('voice.button.title', { key })}
    onpointerdown={(e) => {
      unhovered();
      down(e);
    }}
    onpointerup={() => dictation.release()}
    onpointercancel={() => dictation.release()}
    onkeydown={(e) => key_(e, true)}
    onkeyup={(e) => key_(e, false)}
  >
    <Icon name="mic" size={14} />
  </button>
</div>

<style>
  .mic-wrap {
    position: relative;
    flex: none;
  }
  .mic {
    position: relative;
  }
  /* Listening: in the project's colour, as the band that says it. */
  .mic.on {
    background: color-mix(in oklch, var(--p) 14%, transparent);
    color: var(--p-ink);
  }
  .mic.busy {
    opacity: 0.6;
  }
  .panel {
    position: absolute;
    right: 0;
    bottom: calc(100% + 8px);
    z-index: 40;
    width: 300px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--raise-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 16px 40px -12px rgba(0, 0, 0, calc(0.7 * var(--shade)));
    font-size: 12.5px;
    text-align: left;
  }
  .panel.below {
    bottom: auto;
    top: calc(100% + 8px);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--del);
    animation: pulse 1.2s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }
  .strong {
    font-weight: 600;
    color: var(--text);
  }
  .time {
    margin-left: auto;
    color: var(--muted);
  }
  .muted {
    color: var(--muted);
  }
  .small {
    font-size: 12px;
    line-height: 1.45;
  }
  /* The download's gauge (src/app.css), the width of the panel. */
  .gauge.dl {
    width: 100%;
  }
  /* What the service understood so far (ESC-136): the last lines, as they are written. */
  .partial {
    margin: 0;
    max-height: 7.5em;
    overflow: hidden;
    display: flex;
    flex-direction: column-reverse;
    color: var(--text);
    font-size: 13px;
    line-height: 1.5;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }
  @media (prefers-reduced-motion: reduce) {
    .dot {
      animation: none;
    }
  }
</style>
