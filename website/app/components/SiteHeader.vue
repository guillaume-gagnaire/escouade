<script setup lang="ts">
import { REPO } from '~/data/site';

const { text } = useLang();
</script>

<template>
  <header class="top">
    <div class="wrap bar">
      <a class="brand" href="#top"><LogoMark :size="28" /><span>Escouade</span></a>
      <nav class="nav" :aria-label="text.header.sections">
        <a href="#video">{{ text.header.video }}</a>
        <a :href="`#${text.anchors.features}`">{{ text.header.features }}</a>
        <a :href="`#${text.anchors.install}`">{{ text.header.install }}</a>
        <a href="#faq">{{ text.header.faq }}</a>
      </nav>
      <div class="tools">
        <LangSwitch />
        <a class="gh" :href="REPO">{{ text.header.github }}</a>
      </div>
    </div>
  </header>
</template>

<style scoped>
.top {
  position: sticky;
  top: 0;
  z-index: 10;
  background: rgba(27, 25, 23, 0.82);
  backdrop-filter: blur(10px);
  border-bottom: 1px solid var(--line);
}
.bar {
  height: 64px;
  display: flex;
  align-items: center;
  gap: 28px;
}
.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 20px;
  font-weight: 800;
  text-decoration: none;
}
.nav {
  display: flex;
  gap: 24px;
  margin-left: auto;
}
.tools {
  display: flex;
  align-items: center;
  gap: 12px;
}
.nav a,
.gh {
  color: var(--muted);
  font-size: 15px;
  font-weight: 600;
  text-decoration: none;
}
.nav a:hover,
.gh:hover {
  color: var(--text);
}
.gh {
  padding: 7px 14px;
  border: 1px solid var(--line2);
  border-radius: 8px;
}
@media (max-width: 720px) {
  .nav {
    display: none;
  }
  .tools {
    margin-left: auto;
  }
}
</style>
