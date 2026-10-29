<script setup lang="ts">
// An author's page (STORE-4): who they are on GitHub (their initials, never their avatar: nothing a visitor loads
// comes from a third party) and their published plugins; `escouade` lists Escouade's own.
const route = useRoute();
const { lang, words } = useCatalog();
const login = computed(() => String(route.params.login ?? ''));
const { data } = await useFetch(() => `/api/catalog/authors/${encodeURIComponent(login.value)}`, { key: `author-${login.value}` });
if (!data.value) throw createError({ statusCode: 404, statusMessage: 'No such author', fatal: true });
const page = computed(() => data.value!);
const t = computed(() => words.value.author);
useHead(() => ({ title: page.value.author?.login ?? 'Escouade' }));
</script>

<template>
  <div class="wrap author">
    <section class="head">
      <LogoMark v-if="page.builtin" :size="56" />
      <AuthorAvatar v-else-if="page.author" class="big" :login="page.author.login" :github-id="page.author.githubId" />
      <div>
        <h1 class="page-title">
          {{ page.builtin ? t.builtin : page.author!.login }}
          <span v-if="page.author?.name && page.author.name !== page.author.login" class="more">· {{ page.author.name }}</span>
        </h1>
        <p v-if="page.builtin" class="lead">{{ t.builtinLead }}</p>
        <p v-else class="lead">
          <a :href="`https://github.com/${page.author!.login}`" rel="nofollow noopener noreferrer">{{ t.github }}</a>
          · {{ t.since(words.date(page.author!.since)) }}
        </p>
      </div>
    </section>
    <div class="section-head">
      <h2 class="h2">{{ t.plugins }}</h2>
      <span class="n">{{ page.plugins.length }}</span>
    </div>
    <div v-if="page.plugins.length" class="rows">
      <PluginRow v-for="e in page.plugins" :key="e.id" :entry="e" />
    </div>
    <p v-else class="none">{{ t.none }}</p>
  </div>
</template>

<style scoped>
.head {
  display: flex;
  align-items: center;
  gap: 18px;
  padding-top: 44px;
}
.big {
  width: 56px;
  height: 56px;
  font-size: 20px;
}
.lead {
  margin: 6px 0 0;
  color: var(--muted);
  font-size: 14.5px;
}
.lead a {
  color: var(--soft);
}
.n {
  color: var(--dim);
  font: 12px var(--mono);
}
.none {
  padding: 22px 4px;
  border-top: 1px solid var(--line);
  color: var(--muted);
}
</style>
