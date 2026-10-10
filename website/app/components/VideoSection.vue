<script setup lang="ts">
import { VIDEO, imageOf } from '~/data/site';

const { lang } = useLang();
const asset = useAsset();
const video = computed(() => VIDEO[lang.value]);
</script>

<template>
  <section id="video" class="video">
    <div class="wrap">
      <video class="player" controls preload="none" playsinline width="1920" height="1080" :poster="asset(imageOf(lang, 'poster.jpg'))">
        <source :src="asset(video.src)" type="video/mp4" />
        <!-- The voice over, as text. The voice is French: the English page turns the English track on (an
             English visitor could not follow otherwise); the French page leaves it to the player, as shown
             by default it would hide the app's status bar the video talks about. -->
        <track
          v-for="t in video.tracks"
          :key="t.src"
          kind="captions"
          :srclang="t.srclang"
          :label="t.label"
          :src="asset(t.src)"
          :default="t.default"
        />
      </video>
    </div>
  </section>
</template>

<style scoped>
.video {
  padding: 8px 0 40px;
}
.player {
  width: 100%;
  height: auto;
  border-radius: 16px;
  border: 1px solid var(--line2);
  background: #000;
  box-shadow: 0 40px 120px rgba(0, 0, 0, 0.55);
}
</style>
