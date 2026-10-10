<script setup lang="ts">
import type { Feature } from '~/data/catalogs';

defineProps<{ feature: Feature; reverse?: boolean }>();
const asset = useAsset();
</script>

<template>
  <article :id="feature.id" class="row" :class="{ reverse }">
    <div class="text">
      <h3>{{ feature.title }}</h3>
      <p>{{ feature.text }}</p>
      <ul>
        <li v-for="p in feature.points" :key="p">{{ p }}</li>
      </ul>
    </div>
    <img class="shot" :src="asset(feature.image)" :alt="feature.alt" width="1920" height="960" loading="lazy" />
  </article>
</template>

<style scoped>
.row {
  display: grid;
  grid-template-columns: 5fr 7fr;
  align-items: center;
  gap: 52px;
  padding: 44px 0;
}
.row.reverse .text {
  order: 2;
}
h3 {
  margin: 0 0 12px;
  font-size: 28px;
  line-height: 1.2;
  letter-spacing: -0.01em;
}
p {
  margin: 0 0 16px;
  color: var(--muted);
}
ul {
  display: grid;
  gap: 8px;
  margin: 0;
  padding: 0;
  list-style: none;
}
li {
  position: relative;
  padding-left: 24px;
  font-size: 16px;
}
li::before {
  content: '';
  position: absolute;
  left: 4px;
  top: 0.62em;
  width: 8px;
  height: 8px;
  border-radius: 2px;
  background: var(--brand);
}
.shot {
  width: 100%;
  height: auto;
  border-radius: 12px;
  border: 1px solid var(--line2);
  box-shadow: 0 24px 70px rgba(0, 0, 0, 0.5);
}
@media (max-width: 900px) {
  .row {
    grid-template-columns: 1fr;
    gap: 24px;
    padding: 32px 0;
  }
  .row.reverse .text {
    order: 0;
  }
}
</style>
