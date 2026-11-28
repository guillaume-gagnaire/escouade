<script lang="ts">
  // A gesture of the agent's on a row of « Navigateurs » (`drive_browser`; ESC-680, AB-8; docs/AGENT-BROWSERS.md § 4,
  // docs/AGENT-PAGES.md § 12, direction A « Les repères »), in place of its call's row: its number, the same ③ as the
  // band's caption and the pin on an agent's page's rail, and what it did (« Met en évidence « Offre Pro » »). Its
  // gestures follow each other under the agent's message, as the rows of its tools. A click shows the browser and, on
  // an agent's page, takes the page back where the agent had put it. A failed call keeps its row of a tool (it says why).
  import { devpages, showPage } from '../../lib/devpage.svelte';
  import { driveCall, revisitPin, splitWhat } from '../../lib/drives.svelte';
  import { t } from '../../lib/i18n';
  import { tip } from '../../lib/tip';
  import type { ToolItem } from '../../lib/types';

  let { item, linked = false }: { item: ToolItem; linked?: boolean } = $props();

  const call = $derived(driveCall(item));
  const parts = $derived(call?.what ? splitWhat(call.what) : null);
  /** The row is still there to show. */
  const there = $derived(!!call && !!devpages.pages[call.browser]);
  const isPage = $derived(!!call?.browser.startsWith('page:'));

  function open() {
    if (!call || !there) return;
    showPage(call.browser);
    if (isPage && call.step !== null) void revisitPin(call.browser, call.step);
  }
</script>

{#if call}
  <button
    class="gesture"
    class:linked
    class:running={call.step === null}
    class:gone={!there}
    disabled={!there}
    data-testid="drive-row"
    data-step={call.step ?? undefined}
    use:tip={{
      text: there ? (isPage ? t('devpage.drive.rowPageTip') : t('devpage.drive.rowTip')) : t('devpage.drive.rowGone'),
      aria: false,
    }}
    onclick={open}
  >
    <span class="pin" aria-hidden="true">{call.step ?? ''}</span>
    {#if call.step === null}
      <span class="what">{t('devpage.drive.running')}</span>
    {:else if parts}
      <span class="what"
        ><span class="sr">{t('devpage.drive.stepSr', { n: call.step })}</span><b>{parts.verb}</b
        >{#if parts.rest}{' '}{parts.rest}{/if}</span
      >
    {/if}
  </button>
{/if}

<style>
  /* A line of the turn's thread, as a tool's row: 30 px, the soft ink; its number in the project's tint. */
  .gesture {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    min-height: 30px;
    min-width: 0;
    padding: 0 8px 0 4px;
    border: 0;
    border-radius: 7px;
    background: none;
    color: var(--soft);
    font: inherit;
    font-size: 12.5px;
    line-height: 1.62;
    text-align: left;
    cursor: pointer;
  }
  .gesture:hover:not(:disabled) {
    background: var(--hover);
  }
  .gesture:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: -2px;
  }
  .gesture:disabled {
    cursor: default;
  }
  /* The number: the same ③ as the band and the rail (direction A). */
  .pin {
    flex: none;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 11px;
    background: color-mix(in oklch, var(--p) 20%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--p) 38%, transparent);
    color: var(--p-ink);
    font-size: 11px;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
  }
  .running .pin {
    background: none;
    box-shadow: inset 0 0 0 1px var(--line-2);
    animation: gesture-wait 1.2s var(--ease) infinite;
  }
  .gone .pin {
    background: rgba(var(--fg-rgb), 0.05);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--muted);
  }
  /* The thread from this gesture's number to the next one's. */
  .linked .pin::after {
    content: '';
    position: absolute;
    left: 14.5px;
    top: 26px;
    width: 1px;
    height: 12px;
    background: var(--line-2);
  }
  .what {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .what b {
    color: var(--text);
    font-weight: 600;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  @keyframes gesture-wait {
    50% {
      box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--p) 50%, transparent);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .running .pin {
      animation: none;
    }
  }
</style>
