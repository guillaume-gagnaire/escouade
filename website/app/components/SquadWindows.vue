<script setup lang="ts">
import { fmt, plural } from '~/data/catalog';
import { BEAT_MS, SNAPSHOT, fUsd, squadAt, type Answer } from '~/data/squad';

// The logo's three stacked windows, at the size of a real one: the projects behind, and in front
// the demo's agents at work. Without JavaScript, or with less motion asked, it stands still.

const { lang, text } = useLang();
const t = ref(SNAPSHOT);
const answers = ref<Answer[]>([]);
const squad = computed(() => squadAt(t.value, answers.value, lang.value));

const root = ref<HTMLElement>();
const list = ref<HTMLElement>();
/** Whether the demo should run: the visitor's choice, playing by default unless they ask for less motion. */
const wanted = ref(false);
const inView = ref(true);
const pageShown = ref(true);
const playing = computed(() => wanted.value && inView.value && pageShown.value);

let timer: ReturnType<typeof setInterval> | undefined;
let last = 0;
function tick() {
  const now = performance.now();
  // Nothing moves under a visitor about to answer the question.
  const answering = list.value?.contains(document.activeElement) && document.activeElement?.matches('.option');
  if (!answering) t.value += (now - last) / BEAT_MS;
  last = now;
}
watch(playing, (on) => {
  clearInterval(timer);
  if (!on) return;
  last = performance.now();
  timer = setInterval(tick, 250);
});

const onVisibility = () => (pageShown.value = document.visibilityState === 'visible');
let observer: IntersectionObserver | undefined;
onMounted(() => {
  if (!matchMedia('(prefers-reduced-motion: reduce)').matches) {
    // Still unfolding: start from the beginning, where the first question is about to come.
    if (performance.now() < 1200) t.value = 0;
    wanted.value = true;
  }
  observer = new IntersectionObserver((entries) => (inView.value = entries[entries.length - 1].isIntersecting));
  observer.observe(root.value!);
  document.addEventListener('visibilitychange', onVisibility);
  onVisibility();
});
onBeforeUnmount(() => {
  clearInterval(timer);
  observer?.disconnect();
  document.removeEventListener('visibilitychange', onVisibility);
});

function answer(option: number) {
  answers.value = [...answers.value, { at: t.value, option }];
}
</script>

<template>
  <figure ref="root" class="squad" :class="{ still: !playing }">
    <div class="stack">
      <div v-for="(p, i) in squad.projects" :key="p.name" class="win behind" :class="i === 0 ? 'mid' : 'back'" aria-hidden="true">
        <div class="tabline">
          <i class="swatch" :style="{ background: p.color }" />
          <span class="pname">{{ p.name }}</span>
          <span class="delta">Δ {{ p.delta }}</span>
          <span v-if="p.agents.includes('question')" class="badge">{{ p.agents.filter((s) => s === 'question').length }}</span>
        </div>
        <div class="minis">
          <i v-for="(s, j) in p.agents" :key="j" class="mini" :class="s" />
        </div>
      </div>

      <div class="win front">
        <svg class="spark" viewBox="-23.5 -23.5 47 47" aria-hidden="true">
          <path d="M0 -20V20M-20 0H20" stroke-width="6.2" />
          <path d="M-11.8 -11.8L11.8 11.8M-11.8 11.8L11.8 -11.8" stroke-width="5" />
        </svg>
        <div class="titlebar">
          <span class="tab">
            <i class="swatch" />
            <b>{{ squad.project.name }}</b>
            <!-- As in the app's tab: a pill counting the agents that wait, or a dot while they all work. -->
            <span v-if="squad.counts.waiting" class="badge">{{ squad.counts.waiting }}</span>
            <i v-else-if="squad.counts.running" class="live" aria-hidden="true" />
            <span class="delta">Δ {{ squad.project.delta }}</span>
          </span>
          <span class="branch">{{ squad.project.branch }}</span>
          <svg class="controls" viewBox="0 0 60 12" aria-hidden="true">
            <path d="M2 6h8M26 1.5h8v8h-8zM50 1.5l8 8M58 1.5l-8 8" />
          </svg>
        </div>
        <div class="head">
          <span>{{ text.demo.agents }}</span
          ><span class="n">{{ squad.agents.length }}</span>
        </div>
        <ul ref="list" class="agents">
          <li v-for="a in squad.agents" :key="a.slot" :style="{ '--i': a.slot }">
            <AgentCard :agent="a" @answer="answer" />
          </li>
        </ul>
        <div class="statusbar">
          <span class="count"><i class="mini running" />{{ fmt(text.demo.running, { count: squad.counts.running }) }}</span>
          <span class="count"><i class="mini question" />{{ fmt(text.demo.waiting, { count: squad.counts.waiting }) }}</span>
          <span class="count done"><i class="mini done" />{{ plural(lang, text.demo.done, squad.counts.done) }}</span>
          <span class="today">{{ fmt(text.demo.today, { amount: fUsd(squad.today, lang) }) }}</span>
          <button type="button" class="pause" :aria-label="text.demo.pause" :aria-pressed="!wanted" @click="wanted = !wanted">
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path v-if="wanted" d="M5.5 3.5v9M10.5 3.5v9" />
              <path v-else d="M5 3.2l7.5 4.8L5 12.8z" class="fill" />
            </svg>
          </button>
        </div>
      </div>
    </div>
    <figcaption>
      {{ text.demo.caption }}
      <span class="sr-only">{{ text.demo.description }}</span>
    </figcaption>
  </figure>
