<script setup lang="ts">
import { LANGS, rememberLanguage, type LangInfo } from '~/data/language';

// Two real links, one per page, so that the switch works without a script too; the click also keeps the choice.
const { lang, text } = useLang();
const asset = useAsset();

/** The French link asks for French explicitly: the first visit's redirection would send the visitor away again. */
const href = (l: LangInfo) => `${asset(l.path)}${l.code === 'fr' ? '?lang=fr' : ''}`;

function choose(l: LangInfo, e: MouseEvent) {
  rememberLanguage(l.code);
  // Already there: nothing to load.
  if (l.code === lang.value) e.preventDefault();
}
</script>

<template>
  <nav class="lang" :aria-label="text.header.language">
    <a
      v-for="l in LANGS"
      :key="l.code"
      :href="href(l)"
      :lang="l.code"
      :hreflang="l.code"
      :title="l.name"
      :aria-current="l.code === lang ? 'page' : undefined"
      @click="choose(l, $event)"
      >{{ l.label }}</a
    >
  </nav>
</template>

<style scoped>
.lang {
  display: flex;
  border: 1px solid var(--line2);
  border-radius: 8px;
}
.lang a {
  padding: 7px 11px;
  color: var(--muted);
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-decoration: none;
}
.lang a:first-child {
  border-radius: 7px 0 0 7px;
}
.lang a:last-child {
  border-radius: 0 7px 7px 0;
}
.lang a:hover {
  color: var(--text);
}
.lang a[aria-current='page'] {
  background: var(--elev);
  color: var(--text);
}
/* Inside the frame, which would clip an outline drawn outside. */
.lang a:focus-visible {
  outline-offset: -2px;
}
</style>
