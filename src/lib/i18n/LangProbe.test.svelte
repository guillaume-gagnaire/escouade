<script lang="ts">
  // A component of the tests only: one text read in its markup, one through a $derived.
  import { t } from '.';

  let { count }: { count: number } = $props();
  const files = $derived(t('common.count.files', { count }));
</script>

<button>{t('common.cancel')}</button>
<span data-testid="derived">{files}</span>
