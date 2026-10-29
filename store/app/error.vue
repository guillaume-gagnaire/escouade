<script setup lang="ts">
import type { NuxtError } from '#app';
import { pathIn } from '~/data/language';

// A page that does not exist, or that failed: said in the page's language, in the store's frame.
const props = defineProps<{ error: NuxtError }>();
const { lang, text } = useLang();
useHead(() => ({ htmlAttrs: { lang: lang.value }, title: text.value.title }));
</script>

<template>
  <StoreHeader />
  <main class="wrap gone">
    <p class="code">{{ props.error.statusCode }}</p>
    <h1>{{ props.error.statusCode === 404 ? text.error.missing : text.error.broken }}</h1>
    <NuxtLink class="btn" :to="pathIn(lang, '/')">{{ text.error.back }}</NuxtLink>
  </main>
  <StoreFooter />
</template>

<style scoped>
.gone {
  padding: 96px 0 0;
}
.code {
  color: var(--dim);
  font: 13px var(--mono);
}
h1 {
  margin: 8px 0 24px;
  font-size: clamp(28px, 3vw, 40px);
  font-weight: 640;
  letter-spacing: -0.035em;
}
</style>
