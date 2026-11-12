<script setup lang="ts">
import { fmt, plural } from '~/data/catalog';
import { fDur, fTok, fUsd, type Agent } from '~/data/squad';

// An agent's card as the app's sidebar draws it (2.2): its name and state, its harness, model and time (or its ticket and
// loop) and the temperature of its cache, then what it does now, with what it spent. A question turns the card amber, its answers numbered as in the app.
defineProps<{ agent: Agent }>();
const { lang, text } = useLang();
const emit = defineEmits<{ answer: [option: number] }>();

const card = ref<HTMLElement>();
/** Answered from the keyboard, focus stays on the card rather than on a button that goes away. */
function answer(option: number, e: MouseEvent) {
  emit('answer', option);
  if (e.detail === 0) nextTick(() => card.value?.focus());
}
</script>

<template>
  <article ref="card" class="agent" :class="agent.status" tabindex="-1">
    <div class="a-top">
      <i class="dot" aria-hidden="true" />
      <span class="name">{{ agent.name }}</span>
      <span class="a-state">{{ text.demo.status[agent.status] }}</span>
    </div>
    <div class="a-meta">
      <span class="mark">{{ agent.harness }}</span>
      <template v-if="agent.ticket">
        <b class="key">{{ agent.ticket.key }}</b>
        <span>{{ fmt(text.demo.loop, { loop: agent.ticket.loop, max: agent.ticket.max }) }}</span>
        <span
          class="loops"
          role="img"
          :aria-label="plural(lang, text.demo.criteria, agent.ticket.met.filter(Boolean).length, { total: agent.ticket.met.length })"
        >
          <i v-for="(m, i) in agent.ticket.met" :key="i" :class="{ met: m }" />
        </span>
      </template>
      <template v-else>
        <span class="model">{{ agent.model }}</span>
        <span class="sep">·</span>
        <span class="num">{{ fDur(agent.ms) }}</span>
      </template>
      <span class="heat" :class="agent.heat >= 67 ? 'hot' : agent.heat >= 34 ? 'mid' : 'cool'">{{ agent.heat }} °C</span>
    </div>
    <Transition name="swap" mode="out-in">
      <div v-if="agent.question" key="ask" class="ask">
        <p class="q">{{ agent.question.text }}</p>
        <div class="opts">
          <button v-for="(o, i) in agent.question.options" :key="o" type="button" class="opt" @click="answer(i, $event)">
            <kbd aria-hidden="true">{{ i + 1 }}</kbd
            >{{ o }}
          </button>
        </div>
      </div>
      <div v-else :key="agent.activity ? agent.activity.tool + agent.activity.target : 'result'" class="a-act">
        <template v-if="agent.activity">
          <b>{{ agent.activity.tool }}</b>
          <span class="tgt mono">{{ agent.activity.target }}</span>
          <span v-if="agent.activity.diff" class="diff mono"
            ><span class="add">+{{ agent.activity.diff[0] }}</span> <span class="del">−{{ agent.activity.diff[1] }}</span></span
          >
        </template>
        <span v-else-if="agent.result" class="result">{{ agent.result }}</span>
        <span class="spend">{{ fTok(agent.tokens, lang) }} · {{ fUsd(agent.cost, lang) }}</span>
      </div>
    </Transition>
  </article>
</template>

