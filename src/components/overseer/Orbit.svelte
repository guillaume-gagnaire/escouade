<script lang="ts">
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { toOrb, type Placed } from '../../lib/orbit';
  import type { VoicePhase } from '../../lib/overseer-voice.svelte';

  // The orbit of the Overseer's screen (ESC-468, design/overseer/ecran-discussion.png): its orb in the middle, the tasks
  // of every project in arcs around it by their state (`orbit.ts`), each card in its project's colour. A task just
  // finished melts toward the orb, which lights up while it tells it. The orb follows the voices (voix-tu-parles.png,
  // voix-il-repond.png): it lights up and its rings beat with the user's, it swells with its own; the sentence said is
  // written under it. With prefers-reduced-motion nothing moves: the state is read from the words and the colours.
  let {
    tasks,
    phase,
    level,
    played,
    heard,
    subtitle,
    thinking,
    open,
  }: {
    tasks: Placed[];
    phase: VoicePhase;
    level: number;
    played: number;
    heard: string | null;
    subtitle: string | null;
    thinking: boolean;
    open: (task: Placed) => void;
  } = $props();

  const listening = $derived(phase === 'talking' || phase === 'sending');
  const speaking = $derived(phase === 'speaking');
  const lit = $derived(tasks.some((x) => x.fresh));
  const label = $derived(
    listening
      ? t('overseer.screen.youSpeak')
      : speaking || lit
        ? t('overseer.screen.heSpeaks')
        : thinking
          ? t('overseer.screen.thinking')
          : '',
  );
  /** The words under the orb: the user's as said, else its sentence being read. */
  const caption = $derived(listening ? heard : speaking ? subtitle : null);

  let orb = $state<HTMLDivElement>();
  let still = $state(false);

  // The orb follows the sound, quickly up and slower down: a voice, not a flicker. Still with prefers-reduced-motion.
  onMount(() => {
    const media = typeof matchMedia === 'function' ? matchMedia('(prefers-reduced-motion: reduce)') : null;
    still = !!media?.matches;
    const changed = () => (still = !!media?.matches);
    media?.addEventListener?.('change', changed);
    let frame = 0;
    let lvl = 0;
    const draw = () => {
      const goal = still ? 0 : listening ? level : speaking ? played : 0;
      lvl += (goal - lvl) * (goal > lvl ? 0.45 : 0.18);
      orb?.style.setProperty('--lvl', lvl.toFixed(3));
      frame = requestAnimationFrame(draw);
    };
    if (typeof requestAnimationFrame === 'function') frame = requestAnimationFrame(draw);
    return () => {
      if (frame) cancelAnimationFrame(frame);
      media?.removeEventListener?.('change', changed);
    };
  });

  function why(x: Placed): string {
    if (x.fresh) return t('overseer.orbit.justNow');
    if (x.state === 'waiting' && x.why) return t(`overseer.orbit.${x.why}`);
    return '';
  }
</script>

<div
  class="orbit"
  class:listening
  class:speaking
  class:lit
  class:still
  data-orbit
  data-motion={still ? 'reduced' : 'full'}
  data-voice={phase}
  role="region"
  aria-label={t('overseer.screen.orbit')}