</template>

<style scoped>
.squad {
  margin: 0;
  container-type: inline-size;
}
.stack {
  /* The logo's geometry: windows of 66 offset by 9, on 84. */
  --off: calc(100cqw * 9 / 84);
  --w: calc(100cqw - 2 * var(--off));
  --r: 3.4cqw;
  --stroke: 1.5px;
  position: relative;
  aspect-ratio: 1;
}
.win {
  --edge: var(--brand);
  position: absolute;
  width: var(--w);
  height: var(--w);
  border-radius: var(--r);
  box-shadow:
    inset 0 0 0 var(--stroke) var(--edge),
    0 2.4cqw 7cqw -2cqw rgb(0 0 0 / 0.6);
}
.back {
  --edge: color-mix(in oklch, var(--brand) 35%, transparent);
  z-index: 1;
  top: 0;
  left: calc(2 * var(--off));
  background: color-mix(in oklch, var(--panel) 55%, var(--bg));
}
.mid {
  --edge: color-mix(in oklch, var(--brand) 60%, transparent);
  z-index: 2;
  top: var(--off);
  left: var(--off);
  background: var(--panel);
}
.front {
  z-index: 3;
  top: calc(2 * var(--off));
  left: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: #1d1a17;
  font-size: clamp(11.5px, 2.05cqw, 14.5px);
  line-height: 1.45;
}

/* The windows behind: their tab in the band above the next one, their agents in the band beside it. */
.tabline {
  position: absolute;
  top: calc(var(--off) / 2);
  left: 2.6cqw;
  right: calc(var(--off) + 2cqw);
  display: flex;
  align-items: center;
  gap: 0.6em;
  translate: 0 -50%;
  color: var(--muted);
  font-size: clamp(10px, 1.75cqw, 13.5px);
  white-space: nowrap;
}
.pname {
  color: var(--text);
  font-weight: 700;
}
.swatch {
  flex: none;
  width: 0.62em;
  height: 0.62em;
  border-radius: 0.18em;
  background: var(--accent);
}
.delta {
  padding: 0.05em 0.45em;
  border-radius: 0.35em;
  background: var(--line);
  color: var(--muted);
  font-family: var(--mono);
  font-size: max(10px, 0.86em);
}
.badge {
  min-width: 1.45em;
  padding: 0 0.4em;
  border-radius: 1em;
  background: var(--wait);
  color: var(--ink);
  font-size: max(10px, 0.82em);
  font-weight: 800;
  text-align: center;
}
.minis {
  position: absolute;
  top: calc(var(--off) + 2.4cqw);
  right: calc(var(--off) / 2);
  display: grid;
  gap: 1.6cqw;
  translate: 50% 0;
}
.mini {
  display: inline-block;
  width: max(6px, 1.1cqw);
  height: max(6px, 1.1cqw);
  border-radius: 50%;
  background: var(--ok);
  transition: background-color 0.3s;
}
.mini.question {
  background: var(--wait);
}
.mini.ready {
  background: var(--dim);
}
.mini.done {
  background: color-mix(in oklch, var(--ok) 45%, transparent);
}

