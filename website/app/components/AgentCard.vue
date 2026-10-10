<script setup lang="ts">
import { fmt, plural } from '~/data/catalog';
import { fDur, fTok, fUsd, type Activity, type Agent } from '~/data/squad';

defineProps<{ agent: Agent }>();
const { lang, text } = useLang();
const emit = defineEmits<{ answer: [option: number] }>();

const card = ref<HTMLElement>();
/** Answered from the keyboard, focus stays on the card rather than on a button that goes away. */
function answer(option: number, e: MouseEvent) {
  emit('answer', option);
  if (e.detail === 0) nextTick(() => card.value?.focus());
}

/** Each tool's pictogram, on a 16-unit grid. */
const ICON: Record<Activity['tool'], string> = {
  Read: 'M2.5 8s2-4 5.5-4 5.5 4 5.5 4-2 4-5.5 4S2.5 8 2.5 8zM8 9.6a1.6 1.6 0 1 0 0-3.2 1.6 1.6 0 0 0 0 3.2z',
  Grep: 'M7 11.5a4.5 4.5 0 1 0 0-9 4.5 4.5 0 0 0 0 9zM10.4 10.4l3.1 3.1',
  Edit: 'M10.5 3l2.5 2.5-7 7H3.5V10zM9 4.5l2.5 2.5',
  Write: 'M4 2.5h5.5L12 5v8.5H4zM8 7v4M6 9h4',
  Bash: 'M3 4.5l3.5 3.5L3 11.5M8.5 11.5H13',
};
</script>

<template>
  <article ref="card" class="agent" :class="agent.status" tabindex="-1">
    <div class="row top">
      <i class="dot" aria-hidden="true" />
      <span class="name">{{ agent.name }}</span>
      <span class="model">{{ agent.model }} · {{ fDur(agent.ms) }}</span>
      <span class="state">{{ text.demo.status[agent.status] }}</span>
    </div>
    <div v-if="agent.ticket" class="row ticket">
      <svg class="caret" viewBox="0 0 16 16" aria-hidden="true"><path d="M5.5 3.5l5 4.5-5 4.5z" /></svg>
      <span class="key">{{ agent.ticket.key }} · {{ fmt(text.demo.loop, { loop: agent.ticket.loop, max: agent.ticket.max }) }}</span>
      <span
        class="criteria"
        role="img"
        :aria-label="plural(lang, text.demo.criteria, agent.ticket.met.filter(Boolean).length, { total: agent.ticket.met.length })"
      >
        <svg v-for="(m, i) in agent.ticket.met" :key="i" class="criterion" :class="{ met: m }" viewBox="0 0 16 16" aria-hidden="true">
          <circle cx="8" cy="8" r="6" />
          <path d="M5.2 8.3l1.9 1.9 3.8-4" />
        </svg>
      </span>
      <span class="bar" aria-hidden="true"
        ><span :style="{ transform: `scaleX(${agent.ticket.met.filter(Boolean).length / agent.ticket.met.length})` }"
      /></span>
    </div>
    <div class="row act">
      <Transition name="swap" mode="out-in">
        <div v-if="agent.question" key="ask" class="ask">
          <p class="q">{{ agent.question.text }}</p>
          <div class="options">
            <button v-for="(o, i) in agent.question.options" :key="o" type="button" class="option" @click="answer(i, $event)">
              {{ o }}
            </button>
          </div>
        </div>
        <div v-else-if="agent.activity" :key="agent.activity.tool + agent.activity.target" class="tool">
          <svg class="icon" viewBox="0 0 16 16" aria-hidden="true"><path :d="ICON[agent.activity.tool]" /></svg>
          <b>{{ agent.activity.tool }}</b>
          <span class="target">{{ agent.activity.target }}</span>
          <span v-if="agent.activity.diff" class="diff"
            ><span class="add">+{{ agent.activity.diff[0] }}</span> <span class="del">−{{ agent.activity.diff[1] }}</span></span
          >
        </div>
        <div v-else-if="agent.result" key="result" class="result">
          <svg class="icon" viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 8.5l3 3 6-6.5" /></svg>
          <span>{{ agent.result }}</span>
        </div>
      </Transition>
      <span class="spend">{{ fTok(agent.tokens, lang) }} {{ text.demo.tokens }} · {{ fUsd(agent.cost, lang) }}</span>
    </div>
  </article>
</template>

