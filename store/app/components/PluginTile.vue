<script setup lang="ts">
// A plugin's tile: its icon when it has one (PLG-13, re-encoded by the store, served from /files/), else its two
// letters on its hue, as a project's tile in the app (design/store/, `.tile`). The hue is a class: the pages allow no
// style attribute. Its name is always written beside it: the tile says nothing more to a screen reader. One of
// Escouade's own has its logo drawn inline (shared/marks.ts): its brand's mark, or its category's glyph.
import { BRANDS, GLYPHS, type Logo } from '~~/shared/marks';
import { iconSrc, type Icon } from '~~/shared/tile';

const props = defineProps<{ mark: string; hue: number; size?: 'sm' | 'lg'; icon?: Icon | null; logo?: Logo | null }>();
const brand = computed(() => (props.logo && 'brand' in props.logo ? { id: props.logo.brand, ...BRANDS[props.logo.brand] } : null));
const glyph = computed(() =>
  props.logo && 'category' in props.logo ? { id: props.logo.category, paths: GLYPHS[props.logo.category] } : null,
);
// 64 px for the small tile (28 px, twice as dense), 256 px for the others.
const src = computed(() => (props.icon ? iconSrc(props.icon, props.size === 'sm' ? '64' : '256') : null));
</script>

<template>
  <span v-if="brand" class="tile brand" :class="[size, brand.id]" aria-hidden="true"
    ><svg viewBox="0 0 24 24"><path :fill="brand.color ?? 'currentColor'" :d="brand.path" /></svg
  ></span>
  <span v-else-if="glyph" class="tile glyph" :class="[size, glyph.id]" aria-hidden="true"
    ><svg viewBox="0 0 100 100">
      <path
        v-for="d in glyph.paths"
        :key="d"
        :d="d"
        fill="none"
        stroke="currentColor"
        stroke-width="8.5"
        stroke-linecap="round"
        stroke-linejoin="round"
      /></svg
  ></span>
  <span v-else-if="src" class="tile pic" :class="size" aria-hidden="true"><img :src="src" alt="" loading="lazy" decoding="async" /></span>
  <span v-else class="tile" :class="[size, `h${hue}`]" aria-hidden="true">{{ mark }}</span>
</template>

<style scoped>
.tile {
  --s: 40px;
  display: grid;
  flex: none;
  place-items: center;
  width: var(--s);
  height: var(--s);
  border-radius: calc(var(--s) * 0.26);
  background: oklch(0.36 0.07 var(--h));
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.08);
  color: oklch(0.93 0.05 var(--h));
  font-size: calc(var(--s) * 0.38);
  font-weight: 680;
  letter-spacing: -0.02em;
}
@media (prefers-color-scheme: light) {
  .tile {
    background: oklch(0.9 0.05 var(--h));
    box-shadow: inset 0 0 0 1px rgba(15, 17, 22, 0.06);
    color: oklch(0.35 0.09 var(--h));
  }
}
/* A brand on a neutral tile, in its colour; one in black or white only (GitHub), white on dark and black on light. */
.brand {
  background: var(--surface-2);
  box-shadow: inset 0 0 0 1px var(--line-strong);
  color: #fff;
}
.brand svg {
  width: 58%;
  height: 58%;
}
/* The tile of Escouade's logo, the teal of its square; a shade of it by category. */
.glyph {
  background: #35c1bd;
  box-shadow: none;
  color: #041413;
}
.glyph.source {
  background: #7fd9d6;
}
.glyph.forge {
  background: #1f8f8b;
  color: #e9fbfa;
}
.glyph svg {
  width: 74%;
  height: 74%;
}
@media (prefers-color-scheme: light) {
  .brand {
    background: #fff;
    color: #000;
  }
}
/* An icon fills its tile; the ring stays above it, so that a white or a black icon keeps its edge on both themes. */
.pic {
  position: relative;
  overflow: hidden;
  background: var(--surface-2);
}
.pic img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.pic::after {
  position: absolute;
  inset: 0;
  border-radius: inherit;
  box-shadow: inherit;
  content: '';
  pointer-events: none;
}
.lg {
  --s: 72px;
}
.sm {
  --s: 28px;
}
.h0 {
  --h: 0;
}
.h30 {
  --h: 30;
}
.h60 {
  --h: 60;
}
.h90 {
  --h: 90;
}
.h120 {
  --h: 120;
}
.h150 {
  --h: 150;
}
.h180 {
  --h: 180;
}
.h210 {
  --h: 210;
}
.h240 {
  --h: 240;
}
.h270 {
  --h: 270;
}
.h300 {
  --h: 300;
}
.h330 {
  --h: 330;
}
@media (max-width: 720px) {
  .lg {
    --s: 56px;
  }
}
</style>
