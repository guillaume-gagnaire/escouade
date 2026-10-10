<script setup lang="ts">
const { text } = useLang();
</script>

<template>
  <section :id="text.anchors.install" class="section install">
    <div class="wrap">
      <p class="eyebrow">{{ text.install.eyebrow }}</p>
      <h2>{{ text.install.title }}</h2>
      <ol class="steps">
        <li v-for="(s, i) in text.install.steps" :key="s.title" class="step">
          <span class="n">{{ i + 1 }}</span>
          <h3>{{ s.title }}</h3>
          <p>{{ s.text }}</p>
          <a v-if="s.link" :href="s.link.href">{{ s.link.label }} →</a>
        </li>
      </ol>
    </div>
  </section>
</template>

<style scoped>
.install {
  background: var(--panel);
  border-block: 1px solid var(--line);
}
.steps {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 20px;
  margin: 32px 0 0;
  padding: 0;
  list-style: none;
}
.step {
  padding: 26px;
  border-radius: 12px;
  border: 1px solid var(--line2);
  background: var(--bg);
}
.n {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 10px;
  background: var(--brand);
  color: var(--ink);
  font-family: var(--mono);
  font-weight: 600;
}
h3 {
  margin: 16px 0 8px;
  font-size: 20px;
}
p {
  margin: 0 0 12px;
  color: var(--muted);
  font-size: 16px;
}
a {
  color: var(--brand);
  font-weight: 600;
  font-size: 15px;
  text-decoration: none;
}
a:hover {
  text-decoration: underline;
}
@media (max-width: 900px) {
  .steps {
    grid-template-columns: 1fr;
  }
}
</style>