/* The window in front: the app's title bar, agents and status bar. */
.spark {
  position: absolute;
  top: 1.75cqw;
  left: 2.3cqw;
  width: 3.2cqw;
  height: 3.2cqw;
  fill: none;
  stroke: var(--brand);
  stroke-linecap: round;
}
.titlebar {
  flex: none;
  display: flex;
  align-items: center;
  gap: 0.9em;
  height: 6.7cqw;
  padding: 0 2.4cqw 0 7.2cqw;
  border-bottom: 1px solid var(--line);
}
.tab {
  display: flex;
  align-items: center;
  gap: 0.5em;
  min-width: 0;
}
.tab b {
  font-weight: 700;
}
.live {
  width: 0.45em;
  height: 0.45em;
  border-radius: 50%;
  background: var(--ok);
}
.branch {
  margin-left: auto;
  color: var(--dim);
  font-family: var(--mono);
  font-size: max(10px, 0.86em);
}
.controls {
  flex: none;
  width: 5.2em;
  fill: none;
  stroke: var(--dim);
  stroke-width: 1.3;
  stroke-linecap: round;
}
.head {
  flex: none;
  display: flex;
  gap: 0.6em;
  padding: 0.9em 1.45em 0.35em;
  color: var(--dim);
  font-size: max(10px, 0.8em);
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}
.head .n {
  font-family: var(--mono);
  font-weight: 400;
}
.agents {
  flex: 1;
  display: grid;
  align-content: start;
  gap: 0.2em;
  min-height: 0;
  margin: 0;
  padding: 0 0.6em;
  overflow: hidden;
  list-style: none;
}
.statusbar {
  flex: none;
  display: flex;
  align-items: center;
  gap: 1.1em;
  padding: 0.5em 0.7em 0.5em 1.45em;
  border-top: 1px solid var(--line);
  color: var(--muted);
  font-family: var(--mono);
  font-size: max(10px, 0.82em);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
.count {
  display: inline-flex;
  align-items: center;
  gap: 0.5em;
}
.statusbar .mini {
  width: 0.6em;
  height: 0.6em;
}
.today {
  margin-left: auto;
  color: var(--text);
}
.pause {
  flex: none;
  display: grid;
  place-items: center;
  width: 2.2em;
  height: 2.2em;
  padding: 0;
  border: 1px solid var(--line2);
  border-radius: 50%;
  background: transparent;
  color: var(--muted);
  cursor: pointer;
  transition:
    color 0.15s,
    border-color 0.15s;
}
.pause:hover {
  border-color: var(--brand);
  color: var(--text);
}
.pause svg {
  width: 1.1em;
  height: 1.1em;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.8;
  stroke-linecap: round;
  stroke-linejoin: round;
}
.pause .fill {
  fill: currentColor;
}
figcaption {
  margin-top: 14px;
  color: var(--dim);
  font-size: 13px;
}

/* Below the size where its type stops shrinking, the window keeps three agents (the one at work, the one
   asking, the ticket) and takes their height instead of the logo's square. */
@container (max-width: 569px) {
  /* Padding, not the front window's margin, which would collapse through the stack. */
  .stack {
    aspect-ratio: auto;
    padding-top: calc(2 * var(--off));
  }
  .win {
    height: auto;
  }
  .back {
    bottom: calc(2 * var(--off));
  }
  .mid {
    bottom: var(--off);
  }
  .front {
    position: relative;
    top: auto;
    min-height: 0;
  }
  .agents li:nth-child(n + 4) {
    display: none;
  }
  /* The tab's pill and the cards already say who works and who waits. */
  .head .n,
  .statusbar .count,
  .branch,
  .agents :deep(.spend) {
    display: none;
  }
}

/* The logo unfolds: held a moment at its own size, the stack grows, its strokes thin, the spark files to its corner. */
@media (prefers-reduced-motion: no-preference) {
  .stack {
    animation: unfold 1.3s cubic-bezier(0.16, 1, 0.3, 1) 0.45s both;
  }
  .win {
    animation: logo 1.3s cubic-bezier(0.16, 1, 0.3, 1) 0.45s both;
  }
  .spark {
    animation: spark 1.3s cubic-bezier(0.16, 1, 0.3, 1) 0.45s both;
  }
  .win > :not(.spark, .agents),
  figcaption {
    animation: appear 0.5s ease-out 1.2s both;
  }
  .agents li {
    animation: rise 0.6s cubic-bezier(0.16, 1, 0.3, 1) both;
    animation-delay: calc(1.25s + var(--i) * 70ms);
  }
  .mini.question,
  .badge {
    animation: blink 1.4s ease-in-out infinite;
  }
  /* A window shorter than wide still unfolds from the logo's square, then takes its height. */
  @container (max-width: 569px) {
    .front {
      animation:
        logo 1.3s cubic-bezier(0.16, 1, 0.3, 1) 0.45s both,
        square 1.3s cubic-bezier(0.16, 1, 0.3, 1) 0.45s both;
    }
  }
  /* Paused, nothing blinks either. */
  .squad.still .mini.question,
  .squad.still .badge,
  .squad.still :deep(.agent.question .dot) {
    animation: none;
  }
}
@keyframes unfold {
  from {
    transform: scale(0.15);
  }
}
@keyframes logo {
  from {
    border-radius: 16.6cqw;
    box-shadow:
      inset 0 0 0 6cqw var(--edge),
      0 0 0 0 rgb(0 0 0 / 0);
  }
}
@keyframes square {
  from {
    min-height: var(--w);
  }
}
@keyframes spark {
  from {
    transform: translate(35.4cqw, 36cqw) scale(14.9);
  }
}
@keyframes appear {
  from {
    opacity: 0;
  }
}
@keyframes rise {
  from {
    opacity: 0;
    translate: 0 1.2em;
  }
}
@keyframes blink {
  50% {
    opacity: 0.35;
  }
}
</style>
