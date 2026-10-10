<script lang="ts">
  import Rich from '../../lib/i18n/Rich.svelte';
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import type { Agent } from '../../lib/types';

  // The setup of the agent's new worktree under way: the step running and, unfolded, what it writes as it comes (its
  // last lines: the whole output goes to the agent's setup log).
  let { agent }: { agent: Agent } = $props();

  let open = $state(false);
  let out = $state<HTMLPreElement>();
  // The output follows what comes while it is read at its end, not once the reader scrolled up in it.
  let stick = true;
  const lines = $derived(app.setupOutput[agent.id]?.lines ?? []);
  const outId = $derived(`setup-out-${agent.id}`);

  // After the lines are drawn: at the end of the new ones.
  $effect(() => {
    void lines;
    if (out && stick) out.scrollTop = out.scrollHeight;
  });

  function onScroll() {
    if (out) stick = out.scrollHeight - out.scrollTop - out.clientHeight < 8;
  }

  function toggle() {
    open = !open;
    stick = true;
  }
</script>

<div class="setup">
  <div class="line">
    <span class="dots" aria-hidden="true"><span></span><span></span><span></span></span>
    <span role="status"><Rich k="conv.setup.running">{#snippet step()}<span class="mono">{agent.setup}</span>{/snippet}</Rich></span>
    <button class="see" aria-expanded={open} aria-controls={open ? outId : undefined} onclick={toggle}>
      <span class="chev" aria-hidden="true">{open ? '▾' : '▸'}</span>{t('conv.setup.showOutput')}
    </button>
  </div>
  {#if open}
    <!-- Focusable, as every scrolled region: read from the keyboard too. Not read aloud as it comes (a build writes
         hundreds of lines). -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <pre
      id={outId}
      class="out"
      class:empty={!lines.length}
      role="log"
      aria-live="off"
      aria-label={t('conv.setup.outputLabel', { step: agent.setup ?? '' })}
      tabindex="0"
      bind:this={out}
      onscroll={onScroll}>{lines.length ? lines.join('\n') : t('conv.setup.noOutput')}</pre>
  {/if}
</div>

<style>
  .setup {
    margin-left: 34px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .line {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 10px;
    font-size: 12.5px;
    color: var(--muted);
  }
  .see {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 22px;
    padding: 0 8px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: 11.5px;
    cursor: pointer;
  }
  .see:hover,
  .see[aria-expanded='true'] {
    color: var(--text);
    background: var(--elev);
  }
  .chev {
    font-size: 10px;
  }
  .out {
    margin: 0;
    padding: 10px 12px;
    max-height: 260px;
    overflow: auto;
    border: 1px solid var(--line);
    border-radius: var(--r-sm);
    background: var(--term);
    color: var(--muted);
    font-family: var(--mono);
    font-size: 11.5px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .out:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .out.empty {
    color: var(--dim);
  }
  .dots {
    display: inline-flex;
    gap: 3px;
  }
  .dots span {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--accent);
    animation: ccBlink 1.2s infinite;
  }
  .dots span:nth-child(2) {
    animation-delay: 0.15s;
  }
  .dots span:nth-child(3) {
    animation-delay: 0.3s;
  }
  @media (prefers-reduced-motion: reduce) {
    .dots span {
      animation: none;
      opacity: 0.6;
    }
  }
</style>
