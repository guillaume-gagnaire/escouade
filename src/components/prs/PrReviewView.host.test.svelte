<script lang="ts">
  import { app } from '../../lib/state.svelte';
  import { project } from '../../test/ipc';
  import PrReviewView from './PrReviewView.svelte';

  // The view as PrsView gives it: on the review the app holds, which each call of the backend changes.
  let { me }: { me: string } = $props();
  const review = $derived(app.prReviews.p1?.[12]);
</script>

{#if review}<PrReviewView project={project()} {review} {me} />{/if}
