<script setup lang="ts">
import { pathIn } from '~/data/language';

// The header of the documentation of the site, as the preview draws it (design/store/): the brand and « Store »,
// then the tools on the right. The search and the sign-in come with their pages (STORE-4, STORE-2).
const { lang, text, links } = useLang();
</script>

<template>
  <header class="top">
    <div class="bar">
      <div class="where">
        <a class="brand" :href="links.site"><LogoMark :size="24" /><span>Escouade</span></a>
        <span class="slash" aria-hidden="true">/</span>
        <NuxtLink class="crumb" :to="pathIn(lang, '/')" :aria-label="text.header.home">Store</NuxtLink>
      </div>
      <div class="tools">
        <a class="link" :href="links.docs">{{ text.header.docs }}</a>
      </div>
    </div>
  </header>
</template>

<style scoped>
.top {
  position: sticky;
  top: 0;
  z-index: 10;
  border-bottom: 1px solid var(--line);
  background: color-mix(in srgb, var(--bg) 88%, transparent);
  backdrop-filter: saturate(1.4) blur(14px);
  -webkit-backdrop-filter: saturate(1.4) blur(14px);
}
.bar {
  display: flex;
  align-items: center;
  gap: 28px;
  height: 60px;
  padding: 0 var(--gutter);
}
.where {
  display: flex;
  flex: none;
  align-items: center;
  gap: 12px;
}
.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 16px;
  font-weight: 650;
  letter-spacing: -0.015em;
}
.slash {
  color: var(--line-strong);
  font-size: 20px;
  font-weight: 300;
}
.crumb {
  font-size: 15px;
  font-weight: 560;
  letter-spacing: -0.01em;
}
.tools {
  display: flex;
  align-items: center;
  gap: 20px;
  margin-left: auto;
}
.link {
  color: var(--muted);
  font-size: 14px;
  font-weight: 500;
  white-space: nowrap;
  transition: color 0.2s var(--ease);
}
.link:hover {
  color: var(--text);
}
@media (max-width: 720px) {
  .bar {
    gap: 12px;
    height: 56px;
  }
}
</style>
