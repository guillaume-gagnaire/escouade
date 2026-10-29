<script setup lang="ts">
// Every page in the store's frame: its header, its footer, and the page's language on <html>.
const { lang, text } = useLang();
useHead(() => ({
  htmlAttrs: { lang: lang.value },
  titleTemplate: (title?: string) => (title ? `${title} · ${text.value.title}` : text.value.title),
  meta: [{ name: 'description', content: text.value.description }],
}));
</script>

<template>
  <StoreHeader />
  <main>
    <NuxtPage />
  </main>
  <StoreFooter />
</template>
