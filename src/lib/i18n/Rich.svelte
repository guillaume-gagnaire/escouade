<script lang="ts">
  // A sentence with markup in it (a key, a link, a bold word), translated whole: `text` is cut on its `{name}`
  // placeholders, each rendered by the snippet of that name, or written as the text of that name. A sentence is
  // never put together from pieces: the order of its words changes from one language to the other.
  //   <Rich text={tRich('composer.sendHint')}>{#snippet key()}<kbd>{keyLabel('Ctrl+Enter')}</kbd>{/snippet}</Rich>
  import type { Snippet } from 'svelte';

  type Part = Snippet | string | number | undefined;

  let { text, ...parts }: { text: string; [name: string]: Part } = $props();

  // Split on a capturing group: the placeholders are at the odd indexes, the text between them at the even ones.
  // A placeholder given nothing stays as written, to be seen.
  const pieces = $derived(text.split(/(\{\w+\})/).map((s, i): Part => (i % 2 ? (parts[s.slice(1, -1)] ?? s) : s)));
</script>

{#each pieces as piece}
  {#if typeof piece === 'function'}{@render piece()}{:else}{piece}{/if}
{/each}
