<script setup lang="ts">
// The preview's line icons (design/store/index.html, its <symbol>s), drawn in place: a sprite would be one more
// request, and an inline style is refused by the pages' policy.
defineProps<{ name: string }>();
</script>

<template>
  <svg class="i" viewBox="0 0 16 16" aria-hidden="true">
    <template v-if="name === 'search'">
      <circle cx="7" cy="7" r="4.5" />
      <path d="M10.5 10.5L14 14" />
    </template>
    <template v-else-if="name === 'run'">
      <rect x="1.8" y="2.8" width="12.4" height="10.4" rx="2" />
      <path d="M4.8 6.3L7 8.3L4.8 10.3M8.6 10.3H11" />
    </template>
    <template v-else-if="name === 'net'">
      <circle cx="8" cy="8" r="6" />
      <path d="M2 8H14M8 2C9.8 4 9.8 12 8 14M8 2C6.2 4 6.2 12 8 14" />
    </template>
    <template v-else-if="name === 'key'">
      <circle cx="5.5" cy="10.5" r="3" />
      <path d="M7.7 8.3L13.5 2.5M11.5 4.5L13 6M10 6L11.3 7.3" />
    </template>
    <path v-else-if="name === 'bolt'" d="M9 1.8L3.5 9H8L7 14.2L12.5 7H8Z" />
    <template v-else-if="name === 'eye'">
      <path d="M1.5 8C3 5 5.3 3.5 8 3.5S13 5 14.5 8C13 11 10.7 12.5 8 12.5S3 11 1.5 8Z" />
      <circle cx="8" cy="8" r="2" />
    </template>
    <template v-else-if="name === 'gavel'">
      <path d="M8 1.8L13.5 4V8C13.5 11 11 13.3 8 14.2C5 13.3 2.5 11 2.5 8V4Z" />
      <path d="M6 8.2L7.4 9.6L10.2 6.6" />
    </template>
    <template v-else-if="name === 'leaf'">
      <path d="M3 13C3 7 6.5 3 13 3C13 9.5 9 13 3 13Z" />
      <path d="M3 13L8.5 7.5" />
    </template>
    <path v-else-if="name === 'check'" d="M3.5 8.5L6.5 11.5L12.5 4.5" />
    <path v-else-if="name === 'flag'" d="M3.5 14V2.5M3.5 2.8H12L10.2 5.8L12 8.8H3.5" />
    <template v-else-if="name === 'copy'">
      <rect x="5" y="5" width="8.5" height="8.5" rx="1.6" />
      <path d="M11 5V3.6C11 3 10.5 2.5 9.9 2.5H3.6C3 2.5 2.5 3 2.5 3.6V9.9C2.5 10.5 3 11 3.6 11H5" />
    </template>
    <template v-else-if="name === 'git'">
      <circle cx="4.5" cy="3.5" r="1.6" />
      <circle cx="4.5" cy="12.5" r="1.6" />
      <circle cx="11.5" cy="6" r="1.6" />
      <path d="M4.5 5.1V10.9M11.5 7.6C11.5 9.6 8 9.4 4.5 10.9" />
    </template>
    <template v-else-if="name === 'tag'">
      <path d="M2.5 2.5H8L13.5 8L8 13.5L2.5 8Z" />
      <circle cx="5.5" cy="5.5" r="1" />
    </template>
    <template v-else-if="name === 'box'">
      <path d="M2.5 5L8 2.2L13.5 5V11L8 13.8L2.5 11Z" />
      <path d="M2.5 5L8 7.8L13.5 5M8 7.8V13.8" />
    </template>
    <path v-else-if="name === 'ext'" d="M9 2.5H13.5V7M13.5 2.5L7.5 8.5M11.5 9.5V13.5H2.5V4.5H6.5" />
    <path v-else-if="name === 'chev'" d="M4.5 6.5L8 10L11.5 6.5" />
    <template v-else-if="name === 'palette'">
      <path
        d="M8 2C4.5 2 2 4.6 2 7.8C2 11 4.5 14 7.4 14C8.4 14 8.6 13.2 8.2 12.5C7.7 11.6 8.2 10.6 9.2 10.6H11C12.7 10.6 14 9.3 14 7.6C14 4.5 11.3 2 8 2Z"
      />
      <circle cx="5.3" cy="7" r="0.7" />
      <circle cx="8" cy="5" r="0.7" />
      <circle cx="10.8" cy="6.5" r="0.7" />
    </template>
    <template v-else-if="name === 'bell'">
      <path d="M4 11V7C4 4.8 5.8 3 8 3S12 4.8 12 7V11L13 12.5H3Z" />
      <path d="M6.6 14H9.4" />
    </template>
    <template v-else-if="name === 'cpu'">
      <rect x="3.5" y="3.5" width="9" height="9" rx="1.6" />
      <path d="M6 1.5V3.5M10 1.5V3.5M6 12.5V14.5M10 12.5V14.5M1.5 6H3.5M1.5 10H3.5M12.5 6H14.5M12.5 10H14.5" />
    </template>
    <template v-else-if="name === 'board'">
      <rect x="2" y="2.5" width="12" height="11" rx="1.8" />
      <path d="M6 2.5V13.5M10 2.5V13.5" />
    </template>
    <path
      v-else-if="name === 'tool'"
      d="M10 2.6A3.4 3.4 0 0 0 9.2 6.6L2.8 13L4 14.2L10.4 7.8A3.4 3.4 0 0 0 14.4 7L12.4 6.6L11.8 4.2L10 2.6Z"
    />
    <path
      v-else-if="name === 'globe'"
      d="M2.5 4H9M5.8 2.5V4M8 4C7.5 7 5.5 9.5 3 11M5 7.5C6 9 7.5 10 9 10.5M9.5 14L11.5 8.5L13.5 14M10.2 12.4H12.8"
    />
  </svg>
</template>
