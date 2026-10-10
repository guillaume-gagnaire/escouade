<script setup lang="ts">
import { featuresFor } from '~/data/catalogs';
import { SITE, imageOf } from '~/data/site';

// The whole page, in the language of its address.
const { lang, text } = useLang();
const features = featuresFor(lang.value);
const { meta } = text.value;

useSeoMeta({
  title: meta.title,
  description: meta.description,
  ogTitle: meta.title,
  ogDescription: meta.description,
  ogType: 'website',
  ogUrl: SITE,
  ogImage: `${SITE}${imageOf(lang.value, 'poster.jpg')}`,
  twitterCard: 'summary_large_image',
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
