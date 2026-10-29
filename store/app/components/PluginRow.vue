<script setup lang="ts">
import type { Entry } from '~~/server/utils/listing';

// A plugin on a line of a list (design/store/, `.row`): its tile, its name and trust, two lines of its description,
// its author, category, what it asks and its installations; in the search, its version and last update on the right,
// and the words found marked.
const props = defineProps<{ entry: Entry; side?: boolean; marks?: { name: string[]; desc: string[] } }>();
const { lang, words, to } = useCatalog();
const e = computed(() => props.entry);
</script>

<template>
  <NuxtLink class="row" :to="to.plugin(e.id)">
    <PluginTile :mark="e.mark" :hue="e.hue" :icon="e.icon" :logo="e.logo" />
    <div class="body">
      <div class="name">
        <b v-if="marks"
          ><template v-for="(part, i) in marks.name" :key="i"
            ><mark v-if="i % 2">{{ part }}</mark
            ><template v-else>{{ part }}</template></template
          ></b
        >
        <b v-else>{{ e.name[lang] }}</b>
        <TrustMark :trust="e.trust" :version="e.version" />
      </div>
      <p class="desc">
        <template v-if="marks"
          ><template v-for="(part, i) in marks.desc" :key="i"
            ><mark v-if="i % 2">{{ part }}</mark
            ><template v-else>{{ part }}</template></template
          ></template
        >
        <template v-else>{{ e.description[lang] }}</template>
      </p>
      <div class="meta">
        <span>{{ e.builtin ? 'Escouade' : (e.author?.login ?? words.deleted) }}</span>
        <span v-if="e.categories[0]">{{ words.categories[e.categories[0]] }}</span>
        <PluginAsks :asks="e.asks" />
        <span class="grow"></span>
        <span v-if="!e.builtin" class="num">{{ words.installs(e.installs) }}</span>
      </div>
    </div>
    <div v-if="side" class="side">
      <span v-if="e.version" class="mono">v{{ e.version }}</span>
      <span v-if="e.updatedAt">{{ words.updated(words.date(e.updatedAt)) }}</span>
    </div>
  </NuxtLink>
</template>

<style scoped>
.row {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 14px;
  padding: 18px 4px;
  border-bottom: 1px solid var(--line);
  border-radius: 0;
  transition: background-color 0.15s var(--ease);
}
.row:has(.side) {
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: start;
}
.row:hover {
  background: var(--hover);
}
.name {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
  min-width: 0;
}
.name b {
  font-size: 15px;
  font-weight: 620;
  letter-spacing: -0.01em;
}
.desc {
  display: -webkit-box;
  margin: 4px 0 8px;
  overflow: hidden;
  color: var(--muted);
  font-size: 14px;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}
.meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 14px;
  color: var(--dim);
  font-size: 12.5px;
}
.grow {
  flex: 1;
}
.num {
  font-variant-numeric: tabular-nums;
}
.side {
  display: grid;
  justify-items: end;
  gap: 6px;
  color: var(--dim);
  font-size: 12.5px;
  text-align: right;
}
.mono {
  font-family: var(--mono);
  font-size: 0.86em;
}
mark {
  border-radius: 2px;
  background: color-mix(in srgb, var(--brand) 22%, transparent);
  color: inherit;
}
@media (max-width: 720px) {
  .row:has(.side) {
    grid-template-columns: auto minmax(0, 1fr);
  }
  .side {
    display: none;
  }
}
</style>
