<script lang="ts">
  import { docsStore } from '../../lib/docs.svelte';
  import { app } from '../../lib/state.svelte';
  import type { Project } from '../../lib/types';
  import DecisionsSpace from './DecisionsSpace.svelte';
  import DiscussionView from './DiscussionView.svelte';
  import DocumentsSpace from './DocumentsSpace.svelte';
  import SpecSpace from './SpecSpace.svelte';
  import TeamDocsBands from './TeamDocsBands.svelte';

  // The main area of the Docs tab (ESC-258): the space chosen in the sidebar. Its docs are read on opening, then again
  // each time what is left to share changes (a page written here, by an agent, a « Partager ») and each time a page is
  // compared to the code.
  let { project }: { project: Project } = $props();

  const space = $derived(app.docsSpace[project.id] ?? 'spec');
  const pending = $derived(app.docsShare[project.id]?.pending);
  /** The discussion open (ESC-261), while its agent is there. */
  const discussion = $derived.by(() => {
    const id = app.docsDiscussion[project.id];
    const a = id ? app.agents[id] : undefined;
    return a?.discussion ? a : null;
  });
  /** The pages being compared to the code: each one done brings its gaps (ESC-263). */
  const comparing = $derived((app.docsComparing[project.id] ?? []).join('|'));

  $effect(() => {
    void pending;
    void comparing;
    void project.docsBranch;
    // A teammate's page, decision or document came, a number was given (TS-9).
    void app.teamDocsTick[project.id];
    void docsStore.load(project.id);
  });
</script>

<div class="docs" data-space={discussion ? 'discussion' : space}>
  {#if !discussion}<TeamDocsBands {project} onchange={() => void docsStore.load(project.id)} />{/if}
  {#if discussion}
    <DiscussionView {project} agent={discussion} />
  {:else if space === 'spec'}
    <SpecSpace {project} />
  {:else if space === 'decisions'}
    <DecisionsSpace {project} />
  {:else}
    <DocumentsSpace {project} />
  {/if}
</div>

<style>
  .docs {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
</style>
