<script lang="ts" generics="T extends { id: string; disabled?: boolean; compact?: boolean }">
  import type { Snippet } from 'svelte';

  // The choices of a station (ESC-504, direction C): one radio group, the recommended choice selected, ↑ ↓ to move
  // (over the choices that cannot be taken), Espace to take the one in focus; Entrée goes on (the journey's). What a
  // choice carries that can be acted on (a command to copy) sits beside its radio, in its box, never in it.
  let {
    items,
    value = $bindable(),
    label,
    head,
    extra,
  }: {
    items: T[];
    value: string | null;
    /** The group's name: the station's question. */
    label: string;
    /** What a choice says, in its radio. */
    head: Snippet<[T, boolean]>;
    /** What it carries beside, in its box (shown under it). */
    extra?: Snippet<[T, boolean]>;
  } = $props();

  let els = $state<HTMLElement[]>([]);
  /** The one Tab reaches: the choice taken, else the first that can be. */
  const tabbable = $derived(items.some((i) => i.id === value) ? value : (items.find((i) => !i.disabled)?.id ?? null));

  function onkey(e: KeyboardEvent, at: number) {
    if (e.key === ' ') {
      e.preventDefault();
      if (!items[at].disabled) value = items[at].id;
      return;
    }
    if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
    e.preventDefault();
    e.stopPropagation();
    const dir = e.key === 'ArrowDown' ? 1 : -1;
    const n = items.length;
    for (let k = 1; k <= n; k++) {
      const j = (at + dir * k + n) % n;
      if (!items[j].disabled) {
        value = items[j].id;
        els[j]?.focus();
        return;
      }
    }
  }
</script>

<div class="choices" role="radiogroup" aria-label={label}>
  {#each items as it, i (it.id)}
    {@const on = value === it.id}
    <div class="choice" class:on class:dis={it.disabled} class:compact={it.compact && !on}>
      <div
        class="pick"
        role="radio"
        aria-checked={on}
        aria-disabled={it.disabled || undefined}
        tabindex={tabbable === it.id ? 0 : -1}
        data-choice={it.id}
        bind:this={els[i]}
        onclick={() => !it.disabled && (value = it.id)}
        onkeydown={(e) => onkey(e, i)}
      >
        <span class="rd" aria-hidden="true"></span>
        <span class="body">{@render head(it, on)}</span>
      </div>
      {#if extra}{@render extra(it, on)}{/if}
    </div>
  {/each}
</div>

<style>
  .choices {
    display: grid;
    gap: 8px;
  }
  .choice {
    display: grid;
    border-radius: 10px;
    background: rgba(var(--fg-rgb), 0.025);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .choice.on {
    background: color-mix(in oklch, var(--brand) 7%, transparent);
    box-shadow: inset 0 0 0 1.5px var(--brand);
  }
  .choice.dis {
    opacity: 0.62;
  }
  .pick {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr);
    gap: 12px;
    align-items: start;
    padding: 12px 14px;
    border-radius: 10px;
    cursor: pointer;
    outline: none;
  }
  /* A choice that says little (a harness not found): a slimmer row, so the ones that matter stay in view. */
  .choice.compact .pick {
    padding-top: 7px;
    padding-bottom: 7px;
  }
  .choice.dis .pick {
    cursor: default;
  }
  .pick:focus-visible {
    box-shadow: 0 0 0 2px var(--p-ink);
  }
  .rd {
    width: 16px;
    height: 16px;
    margin-top: 1px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1.5px var(--line-3);
  }
  .choice.on .rd {
    box-shadow: inset 0 0 0 5px var(--brand);
  }
  .body {
    display: grid;
    gap: 4px;
    min-width: 0;
  }
  @media (max-width: 1100px) {
    .pick {
      padding: 9px 12px;
    }
  }
</style>
