<script setup lang="ts">
import { redirectScript } from '~/data/language';

// The French page is the root: the first visit of a browser that has no French is sent to the English one.
const asset = useAsset();
// Server only: the script is in the generated page already, and must not be added (and run) a second time once the page is hydrated.
useServerHead({ script: [{ innerHTML: redirectScript(asset('en/')), tagPriority: 'critical' }] });
</script>

<template>
  <SitePage />
</template>
