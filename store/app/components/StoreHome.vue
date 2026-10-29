<script setup lang="ts">
import { CATEGORY_ICONS, CATEGORY_ORDER } from '~/data/catalog';
import { pathIn } from '~/data/language';
import type { Entry } from '~~/server/utils/listing';

// The home page (design/store/, #home; docs/STORE.md §6): the search first, the categories, « Vérifiés par
// Escouade » in a row, « Nouveaux » and « Les plus installés », and the « Publier un plugin » band that leads to the
// documentation and the submission.
const { lang, text, links } = useLang();
const { words, to } = useCatalog();
const { data } = await useFetch('/api/catalog/home', { key: 'catalog-home' });
const cats = computed(() => CATEGORY_ORDER.filter((c) => data.value?.counts[c]));
const by = (e: Entry) => (e.builtin ? 'Escouade' : (e.author?.login ?? words.value.deleted));
</script>

<template>
  <div class="wrap">
    <section class="hero">
      <div>
        <h1>
          {{ text.home.title }} <span class="more">{{ text.home.more }}</span>
        </h1>
        <p>{{ text.home.lead }}</p>
      </div>
      <div>
        <form class="bigfind" role="search" method="get" :action="pathIn(lang, '/search')">
          <StoreIcon name="search" />
          <input name="q" type="search" :placeholder="words.findBig" :aria-label="words.findLabel" autocomplete="off" />
          <kbd>/</kbd>
        </form>
        <div class="trust">
          <span><VerifiedSeal /></span>
          <span><StoreIcon name="leaf" />{{ text.home.trust[0] }}</span>
          <span><StoreIcon name="box" />{{ text.home.trust[1] }}</span>
        </div>
      </div>
    </section>

    <nav class="cats" :aria-label="words.search.categories">
      <NuxtLink v-for="c in cats" :key="c" class="chip" :to="to.search({ cat: c })"
        ><StoreIcon :name="CATEGORY_ICONS[c]" />{{ words.categories[c] }}<span class="n">{{ data?.counts[c] }}</span></NuxtLink
      >
    </nav>

    <template v-if="data?.verified.length">
      <div class="section-head">
        <h2 class="h2">{{ words.home.verified }}</h2>
        <NuxtLink class="arrow" :to="to.search({ verified: '1' })">{{ words.home.allVerified }}</NuxtLink>
      </div>
      <div class="rows">
        <PluginRow v-for="e in data.verified" :key="e.id" :entry="e" />
      </div>
    </template>

    <div v-if="data?.fresh.length" class="cols">
      <div>
        <div class="section-head">
          <h2 class="h2">{{ words.home.fresh }}</h2>
          <NuxtLink class="arrow" :to="to.search({ sort: 'recent' })">{{ words.home.all }}</NuxtLink>
        </div>
        <div class="mini">
          <NuxtLink v-for="e in data.fresh" :key="e.id" :to="to.plugin(e.id)"
            ><PluginTile :mark="e.mark" :hue="e.hue" size="sm" />
            <div class="t">
              <b>{{ e.name[lang] }}</b>
              <div>{{ by(e) }} · {{ e.categories[0] ? words.categories[e.categories[0]] : '' }}</div>
            </div>
            <div class="r"><TrustMark :trust="e.trust" :version="e.version" /></div
          ></NuxtLink>
        </div>
      </div>
      <div v-if="data.top.length">
        <div class="section-head">
          <h2 class="h2">{{ words.home.top }}</h2>
          <NuxtLink class="arrow" :to="to.search({ sort: 'installs' })">{{ words.home.all }}</NuxtLink>
        </div>
        <div class="mini">
          <NuxtLink v-for="e in data.top" :key="e.id" :to="to.plugin(e.id)"
            ><PluginTile :mark="e.mark" :hue="e.hue" size="sm" />
            <div class="t">
              <b>{{ e.name[lang] }}</b>
              <div>{{ by(e) }} · {{ e.categories[0] ? words.categories[e.categories[0]] : '' }}</div>
            </div>
            <div class="r num">{{ e.installs.toLocaleString(lang === 'fr' ? 'fr-FR' : 'en-US') }}</div></NuxtLink
          >
        </div>
      </div>
    </div>
    <p v-else class="opening">
      <b>{{ text.home.opening }}</b> {{ words.home.empty }}
    </p>

    <section class="publish">
      <div>
        <h2>{{ text.home.publishTitle }}</h2>
        <p>{{ text.home.publishLead }}</p>
        <div class="acts">
          <NuxtLink class="btn primary" :to="pathIn(lang, '/submit')">{{ text.header.publish }}</NuxtLink>
          <a class="btn ghost" :href="links.docs">{{ text.home.guide }}</a>
        </div>
      </div>
      <ol class="steps">
        <li v-for="(step, i) in text.home.steps" :key="i">
          <span
            ><b>{{ step[0] }}</b
            >{{ step[1] }}<code v-if="i === 1">npx escouade-plugin validate</code></span
          >
        </li>
      </ol>
    </section>
  </div>
