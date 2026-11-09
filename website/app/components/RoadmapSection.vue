<script setup lang="ts">
import { fmt } from '~/data/catalog';
import { CHANGELOG, NEXT, PLAN } from '~/data/site';

// What comes next: the version in the works and what it brings, with its detailed plan and what is already out.
const { lang, text } = useLang();
</script>

<template>
  <section :id="text.anchors.roadmap" class="section">
    <div class="wrap">
      <div class="panel">
        <p class="next"><i aria-hidden="true" />{{ fmt(text.roadmap.next, { version: NEXT }) }}</p>
        <h2 class="title">{{ text.roadmap.title }}</h2>
        <p class="lead">{{ text.roadmap.lead }}</p>
        <ul class="items">
          <li v-for="i in text.roadmap.items" :key="i.title">
            <h3>{{ i.title }}</h3>
            <p>{{ i.text }}</p>
            <a v-if="i.link" class="arrow more" :href="i.link.href">{{ i.link.label }}</a>
          </li>
        </ul>
        <p class="links">
          <a class="arrow" :href="PLAN">{{ text.roadmap.plan }}</a>
          <a class="arrow" :href="CHANGELOG[lang]">{{ text.roadmap.changelog }}</a>
        </p>
      </div>
    </div>
  </section>
</template>

<style scoped>
.panel {
  position: relative;
  overflow: hidden;
  padding: clamp(32px, 5vw, 56px);
  border: 1px solid var(--line);
  border-radius: 24px;
  background: radial-gradient(70% 90% at 100% 0%, color-mix(in srgb, var(--brand) 9%, transparent), transparent 70%), var(--surface);
}
.next {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 24px;
  color: var(--muted);
  font-size: 13.5px;
  font-weight: 500;
}
.next i {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  border: 1.5px solid var(--brand);
}
.lead {
  max-width: 620px;
  margin-top: 16px;
  color: var(--muted);
  font-size: 17.5px;
}
.items {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 32px;
  margin: 44px 0 0;
  padding: 0;
  list-style: none;
}
.items li {
  padding-top: 20px;
  border-top: 1px solid var(--line);
}
h3 {
  font-size: 16px;
  font-weight: 600;
  letter-spacing: -0.015em;
}
.items p {
  margin-top: 8px;
  color: var(--muted);
  font-size: 15px;
  line-height: 1.55;
}
.more {
  margin-top: 12px;
  font-size: 15px;
}
.links {
  display: flex;
  flex-wrap: wrap;
  gap: 12px 32px;
  margin-top: 40px;
  font-size: 15px;
}

@media (max-width: 900px) {
  .items {
    grid-template-columns: minmax(0, 1fr);
    gap: 0;
  }
  .items li {
    padding: 20px 0;
  }
}
</style>