>
  <div class="ring" aria-hidden="true"></div>
  <div class="core">
    <div class="orb" bind:this={orb} aria-hidden="true">
      <span class="beat b1"></span><span class="beat b2"></span>
    </div>
    <div class="label" role="status" aria-live="polite" data-orb-label>{label}</div>
  </div>
  {#if caption}
    <p class="caption" class:live={listening} data-orb-caption>
      « {caption}{#if listening}<span class="caret" aria-hidden="true"></span>{:else}&nbsp;»{/if}
    </p>
  {/if}
  {#if !tasks.length}
    <p class="none">{t('overseer.screen.noTask')}</p>
  {/if}
  {#each tasks as x (x.id)}
    {@const d = toOrb(x)}
    <button
      class="card {x.state}"
      class:fresh={x.fresh}
      style:left="{x.x}%"
      style:top="{x.y}%"
      style:--c={x.color}
      style:--fade={x.fade}
      style:--dx="{d.dx}cqw"
      style:--dy="{d.dy}cqh"
      data-orbit-task={x.id}
      data-state={x.state}
      data-fresh={x.fresh ? '' : undefined}
      aria-label={x.key ? t('overseer.orbit.openTicket', { key: x.key }) : t('overseer.orbit.openAgent', { name: x.title })}
      onclick={() => open(x)}
    >
      <span class="head">
        <i class="dot" aria-hidden="true"></i>
        {#if x.key}<span class="key">{x.key}</span>{/if}
        <span class="title">{x.title}</span>
      </span>
      {#if x.progress !== null}
        <span class="bar" aria-hidden="true"><span style:transform="scaleX({x.progress})"></span></span>
      {/if}
      {#if why(x)}<span class="why">{why(x)}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .orbit {
    --ov: var(--brand);
    --lvl: 0;
    position: relative;
    container-type: size;
    width: 100%;
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }
  /* The orbit's ring, and the halo of the orb's colour. */
  .ring {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 74cqmin;
    height: 74cqmin;
    transform: translate(-50%, -50%);
    border-radius: 50%;
    border: 1px solid rgba(var(--fg-rgb), 0.06);
    background: radial-gradient(closest-side, color-mix(in oklch, var(--ov) 10%, transparent), transparent 75%);
  }
  .core {
    position: absolute;
    left: 50%;
    top: 50%;
    transform: translate(-50%, -50%);
    display: grid;
    justify-items: center;
    gap: 14px;
  }
  .orb {
    --size: clamp(96px, 18cqmin, 170px);
    position: relative;
    width: var(--size);
    height: var(--size);
    border-radius: 50%;
    background:
      radial-gradient(circle at 38% 32%, color-mix(in oklch, var(--ov) 50%, white) 0, transparent 46%),
      radial-gradient(circle at 60% 70%, color-mix(in oklch, var(--ov) 80%, black), transparent 60%),
      color-mix(in oklch, var(--ov) 78%, var(--orb-base));
    box-shadow:
      0 0 0 1px rgba(var(--fg-rgb), 0.08),
      0 0 calc(50px + var(--lvl) * 60px) 6px color-mix(in oklch, var(--ov) 26%, transparent);
    transform: scale(calc(1 + var(--lvl) * 0.14));
    transition: filter 0.3s var(--ease);
  }
  /* The user speaks: the orb lights up, its rings beat with the voice. */
  .listening .orb,
  .lit .orb {
    filter: brightness(1.18) saturate(1.1);
  }
  .beat {
    position: absolute;
    inset: calc(-14px - var(--lvl) * 16px);
    border-radius: 50%;
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--ov) 45%, transparent);
    opacity: 0;
  }
  .b2 {
    inset: calc(-34px - var(--lvl) * 26px);
  }
  .listening .beat {
    opacity: 1;
  }
  .label {
    min-height: 1em;
    color: var(--muted);
    font-size: 12px;
    font-weight: 550;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .caption {
    position: absolute;
    left: 50%;
    top: calc(50% + clamp(96px, 18cqmin, 170px) / 2 + 48px);
    transform: translateX(-50%);
    width: min(560px, 80cqw);
    margin: 0;
    text-align: center;
    color: var(--text);
    font-size: clamp(16px, 2.2cqw, 24px);
    font-weight: 600;
    line-height: 1.35;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
    z-index: 2;
  }
  .caret {
    display: inline-block;
    width: 2px;
    height: 1em;
    margin-left: 2px;
    vertical-align: -0.12em;
    background: var(--ov);
    animation: caret 1s steps(1) infinite;
  }
  @keyframes caret {
    50% {
      opacity: 0;
    }
  }
  .none {
    position: absolute;
    left: 50%;
    top: 14%;
    transform: translateX(-50%);
    margin: 0;
    color: var(--muted);
    font-size: 13px;
    white-space: nowrap;
  }
  /* A task: a card in its project's colour. */
  .card {
    position: absolute;
    transform: translate(-50%, -50%);
    width: clamp(150px, 24cqw, 250px);
    display: grid;
    gap: 6px;
    padding: 9px 12px;
    border: none;
    border-radius: 10px;
    background: color-mix(in oklch, var(--c) 6%, var(--raise));
    box-shadow:
      inset 0 0 0 1px rgba(var(--fg-rgb), 0.16),
      0 6px 24px rgb(0 0 0 / 0.22);
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
    transition:
      left 0.6s var(--ease),
      top 0.6s var(--ease),
      opacity 0.3s;
    z-index: 1;
  }
  .card:hover,
  .card:focus-visible {
    box-shadow:
      inset 0 0 0 1px color-mix(in oklch, var(--c) 60%, transparent),
      0 6px 24px rgb(0 0 0 / 0.22);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    font-size: 13px;
    font-weight: 600;
  }
  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--c);
  }
  .key {
    flex: none;
    color: var(--soft);
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 600;
  }
  .title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bar {
    height: 3px;
    border-radius: 2px;
    background: rgba(var(--fg-rgb), 0.1);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--run);
    transform-origin: left;
  }
  .why {
    font-size: 12px;
    font-weight: 600;
  }
  /* What waits for the user: amber, closer to the orb. */
  .waiting {
    background: color-mix(in oklch, var(--wait) 7%, var(--raise));
    box-shadow:
      inset 0 0 0 1px var(--wait-line),
      0 6px 24px rgb(0 0 0 / 0.22);
  }
  .waiting .why {
    color: var(--wait);
  }
  /* What is done goes away: fainter as it is older. */
  .done {
    opacity: calc(0.78 - var(--fade) * 0.4);
    background: color-mix(in oklch, var(--c) 3%, var(--panel));
  }
  .done .head {
    color: var(--soft);
  }
  /* Just finished: it melts toward the orb, lit, while the Overseer tells it, then settles in its arc. */
  .card.fresh {
    opacity: 1;
    box-shadow:
      inset 0 0 0 1px color-mix(in oklch, var(--run) 70%, transparent),
      0 0 34px color-mix(in oklch, var(--run) 30%, transparent);
    animation: melt 2.6s var(--ease) both;
    z-index: 3;
  }
  .card.fresh .why {
    color: var(--run);
  }
  @keyframes melt {
    0% {
      transform: translate(-50%, -50%);
    }
    45% {
      transform: translate(calc(-50% + var(--dx) * 0.7), calc(-50% + var(--dy) * 0.7)) scale(0.62);
      filter: brightness(1.5);
      opacity: 0.85;
    }
    100% {
      transform: translate(-50%, -50%);
    }
  }
  /* Nothing moves: the state is read from the words and the colours. */
  .still .card,
  .still .orb {
    transition: none;
    animation: none;
  }
  .still .caret {
    animation: none;
  }
  @media (prefers-reduced-motion: reduce) {
    .card,
    .orb,
    .caret {
      transition: none;
      animation: none !important;
    }
  }
</style>
