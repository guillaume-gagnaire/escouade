<script setup lang="ts">
import { CATALOGS, featuresFor } from '~/data/catalogs';
import { LANGS } from '~/data/language';
import { SITE, imageOf, pageUrl } from '~/data/site';

// The whole page, in the language of its address.
const { lang, text } = useLang();
const features = featuresFor(lang.value);
const { meta } = text.value;
const other = LANGS.find((l) => l.code !== lang.value)!.code;
const image = `${SITE}${imageOf(lang.value, 'poster.jpg')}`;

useSeoMeta({
  title: meta.title,
  description: meta.description,
  ogTitle: meta.title,
  ogDescription: meta.description,
  ogType: 'website',
  ogUrl: pageUrl(lang.value),
  ogImage: image,
  ogSiteName: 'Escouade',
  ogLocale: meta.locale,
  ogLocaleAlternate: CATALOGS[other].meta.locale,
  twitterCard: 'summary_large_image',
  twitterTitle: meta.title,
  twitterDescription: meta.description,
  twitterImage: image,
});

// Each page names itself the one to index and points to the other version; the French root is the default
// for a visitor whose language neither covers.
useHead({
  htmlAttrs: { lang: lang.value },
  link: [
    { rel: 'canonical', href: pageUrl(lang.value) },
    ...LANGS.map((l) => ({ rel: 'alternate', hreflang: l.code, href: pageUrl(l.code) })),
    { rel: 'alternate', hreflang: 'x-default', href: pageUrl('fr') },
  ],
});
</script>

<template>
  <SiteHeader />
  <main>
    <HeroSection />
    <VideoSection />
    <section :id="text.anchors.features" class="section">
      <div class="wrap">
        <p class="eyebrow">{{ text.features.eyebrow }}</p>
        <h2>{{ text.features.title }}</h2>
        <p class="lead">{{ text.features.lead }}</p>
        <FeatureRow v-for="(f, i) in features" :key="f.id" :feature="f" :reverse="i % 2 === 1" />
        <FeatureCards />
      </div>
    </section>
    <InstallSteps />
    <FaqSection />
  </main>
  <SiteFooter />
</template>
