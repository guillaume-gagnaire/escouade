<script lang="ts">
  import { ansiRuns, logLineKind } from '../lib/branch-ci';

  // A job's log in reading only, as a terminal shows it (ESC-63): its colours (ANSI), its errors in red, its groups as
  // titles, on the terminal's background; selectable and copied as text, never typed into.
  let { text, tall = false }: { text: string; tall?: boolean } = $props();

  const lines = $derived(text.split('\n').map((line) => ({ kind: logLineKind(line), runs: ansiRuns(line.replace(/\r$/, '')) })));
</script>

<!-- Focusable: a long log scrolls with the keyboard too. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<pre class="term mono" class:tall tabindex="0">{#each lines as l, i (i)}<span class="ln {l.kind ?? ''}"
      >{#each l.runs as r, j (j)}{#if r.cls}<span class={r.cls}>{r.text}</span>{:else}{r.text}{/if}{/each}</span
    >{'\n'}{/each}</pre>

<style>
  .term {
    max-height: 320px;
    margin: 0;
    padding: 8px 10px;
    overflow: auto;
    border-radius: 7px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--code-fg);
    font-size: 11px;
    line-height: 1.45;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }
  .term.tall {
    max-height: min(56vh, 520px);
  }
  .term:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 1px;
  }
  .mono {
    font-family: var(--mono);
  }
  .ln.error {
    color: var(--del);
  }
  .ln.warning {
    color: var(--wait);
  }
  .ln.group {
    color: var(--soft);
    font-weight: 600;
  }
  .b {
    font-weight: 700;
  }
  .d {
    opacity: 0.65;
  }
  /* The 16 colours of the terminals' theme (xterm's), else close ones. */
  .fg-0,
  .fg-8 {
    color: var(--muted);
  }
  .fg-1,
  .fg-9 {
    color: var(--del);
  }
  .fg-2,
  .fg-10 {
    color: var(--run);
  }
  .fg-3,
  .fg-11 {
    color: var(--wait);
  }
  .fg-4,
  .fg-12 {
    color: var(--p-ink);
  }
  .fg-5,
  .fg-13 {
    color: var(--syn-k);
  }
  .fg-6,
  .fg-14 {
    color: var(--syn-s);
  }
  .fg-7,
  .fg-15 {
    color: var(--text);
  }
</style>
