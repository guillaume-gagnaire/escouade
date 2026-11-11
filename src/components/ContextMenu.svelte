<script lang="ts">
  import { tick } from 'svelte';
  import { t } from '../lib/i18n';
  import { beside, EDGE, menu, sideOf, type MenuItem, type Side } from '../lib/menu.svelte';
  import { tip } from '../lib/tip';
  import EngineMark from './EngineMark.svelte';
  import Icon from './Icon.svelte';

  /** How long the pointer rests on an entry before its submenu opens, or the one open closes. */
  const HOVER_MS = 150;

  /** Both menus, for the focus that goes back once they close. */
  let box = $state<HTMLDivElement>();
  let el = $state<HTMLDivElement>();
  let subEl = $state<HTMLDivElement>();
  let pos = $state({ x: 0, y: 0 });
  let subPos = $state({ x: 0, y: 0 });
  /** The entry whose submenu is open, by its index. */
  let openSub = $state<number | null>(null);
  /** Where the submenus of the menu open, their chevrons on that side. */
  let side = $state<Side>('right');
  let hoverTimer: ReturnType<typeof setTimeout> | undefined;
  type Edges = { up: boolean; down: boolean };
  /** Entries out of view, above and below, in the menu and in its submenu. */
  let edges = $state<Edges>({ up: false, down: false });
  let subEdges = $state<Edges>({ up: false, down: false });

  const sub = $derived(openSub === null ? null : (menu.open?.items[openSub] ?? null));

  const measure = (list: Element | null | undefined): Edges =>
    list ? { up: list.scrollTop > 0, down: list.scrollTop + list.clientHeight < list.scrollHeight - 1 } : { up: false, down: false };

  $effect(() => {
    const m = menu.open;
    // A new menu, or none: no submenu of the one before.
    openSub = null;
    clearTimeout(hoverTimer);
    if (!m || !el) return;
    const r = el.getBoundingClientRect();
    const x = Math.max(EDGE, Math.min(m.x, window.innerWidth - r.width - EDGE));
    // Never taller than the window (its CSS): above where it was asked when it has no room below.
    pos = { x, y: m.y + r.height > window.innerHeight - EDGE ? Math.max(EDGE, m.y - r.height) : m.y };
    // Not read back from `pos`: this effect would depend on what it writes.
    side = sideOf({ left: x, right: x + r.width }, window.innerWidth);
    edges = measure(el.querySelector('.items'));
  });

  $effect(() => {
    const i = openSub;
    if (i === null || !el || !subEl) return;
    const entry = el.querySelector(`[data-i="${i}"]`);
    if (!entry) return;
    const r = subEl.getBoundingClientRect();
    subPos = beside(
      entry.getBoundingClientRect(),
      el.getBoundingClientRect(),
      { w: r.width, h: r.height },
      { w: window.innerWidth, h: window.innerHeight },
      side,
    );
    subEdges = measure(subEl.querySelector('.items'));
  });

  /** What the keys reach in a menu: not the entries that are disabled. */
  const reachable = (panel: Element | null | undefined) => [
    ...(panel?.querySelectorAll<HTMLElement>('[role^="menuitem"]:not(:disabled)') ?? []),
  ];

  // A menu opened from an element takes the focus, so that the keyboard can work it, and gives it back when it closes.
  $effect(() => {
    const [menuEl, both] = [el, box];
    if (!menu.open?.keyboard || !menuEl || !both) return;
    const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    reachable(menuEl)[0]?.focus();
    return () => {
      // Not when an entry opened a dialog meanwhile: the focus is then the dialog's.
      const now = document.activeElement;
      if ((!now || now === document.body || both.contains(now)) && opener?.isConnected) opener.focus();
    };
  });

  /** The pointer on the entry `i` of the menu: after a while, its submenu opens, or the one open closes. */
  function hover(i: number) {
    clearTimeout(hoverTimer);
    if (openSub === i) return;
    hoverTimer = setTimeout(() => {
      const item = menu.open?.items[i];
      openSub = item?.submenu && !item.disabled ? i : null;
    }, HOVER_MS);
  }

  /** From the keyboard: the submenu of the entry `i`, the focus on its first entry. */
  async function enterSub(i: number) {
    clearTimeout(hoverTimer);
    openSub = i;
    await tick();
    reachable(subEl)[0]?.focus();
  }

  /** Closes the submenu; the focus in it goes back to its entry. */
  function closeSub() {
    const i = openSub;
    const focused = !!subEl?.contains(document.activeElement);
    clearTimeout(hoverTimer);
    openSub = null;
    if (focused) el?.querySelector<HTMLElement>(`[data-i="${i}"]`)?.focus();
  }

  /** Escape closes the submenu first, then the menu. */
  const escape = () => (openSub === null ? menu.close() : closeSub());

  function scrolled(list: HTMLElement, inSub: boolean) {
    if (inSub) {
      subEdges = measure(list);
      return;
    }
    edges = measure(list);
    // As on macOS: the submenu does not follow its entry.
    if (openSub !== null) closeSub();
  }

  function onKeydown(e: KeyboardEvent) {
    // Its own Escape: a dialog it was opened from (the branch of a ticket) stays, as the window's listeners would close it.
    if (e.key === 'Escape') {
      e.stopPropagation();
      escape();
      return;
    }
    if (e.key === 'Tab') {
      e.preventDefault();
      menu.close();
      return;
    }
    const focused = document.activeElement as HTMLElement | null;
    const inSub = !!subEl?.contains(focused);
    if (inSub && e.key === 'ArrowLeft') {
      e.preventDefault();
      closeSub();
      return;
    }
    if (!inSub && (e.key === 'ArrowRight' || e.key === 'Enter' || e.key === ' ')) {
      const i = Number(focused?.dataset.i);
      const item = menu.open?.items[i];
      if (item?.submenu && !item.disabled) {
        e.preventDefault();
        void enterSub(i);
        return;
      }
    }
    const panel = inSub ? subEl : el;
    const list = reachable(panel);
    const at = list.indexOf(focused as HTMLElement);
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
        const hit = /^[0-9]$/.test(e.key) ? panel?.querySelector<HTMLButtonElement>(`[data-key="${e.key}"]:not(:disabled)`) : null;
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

<svelte:window onkeydown={(e) => e.key === 'Escape' && menu.open && (escape(), e.stopPropagation())} onblur={() => menu.close()} />

<!-- The entries of a menu, in a list that scrolls when the window is too short for it, its edges faded and marked. -->
{#snippet body(items: MenuItem[], inSub: boolean, more: Edges)}
  <div class="items" class:more-up={more.up} class:more-down={more.down} onscroll={(e) => scrolled(e.currentTarget, inSub)}>
    {#each items as item, i (i)}
      {@const parent = !inSub && !!item.submenu}
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
          aria-haspopup={parent ? 'menu' : undefined}
          aria-expanded={parent ? openSub === i : undefined}
          class:danger={item.danger}
          class:iconed={!!item.icon || !!item.swatch}
          class:marked={!!item.engine}
          class:open={parent && openSub === i}
          disabled={item.disabled}
          data-key={item.key}
          data-i={inSub ? undefined : i}
          use:tip={item.title}
          onmouseenter={inSub ? undefined : () => hover(i)}
          onmouseleave={inSub ? undefined : () => clearTimeout(hoverTimer)}
          onclick={() => {
            // Its submenu opens on hovering it, never on a click.
            if (parent) return;
            menu.close();
            item.onClick?.();
          }}
        >
          {#if parent && side === 'left'}<span class="chev left"><Icon name="left" size={12} /></span>{/if}
          {#if item.engine}<span class="mk"><EngineMark engine={item.engine} always /></span>{:else if item.swatch}<span
              class="ic tile"
              style:background={item.swatch}
            ></span>{:else if item.icon}<span class="ic"><Icon name={item.icon} /></span>{/if}
          <span class="ml"
            ><span class="l">{item.label}</span>{#if item.detail}<span class="reason">{item.detail}</span
              >{/if}{#if item.disabled && item.reason}<span class="reason">{item.reason}</span>{/if}</span
          >
          {#if parent && side === 'right'}<span class="chev"><Icon name="right" size={12} /></span>{:else if item.key}<kbd class="kbd"
              >{item.key}</kbd
            >{:else if item.hint}<span class="hint">{item.hint}</span>{/if}
        </button>
      {/if}
    {/each}
  </div>
  {#if more.up}<span class="edge up" aria-hidden="true"><Icon name="down" size={12} /></span>{/if}
  {#if more.down}<span class="edge down" aria-hidden="true"><Icon name="down" size={12} /></span>{/if}
{/snippet}

{#if menu.open}
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="backdrop" onclick={() => menu.close()} oncontextmenu={(e) => (e.preventDefault(), menu.close())}></div>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="menus" bind:this={box} onkeydown={onKeydown}>
    <div
      class="menu"
      bind:this={el}
      style:left="{pos.x || menu.open.x}px"
      style:top="{pos.y || menu.open.y}px"
      role="menu"
      tabindex="-1"
      oncontextmenu={(e) => e.preventDefault()}
    >
      {@render body(menu.open.items, false, edges)}
    </div>
    {#if sub?.submenu}
      <div
        class="menu"
        bind:this={subEl}
        style:left="{subPos.x}px"
        style:top="{subPos.y}px"
        role="menu"
        tabindex="-1"
        aria-label={sub.label}
        onmouseenter={() => clearTimeout(hoverTimer)}
        oncontextmenu={(e) => e.preventDefault()}
      >
        {@render body(sub.submenu, true, subEdges)}
      </div>
    {/if}
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 90;
  }
  .menus {
    display: contents;
  }
  /* Never taller than the window, 8 px from its edges: its list scrolls instead. */
  .menu {
    position: fixed;
    z-index: 91;
    /* As wide as its entries wherever it is first laid: near the window's right edge, it would shrink to the room left
       there, and be measured, then placed, at that width. */
    width: max-content;
    min-width: 200px;
    /* MENU_MAX_W, which tells the side of the submenus. */
    max-width: 380px;
    max-height: calc(100vh - 16px);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-radius: 11px;
    background: var(--raise-2);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 18px 48px -10px rgba(0, 0, 0, calc(0.8 * var(--shade)));
    animation: ccFadeIn 0.2s var(--ease);
  }
  .items {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 5px;
    overflow-y: auto;
    overscroll-behavior: contain;
    /* The entry the keys bring into view comes out of the fade. */
    scroll-padding-block: 28px;
  }
  /* The edges with more to see fade out, as the projects' rail does, a chevron over them. */
  .items.more-down {
    mask-image: linear-gradient(to bottom, #000 calc(100% - 28px), transparent);
  }
  .items.more-up {
    mask-image: linear-gradient(to bottom, transparent, #000 28px);
  }
  .items.more-up.more-down {
    mask-image: linear-gradient(to bottom, transparent, #000 28px, #000 calc(100% - 28px), transparent);
  }
  .edge {
    position: absolute;
    left: 50%;
    display: inline-flex;
    color: var(--muted);
    pointer-events: none;
    transform: translateX(-50%);
  }
  .edge.up {
    top: 4px;
    transform: translateX(-50%) rotate(180deg);
  }
  .edge.down {
    bottom: 4px;
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
  button:focus-visible,
  button.open {
    background: rgba(var(--fg-rgb), 0.06);
  }
  /* An entry that opens a submenu: its chevron on the side the submenu opens. */
  .chev {
    display: inline-flex;
    flex: none;
    margin-right: -3px;
    color: var(--muted);
  }
  .chev.left {
    margin-right: 0;
    margin-left: -3px;
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
