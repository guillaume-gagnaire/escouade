<script setup lang="ts">
// « Signaler » (docs/STORE.md §4.6): open to anyone, with an e-mail address and no GitHub account, as the DSA's
// notice and action asks (decided on 2026-10-29). A plain form: it works without script, under the strict policy,
// and the server answers with this page and a notice.
const route = useRoute();
const { lang, words, to } = useCatalog();
const id = computed(() => String(route.params.id ?? ''));
const { data } = await useFetch(() => `/api/catalog/plugins/${encodeURIComponent(id.value)}`, { key: `plugin-${id.value}` });
if (!data.value) throw createError({ statusCode: 404, statusMessage: 'No such plugin', fatal: true });
const e = computed(() => data.value!.entry);
const t = computed(() => words.value.report);
useHead(() => ({ title: `${t.value.title} · ${e.value.name[lang.value]}`, meta: [{ name: 'robots', content: 'noindex' }] }));
const notice = computed(() => {
  const key = typeof route.query.notice === 'string' ? route.query.notice : '';
  const n = t.value.notices[key];
  if (!n) return null;
  return { ok: key === 'sent', text: typeof n === 'function' ? n(String(route.query.n ?? '')) : n };
});
</script>

<template>
  <div class="wrap report-layout">
    <h1 class="page-title">
      {{ t.title }} <span class="more">{{ e.name[lang] }}</span>
    </h1>
    <p class="lead">{{ t.lead }}</p>
    <p v-if="notice" class="notice" :class="notice.ok ? 'ok' : 'bad'" role="status">{{ notice.text }}</p>
    <form v-if="!notice?.ok" method="post" action="/api/reports" class="form">
      <input type="hidden" name="plugin" :value="e.id" />
      <input type="hidden" name="lang" :value="lang" />
      <fieldset>
        <legend>{{ t.reason }}</legend>
        <label v-for="(label, k) in t.reasons" :key="k" class="choice"
          ><input type="radio" name="reason" :value="k" required />{{ label }}</label
        >
      </fieldset>
      <div v-if="data!.versions.length > 1" class="field">
        <label for="report-version">{{ t.version }}</label>
        <select id="report-version" name="version" class="input">
          <option v-for="v in data!.versions" :key="v.version" :value="v.version">v{{ v.version }}</option>
        </select>
      </div>
      <div class="field">
        <label for="report-details">{{ t.details }}</label>
        <textarea id="report-details" name="details" class="input area" rows="6" minlength="20" maxlength="4000" required></textarea>
        <small>{{ t.detailsHint }}</small>
      </div>
      <div class="field">
        <label for="report-email">{{ t.email }}</label>
        <input id="report-email" name="email" type="email" class="input" autocomplete="email" maxlength="254" required />
      </div>
      <label class="choice faith"><input type="checkbox" name="faith" value="yes" required />{{ t.faith }}</label>
      <div class="acts">
        <button type="submit" class="btn primary">{{ t.send }}</button>
        <NuxtLink class="btn ghost" :to="to.plugin(e.id)">{{ t.back }}</NuxtLink>
      </div>
    </form>
    <NuxtLink v-else class="arrow" :to="to.plugin(e.id)">{{ t.back }}</NuxtLink>
  </div>
</template>

<style scoped>
.report-layout {
  max-width: 640px;
  padding-top: 44px;
}
.lead {
  margin: 12px 0 0;
  color: var(--muted);
}
.notice {
  margin: 20px 0 0;
  padding: 12px 14px;
  border: 1px solid var(--line-strong);
  border-radius: 10px;
  background: var(--surface);
  font-size: 14px;
}
.notice.ok {
  border-color: color-mix(in oklch, var(--ok) 40%, transparent);
}
.notice.bad {
  border-color: color-mix(in oklch, var(--danger) 45%, transparent);
  background: var(--danger-bg);
}
.form {
  display: grid;
  gap: 22px;
  margin-top: 28px;
}
fieldset {
  display: grid;
  gap: 6px;
  margin: 0;
  padding: 0;
  border: 0;
}
legend,
.field > label {
  margin-bottom: 6px;
  color: var(--dim);
  font-size: 12.5px;
  font-weight: 600;
}
.choice {
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--soft);
  font-size: 14px;
}
.choice input {
  accent-color: var(--brand);
}
.faith {
  align-items: start;
}
.faith input {
  margin-top: 4px;
}
.field {
  display: grid;
  gap: 6px;
}
.field small {
  color: var(--dim);
  font-size: 12.5px;
}
.input {
  width: 100%;
  height: 36px;
  padding: 0 11px;
  border: 1px solid var(--line-strong);
  border-radius: 8px;
  background: var(--surface);
  color: var(--text);
  font-size: 14px;
}
.input:focus-visible {
  border-color: var(--brand);
  outline: none;
}
.area {
  height: auto;
  padding: 9px 11px;
  resize: vertical;
}
.acts {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.btn.ghost {
  border-color: transparent;
  background: none;
  color: var(--muted);
}
.arrow {
  margin-top: 20px;
}
</style>
