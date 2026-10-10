<script setup lang="ts">
const { text } = useLang();
</script>

<template>
  <section id="faq" class="section">
    <div class="wrap narrow">
      <p class="eyebrow">{{ text.faq.eyebrow }}</p>
      <h2>{{ text.faq.title }}</h2>
      <details v-for="f in text.faq.items" :key="f.q" class="item">
        <summary>{{ f.q }}</summary>
        <p>{{ f.a }}</p>
      </details>
    </div>
  </section>
</template>

<style scoped>
.narrow {
  max-width: 820px;
}
.item {
  border-bottom: 1px solid var(--line2);
}
summary {
  padding: 20px 0;
  font-size: 19px;
  font-weight: 700;
  cursor: pointer;
  list-style: none;
  display: flex;
  justify-content: space-between;
  gap: 16px;
}
summary::-webkit-details-marker {
  display: none;
}
summary::after {
  content: '+';
  color: var(--brand);
  font-family: var(--mono);
}
.item[open] summary::after {
  content: '−';
}
p {
  margin: 0 0 20px;
  color: var(--muted);
}
</style>
