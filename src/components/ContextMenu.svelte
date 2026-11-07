<script lang="ts">
  import { t } from '../lib/i18n';
  import { menu } from '../lib/menu.svelte';
  import { tip } from '../lib/tip';
  import EngineMark from './EngineMark.svelte';
  import Icon from './Icon.svelte';

  let el = $state<HTMLDivElement>();
  let pos = $state({ x: 0, y: 0 });

  $effect(() => {
    const m = menu.open;
    if (!m || !el) return;
    const r = el.getBoundingClientRect();
    pos = {
      x: Math.min(m.x, window.innerWidth - r.width - 8),
      y: m.y + r.height > window.innerHeight - 8 ? Math.max(8, m.y - r.height) : m.y,
    };
  });

  /** What the keys reach: not the entries that are disabled. */
  const reachable = () => [...(el?.querySelectorAll<HTMLElement>('[role^="menuitem"]:not(:disabled)') ?? [])];

  // A menu opened from an element takes the focus, so that the keyboard can work it, and gives it back when it closes.
  $effect(() => {
    const menuEl = el;
    if (!menu.open?.keyboard || !menuEl) return;
    const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    reachable()[0]?.focus();
    return () => {
      // Not when an entry opened a dialog meanwhile: the focus is then the dialog's.
      const now = document.activeElement;
      if ((!now || now === document.body || menuEl.contains(now)) && opener?.isConnected) opener.focus();
    };
  });

  function onKeydown(e: KeyboardEvent) {
    // Its own Escape: a dialog it was opened from (the branch of a ticket) stays, as the window's listeners would close it.
    if (e.key === 'Escape') {
      e.stopPropagation();
      menu.close();
      return;
    }
    if (e.key === 'Tab') {
      e.preventDefault();
      menu.close();
      return;
    }
    const list = reachable();
    const at = list.indexOf(document.activeElement as HTMLElement);
    const last = list.length - 1;
    let to: number;
    switch (e.key) {
      case 'ArrowDown':
        to = at >= last ? 0 : at + 1;
        break;
      case 'ArrowUp':
        to = at <= 0 ? last : at - 1;
        break;
      case 'Home':
        to = 0;
        break;
      case 'End':
        to = last;
        break;
      default: {
        // A numbered entry (the menu « Harness »): its key picks it.
        const hit = /^[0-9]$/.test(e.key) ? el?.querySelector<HTMLButtonElement>(`[data-key="${e.key}"]:not(:disabled)`) : null;
        if (hit) {
          e.preventDefault();
          hit.click();
        }
        return;
      }
    }
    e.preventDefault();
    list[to]?.focus();
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && menu.open && (menu.close(), e.stopPropagation())} onblur={() => menu.close()} />

