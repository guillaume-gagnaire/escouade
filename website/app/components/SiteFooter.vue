<script setup lang="ts">
import { REPO } from '~/data/site';

const { text } = useLang();
</script>

<template>
  <footer class="foot">
    <div class="wrap bar">
      <div class="brand"><LogoMark :size="22" /> Escouade</div>
      <div class="links">
        <a :href="REPO">{{ text.footer.github }}</a>
        <a :href="`${REPO}/blob/main/LICENSE`">{{ text.footer.license }}</a>
      </div>
    </div>
    <p class="wrap note">{{ text.footer.note }}</p>
  </footer>
</template>

<style scoped>
.foot {
  padding: 40px 0 48px;
  border-top: 1px solid var(--line);
  font-size: 15px;
}
.bar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}
.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  font-weight: 700;
}
.links {
  display: flex;
  gap: 20px;
}
.links a {
  color: var(--muted);
  text-decoration: none;
}
.links a:hover {
  color: var(--text);
}
.note {
  margin-top: 18px;
  color: var(--dim);
  font-size: 14px;
}
</style>
