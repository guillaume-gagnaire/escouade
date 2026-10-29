<script setup lang="ts">
const route = useRoute();
</script>

<template>
  <StoreSubmit :key="route.fullPath" />
</template>
