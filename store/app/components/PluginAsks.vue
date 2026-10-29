<script setup lang="ts">
import type { Ask } from '~~/server/utils/listing';

// What a plugin asks for, in one line of a list (design/store/, `.asks`): a program in the strong color, the
// permissions' decision in red, the hosts in a fixed-width font.
defineProps<{ asks: Ask[] }>();
const { words } = useCatalog();
const ICON = { run: 'run', decide: 'gavel', events: 'bolt', secret: 'key', none: 'leaf' } as const;
</script>

<template>
  <span class="asks">
    <template v-for="(a, i) in asks" :key="i">
      <span v-if="a.kind === 'net'"
        ><StoreIcon name="net" /><span class="mono">{{ a.host }}</span></span
      >
      <span v-else :class="{ strong: a.kind === 'run', red: a.kind === 'decide' }"
        ><StoreIcon :name="ICON[a.kind]" />{{ words.asks[a.kind] }}</span
      >
    </template>
  </span>
</template>

<style scoped>
.asks {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 14px;
  color: var(--dim);
  font-size: 12.5px;
}
.asks > span {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
.asks :deep(svg.i) {
  width: 13px;
  height: 13px;
}
.strong {
  color: var(--soft);
}
.red {
  color: var(--danger);
}
.mono {
  font-family: var(--mono);
  font-size: 0.86em;
}
</style>
