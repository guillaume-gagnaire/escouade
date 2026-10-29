<script setup lang="ts">
import type { Trust } from '~~/server/utils/listing';

// Where a version stands with the review, on every line of a list (docs/STORE.md §4.4): the seal once signed,
// « Livré avec Escouade » for Escouade's own (it is worth the seal, docs/PLUGINS.md §4.3), « En revue » when the
// author asked for it, « Non vérifié » otherwise.
const props = defineProps<{ trust: Trust; version?: string | null; large?: boolean }>();
const { words } = useCatalog();
</script>

<template>
  <VerifiedSeal v-if="props.trust === 'verified'" :version="props.version ?? undefined" :large="props.large" />
  <span v-else-if="props.trust === 'builtin'" class="own" :class="{ lg: props.large }" :title="words.trust.builtinTitle"
    ><LogoMark :size="props.large ? 20 : 16" />{{ words.trust.builtin }}</span
  >
  <span
    v-else
    class="tag"
    :class="{ review: props.trust === 'review' }"
    :title="props.trust === 'review' ? words.trust.reviewTitle : words.trust.unverifiedTitle"
    >{{ props.trust === 'review' ? words.trust.review : words.trust.unverified }}</span
  >
</template>

<style scoped>
.tag {
  display: inline-flex;
  align-items: center;
  height: 22px;
  padding: 0 8px;
  border: 1px solid var(--line-strong);
  border-radius: 6px;
  color: var(--muted);
  font-size: 12px;
  font-weight: 550;
  letter-spacing: normal;
  white-space: nowrap;
}
.review {
  border-style: dashed;
}
/* Escouade's own: the seal's colors, the logo in place of its spark. */
.own {
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
.own.lg {
  height: 28px;
  padding: 0 11px 0 4px;
  border-radius: 8px;
  font-size: 13px;
}
</style>
