<script lang="ts">
  import { shownTheme } from '../../lib/appearance';
  import { FEATURE_ICONS, featureSwitch, NEEDS } from '../../lib/features';
  import { lockOf, setFeature, usedOf, type TourMode } from '../../lib/features-tour.svelte';
  import { locale, t } from '../../lib/i18n';
  import { lockLine } from '../../lib/locks.svelte';
  import { app } from '../../lib/state.svelte';
  import type { Feature, FeatureUse } from '../../lib/types';
  import Icon from '../Icon.svelte';

  // The assistant of the features (F-2, ESC-559, maquette design/features/assistant.png): one feature per screen. On top,
  // the features as chips (the essential first, always there), the one shown current; then a capture of the real app,
  // what it does (a sentence and three points) and a big switch « Activer », saved as Réglages › Fonctionnalités saves it.
  // Under it, what is used (« Tu l'utilises déjà sur 2 projets : il reste allumé »), what it needs, who imposes it. The
  // screens go by ← →, the chips, or the buttons of whoever holds it (the station's foot, the window's).
  let {
    list,
    at = $bindable(0),
    mode,
    used,
  }: {
    /** The features shown, in their order. */
    list: Feature[];
    at?: number;
    mode: TourMode;
    used?: FeatureUse[];
  } = $props();

  const f = $derived(list[Math.min(at, list.length - 1)]);
  const on = $derived(featureSwitch(app.settings, f));
  const lock = $derived(lockOf(f));
  const need = $derived(NEEDS[f]);
  const needOff = $derived(!!need && !featureSwitch(app.settings, need));
  const use = $derived(usedOf(f, used));
  /** « Il reste allumé » when the assistant opens for the first launch or the update; what is used, said plainly, when it
   * is seen again (the user may have switched it off since). */
  const usedLine = $derived.by(() => {
    if (!use) return '';
    if (mode === 'review') return use.projects > 0 ? t('features.page.usedOn', { count: use.projects }) : t('features.page.used');
    return use.projects > 0 ? t(`features.screens.${f}.keptOn`, { count: use.projects }) : t(`features.screens.${f}.kept`);
  });
  /** The capture of the real app, in the language and the theme of the window (tests/site-shots/features-tour.spec.ts). */
  const shot = $derived.by(() => {
    void app.settings.theme;
    return `/features/${locale.ui === 'en' ? 'en' : 'fr'}/${f}-${shownTheme()}.jpg`;
  });
  let broken = $state<Record<string, boolean>>({});
  let busy = $state(false);

  async function toggle() {
    if (lock || needOff || busy) return;
    busy = true;
    try {
      await setFeature(f, !on);
    } finally {
      busy = false;
    }
  }
  function onkeydown(e: KeyboardEvent) {
    if ((e.target as HTMLElement).closest('input, textarea, select')) return;
    if (e.key === 'ArrowRight' && at < list.length - 1) {
      e.preventDefault();
      at += 1;
    } else if (e.key === 'ArrowLeft' && at > 0) {
      e.preventDefault();
      at -= 1;
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions: ← → go from one screen to the next, wherever the focus is in it. -->
<div class="tour" {onkeydown} data-testid="features-tour" data-feature={f}>
  <div class="top">
    <nav class="chips" aria-label={t('features.tour.list')}>
      <span class="chip ess"><Icon name="check" size={12} />{t('features.essential.name')}</span>
      {#each list as x, i (x)}
        <button
          class="chip"
          class:cur={i === at}
          class:on={featureSwitch(app.settings, x)}
          aria-current={i === at ? 'step' : undefined}
          onclick={() => (at = i)}
          >{#if featureSwitch(app.settings, x)}<Icon name="check" size={12} />{/if}{t(`features.${x}.name`)}</button
        >
      {/each}
    </nav>
    <span class="of" aria-label={t('features.tour.ofLong', { n: at + 1, total: list.length })}
      >{t('features.tour.of', { n: at + 1, total: list.length })}</span
    >
  </div>
  {#key f}
    <section class="screen" aria-labelledby="tour-title">
      <div class="shot">
        {#if broken[shot]}
          <span class="noshot" aria-hidden="true"><Icon name={FEATURE_ICONS[f]} size={40} /></span>
        {:else}
          <img src={shot} alt={t('features.tour.shot', { name: t(`features.${f}.name`) })} onerror={() => (broken[shot] = true)} />
        {/if}
      </div>
      <div class="text">
        <h2 id="tour-title">{t(`features.screens.${f}.title`)}</h2>
        <p class="lead">{t(`features.screens.${f}.lead`)}</p>
        <ul>
          <li>{t(`features.screens.${f}.p1`)}</li>
          <li>{t(`features.screens.${f}.p2`)}</li>
          <li>{t(`features.screens.${f}.p3`)}</li>
        </ul>
        <button
          class="act"
          class:on
          role="switch"
          aria-checked={on}
          aria-describedby="tour-takes"
          disabled={!!lock || needOff || busy}
          data-tour
          onclick={toggle}
        >
          <span class="al">{t(`features.screens.${f}.activate`)}</span>
          <span class="sw" aria-hidden="true"><i></i></span>
        </button>
        <p class="takes" id="tour-takes">{t(`features.${f}.takes`)}</p>
        {#if usedLine}<p class="used"><Icon name="check" size={13} />{usedLine}</p>{/if}
        {#if needOff && need}<p class="note">{t('features.page.needs', { need: t(`features.${need}.name`) })}</p>{/if}
        {#if lock}<p class="lock"><Icon name="lock" size={13} />{lockLine(lock)}</p>{/if}
      </div>
    </section>
  {/key}
</div>

<style>
  .tour {
    container-type: inline-size;
    display: grid;
    gap: 18px;
    min-width: 0;
  }
  .top {
    display: flex;
    align-items: flex-start;
    gap: 12px;
  }
  .chips {
    display: flex;
    flex: 1;
    flex-wrap: wrap;
    gap: 6px;
    min-width: 0;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 28px;
    padding: 0 11px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: none;
    color: var(--muted);
    font: inherit;
    font-size: 12.5px;
    white-space: nowrap;
    cursor: pointer;
  }
  .chip.on {
    color: var(--text);
  }
  .chip.cur {
    border-color: transparent;
    background: var(--hover-strong, var(--hover));
    color: var(--text);
    font-weight: 600;
  }
  .chip:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 2px;
  }
  .chip.ess {
    border-color: color-mix(in oklch, var(--add) 45%, transparent);
    color: var(--add);
    cursor: default;
  }
  .of {
    flex: none;
    padding-top: 5px;
    color: var(--muted);
    font-size: 12.5px;
    font-variant-numeric: tabular-nums;
  }
  .screen {
    display: grid;
    grid-template-columns: minmax(0, 1.35fr) minmax(0, 1fr);
    align-items: start;
    gap: 28px;
    animation: arrive 240ms var(--ease, ease-out);
  }
  @container (max-width: 820px) {
    .screen {
      grid-template-columns: minmax(0, 1fr);
      gap: 16px;
    }
    .shot {
      max-width: 520px;
    }
  }
  @keyframes arrive {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  .shot {
    display: grid;
    overflow: hidden;
    aspect-ratio: 16 / 9;
    border-radius: 10px;
    background: var(--frame);
    box-shadow:
      0 0 0 1px var(--line),
      0 18px 40px -24px rgba(0, 0, 0, 0.6);
  }
  .shot img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: top left;
  }
  .noshot {
    place-self: center;
    color: var(--dim);
  }
  .text {
    display: grid;
    align-content: start;
    gap: 12px;
    min-width: 0;
  }
  h2 {
    margin: 0;
    font-size: 24px;
    font-weight: 650;
    letter-spacing: -0.02em;
  }
  .lead {
    margin: 0;
    color: var(--soft, var(--text));
    font-size: 14px;
    line-height: 1.55;
  }
  ul {
    display: grid;
    gap: 6px;
    margin: 0;
    padding-left: 18px;
    color: var(--muted);
    font-size: 13.5px;
    line-height: 1.5;
  }
  .act {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin-top: 6px;
    padding: 16px 18px;
    border: 1px solid var(--line);
    border-radius: 12px;
    background: var(--frame);
    color: var(--text);
    font: inherit;
    font-size: 16px;
    font-weight: 650;
    text-align: left;
    cursor: pointer;
  }
  .act.on {
    border-color: color-mix(in oklch, var(--brand) 55%, transparent);
    background: color-mix(in oklch, var(--brand) 9%, var(--frame));
  }
  .act:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 2px;
  }
  .act:disabled {
    cursor: default;
    opacity: 0.6;
  }
  .sw {
    position: relative;
    flex: none;
    width: 48px;
    height: 28px;
    border-radius: 999px;
    background: var(--line);
    transition: background 160ms;
  }
  .sw i {
    position: absolute;
    top: 4px;
    left: 4px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #fff;
    transition: transform 160ms var(--ease, ease-out);
  }
  .act.on .sw {
    background: var(--brand);
  }
  .act.on .sw i {
    transform: translateX(20px);
  }
  .takes,
  .note,
  .lock,
  .used {
    margin: 0;
    color: var(--dim);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .used,
  .lock {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .used {
    color: var(--add);
    font-size: 13px;
  }
  .note {
    color: var(--warn, var(--muted));
  }
  .lock {
    color: var(--muted);
  }
  @media (prefers-reduced-motion: reduce) {
    .screen {
      animation: fade 120ms linear;
    }
    .sw,
    .sw i {
      transition: none;
    }
    @keyframes fade {
      from {
        opacity: 0;
      }
    }
  }
</style>
