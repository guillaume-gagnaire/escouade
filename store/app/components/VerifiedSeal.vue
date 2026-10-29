<script setup lang="ts">
// The seal « Vérifié par Escouade »: the spark of the logo on the teal, the words beside it, and the version it
// covers when there is one (the badge is per version, docs/STORE.md §4.4).
defineProps<{ version?: string; large?: boolean }>();
const { text } = useLang();
</script>

<template>
  <span class="seal" :class="{ lg: large }"
    ><span class="spark"
      ><svg viewBox="-14 -14 28 28" aria-hidden="true">
        <g stroke="#041413" stroke-linecap="round" fill="none">
          <path d="M0 -11V11M-11 0H11" stroke-width="4.2" />
          <path d="M-6.6 -6.6L6.6 6.6M-6.6 6.6L6.6 -6.6" stroke-width="3.4" />
        </g></svg></span
    >{{ text.seal }}<span v-if="version" class="v">v{{ version }}</span></span
  >
</template>

<style scoped>
.seal {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 22px;
  padding: 0 8px 0 3px;
  border: 1px solid var(--seal-line);
  border-radius: 6px;
  background: var(--seal-bg);
  color: var(--brand-ink);
  font-size: 12px;
  font-weight: 600;
  line-height: 1;
  letter-spacing: normal;
  white-space: nowrap;
}
.spark {
  display: grid;
  place-items: center;
  width: 16px;
  height: 16px;
  border-radius: 4px;
  background: var(--brand);
}
.spark svg {
  width: 10px;
  height: 10px;
}
.lg {
  height: 28px;
  padding: 0 11px 0 4px;
  border-radius: 8px;
  font-size: 13px;
}
.lg .spark {
  width: 20px;
  height: 20px;
  border-radius: 5px;
}
.lg .spark svg {
  width: 12px;
  height: 12px;
}
.v {
  font: 500 11px var(--mono);
  opacity: 0.85;
}
</style>
