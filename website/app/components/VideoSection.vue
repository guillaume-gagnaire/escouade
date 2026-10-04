<script setup lang="ts">
const asset = useAsset();
</script>

<template>
  <section id="video" class="video">
    <div class="wrap">
      <video class="player" controls preload="none" playsinline width="1920" height="1080" :poster="asset('images/poster.jpg')">
        <source :src="asset('escouade.mp4')" type="video/mp4" />
        <!-- The voice over, as text, for anyone watching without sound: turned on from the player, shown by
             default it would hide the app's status bar the video talks about. -->
        <track kind="captions" srclang="fr" label="Français" :src="asset('escouade.vtt')" />
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
