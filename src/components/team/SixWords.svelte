<script lang="ts">
  // Six words to compare out of band: the team's fingerprint, or its head (TS-3, §3.7).
  let { words, label, hint }: { words: string[]; label: string; hint?: string } = $props();
</script>

<div class="six" role="group" aria-label={label}>
  <span class="lbl">{label}</span>
  <ol class="words mono">
    {#each words as w, i (i)}<li>{w}</li>{/each}
  </ol>
  {#if hint}<span class="hint">{hint}</span>{/if}
</div>

<style>
  .six {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .lbl {
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 560;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }
  .words {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .words li {
    padding: 3px 9px;
    border-radius: 6px;
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--text);
    font-size: 12.5px;
  }
  .hint {
    color: var(--dim);
    font-size: 12px;
    line-height: 1.45;
  }
</style>
