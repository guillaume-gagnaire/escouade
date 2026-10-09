<script lang="ts">
  import { onMount } from 'svelte';
  import type { NavTarget } from '../../lib/editor/goto';

  /** More places than that are not listed. */
  const MAX = 50;

  // Under the word followed (`at`, its box on the screen; null when it is not drawn): the places it may lead to.
  let {
    targets,
    label,
    at,
    onpick,
    onclose,
  }: {
    targets: NavTarget[];
    /** What is looked for, to title the list. */
    label: string;
    at: { left: number; top: number; bottom: number } | null;
    onpick: (t: NavTarget) => void;
    onclose: () => void;
  } = $props();

  const uid = $props.id();
  const shown = $derived(targets.slice(0, MAX));
  let sel = $state(0);
  let box = $state<HTMLDivElement>();
  let list = $state<HTMLUListElement>();
  let pos = $state<{ x: number; y: number } | null>(null);
  /** Where the focus was: Escape gives it back (the editor), as if nothing had been followed. */
  const before = document.activeElement as HTMLElement | null;

  onMount(() => list?.focus());

  // Under the word, above it when there is no room below, always in the window.
  $effect(() => {
    if (!box) return;
    const r = box.getBoundingClientRect();
    const [w, h] = [window.innerWidth, window.innerHeight];
    if (!at) {
      pos = { x: Math.max(8, (w - r.width) / 2), y: Math.round(h / 5) };
      return;
    }
    const below = at.bottom + 4;
    pos = {
      x: Math.max(8, Math.min(at.left, w - r.width - 8)),
      y: below + r.height > h - 8 ? Math.max(8, at.top - r.height - 4) : below,
    };
  });

  $effect(() => {
    void sel;
    list?.querySelector('[aria-selected="true"]')?.scrollIntoView?.({ block: 'nearest' });
  });

  function key(e: KeyboardEvent) {
    const n = shown.length;
    if (e.key === 'ArrowDown') sel = (sel + 1) % n;
    else if (e.key === 'ArrowUp') sel = (sel - 1 + n) % n;
    else if (e.key === 'Enter' && shown[sel]) onpick(shown[sel]);
    else if (e.key === 'Escape') {
      before?.focus();
      onclose();
    } else return;
    e.preventDefault();
    e.stopPropagation();
  }
</script>

<svelte:window onpointerdown={(e) => box && !box.contains(e.target as Node) && onclose()} />

<div class="picker" bind:this={box} style:left="{pos?.x ?? at?.left ?? 8}px" style:top="{pos?.y ?? (at ? at.bottom + 4 : 8)}px">
  <div class="title" id="{uid}-title">Définitions de « {label} »</div>
  <ul
    class="list"
    role="listbox"
    tabindex="0"
    aria-labelledby="{uid}-title"
    aria-activedescendant="{uid}-{sel}"
    bind:this={list}
    onkeydown={key}
  >
    {#each shown as t, i (i)}
      <!-- The list takes the keys; the mouse keeps the focus on it. -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <li
        id="{uid}-{i}"
        class="opt"
        class:on={i === sel}
        role="option"
        aria-selected={i === sel}
        onmouseenter={() => (sel = i)}
        onmousedown={(e) => e.preventDefault()}
        onclick={() => onpick(t)}
      >
        <span class="where mono">{t.path}{t.line ? `:${t.line}` : ''}</span>
        <span class="text mono">{t.text?.trim() ?? ''}</span>
      </li>
    {/each}
  </ul>
</div>

<style>
  .picker {
    position: fixed;
    z-index: 40;
    width: min(560px, calc(100vw - 16px));
    display: flex;
    flex-direction: column;
    padding: 5px;
    border-radius: var(--r);
    border: 1px solid var(--line2);
    background: var(--elev);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.45);
    animation: ccFadeIn 0.08s ease-out;
  }
  .title {
    padding: 6px 10px 4px;
    font-size: 11px;
    color: var(--dim);
  }
  .list {
    max-height: 340px;
    margin: 0;
    padding: 0;
    overflow: auto;
    list-style: none;
    outline: none;
  }
  .opt {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px 10px;
    border-radius: var(--r-sm);
    cursor: pointer;
  }
  .opt.on {
    background: var(--elev2);
  }
  /* Where the keys are: the list has the focus. */
  .list:focus .opt.on {
    box-shadow: inset 2px 0 0 var(--accent);
  }
  .where,
  .text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .where {
    font-size: 12px;
    color: var(--text);
  }
  .text {
    font-size: 11px;
    color: var(--muted);
  }
  @media (prefers-reduced-motion: reduce) {
    .picker {
      animation: none;
    }
  }
</style>