<style scoped>
.agent {
  --tone: var(--ok);
  display: grid;
  gap: 0.32em;
  padding: 0.62em 0.85em 0.66em;
  border-radius: 0.7em;
  border: 1px solid transparent;
  transition:
    background-color 0.3s,
    border-color 0.3s;
}
/* Inside the list, which clips what overflows. */
.agent:focus-visible {
  outline-offset: -2px;
}
.agent.question {
  --tone: var(--wait);
  background: var(--wait-soft);
  border-color: var(--wait-ring);
}
.agent.totest {
  --tone: var(--wait);
}
.agent.ready {
  --tone: var(--dim);
}
.row {
  display: flex;
  align-items: center;
  gap: 0.55em;
  min-width: 0;
}
.dot {
  flex: none;
  width: 0.56em;
  height: 0.56em;
  border-radius: 50%;
  background: var(--tone);
  transition: background-color 0.3s;
}
.name {
  min-width: 0;
  overflow: hidden;
  color: var(--text);
  font-weight: 700;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.model,
.spend {
  flex: none;
  color: var(--dim);
  font-family: var(--mono);
  font-size: max(10px, 0.86em);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
.state {
  flex: none;
  margin-left: auto;
  color: var(--tone);
  font-size: 0.9em;
  font-weight: 600;
  transition: color 0.3s;
}
.ticket {
  padding-left: 1.1em;
  color: var(--muted);
  font-family: var(--mono);
  font-size: max(10px, 0.86em);
}
.caret {
  flex: none;
  width: 0.9em;
  height: 0.9em;
  margin-left: -1.05em;
  fill: var(--brand);
}
.key {
  white-space: nowrap;
}
.criteria {
  display: inline-flex;
  gap: 0.25em;
}
.criterion {
  width: 1.1em;
  height: 1.1em;
  fill: none;
  stroke-width: 1.5;
  stroke-linecap: round;
  stroke-linejoin: round;
}
.criterion circle {
  stroke: var(--dim);
  transition:
    stroke 0.3s,
    fill 0.3s;
}
.criterion path {
  stroke: var(--ink);
  stroke-dasharray: 10;
  stroke-dashoffset: 10;
  transition: stroke-dashoffset 0.45s cubic-bezier(0.16, 1, 0.3, 1) 0.1s;
}
.criterion.met circle {
  stroke: var(--ok);
  fill: var(--ok);
}
.criterion.met path {
  stroke-dashoffset: 0;
}
.bar {
  flex: 1;
  max-width: 7em;
  height: 0.28em;
  margin-left: auto;
  overflow: hidden;
  border-radius: 1em;
  background: var(--line2);
}
.bar span {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--ok);
  transform-origin: left;
  transition: transform 0.6s cubic-bezier(0.16, 1, 0.3, 1);
}
.act {
  padding-left: 1.1em;
  min-height: 1.6em;
}
.tool,
.result {
  display: flex;
  align-items: center;
  gap: 0.5em;
  min-width: 0;
  color: var(--muted);
  font-size: 0.92em;
}
.tool b {
  flex: none;
  color: var(--text);
  font-weight: 600;
}
.icon {
  flex: none;
  width: 1.05em;
  height: 1.05em;
  fill: none;
  stroke: var(--accent);
  stroke-width: 1.5;
  stroke-linecap: round;
  stroke-linejoin: round;
}
.result .icon {
  stroke: var(--tone);
}
.target {
  min-width: 0;
  overflow: hidden;
  font-family: var(--mono);
  font-size: 0.94em;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.diff {
  flex: none;
  font-family: var(--mono);
  font-size: 0.94em;
}
.add {
  color: var(--add);
}
.del {
  color: var(--del);
}
.spend {
  margin-left: auto;
}
.ask {
  display: grid;
  gap: 0.45em;
  min-width: 0;
}
.q {
  margin: 0;
  color: var(--text);
  font-size: 0.95em;
  line-height: 1.35;
}
.options {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4em;
}
.option {
  padding: 0.28em 0.8em;
  border: 1px solid var(--wait-ring);
  border-radius: 0.5em;
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: 0.9em;
  font-weight: 600;
  cursor: pointer;
  transition:
    background-color 0.15s,
    color 0.15s;
}
.option:hover {
  background: var(--wait);
  color: var(--ink);
}
.option:focus-visible {
  outline: 2px solid var(--wait);
  outline-offset: 2px;
}
.agent.question .act .spend {
  align-self: flex-start;
}

.swap-enter-active,
.swap-leave-active {
  transition:
    opacity 0.18s,
    translate 0.18s;
}
.swap-enter-active {
  transition-timing-function: cubic-bezier(0.16, 1, 0.3, 1);
}
.swap-enter-from {
  opacity: 0;
  translate: 0 0.35em;
}
.swap-leave-to {
  opacity: 0;
}
/* A question on its way out takes no more clicks. */
.swap-leave-active {
  pointer-events: none;
}

@media (prefers-reduced-motion: no-preference) {
  .agent.question .dot {
    animation: pulse 1.4s ease-in-out infinite;
  }
}
@media (prefers-reduced-motion: reduce) {
  .swap-enter-from {
    translate: none;
  }
}
@keyframes pulse {
  50% {
    box-shadow: 0 0 0 0.4em oklch(0.82 0.13 80 / 0);
    scale: 1.25;
  }
  0%,
  100% {
    box-shadow: 0 0 0 0 var(--wait-ring);
  }
}
</style>
