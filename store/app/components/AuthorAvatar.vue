<script setup lang="ts">
import { avatarHue, initials } from '~/data/account';

// An author's initials on a hue drawn from their GitHub id, as the preview draws them: no image is loaded from
// GitHub (docs/STORE.md, §1.2: nothing a visitor loads comes from a third party). A class, not a style attribute:
// the pages' Content-Security-Policy allows no inline style.
const props = defineProps<{ login: string; githubId: number }>();
</script>

<template>
  <span class="avatar" :class="`h${avatarHue(props.githubId)}`" aria-hidden="true">{{ initials(props.login) }}</span>
</template>

<style scoped>
.avatar {
  display: grid;
  flex: none;
  place-items: center;
  width: 26px;
  height: 26px;
  border-radius: 50%;
  background: oklch(0.55 0.1 var(--h, 30));
  color: #fff;
  font-size: 11px;
  font-weight: 650;
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
</style>
