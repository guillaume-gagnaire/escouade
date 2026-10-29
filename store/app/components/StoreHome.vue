<script setup lang="ts">
// The home page, before the first plugin (STORE-1): the preview's hero and its « Publier un plugin » band
// (design/store/, #home). The search, the categories and the lists of plugins come with STORE-4.
const { text, links } = useLang();
</script>

<template>
  <div class="wrap">
    <section class="hero">
      <div>
        <h1>
          {{ text.home.title }} <span class="more">{{ text.home.more }}</span>
        </h1>
        <p>{{ text.home.lead }}</p>
      </div>
      <div>
        <div class="opening">
          <b>{{ text.home.opening }}</b> {{ text.home.openingMore }}
        </div>
        <div class="trust">
          <span><VerifiedSeal /></span>
          <span
            ><svg class="i" viewBox="0 0 16 16" aria-hidden="true">
              <path d="M3 13C3 7 6.5 3 13 3C13 9.5 9 13 3 13Z" />
              <path d="M3 13L8.5 7.5" /></svg
            >{{ text.home.trust[0] }}</span
          >
          <span
            ><svg class="i" viewBox="0 0 16 16" aria-hidden="true">
              <path d="M2.5 5L8 2.2L13.5 5V11L8 13.8L2.5 11Z" />
              <path d="M2.5 5L8 7.8L13.5 5M8 7.8V13.8" /></svg
            >{{ text.home.trust[1] }}</span
          >
        </div>
      </div>
    </section>

    <section class="publish">
      <div>
        <h2>{{ text.home.publishTitle }}</h2>
        <p>{{ text.home.publishLead }}</p>
        <a class="btn" :href="links.docs">{{ text.home.guide }}</a>
      </div>
      <ol class="steps">
        <li v-for="(step, i) in text.home.steps" :key="i">
          <span
            ><b>{{ step[0] }}</b
            >{{ step[1] }}<code v-if="i === 1">npx escouade-plugin validate</code></span
          >
        </li>
      </ol>
    </section>
  </div>
</template>

<style scoped>
.hero {
  display: grid;
  grid-template-columns: minmax(0, 1.25fr) minmax(0, 1fr);
  gap: 64px;
  align-items: end;
  padding: 72px 0 8px;
}
h1 {
  font-size: clamp(32px, 3.5vw, 50px);
  font-weight: 640;
  line-height: 1.02;
  letter-spacing: -0.04em;
  text-wrap: balance;
}
.more {
  color: var(--muted);
}
.hero p {
  max-width: 34em;
  margin-top: 18px;
  color: var(--muted);
  font-size: 16.5px;
}
/* Where the preview's large search stands: what the store will hold, until it holds it. */
.opening {
  padding: 14px 16px;
  border: 1px solid var(--line-strong);
  border-radius: 12px;
  background: var(--surface);
  color: var(--muted);
  font-size: 15px;
}
.opening b {
  color: var(--text);
  font-weight: 600;
}
.trust {
  display: flex;
  flex-wrap: wrap;
  gap: 18px;
  margin-top: 14px;
  color: var(--dim);
  font-size: 13px;
}
.trust > span {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
.publish {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1.15fr);
  gap: 48px;
  margin-top: 72px;
  padding: 36px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: var(--surface);
}
.publish h2 {
  font-size: 24px;
  font-weight: 620;
  line-height: 1.2;
  letter-spacing: -0.03em;
}
.publish p {
  margin: 10px 0 20px;
  color: var(--muted);
}
.steps {
  display: grid;
  gap: 14px;
  margin: 0;
  padding: 0;
  list-style: none;
  counter-reset: s;
}
.steps li {
  display: grid;
  grid-template-columns: 26px 1fr;
  gap: 12px;
  color: var(--muted);
  font-size: 14px;
  counter-increment: s;
}
.steps li::before {
  content: counter(s);
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  border: 1px solid var(--line-strong);
  border-radius: 7px;
  color: var(--text);
  font: 12px var(--mono);
}
.steps b {
  color: var(--text);
  font-weight: 600;
}
@media (max-width: 720px) {
  .hero {
    grid-template-columns: 1fr;
    gap: 24px;
    padding-top: 36px;
  }
  .publish {
    grid-template-columns: 1fr;
    gap: 24px;
    margin-top: 48px;
    padding: 22px;
  }
}
</style>
