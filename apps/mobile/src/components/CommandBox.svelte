<script lang="ts">
  // The command of an authorization, whole (MO-5, M4, `docs/MOBILE.md` § 3.4): every line, in JetBrains Mono, cut at
  // the character as a terminal does, never folded. A mark after its last line says when it was seen: « Autoriser »
  // arms only then (`onEnd`). Beyond twelve lines at the phone's width, the fold shows the first twelve and offers the
  // full screen (`full`), where the command is read to its end.
  import { onMount } from 'svelte';

  let {
    text,
    prompt = false,
    clamp = false,
    onEnd,
    onLines,
  }: {
    text: string;
    /** A shell command: each of its lines after « $ ». */
    prompt?: boolean;
    /** Show the first twelve lines alone (the rest is read in full screen). */
    clamp?: boolean;
    /** Its last line was seen. */
    onEnd?: () => void;
    /** How many lines it takes at this width. */
    onLines?: (n: number) => void;
  } = $props();

  let pre = $state<HTMLElement>();
  let end = $state<HTMLElement>();

  onMount(() => {
    if (pre && onLines) {
      const lh = parseFloat(getComputedStyle(pre).lineHeight) || 20;
      const pad = parseFloat(getComputedStyle(pre).paddingTop) + parseFloat(getComputedStyle(pre).paddingBottom);
      onLines(Math.max(1, Math.round((pre.scrollHeight - pad) / lh)));
    }
  });

  $effect(() => {
    if (!end || !onEnd || clamp) return;
    const seen = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          onEnd?.();
          seen.disconnect();
        }
      },
      { threshold: 1 },
    );
    seen.observe(end);
    return () => seen.disconnect();
  });
</script>

<div class="cmd" class:clamp>
  <pre bind:this={pre} data-testid="command"><code
      >{#each text.split('\n') as l, i (i)}<span class="ln"
          >{#if prompt}<span class="p" aria-hidden="true">{i === 0 ? '$ ' : '  '}</span>{/if}{l}</span
        >{/each}</code
    ></pre>
  <span class="end" bind:this={end} data-testid="command-end" aria-hidden="true"></span>
</div>

<style>
  .cmd {
    position: relative;
    border-radius: var(--r-m);
    background: var(--well, rgba(0, 0, 0, 0.35));
    box-shadow: inset 0 0 0 1px var(--line);
    overflow: hidden;
  }
  pre {
    margin: 0;
    padding: 12px 14px;
    font-family: var(--mono);
    font-size: 13px;
    line-height: 20px;
    color: var(--text);
    white-space: pre-wrap;
    word-break: break-all;
    overflow-wrap: anywhere;
  }
  .ln {
    display: block;
  }
  .p {
    color: var(--muted);
  }
  .clamp pre {
    max-height: calc(12 * 20px + 24px);
    overflow: hidden;
    -webkit-mask-image: linear-gradient(to bottom, #000 75%, transparent);
    mask-image: linear-gradient(to bottom, #000 75%, transparent);
  }
  .end {
    display: block;
    height: 1px;
  }
</style>
