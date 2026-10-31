<script setup lang="ts">
import { AGENTS_TEXTS } from '~/data/agents';
import { pathIn } from '~/data/language';

// The consent page of the MCP server (/oauth/consent, /en/oauth/consent; ESC-317): the client in its own words, the
// account, what it may do (`review` apart, and refused to an account that does not administer the store), what it
// never does; « Autoriser » or « Refuser », a plain form. The page's policy lets that form send the browser back to
// the client's address, and to no other (server/plugins/csp.ts).
interface RequestView {
  client: { name: string; uri: string | null; redirectHost: string };
  account: { login: string; name: string | null };
  asked: string[];
  granted: string[];
  refused: string[];
  redirectOrigin: string;
}
const { lang } = useLang();
const route = useRoute();
const t = computed(() => AGENTS_TEXTS[lang.value].consent);
const id = computed(() => (typeof route.query.request === 'string' ? route.query.request : ''));
const { data: me } = await useMe();
const { data } = await useFetch<RequestView>('/api/oauth/request', {
  key: `oauth-request-${id.value}`,
  query: { request: id },
  immediate: !!id.value && !!me.value?.account,
});
if (import.meta.server && data.value) {
  const event = useRequestEvent();
  if (event) event.context.formAction = data.value.redirectOrigin;
}
const signIn = computed(() => `/api/auth/github?to=${encodeURIComponent(route.fullPath)}`);
useHead(() => ({ title: t.value.title }));
</script>

<template>
  <article class="wrap consent">
    <h1>{{ t.title }}</h1>
    <template v-if="!me?.account">
      <p class="lead">{{ t.guest }}</p>
      <p>
        <a class="btn primary" :href="signIn">{{ t.signIn }}</a>
      </p>
    </template>
    <template v-else-if="data">
      <p class="lead">{{ t.lead(data.client.name) }}</p>
      <p class="small">{{ t.words }}</p>
      <p class="who">
        <b>{{ t.account(data.account.login) }}</b> · {{ t.from(data.client.redirectHost) }}
      </p>
      <section v-for="s in data.granted" :key="s" class="scope">
        <h2>{{ t.scopes[s]?.title ?? s }}</h2>
        <ul>
          <li v-for="item in t.scopes[s]?.items ?? []" :key="item">{{ item }}</li>
        </ul>
      </section>
      <p v-if="data.refused.length" class="notice warn">{{ t.refused }}</p>
      <p class="never">{{ t.never }}</p>
      <p class="small">{{ t.revoke }}</p>
      <form class="answer" method="post" action="/api/oauth/consent">
        <input type="hidden" name="request" :value="id" />
        <input type="hidden" name="lang" :value="lang" />
        <button class="btn primary" type="submit" name="decision" value="allow" :disabled="!data.granted.length">{{ t.allow }}</button>
        <button class="btn" type="submit" name="decision" value="deny">{{ t.deny }}</button>
      </form>
    </template>
    <p v-else class="notice warn">{{ t.expired }}</p>
    <p class="see">
      <NuxtLink :to="pathIn(lang, '/agents')">{{ t.back }}</NuxtLink>
    </p>
  </article>
</template>

<style scoped>
.consent {
  max-width: 620px;
  padding-top: 56px;
}
h1 {
  font-size: clamp(24px, 2.4vw, 30px);
  font-weight: 620;
  letter-spacing: -0.03em;
}
.lead {
  margin-top: 18px;
  font-size: 17px;
}
.small {
  margin-top: 6px;
  color: var(--dim);
  font-size: 13px;
}
.who {
  margin-top: 16px;
  color: var(--soft);
}
h2 {
  margin-top: 26px;
  font-size: 16px;
  font-weight: 620;
}
ul {
  display: grid;
  gap: 6px;
  margin: 10px 0 0;
  padding-left: 20px;
  color: var(--soft);
}
.never {
  margin-top: 22px;
  color: var(--soft);
}
.notice {
  margin-top: 18px;
  padding: 10px 14px;
  border: 1px solid var(--line);
  border-radius: 8px;
}
.answer {
  display: flex;
  gap: 10px;
  margin-top: 26px;
}
.see {
  margin-top: 36px;
  font-size: 14px;
}
.see a {
  color: var(--muted);
  text-decoration: underline;
  text-underline-offset: 3px;
}
</style>
