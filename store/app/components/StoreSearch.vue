<script setup lang="ts">
import { CATEGORY_ICONS, CATEGORY_ORDER, type Category } from '~/data/catalog';
import { highlight } from '~~/shared/search';

// The search (design/store/, #search; docs/STORE.md §6): the results in a list, the filters on the left (category,
// verified only, what the plugin asks), the sort. Made on the server (decided on 2026-10-29); every filter is a link
// and the words a form, so the page works before its script, and the address can be shared.
const route = useRoute();
const { words, to } = useCatalog();
const { lang } = useLang();
const query = computed(() => route.query);
const { data } = await useFetch('/api/catalog/search', { query, key: 'catalog-search' });

const one = (k: string) => (typeof route.query[k] === 'string' ? (route.query[k] as string) : '');
const q = computed(() => one('q'));
const cat = computed(() => (CATEGORY_ORDER.includes(one('cat') as Category) ? (one('cat') as Category) : null));
const sort = computed(() => (['relevance', 'installs', 'recent'].includes(one('sort')) ? one('sort') : q.value ? 'relevance' : 'installs'));
const FLAGS = ['verified', 'noprog', 'nonet', 'nosecret'] as const;
const on = (k: string) => one(k) === '1';

/** The address of this search with `change` made; a filter set to null goes. */
function link(change: Record<string, string | null>) {
  const now: Record<string, string | undefined> = {};
  for (const k of ['q', 'cat', 'sort', ...FLAGS]) now[k] = one(k) || undefined;
  for (const [k, v] of Object.entries(change)) now[k] = v ?? undefined;
  return to.search(now);
}
const toggle = (k: string) => link({ [k]: on(k) ? null : '1' });
const verifiedCount = computed(() => data.value?.results.filter((e) => e.trust === 'verified' || e.trust === 'builtin').length ?? 0);
const marks = (name: string, desc: string) => ({ name: highlight(name, q.value), desc: highlight(desc, q.value) });
const sortOpen = ref(false);
</script>

<template>
  <div class="wrap search-layout">
    <aside class="filters">
      <div class="chips-sm">
        <NuxtLink v-if="cat" class="chip on" :to="link({ cat: null })"
          ><StoreIcon :name="CATEGORY_ICONS[cat]" />{{ words.categories[cat] }}</NuxtLink
        >
        <NuxtLink class="chip" :class="{ on: on('verified') }" :to="toggle('verified')">{{ words.search.verified }}</NuxtLink>
        <NuxtLink class="chip" :class="{ on: on('noprog') }" :to="toggle('noprog')">{{ words.search.noProgram }}</NuxtLink>
        <NuxtLink
          class="chip"
          :to="link({ sort: sort === 'relevance' ? 'installs' : sort === 'installs' ? 'recent' : q ? 'relevance' : 'installs' })"
          >{{ words.search.sorts[sort as 'relevance'] }}<StoreIcon name="chev"
        /></NuxtLink>
      </div>
      <h3>{{ words.search.categories }}</h3>
      <NuxtLink class="opt" :class="{ on: !cat }" :to="link({ cat: null })"
        >{{ words.search.all }}<span class="n">{{ data?.all ?? 0 }}</span></NuxtLink
      >
      <NuxtLink v-for="c in CATEGORY_ORDER" :key="c" class="opt" :class="{ on: cat === c }" :to="link({ cat: c })"
        >{{ words.categories[c] }}<span class="n">{{ data?.counts[c] ?? 0 }}</span></NuxtLink
      >
      <h3>{{ words.search.trust }}</h3>
      <NuxtLink class="opt" role="checkbox" :aria-checked="on('verified')" :to="toggle('verified')"
        ><span class="switch" :class="{ on: on('verified') }"></span>{{ words.search.verified }}</NuxtLink
      >
      <h3>{{ words.search.asks }}</h3>
      <NuxtLink
        v-for="[k, label] in [
          ['noprog', words.search.noProgram],
          ['nonet', words.search.noNetwork],
          ['nosecret', words.search.noSecret],
        ]"
        :key="k"
        class="opt"
        role="checkbox"
        :aria-checked="on(k!)"
        :to="toggle(k!)"
        ><span class="box" :class="{ on: on(k!) }"></span>{{ label }}</NuxtLink
      >
    </aside>

    <section class="results">
      <div class="results-head">
        <form class="q" role="search" method="get" :action="to.search()">
          <StoreIcon name="search" />
          <input name="q" type="search" :value="q" :placeholder="words.find" :aria-label="words.findLabel" autocomplete="off" />
          <input v-if="cat" type="hidden" name="cat" :value="cat" />
          <template v-for="k in FLAGS" :key="k"><input v-if="on(k)" type="hidden" :name="k" value="1" /></template>
        </form>
        <details class="sort hide-sm" :open="sortOpen" @toggle="sortOpen = ($event.target as HTMLDetailsElement).open">
          <summary>{{ words.search.sorts[sort as 'relevance'] }}<StoreIcon name="chev" /></summary>
          <div class="menu">
            <NuxtLink
              v-for="s in ['relevance', 'installs', 'recent'] as const"
              :key="s"
              :class="{ on: sort === s }"
              :to="link({ sort: s })"
              @click="sortOpen = false"
              >{{ words.search.sorts[s] }}</NuxtLink
            >
          </div>
        </details>
      </div>
      <p class="count">
        {{ words.search.count(data?.results.length ?? 0) }}{{ cat ? words.search.inCat(words.categories[cat]) : ''
        }}{{ verifiedCount ? words.search.verifiedCount(verifiedCount) : '' }}
      </p>
      <div v-if="data?.results.length" class="rows">
        <PluginRow
          v-for="e in data.results"
          :key="e.id"
          :entry="e"
          side
          :marks="q ? marks(e.name[lang], e.description[lang]) : undefined"
        />
      </div>
      <div v-else class="none">
        <b>{{ words.search.none }}</b>
        <span>{{ words.search.noneHint }}</span>
      </div>
    </section>
  </div>
