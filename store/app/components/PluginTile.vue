<script setup lang="ts">
// A plugin's tile: its two letters on its hue, as a project's tile in the app (design/store/, `.tile`). The hue is a
// class: the pages allow no style attribute.
defineProps<{ mark: string; hue: number; size?: 'sm' | 'lg' }>();
</script>

<template>
  <span class="tile" :class="[size, `h${hue}`]" aria-hidden="true">{{ mark }}</span>
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
