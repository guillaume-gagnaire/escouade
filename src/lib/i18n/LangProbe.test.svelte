<script lang="ts">
  // A component of the tests only: one text read in its markup, one through a $derived, one through Rich and a
  // snippet given as its child (which `npm run check` checks against the placeholders of the key).
  import { t } from '.';
  import Rich from './Rich.svelte';

  let { count }: { count: number } = $props();
  const files = $derived(t('common.count.files', { count }));
</script>

<button>{t('common.cancel')}</button>
<span data-testid="derived">{files}</span>
<span data-testid="rich"><Rich k="format.when.day" date="vendredi">{#snippet time()}<kbd>09:30</kbd>{/snippet}</Rich></span>
