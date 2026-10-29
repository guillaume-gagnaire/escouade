<script setup lang="ts">
// Escouade's logo, as the site draws it.
withDefaults(defineProps<{ size?: number }>(), { size: 24 });
</script>

<template>
  <svg :width="size" :height="size" viewBox="0 0 100 100" aria-hidden="true">
    <rect x="8" y="8" width="84" height="84" rx="24" fill="#35C1BD" />
    <path d="M25 33L42 50L25 67" fill="none" stroke="#08090A" stroke-width="8.5" stroke-linecap="round" stroke-linejoin="round" />
    <g transform="translate(65 50)" stroke="#08090A" stroke-linecap="round">
      <path d="M0 -12V12M-12 0H12" stroke-width="5.4" />
      <path d="M-7.1 -7.1L7.1 7.1M-7.1 7.1L7.1 -7.1" stroke-width="4.4" />
    </g>
  </svg>
</template>
