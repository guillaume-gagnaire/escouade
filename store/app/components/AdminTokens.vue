<script setup lang="ts">
import { REVIEW_TEXTS } from '~/data/review';

// The administration tokens, in an administrator's account (docs/STORE.md §4.1): the review's script signs in with
// one. A token is made by a request of this card and shown here once, never in an address, a cookie or a log; the
// store keeps its hash. Each one is revoked here. Read in the browser alone (no token list in the page's HTML).
interface Line {
  ref: string;
  label: string;
  hint: string;
  createdAt: string;
  usedAt: string | null;
}
const { lang } = useLang();
const t = computed(() => REVIEW_TEXTS[lang.value].tokens);
const lines = ref<Line[]>([]);
const label = ref('');
const made = ref('');
const failed = ref(false);
const copied = ref(false);
const date = (iso: string) =>
  new Date(iso).toLocaleDateString(lang.value === 'fr' ? 'fr-FR' : 'en-GB', { day: 'numeric', month: 'short', year: 'numeric' });

async function load() {
  lines.value = (await $fetch<{ tokens: Line[] }>('/api/review/tokens').catch(() => ({ tokens: [] }))).tokens;
}
async function make() {
  failed.value = false;
  made.value = '';
  try {
    const r = await $fetch<{ token: string }>('/api/review/tokens', { method: 'POST', body: { label: label.value } });
    made.value = r.token;
    label.value = '';
  } catch {
    failed.value = true;
  }
  await load();
}
async function revoke(ref: string) {
  await $fetch(`/api/review/tokens/${ref}/revoke`, { method: 'POST' }).catch(() => {});
  await load();
}
async function copy() {
  await navigator.clipboard?.writeText(made.value).catch(() => {});
  copied.value = true;
}
onMounted(load);
</script>

<template>
  <div id="tokens" class="side-card tokens">
    <h2 class="h2">{{ t.title }}</h2>
    <p>{{ t.about }}</p>
    <div v-if="made" class="made" role="status">
      <code>{{ made }}</code>
      <button type="button" class="btn sm" @click="copy">{{ copied ? t.copiedNow : t.copy }}</button>
      <p class="warn">{{ t.once }}</p>
    </div>
    <ul v-if="lines.length" class="lines">
      <li v-for="l in lines" :key="l.ref">
        <span
          ><b>{{ l.label }}</b
          ><small class="mono">{{ l.hint }}</small
          ><small>{{ l.usedAt ? t.used(date(l.usedAt)) : t.unused }}</small></span
        >
        <button type="button" class="btn sm danger" @click="revoke(l.ref)">{{ t.revoke }}</button>
      </li>
    </ul>
    <p v-else>{{ t.none }}</p>
    <form class="make" @submit.prevent="make">
      <label for="token-label">{{ t.label }}</label>
      <input id="token-label" v-model="label" class="input" maxlength="60" required autocomplete="off" />
      <button class="btn sm" type="submit">{{ t.make }}</button>
      <p v-if="failed" class="warn">{{ t.failed }}</p>
    </form>
  </div>
</template>

<style scoped>
.tokens .h2 {
  margin: 0;
  font-size: 15px;
  font-weight: 620;
}
.made {
  display: grid;
  gap: 8px;
  padding: 10px 12px;
  border: 1px solid var(--seal-line);
  border-radius: 9px;
  background: var(--seal-bg);
}
.made code {
  font: 12px var(--mono);
  word-break: break-all;
}
.made .btn {
  justify-self: start;
}
.lines {
  display: grid;
  gap: 10px;
  margin: 0;
  padding: 0;
  list-style: none;
}
.lines li {
  display: flex;
  gap: 10px;
  align-items: center;
  justify-content: space-between;
}
.lines small {
  display: block;
  color: var(--dim);
  font-size: 12px;
}
.mono {
  font-family: var(--mono);
}
.make {
  display: grid;
  gap: 6px;
}
.make label {
  color: var(--dim);
  font-size: 12.5px;
  font-weight: 600;
}
.make .btn {
  justify-self: start;
}
.warn {
  color: var(--warn);
}
.btn.sm {
  height: 30px;
  padding: 0 11px;
  border-radius: 7px;
  font-size: 13px;
}
.btn.danger {
  color: var(--danger);
}
</style>
