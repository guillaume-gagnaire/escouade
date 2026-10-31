<script setup lang="ts">
import { AGENTS_TEXTS, type ConnectedAppLine } from '~/data/agents';
import { pathIn } from '~/data/language';

// « Applications connectées » (ESC-317, docs/STORE.md §2.5): the MCP clients the author allowed, their scopes, their
// last use and the last actions of their tools; « Déconnecter » is a plain form, like every action of the account.
defineProps<{ apps: ConnectedAppLine[]; now: string }>();
const { lang } = useLang();
const t = computed(() => AGENTS_TEXTS[lang.value].apps);
const date = (iso: string) =>
  new Date(iso).toLocaleDateString(lang.value === 'fr' ? 'fr-FR' : 'en-GB', { day: 'numeric', month: 'short', year: 'numeric' });
</script>

<template>
  <div id="apps" class="side-card apps">
    <h3 class="h2">{{ t.title }}</h3>
    <p>{{ t.about }}</p>
    <ul v-if="apps.length" class="lines">
      <li v-for="app in apps" :key="app.ref">
        <div class="head">
          <span class="who"
            ><b>{{ app.client }}</b
            ><small class="mono">{{ app.redirectHost }}</small></span
          >
          <form method="post" :action="`/api/account/apps/${app.ref}/revoke`">
            <input type="hidden" name="lang" :value="lang" />
            <button class="btn sm danger" type="submit">{{ t.revoke }}</button>
          </form>
        </div>
        <small
          >{{ app.scopes.map((s) => t.scopes[s] ?? s).join(' · ') }} · {{ t.since(date(app.createdAt)) }} ·
          {{ app.usedAt ? t.used(date(app.usedAt)) : t.unused }}</small
        >
        <details v-if="app.actions.length">
          <summary>{{ t.actions }}</summary>
          <ul class="acts">
            <li v-for="(act, i) in app.actions" :key="i">
              <span class="mono">{{ act.tool }}</span>
              <small
                >{{ date(act.at) }}<template v-if="!act.ok"> · {{ t.failed }}</template></small
              >
            </li>
          </ul>
        </details>
      </li>
    </ul>
    <p v-else class="none">{{ t.none }}</p>
    <NuxtLink class="how" :to="pathIn(lang, '/agents')">{{ t.how }}</NuxtLink>
  </div>
</template>

<style scoped>
.apps p {
  color: var(--muted);
  font-size: 13px;
}
.lines {
  display: grid;
  gap: 12px;
  margin: 12px 0 0;
  padding: 0;
  list-style: none;
}
.lines > li {
  display: grid;
  gap: 4px;
  padding-top: 12px;
  border-top: 1px solid var(--line);
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}
.who {
  display: grid;
  min-width: 0;
}
.who b {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
small {
  color: var(--dim);
  font-size: 12px;
}
details summary {
  cursor: pointer;
  color: var(--muted);
  font-size: 12px;
}
.acts {
  display: grid;
  gap: 2px;
  margin: 6px 0 0;
  padding: 0;
  list-style: none;
  font-size: 12px;
}
.acts li {
  display: flex;
  justify-content: space-between;
  gap: 8px;
}
.how {
  display: inline-block;
  margin-top: 12px;
  font-size: 13px;
  text-decoration: underline;
  text-underline-offset: 3px;
}
</style>
