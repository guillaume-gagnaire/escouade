<script setup lang="ts">
import { fmt } from '~/data/catalog';
import { DOWNLOAD, REPO } from '~/data/site';

const { text } = useLang();
const { version } = useRuntimeConfig().public;
</script>

<template>
  <section id="top" class="hero">
    <div class="stage">
      <div class="copy">
        <h1>
          <span class="line">{{ text.hero.line1 }}</span>
          <span class="line accent">{{ text.hero.line2 }}</span>
        </h1>
        <p class="lede">{{ text.hero.lede }}</p>
        <div class="cta">
          <a class="btn primary" :href="DOWNLOAD">{{ text.hero.download }}</a>
          <a class="btn" :href="REPO">{{ text.hero.github }}</a>
        </div>
        <p class="meta">{{ fmt(text.hero.meta, { version }) }}</p>
      </div>
      <SquadWindows class="squad" />
    </div>
    <a class="next" href="#video">
      {{ text.hero.video }}
      <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
        <path
          d="M3.5 6l4.5 4.5L12.5 6"
          fill="none"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </a>
  </section>
</template>

<style scoped>
.hero {
  --pad: clamp(28px, 5svh, 64px);
  position: relative;
  display: grid;
  align-items: center;
  min-height: calc(100vh - 64px);
  min-height: calc(100svh - 64px);
  padding: var(--pad) 0 calc(var(--pad) + 28px);
  background:
    radial-gradient(60% 70% at 78% 45%, color-mix(in oklch, var(--brand) 11%, transparent), transparent 70%),
    radial-gradient(900px 480px at 30% 0%, color-mix(in oklch, var(--brand) 7%, transparent), transparent 70%);
  overflow: hidden;
}
.stage {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  align-items: center;
  gap: clamp(32px, 5vw, 80px);
  width: min(1400px, 100% - clamp(32px, 6vw, 96px));
  margin-inline: auto;
}
.copy {
  max-width: 680px;
}
h1 {
  margin: 0;
  font-size: clamp(44px, 5.1vw, 84px);
  font-weight: 800;
  line-height: 0.98;
  letter-spacing: -0.035em;
  text-wrap: balance;
}
.line {
  display: block;
}
.accent {
  color: var(--brand);
}
.lede {
  max-width: 34em;
  margin: clamp(22px, 3.4svh, 34px) 0 0;
  color: var(--muted);
  font-size: clamp(17px, 1.35vw, 19px);
  text-wrap: pretty;
}
.cta {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  margin-top: clamp(24px, 4svh, 36px);
}
.meta {
  margin: 18px 0 0;
  font-family: var(--mono);
  font-size: 13px;
  color: var(--dim);
}
.squad {
  justify-self: end;
  /* The first viewport holds the window and its caption: the header, both paddings and the caption come off. */
  width: min(100%, 760px, calc(100svh - 64px - 2 * var(--pad) - 68px));
  min-width: min(100%, 440px);
}
.next {
  position: absolute;
  left: 50%;
  bottom: 18px;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  translate: -50% 0;
  padding: 6px 10px;
  border-radius: 8px;
  color: var(--dim);
  font-size: 14px;
  text-decoration: none;
  transition: color 0.15s;
}
.next:hover {
  color: var(--text);
}

/* The payoff holds on one line wherever the column can carry it. */
@media (min-width: 961px) {
  .accent {
    white-space: nowrap;
  }
}
@media (max-width: 960px) {
  .hero {
    min-height: auto;
    padding-bottom: var(--pad);
  }
  .stage {
    grid-template-columns: minmax(0, 1fr);
  }
  .copy {
    max-width: 640px;
  }
  .squad {
    justify-self: center;
    width: min(100%, 600px);
    min-width: 0;
  }
  .next {
    display: none;
  }
}
</style>
