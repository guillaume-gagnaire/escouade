<script setup lang="ts">
import { docPath } from '~/data/docs';
import { legalPath, privacyPath } from '~/data/privacy';
import { DISCORD, REPO, STORE, SUPPORT } from '~/data/site';

const { lang, text } = useLang();
</script>

<template>
  <footer class="foot">
    <div class="wrap">
      <div class="line">
        <span class="brand"><LogoMark :size="22" /> Escouade</span>
        <nav class="links">
          <NuxtLink :to="docPath(lang)">{{ text.header.docs }}</NuxtLink>
          <a :href="STORE[lang]">{{ text.header.plugins }}</a>
          <NuxtLink :to="privacyPath(lang)">{{ text.footer.privacy }}</NuxtLink>
          <NuxtLink :to="legalPath(lang)">{{ text.footer.legal }}</NuxtLink>
          <a :href="REPO">{{ text.footer.github }}</a>
          <a :href="DISCORD">{{ text.footer.discord }}</a>
          <a :href="SUPPORT[lang]">{{ text.footer.support }}</a>
          <a :href="`${REPO}/blob/main/LICENSE`">{{ text.footer.license }}</a>
        </nav>
      </div>
      <p class="note">{{ text.footer.note }}</p>
    </div>
  </footer>
</template>

<style scoped>
.foot {
  padding: 40px 0 56px;
  border-top: 1px solid var(--line);
  font-size: 14.5px;
}
.line {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}
.brand {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  font-weight: 600;
  letter-spacing: -0.01em;
}
/* Eight links: on a phone, they go on to a second line rather than past the screen's edge. */
.links {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 24px;
}
.links a {
  color: var(--muted);
  transition: color 0.2s var(--ease);
}
.links a:hover {
  color: var(--text);
}
.note {
  margin-top: 16px;
  color: var(--dim);
  font-size: 13.5px;
}
</style>
