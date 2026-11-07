<script setup lang="ts">
import { AGENTS_TEXTS, MCP_ADD, MCP_URL } from '~/data/agents';
import { pathIn } from '~/data/language';

// « Gérer ses plugins depuis un agent » (/agents, /en/agents; ESC-317): how to connect an agent to the store's MCP
// server, what the first connection does, what it allows, what it never does, and how to disconnect a client.
const { lang, links } = useLang();
const t = computed(() => AGENTS_TEXTS[lang.value].agents);
const docs = computed(() => `${links.value.docs.replace(/\/+$/, '')}/plugins/store-mcp/`);
useHead(() => ({ title: t.value.title }));
</script>

<template>
  <article class="wrap agents">
    <h1>{{ t.title }}</h1>
    <p class="lead">{{ t.lead }}</p>

    <section>
      <h2>{{ t.address }}</h2>
      <pre><code>{{ MCP_URL }}</code></pre>
    </section>

    <section>
      <h2>{{ t.addTitle }}</h2>
      <p>{{ t.addLead }}</p>
      <pre><code>{{ MCP_ADD }}</code></pre>
    </section>

    <section>
      <h2>{{ t.firstTitle }}</h2>
      <ol>
        <li v-for="item in t.first" :key="item">{{ item }}</li>
      </ol>
    </section>

    <section>
      <h2>{{ t.canTitle }}</h2>
      <ul>
        <li v-for="item in t.can" :key="item">{{ item }}</li>
      </ul>
      <h2>{{ t.adminTitle }}</h2>
      <p>{{ t.admin }}</p>
    </section>

    <section>
      <h2>{{ t.neverTitle }}</h2>
      <ul>
        <li v-for="item in t.never" :key="item">{{ item }}</li>
      </ul>
    </section>

    <section>
      <h2>{{ t.revokeTitle }}</h2>
      <p>{{ t.revoke }}</p>
      <p>
        <NuxtLink class="btn sm" :to="`${pathIn(lang, '/account')}#apps`">{{ t.account }}</NuxtLink>
      </p>
    </section>

    <section>
      <h2>{{ t.othersTitle }}</h2>
      <p>{{ t.others }}</p>
    </section>

    <p class="see">
      <a :href="docs">{{ t.more }}</a>
    </p>
  </article>
</template>

<style scoped>
.agents {
  max-width: 720px;
  padding-top: 56px;
}
h1 {
  font-size: clamp(26px, 2.6vw, 34px);
  font-weight: 620;
  letter-spacing: -0.03em;
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
p,
ul,
ol {
  margin-top: 12px;
  color: var(--soft);
}
ul,
ol {
  display: grid;
  gap: 8px;
  padding-left: 20px;
}
pre {
  margin-top: 12px;
  padding: 12px 14px;
  overflow-x: auto;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--panel, transparent);
  font-size: 13px;
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
