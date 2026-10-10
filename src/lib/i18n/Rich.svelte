<script lang="ts" generics="K extends Key">
  // A sentence with markup in it (a key, a link, a bold word), translated whole: the text of `k` is cut on its
  // `{name}` placeholders, each rendered by the snippet of that name, or written as the text of that name. A sentence
  // is never put together from pieces: the order of its words changes from one language to the other. Each
  // placeholder is a required prop: a snippet forgotten fails to compile.
  //   <Rich k="composer.sendHint">{#snippet key()}<kbd>{keyLabel('Ctrl+Enter')}</kbd>{/snippet}</Rich>
  //   <Rich k="git.changedIn" count={n} branch={name} />
  import { tRaw, type Key, type RichPart, type RichProps } from '.';

  let { k, ...rest }: RichProps<K> = $props();

  const parts = $derived(rest as unknown as Record<string, RichPart | undefined>);
  const count = $derived(typeof parts.count === 'number' ? parts.count : undefined);

  // Split on a capturing group: the placeholders are at the odd indexes, the text between them at the even ones.
  // A placeholder given nothing (the types prevent it) stays as written, to be seen.
  const pieces = $derived(
    tRaw(k, count)
      .split(/(\{\w+\})/)
      .map((s, i): RichPart => (i % 2 ? (parts[s.slice(1, -1)] ?? s) : s)),
  );
</script>

{#each pieces as piece}
  {#if typeof piece === 'function'}{@render piece()}{:else}{piece}{/if}
{/each}
