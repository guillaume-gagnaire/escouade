<script setup lang="ts">
import { docPath } from '~/data/docs';
import { pathIn } from '~/data/language';
import { DISCORD, DOWNLOAD, REPO, STORE } from '~/data/site';

// The home page's header leads to its sections; the documentation's (`docs`) leads back to the home page, and holds
// the search and, on a narrow screen, the button that opens the sidebar. A page of its own (`page`) leads back to the
// home page too, and has no sections to lead to.
defineProps<{ docs?: boolean; page?: boolean }>();

const { lang, text } = useLang();
const asset = useAsset();
const menu = useDocsMenu();
</script>

<template>
  <header class="top" :class="{ 'in-docs': docs }">
    <div class="bar">
      <button
        v-if="docs"
        class="menu"
        type="button"
        aria-controls="docs-nav"
        :aria-expanded="menu"
        :aria-label="text.docs.menu"
        @click="menu = !menu"
      >
        <svg width="18" height="18" viewBox="0 0 18 18" aria-hidden="true">
          <path d="M3 5h12M3 9h12M3 13h12" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
        </svg>
      </button>
      <div class="where">
        <a class="brand" :href="docs || page ? asset(pathIn(lang, '/')) : '#top'"
          ><LogoMark :size="26" /><span class="name">Escouade</span></a
        >
        <template v-if="docs">
          <span class="slash" aria-hidden="true">/</span>
          <NuxtLink class="crumb" :to="docPath(lang)">{{ text.docs.title }}</NuxtLink>
        </template>
      </div>
      <DocsSearch v-if="docs" />
      <nav v-else-if="!page" class="nav" :aria-label="text.header.sections">
        <a href="#video">{{ text.header.video }}</a>
        <a :href="`#${text.anchors.features}`">{{ text.header.features }}</a>
        <a :href="`#${text.anchors.install}`">{{ text.header.install }}</a>
        <a :href="`#${text.anchors.roadmap}`">{{ text.header.roadmap }}</a>
        <a href="#faq">{{ text.header.faq }}</a>
      </nav>
      <div class="tools">
        <LangSwitch />
        <NuxtLink v-if="!docs" class="link" :to="docPath(lang)">{{ text.header.docs }}</NuxtLink>
        <a class="link optional" :href="STORE[lang]">{{ text.header.plugins }}</a>
        <a class="link optional" :href="DISCORD">{{ text.header.discord }}</a>
        <a class="link" :class="{ optional: docs }" :href="REPO">{{ text.header.github }}</a>
        <a class="get" :class="{ optional: docs }" :href="DOWNLOAD">{{ text.header.download }}</a>
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
  background: color-mix(in srgb, var(--bg) 72%, transparent);
  backdrop-filter: saturate(1.4) blur(14px);
  -webkit-backdrop-filter: saturate(1.4) blur(14px);
}
/* Over a page of text, almost opaque: the lines scrolling under it would show through the search. */
.top.in-docs {
  background: color-mix(in srgb, var(--bg) 94%, transparent);
}
.bar {
  display: flex;
  align-items: center;
  gap: 40px;
  height: 64px;
  padding: 0 var(--gutter);
}
.where {
  display: flex;
  align-items: center;
  gap: 12px;
}
.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 17px;
  font-weight: 650;
  letter-spacing: -0.015em;
}
.slash {
  color: var(--line-strong);
  font-size: 20px;
  font-weight: 300;
}
.crumb {
  color: var(--text);
  font-size: 15px;
  font-weight: 550;
  letter-spacing: -0.01em;
}
.menu {
  display: none;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  margin-right: -24px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: none;
  color: var(--muted);
  cursor: pointer;
}
.menu[aria-expanded='true'] {
  color: var(--text);
  background: rgba(255, 255, 255, 0.06);
}
.nav {
  display: flex;
  gap: 28px;
}
.tools {
  display: flex;
  align-items: center;
  gap: 20px;
  margin-left: auto;
}
.nav a,
.link {
  color: var(--muted);
  font-size: 14.5px;
  font-weight: 500;
  white-space: nowrap;
  transition: color 0.2s var(--ease);
}
.nav a:hover,
.link:hover {
  color: var(--text);
}
.get {
  display: inline-flex;
  align-items: center;
  height: 34px;
  padding: 0 14px;
  border-radius: 8px;
  background: var(--text);
  color: var(--bg);
  font-size: 14px;
  font-weight: 600;
  letter-spacing: -0.01em;
  white-space: nowrap;
  transition: background-color 0.2s var(--ease);
}
.get:hover {
  background: #fff;
}

/* Five links on one line need the room: below it, only the brand, the language, the documentation, GitHub and the
   download stay. */
@media (max-width: 1180px) {
  .nav {
    display: none;
  }
}
/* The documentation's header, once its sidebar has gone behind the menu button: the search shrinks to its icon and
   GitHub leaves (the footer keeps it), then the crumb, then the download. */
@media (max-width: 900px) {
  .menu {
    display: inline-flex;
    margin-right: -8px;
  }
  .in-docs .bar {
    gap: 24px;
  }
  .link.optional {
    display: none;
  }
}
@media (max-width: 720px) {
  .slash,
  .crumb {
    display: none;
  }
}
@media (max-width: 620px) {
  .link {
    display: none;
  }
}
@media (max-width: 560px) {
  .get.optional {
    display: none;
  }
  .bar {
    gap: 16px;
  }
  .menu {
    margin-right: 0;
  }
}
/* The smallest phones: the logo alone, its name kept for screen readers. */
@media (max-width: 360px) {
  .brand .name {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
}
</style>
