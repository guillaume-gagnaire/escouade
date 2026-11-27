<script lang="ts">
  import { onDestroy } from 'svelte';
  import { t } from '../../lib/i18n';
  import { dbfs, meterPos, micMeter, SENSITIVITY_MAX, sensitivityText } from '../../lib/mic-meter.svelte';
  import { settingsForm } from '../../lib/settings.svelte';
  import Row from './Row.svelte';

  // « Sensibilité du micro » (ESC-665), in Réglages › Voix, under « Délai avant l’envoi »: the slider, kept with the other
  // settings (on the machine), and « Tester », which listens to the microphone chosen and shows, live, the level of the
  // voice, the room's noise learnt, the line of the threshold (moved by the slider at once), and « Voix détectée » while
  // the detector hears a sentence. It stops by « Arrêter » or when the page is left; nothing of the sound is kept.
  const s = $derived(settingsForm.settings.voice.speech);
  const voice = $derived(settingsForm.settings.voice);

  $effect(() => {
    micMeter.set(s.sensitivity);
  });
  onDestroy(() => micMeter.stop());

  const toggle = () => (micMeter.running ? micMeter.stop() : void micMeter.start(voice.device, s.sensitivity, s.silenceMs));
  const pct = (rms: number) => `${(meterPos(rms) * 100).toFixed(1)}%`;
  /** In dB; under -90 (a microphone that gives digital silence), said as such. */
  const db = (rms: number) => {
    const n = Math.round(dbfs(rms));
    return n < -90 ? t('voice.meter.under', { n: -90 }) : t('voice.meter.db', { n });
  };
  const value = $derived(sensitivityText(s.sensitivity, t('voice.speech.sensitivityDefault')));
</script>

<Row
  label={t('voice.speech.sensitivity')}
  desc={t('voice.speech.sensitivityDesc')}
  error={micMeter.error ? t('voice.meter.failed', { error: micMeter.error }) : ''}
>
  <div class="sens">
    <div class="slider">
      <span class="end">{t('voice.speech.sensitivityLess')}</span>
      <input
        type="range"
        min={-SENSITIVITY_MAX}
        max={SENSITIVITY_MAX}
        step="1"
        aria-label={t('voice.speech.sensitivity')}
        aria-valuetext={value}
        bind:value={s.sensitivity}
      />
      <span class="end">{t('voice.speech.sensitivityMore')}</span>
      <span class="meta mono" data-sensitivity-value>{value}</span>
    </div>
    <button class="btn small" aria-pressed={micMeter.running} onclick={toggle} data-mic-test
      >{micMeter.running ? t('voice.meter.stop') : t('voice.meter.test')}</button
    >
  </div>
</Row>
{#if micMeter.running}
  <div class="tester" role="group" aria-label={t('voice.meter.label')} data-mic-tester>
    <div class="top">
      <span class="witness" class:on={micMeter.speech} data-voice-heard={micMeter.speech} role="status"
        ><span class="dot" aria-hidden="true"></span>{micMeter.speech ? t('voice.meter.heard') : t('voice.meter.notHeard')}</span
      >
      <span class="meta">{micMeter.heard ? t('voice.meter.hint') : t('voice.meter.learning')}</span>
    </div>
    <div
      class="track"
      role="meter"
      aria-label={t('voice.meter.level')}
      aria-valuemin="0"
      aria-valuemax="100"
      aria-valuenow={Math.round(meterPos(micMeter.level) * 100)}
      aria-valuetext={db(micMeter.level)}
    >
      <div class="fill" class:over={micMeter.level > micMeter.threshold} style:width={pct(micMeter.level)}></div>
      {#if micMeter.heard}
        <div class="noise" style:width={pct(micMeter.noise)} data-noise={micMeter.noise}></div>
      {/if}
      <div class="line" style:left={pct(micMeter.threshold)} data-threshold={micMeter.threshold}></div>
    </div>
    <dl class="legend">
      <div>
        <dt><span class="key k-level"></span>{t('voice.meter.level')}</dt>
        <dd class="mono">{db(micMeter.level)}</dd>
      </div>
      <div>
        <dt><span class="key k-noise"></span>{t('voice.meter.noise')}</dt>
        <dd class="mono">{micMeter.heard ? db(micMeter.noise) : '…'}</dd>
      </div>
      <div>
        <dt><span class="key k-line"></span>{t('voice.meter.threshold')}</dt>
        <dd class="mono" data-threshold-db>{db(micMeter.threshold)}</dd>
      </div>
    </dl>
    <p class="meta private">{t('voice.meter.private')}</p>
  </div>
{/if}

<style>
  .sens {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: 8px 12px;
  }
  .slider {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
    min-width: 0;
  }
  .slider input {
    width: 140px;
    max-width: 100%;
    accent-color: color-mix(in oklch, var(--p) 75%, black);
  }
  .end {
    color: var(--muted);
    font-size: 12px;
  }
  .meta {
    color: var(--muted);
    font-size: 12px;
  }
  .slider .meta {
    min-width: 64px;
    font-variant-numeric: tabular-nums;
  }
  .small {
    height: 26px;
    padding: 0 9px;
    font-size: 12px;
  }
  .tester {
    margin: 0 0 12px;
    padding: 12px;
    border: 1px solid var(--line);
    border-radius: var(--r);
    background: var(--well);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 12px;
  }
  .witness {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 3px 10px 3px 8px;
    border-radius: 999px;
    border: 1px solid var(--line-2);
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
    transition:
      background-color 120ms var(--ease),
      color 120ms var(--ease),
      border-color 120ms var(--ease);
  }
  .witness .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--line-3);
  }
  .witness.on {
    color: var(--p-ink);
    background: var(--p-soft);
    border-color: color-mix(in oklch, var(--p) 45%, transparent);
  }
  .witness.on .dot {
    background: var(--p);
    box-shadow: 0 0 0 3px color-mix(in oklch, var(--p) 25%, transparent);
  }
  .track {
    position: relative;
    height: 14px;
    border-radius: 7px;
    background: var(--panel);
    border: 1px solid var(--line);
    overflow: hidden;
  }
  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    background: color-mix(in oklch, var(--muted) 45%, transparent);
    transition: width 60ms linear;
  }
  .fill.over {
    background: var(--p);
  }
  .noise {
    position: absolute;
    inset: 0 auto 0 0;
    background: repeating-linear-gradient(135deg, color-mix(in oklch, var(--text) 18%, transparent) 0 3px, transparent 3px 6px);
  }
  .line {
    position: absolute;
    top: -1px;
    bottom: -1px;
    width: 2px;
    margin-left: -1px;
    background: var(--text);
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 18px;
    margin: 0;
    font-size: 12px;
  }
  .legend div {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .legend dt {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
  }
  .legend dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }
  .key {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 2px;
  }
  .k-level {
    background: var(--p);
  }
  .k-noise {
    background: repeating-linear-gradient(135deg, color-mix(in oklch, var(--text) 35%, transparent) 0 2px, transparent 2px 4px);
    border: 1px solid var(--line-2);
  }
  .k-line {
    width: 2px;
    background: var(--text);
  }
  .private {
    margin: 0;
  }
  @media (prefers-reduced-motion: reduce) {
    .fill,
    .witness {
      transition: none;
    }
  }
</style>
