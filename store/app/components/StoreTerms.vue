<script setup lang="ts">
import { TERMS } from '~/data/terms';

// The authors' terms (/terms, /en/terms), versioned: read before the first submission, linked from the footer.
const { lang, links } = useLang();
const terms = computed(() => TERMS[lang.value]);
useHead(() => ({ title: terms.value.title }));
</script>

<template>
  <article class="wrap terms">
    <h1>{{ terms.title }}</h1>
    <p class="version">{{ terms.version }}</p>
    <p class="lead">{{ terms.lead }}</p>
    <section v-for="s in terms.sections" :key="s.title">
      <h2>{{ s.title }}</h2>
      <ul>
        <li v-for="item in s.items" :key="item">{{ item }}</li>
      </ul>
    </section>
    <p class="see">
      <a :href="links.privacy">{{ lang === 'fr' ? 'La politique de confidentialité' : 'The privacy policy' }}</a> ·
      <a :href="links.legal">{{ lang === 'fr' ? 'Les mentions légales' : 'The legal notice' }}</a>
    </p>
  </article>
</template>

<style scoped>
.terms {
  max-width: 720px;
  padding-top: 56px;
}
h1 {
  font-size: clamp(26px, 2.6vw, 34px);
  font-weight: 620;
  letter-spacing: -0.03em;
}
.version {
  margin-top: 6px;
  color: var(--dim);
  font-size: 13px;
}
.lead {
  margin-top: 20px;
  color: var(--soft);
  font-size: 16px;
}
h2 {
  margin-top: 36px;
  font-size: 17px;
  font-weight: 620;
  letter-spacing: -0.015em;
}
ul {
  display: grid;
  gap: 8px;
  margin: 12px 0 0;
  padding-left: 20px;
  color: var(--soft);
}
.see {
  margin-top: 40px;
  color: var(--muted);
  font-size: 14px;
}
.see a {
  text-decoration: underline;
  text-underline-offset: 3px;
}
</style>