{#if menu.open}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={() => menu.close()} oncontextmenu={(e) => (e.preventDefault(), menu.close())}></div>
  <div
    class="menu"
    bind:this={el}
    style:left="{pos.x || menu.open.x}px"
    style:top="{pos.y || menu.open.y}px"
    role="menu"
    tabindex="-1"
    onkeydown={onKeydown}
    oncontextmenu={(e) => e.preventDefault()}
  >
    {#each menu.open.items as item, i (i)}
      {#if item.separator}
        <div class="sep"></div>
      {:else if item.heading}
        <div class="heading" role="presentation">
          <span>{item.label}</span>{#if item.hint}<kbd class="kbd">{item.hint}</kbd>{/if}
        </div>
      {:else if item.colors}
        {@const colors = item.colors}
        <div class="colors" role="group" aria-label={item.label}>
          <span class="clabel">{item.label}</span>
          <div class="swatches">
            {#each colors.values as c, ci (c)}
              <button
                class="swatch"
                class:on={c === colors.selected}
                role="menuitemradio"
                aria-checked={c === colors.selected}
                aria-label={t('nav.menu.swatch', { label: item.label, n: ci + 1 })}
                style:background={c}
                onclick={() => {
                  menu.close();
                  colors.onPick(c);
                }}
              ></button>
            {/each}
          </div>
        </div>
      {:else}
        <button
          role={item.checked === undefined ? 'menuitem' : 'menuitemradio'}
          aria-checked={item.checked}
          aria-label={item.engine ? item.label : undefined}
          class:danger={item.danger}
          class:iconed={!!item.icon || !!item.swatch}
          class:marked={!!item.engine}
          disabled={item.disabled}
          data-key={item.key}
          use:tip={item.title}
          onclick={() => {
            menu.close();
            item.onClick?.();
          }}
        >
          {#if item.engine}<span class="mk"><EngineMark engine={item.engine} always /></span>{:else if item.swatch}<span
              class="ic tile"
              style:background={item.swatch}
            ></span>{:else if item.icon}<span class="ic"><Icon name={item.icon} /></span>{/if}
          <span class="ml"
            ><span class="l">{item.label}</span>{#if item.detail}<span class="reason">{item.detail}</span
              >{/if}{#if item.disabled && item.reason}<span class="reason">{item.reason}</span>{/if}</span
          >
          {#if item.key}<kbd class="kbd">{item.key}</kbd>{:else if item.hint}<span class="hint">{item.hint}</span>{/if}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 90;
  }
  .menu {
    position: fixed;
    z-index: 91;
    min-width: 200px;
    max-width: 380px;
    display: flex;
    flex-direction: column;
    padding: 5px;
    border-radius: 11px;
    background: var(--raise-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 18px 48px -10px rgba(0, 0, 0, calc(0.8 * var(--shade)));
    animation: ccFadeIn 0.2s var(--ease);
  }
  button {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 34px;
    padding: 6px 9px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--text);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  button:hover:not(:disabled),
  button:focus-visible {
    background: rgba(var(--fg-rgb), 0.06);
  }
  /* What cannot be done stays legible: its label greyed, its reason in full (choice 17). */
  button:disabled {
    cursor: default;
  }
  button:disabled .l,
  button:disabled .ic {
    opacity: 0.55;
  }
  .ic {
    display: inline-flex;
    flex: none;
    color: var(--muted);
  }
  /* A closed project's colour, a small tile. */
  .ic.tile {
    width: 14px;
    height: 14px;
    margin: 0 1px;
    border-radius: 4px;
  }
  .ml {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .l {
    font-weight: 550;
  }
  .reason {
    color: var(--muted);
    font-size: 12px;
    line-height: 1.4;
  }
  .danger,
  .danger .ic {
    color: var(--del);
  }
  .hint {
    flex: none;
    font-size: 12px;
    color: var(--muted);
  }
  /* A title over the entries that follow (« Harness », « Worktree »), its shortcut in a key. */
  .heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 7px 9px 5px;
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 600;
  }
  /* A harness's brand, in a column of its own: the names line up. */
  .mk {
    display: inline-flex;
    flex: none;
    width: 62px;
  }
  button[aria-checked='true'] .l::after {
    content: '';
    display: inline-block;
    width: 6px;
    height: 6px;
    margin-left: 7px;
    border-radius: 50%;
    background: var(--p-ink);
    vertical-align: 2px;
  }
  .marked .l {
    font-weight: 550;
  }
  .sep {
    height: 1px;
    margin: 5px 4px;
    background: var(--line);
  }
  .colors {
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 7px 9px 8px;
  }
  .clabel {
    font-size: 13px;
  }
  .swatches {
    display: grid;
    grid-template-columns: repeat(7, 16px);
    gap: 7px;
  }
  .swatches .swatch {
    width: 16px;
    height: 16px;
    min-height: 0;
    padding: 0;
    border-radius: 5px;
  }
  .swatches .swatch:hover {
    outline: 1px solid var(--line-3);
    outline-offset: 1px;
  }
  .swatches .swatch.on {
    box-shadow:
      0 0 0 2px var(--raise-2),
      0 0 0 3px var(--text);
  }
</style>
