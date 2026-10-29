<script setup lang="ts">
import { otherLang, pathIn } from '~/data/language';

// The footer of the preview: the store's name, then the legal pages (the site's), the documentation and the other
// language. The authors' terms (/terms/) join them with the accounts (STORE-2).
const { lang, text, links } = useLang();
const route = useRoute();
const other = computed(() => otherLang(lang.value));
</script>

<template>
  <footer class="foot">
    <div class="wrap">
      <span>{{ text.title }}</span>
      <span class="grow"></span>
      <a :href="links.privacy">{{ text.footer.privacy }}</a>
      <a :href="links.legal">{{ text.footer.legal }}</a>
      <a :href="links.docs">{{ text.footer.docs }}</a>
      <NuxtLink :to="pathIn(other, route.path)" :lang="other" :hreflang="other">{{ text.footer.other }}</NuxtLink>
    </div>
  </footer>
</template>

<style scoped>
.foot {
  margin-top: 96px;
  padding: 28px 0;
  border-top: 1px solid var(--line);
  color: var(--dim);
  font-size: 13px;
}
.wrap {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 22px;
}
.grow {
  flex: 1;
}
a {
  transition: color 0.2s var(--ease);
}
a:hover {
  color: var(--text);
}
</style>
