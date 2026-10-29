<script setup lang="ts">
import { ACCOUNT_TEXTS } from '~/data/account';
import { pathIn } from '~/data/language';

// The header of the documentation of the site, as the preview draws it (design/store/): the brand and « Store »,
// then the tools on the right: the sign-in with GitHub for a visitor, « Mes plugins » and the author's initials once
// signed in. The search comes with its page (STORE-4).
const { lang, text, links } = useLang();
const { data: me } = await useMe();
const words = computed(() => ACCOUNT_TEXTS[lang.value]);
const account = computed(() => pathIn(lang.value, '/account'));
const signIn = computed(() => `/api/auth/github?to=${encodeURIComponent(account.value)}`);
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
        <a class="link wide" :href="links.docs">{{ text.header.docs }}</a>
        <template v-if="me.account">
          <NuxtLink class="link" :to="account">{{ words.mine }}</NuxtLink>
          <NuxtLink class="btn sm primary" :to="pathIn(lang, '/submit')">{{ text.header.submit }}</NuxtLink>
          <NuxtLink class="me" :to="account"
            ><AuthorAvatar :login="me.account.login" :github-id="me.account.githubId" /><span>{{ me.account.login }}</span></NuxtLink
          >
        </template>
        <NuxtLink v-if="!me.account" class="link" :to="pathIn(lang, '/submit')">{{ text.header.publish }}</NuxtLink>
        <a v-if="!me.account" class="btn sm" :href="signIn" :aria-label="words.signIn"
          ><svg class="i" viewBox="0 0 16 16" aria-hidden="true">
            <circle cx="4.5" cy="3.5" r="1.6" />
            <circle cx="4.5" cy="12.5" r="1.6" />
            <circle cx="11.5" cy="6" r="1.6" />
            <path d="M4.5 5.1V10.9M11.5 7.6C11.5 9.6 8 9.4 4.5 10.9" /></svg
          ><span class="gh-label">{{ words.signIn }}</span></a
        >
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
.me {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  font-weight: 550;
}
.btn.sm {
  display: inline-flex;
  gap: 8px;
  height: 30px;
  padding: 0 11px;
  border-radius: 7px;
  font-size: 13px;
  font-weight: 600;
}
@media (max-width: 1180px) {
  .wide {
    display: none;
  }
}
@media (max-width: 720px) {
  .bar {
    gap: 12px;
    height: 56px;
  }
  .tools {
    gap: 8px;
  }
  .tools .link,
  .me span,
  .gh-label {
    display: none;
  }
}
</style>
