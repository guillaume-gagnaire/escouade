<script setup lang="ts">
import type { Trust } from '~~/server/utils/listing';

// Where a version stands with the review, on every line of a list (docs/STORE.md §4.4): the seal once signed, « En revue »
// when the author asked for it, « Non vérifié » otherwise. Escouade's own are signed like any other (ESC-355).
const props = defineProps<{ trust: Trust; version?: string | null; large?: boolean }>();
const { words } = useCatalog();
</script>

<template>
  <VerifiedSeal v-if="props.trust === 'verified'" :version="props.version ?? undefined" :large="props.large" />
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
</style>
