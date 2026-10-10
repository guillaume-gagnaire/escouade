<script setup lang="ts">
import type { NuxtError } from '#app';
import { LANGS } from '~/data/language';

// GitHub Pages answers an unknown address with 404.html, which shows this in the language of the address
// asked for; Nuxt's own page for it is white, and in English.
defineProps<{ error: NuxtError }>();
const { lang, text } = useLang();
const asset = useAsset();
const home = asset(LANGS.find((l) => l.code === lang.value)!.path);

useHead({ htmlAttrs: { lang: lang.value }, title: text.value.notFound.title, meta: [{ name: 'robots', content: 'noindex' }] });
</script>

<template>
  <main v-if="error.statusCode === 404" class="gone">
    <LogoMark :size="56" />
    <h1>{{ text.notFound.title }}</h1>
    <p>{{ text.notFound.text }}</p>
    <a class="btn primary" :href="home">{{ text.notFound.home }}</a>
  </main>
  <NuxtError v-else :error="error" />
</template>

<style scoped>
.gone {
  min-height: 100svh;
  display: grid;
  place-content: center;
  justify-items: center;
  gap: 14px;
  padding: 32px 16px;
  text-align: center;
}
h1 {
  margin: 8px 0 0;
  font-size: clamp(30px, 4vw, 44px);
  line-height: 1.15;
  letter-spacing: -0.02em;
}
p {
  margin: 0 0 12px;
  color: var(--muted);
}
</style>
