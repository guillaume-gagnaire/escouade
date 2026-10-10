<script setup lang="ts">
const { text } = useLang();
</script>

<template>
  <div class="cards">
    <div v-for="c in text.cards" :key="c.title" class="card">
      <h3>{{ c.title }}</h3>
      <p>{{ c.text }}</p>
    </div>
  </div>
</template>

<style scoped>
.cards {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
  margin-top: 40px;
}
.card {
  padding: 24px;
  border-radius: 12px;
  border: 1px solid var(--line);
  background: var(--panel);
}
h3 {
  margin: 0 0 8px;
  font-size: 19px;
}
p {
  margin: 0;
  color: var(--muted);
  font-size: 16px;
}
@media (max-width: 900px) {
  .cards {
    grid-template-columns: 1fr;
  }
}
</style>