<style scoped>
/* The temperature of its cache (the app's HeatFigure.svelte): red-orange when hot, amber half way, blue cooling. */
.heat {
  margin-left: auto;
  flex: none;
  padding-left: 8px;
  font-family: var(--mono);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
.heat.hot {
  color: oklch(0.72 0.17 38);
}
.heat.mid {
  color: oklch(0.82 0.13 75);
}
.heat.cool {
  color: oklch(0.78 0.11 235);
}
/* The values of the app's sidebar (src/components/Sidebar.svelte, EngineMark.svelte). */
.agent {
  --tone: var(--run);
  display: grid;
  gap: 3px;
  padding: 8px 10px 9px;
  border-radius: 9px;
  transition:
    background-color 0.3s,
    box-shadow 0.3s;
}
/* Inside the list, which clips what overflows. */
.agent:focus-visible {
  outline-offset: -2px;
}
.agent.question {
  --tone: var(--wait);
  background: var(--wait-bg);
  box-shadow: inset 0 0 0 1px var(--wait-line);
}
.agent.totest {
  --tone: var(--wait);
}
.agent.done,
.agent.ready {
  --tone: var(--muted);
}
.a-top,
.a-meta,
.a-act {
  display: flex;
  align-items: center;
  min-width: 0;
  white-space: nowrap;
}
.a-top {
  gap: 8px;
}
.dot {
  flex: none;
  width: 7px;
  height: 7px;
  margin: 0 1px;
  border-radius: 50%;
  background: var(--tone);
  transition: background-color 0.3s;
}
.agent.question .dot {
  width: 9px;
  height: 9px;
  margin: 0;
  background: none;
  box-shadow: inset 0 0 0 1.6px var(--tone);
}
.name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  color: var(--text);
  font-weight: 590;
  letter-spacing: -0.01em;
  text-overflow: ellipsis;
}
.a-state {
  flex: none;
  color: var(--tone);
  font-size: 11.5px;
  font-weight: 550;
  transition: color 0.3s;
}
.agent.done .a-state,
.agent.ready .a-state {
  font-weight: 500;
}
.a-meta,
.a-act {
  gap: 6px;
  padding-left: 16px;
  color: var(--muted);
  font-size: 12px;
}
.a-meta {
  overflow: hidden;
}
.mark {
  flex: none;
  display: inline-flex;
  align-items: center;
  height: 16px;
  padding: 0 5px;
  border-radius: 4px;
  box-shadow: inset 0 0 0 1px var(--line-2);
  color: var(--soft);
  font-size: 10.5px;
  font-weight: 560;
  line-height: 1;
  letter-spacing: 0.01em;
}
.model {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}
.sep {
  color: var(--sep);
}
.num,
.spend {
  font-variant-numeric: tabular-nums;
}
.key {
  color: var(--p-ink);
  font-weight: 600;
}
.loops {
  display: inline-flex;
  gap: 3px;
}
.loops i {
  width: 12px;
  height: 3px;
  border-radius: 2px;
  background: var(--line-3);
  transition: background-color 0.4s;
}
.loops i.met {
  background: var(--run);
}
.a-act {
  min-height: 18px;
  color: var(--soft);
}
.a-act b {
  flex: none;
  font-weight: 550;
}
.tgt {
  min-width: 0;
  overflow: hidden;
  color: var(--muted);
  text-overflow: ellipsis;
}
.mono {
  font-family: var(--mono);
  font-size: 0.92em;
  letter-spacing: -0.01em;
}
.diff {
  flex: none;
}
.add {
  color: var(--add);
}
.del {
  color: var(--del);
}
.result {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}
.spend {
  flex: none;
  margin-left: auto;
  padding-left: 6px;
  color: var(--muted);
}
.ask {
  display: grid;
  gap: 6px;
  min-width: 0;
  padding: 2px 0 1px 16px;
}
.q {
  margin: 0;
  color: var(--text);
  font-size: 12.5px;
  font-weight: 500;
  line-height: 1.4;
}
.opts {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.opt {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 26px;
  padding: 0 9px 0 6px;
  border: none;
  border-radius: 7px;
  background: rgba(255, 255, 255, 0.06);
  box-shadow: inset 0 0 0 1px var(--line-2);
  color: var(--text);
  font: inherit;
  font-size: 12px;
  font-weight: 550;
  cursor: pointer;
  transition:
    background-color 0.15s,
    box-shadow 0.15s;
}
.opt:first-child,
.opt:hover {
  background: oklch(from var(--wait) l c h / 0.14);
  box-shadow: inset 0 0 0 1px var(--wait-line);
}
.opt:focus-visible {
  outline: 2px solid var(--wait);
  outline-offset: 2px;
}
kbd {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 16px;
  height: 16px;
  padding: 0 3px;
  border-radius: 4px;
  box-shadow: inset 0 0 0 1px var(--line-2);
  color: var(--muted);
  font-family: var(--font);
  font-size: 10.5px;
  font-weight: 500;
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
    box-shadow:
      inset 0 0 0 1.6px var(--tone),
      0 0 0 4px oklch(0.84 0.14 82 / 0);
    scale: 1.2;
  }
  0%,
  100% {
    box-shadow:
      inset 0 0 0 1.6px var(--tone),
      0 0 0 0 var(--wait-line);
  }
}
</style>
