<script lang="ts">
  import { t } from '../lib/i18n';
  import { menu } from '../lib/menu.svelte';

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
      default:
        return;
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
          role="menuitem"
          class:danger={item.danger}
          disabled={item.disabled}
          title={item.title}
          onclick={() => {
            menu.close();
            item.onClick?.();
          }}
        >
          <span>{item.label}</span>
          {#if item.hint}<span class="hint">{item.hint}</span>{/if}
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
    min-width: 190px;
    display: flex;
    flex-direction: column;
    padding: 5px;
    border-radius: var(--r);
    border: 1px solid var(--line2);
    background: var(--elev);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.45);
    animation: ccFadeIn 0.08s ease-out;
  }
  button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    height: 32px;
    padding: 0 10px;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    background: var(--elev2);
  }
  button:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .danger {
    color: var(--del);
  }
  .hint {
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--dim);
  }
  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--line);
  }
  .colors {
    display: flex;
    flex-direction: column;
    gap: 7px;
    padding: 7px 10px 8px;
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
    padding: 0;
    border-radius: 5px;
  }
  .swatches .swatch:hover {
    outline: 1px solid var(--line2);
    outline-offset: 1px;
  }
  .swatches .swatch.on {
    box-shadow:
      0 0 0 2px var(--elev),
      0 0 0 3px var(--text);
  }
</style>