</template>

<style scoped>
.search-layout {
  display: grid;
  grid-template-columns: 220px minmax(0, 1fr);
  gap: 48px;
  padding-top: 36px;
}
.filters h3 {
  margin: 26px 0 8px;
  color: var(--dim);
  font-size: 12.5px;
  font-weight: 600;
}
.opt {
  display: flex;
  align-items: center;
  gap: 9px;
  margin: 0 -8px;
  padding: 6px 8px;
  border-radius: 7px;
  color: var(--soft);
  font-size: 14px;
}
.opt:hover {
  background: var(--hover);
}
.opt .n {
  margin-left: auto;
  color: var(--dim);
  font: 11.5px var(--mono);
}
.opt.on {
  background: var(--surface-2);
  color: var(--text);
  font-weight: 560;
}
.box {
  display: grid;
  flex: none;
  place-items: center;
  width: 15px;
  height: 15px;
  border: 1.5px solid var(--line-strong);
  border-radius: 4px;
}
.box.on {
  border-color: var(--brand);
  background: var(--brand);
}
.box.on::after {
  content: '';
  width: 7px;
  height: 4px;
  border: 2px solid var(--ink);
  border-top: 0;
  border-right: 0;
  transform: translateY(-1px) rotate(-45deg);
}
.switch {
  position: relative;
  flex: none;
  width: 30px;
  height: 18px;
  border-radius: 9px;
  background: var(--line-strong);
}
.switch::after {
  content: '';
  position: absolute;
  top: 2px;
  left: 2px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--text);
}
.switch.on {
  background: var(--brand);
}
.switch.on::after {
  left: 14px;
  background: var(--ink);
}
.results-head {
  display: flex;
  align-items: center;
  gap: 12px;
  padding-bottom: 14px;
}
.q {
  display: flex;
  flex: 1;
  align-items: center;
  gap: 10px;
  height: 40px;
  padding: 0 12px;
  border: 1px solid var(--line-strong);
  border-radius: 10px;
  background: var(--surface);
  font-size: 15px;
}
.q:focus-within {
  border-color: var(--brand);
}
.q svg.i {
  color: var(--dim);
}
.q input {
  flex: 1;
  min-width: 0;
  border: 0;
  outline: 0;
  background: none;
  color: var(--text);
}
.q input::placeholder {
  color: var(--dim);
  opacity: 1;
}
.sort {
  position: relative;
  height: 40px;
  border: 1px solid var(--line);
  border-radius: 10px;
  color: var(--muted);
  font-size: 13.5px;
  white-space: nowrap;
}
.sort summary {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 100%;
  padding: 0 12px;
  list-style: none;
  cursor: pointer;
}
.sort summary::-webkit-details-marker {
  display: none;
}
.sort .menu {
  position: absolute;
  top: 46px;
  right: 0;
  z-index: 5;
  display: grid;
  min-width: 160px;
  padding: 4px;
  border: 1px solid var(--line-strong);
  border-radius: 10px;
  background: var(--surface);
  box-shadow: var(--shot);
}
.sort .menu a {
  padding: 7px 10px;
  border-radius: 7px;
  color: var(--soft);
  font-size: 13.5px;
}
.sort .menu a:hover,
.sort .menu a.on {
  background: var(--surface-2);
  color: var(--text);
}
.count {
  margin-bottom: 4px;
  color: var(--dim);
  font-size: 13px;
}
.results .rows {
  grid-template-columns: 1fr;
}
.none {
  display: grid;
  gap: 4px;
  padding: 28px 4px;
  border-top: 1px solid var(--line);
  color: var(--muted);
}
.none b {
  color: var(--text);
  font-weight: 600;
}
.chips-sm {
  display: none;
}
@media (max-width: 1180px) {
  .search-layout {
    grid-template-columns: 190px minmax(0, 1fr);
    gap: 36px;
  }
}
@media (max-width: 720px) {
  .search-layout {
    grid-template-columns: 1fr;
    gap: 16px;
    padding-top: 20px;
  }
  .filters {
    display: flex;
    gap: 8px;
    margin: 0 calc(-1 * var(--gutter));
    padding: 0 var(--gutter);
    overflow-x: auto;
    scrollbar-width: none;
  }
  .filters > :not(.chips-sm) {
    display: none;
  }
  .chips-sm {
    display: flex;
    gap: 8px;
  }
  .hide-sm {
    display: none;
  }
}
</style>