</template>

<style scoped>
.hero {
  display: grid;
  grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr);
  gap: 64px;
  align-items: end;
  padding: 72px 0 8px;
}
h1 {
  font-size: clamp(32px, 3.5vw, 50px);
  font-weight: 640;
  line-height: 1.02;
  letter-spacing: -0.04em;
  text-wrap: balance;
}
.more {
  color: var(--muted);
}
.hero p {
  max-width: 34em;
  margin-top: 18px;
  color: var(--muted);
  font-size: 16.5px;
}
.bigfind {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 52px;
  padding: 0 16px;
  border: 1px solid var(--line-strong);
  border-radius: 12px;
  background: var(--surface);
  color: var(--dim);
  font-size: 16px;
}
.bigfind:focus-within {
  border-color: var(--brand);
}
.bigfind svg.i {
  width: 18px;
  height: 18px;
}
.bigfind input {
  flex: 1;
  min-width: 0;
  border: 0;
  outline: 0;
  background: none;
  color: var(--text);
  font-size: 16px;
}
.bigfind input::placeholder {
  color: var(--dim);
  opacity: 1;
}
.bigfind input::-webkit-search-cancel-button {
  display: none;
}
kbd {
  padding: 1px 6px;
  border: 1px solid var(--line);
  border-radius: 5px;
  color: var(--dim);
  font: 11.5px var(--mono);
}
.trust {
  display: flex;
  flex-wrap: wrap;
  gap: 18px;
  margin-top: 14px;
  color: var(--dim);
  font-size: 13px;
}
.trust > span {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.cats {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 40px;
}
.cols {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 40px;
}
.mini {
  border-top: 1px solid var(--line);
}
.mini a {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  gap: 12px;
  align-items: center;
  padding: 11px 4px;
  border-bottom: 1px solid var(--line);
}
.mini a:hover {
  background: var(--hover);
}
.mini .t {
  min-width: 0;
}
.mini .t b {
  font-size: 14px;
  font-weight: 600;
}
.mini .t div {
  overflow: hidden;
  color: var(--dim);
  font-size: 12.5px;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.mini .r {
  color: var(--dim);
  font-size: 12.5px;
  text-align: right;
}
.opening {
  margin-top: 56px;
  padding: 14px 16px;
  border: 1px solid var(--line-strong);
  border-radius: 12px;
  background: var(--surface);
  color: var(--muted);
  font-size: 15px;
}
.opening b {
  color: var(--text);
  font-weight: 600;
}
.publish {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1.15fr);
  gap: 48px;
  margin-top: 72px;
  padding: 36px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.publish .acts {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.btn.ghost {
  border-color: transparent;
  background: none;
  color: var(--muted);
}
.publish h2 {
  font-size: 24px;
  font-weight: 620;
  line-height: 1.2;
  letter-spacing: -0.03em;
}
.publish p {
  margin: 10px 0 20px;
  color: var(--muted);
}
.steps {
  display: grid;
  gap: 14px;
  margin: 0;
  padding: 0;
  list-style: none;
  counter-reset: s;
}
.steps li {
  display: grid;
  grid-template-columns: 26px 1fr;
  gap: 12px;
  color: var(--muted);
  font-size: 14px;
  counter-increment: s;
}
.steps li::before {
  content: counter(s);
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  border: 1px solid var(--line-strong);
  border-radius: 7px;
  color: var(--text);
  font: 12px var(--mono);
}
.steps b {
  color: var(--text);
  font-weight: 600;
}
@media (max-width: 720px) {
  .hero {
    grid-template-columns: 1fr;
    gap: 24px;
    padding-top: 36px;
  }
  .cats {
    flex-wrap: nowrap;
    margin: 28px calc(-1 * var(--gutter)) 0;
    padding: 0 var(--gutter);
    overflow-x: auto;
    scrollbar-width: none;
  }
  .cols {
    grid-template-columns: 1fr;
  }
  .publish {
    grid-template-columns: 1fr;
    gap: 24px;
    margin-top: 48px;
    padding: 24px;
  }
}
</style>
