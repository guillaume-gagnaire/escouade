<script lang="ts">
  // A ticket of the phone's Kanban (MO-7, direction B): its key, its title, where it stands (its agent, its loop) and
  // the squares of its criteria. Its gestures (M8) two ways:
  // - **swiped to the left**, the line uncovers them on its right, as the design shows (« Renvoyer », « Valider »);
  // - **touched** (or by the keyboard, or a screen reader: wherever the swipe is impossible), it unfolds them under it
  //   as buttons. Both lead to the same gestures; the lock is asked by the gesture itself (`board.svelte.ts`).
  // « Renvoyer » asks what to change, a word, typed or dictated by the keyboard's mic, then sends it to the agent.
  import { tick } from 'svelte';
  import Icon from '$shared/components/Icon.svelte';
  import { t } from '$shared/lib/i18n';
  import type { IconName } from '$shared/lib/icons';
  import { approve, gesturesOf, kanban, metOf, reject, resolve, start, type BoardTicket, type Gesture } from '../lib/board.svelte';

  let { ticket }: { ticket: BoardTicket } = $props();

  /** The width each gesture takes when the line is swiped. */
  const SLOT = 92;

  const gestures = $derived(gesturesOf(ticket));
  const width = $derived(gestures.length * SLOT);
  const busy = $derived(kanban.busy.includes(ticket.id));
  let mode = $state<'swipe' | 'tap'>('tap');
  const revealed = $derived(kanban.open === ticket.id && mode === 'swipe');
  const unfolded = $derived(kanban.open === ticket.id && mode === 'tap');
  const rejecting = $derived(kanban.rejecting === ticket.id);
  let comment = $state('');
  let field = $state<HTMLTextAreaElement>();

  // The swipe under way: where it started, how far it went, whether it is the line's (sideways) or the page's.
  let from: { x: number; y: number; at: number } | null = null;
  let dx = $state(0);
  let dragging = $state(false);
  let moved = false;
  const offset = $derived(dragging ? dx : revealed ? -width : 0);

  const ICONS: Record<Gesture, IconName> = { approve: 'check', reject: 'undo', start: 'play', resolve: 'sparkle' };

  function down(e: PointerEvent) {
    if (!gestures.length || busy || e.button > 0) return;
    from = { x: e.clientX, y: e.clientY, at: revealed ? -width : 0 };
    moved = false;
  }

  function move(e: PointerEvent) {
    if (!from) return;
    const x = e.clientX - from.x;
    const y = e.clientY - from.y;
    if (!dragging) {
      // The page's scroll: up or down first.
      if (Math.abs(y) > 10 && Math.abs(y) > Math.abs(x)) {
        from = null;
        return;
      }
      if (Math.abs(x) < 8) return;
      dragging = true;
      moved = true;
      (e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId);
    }
    dx = Math.max(-width - 24, Math.min(0, from.at + x));
  }

  function up() {
    if (!from) return;
    from = null;
    if (!dragging) return;
    dragging = false;
    const open = dx < -width * 0.4;
    mode = 'swipe';
    kanban.open = open ? ticket.id : kanban.open === ticket.id ? null : kanban.open;
    if (open) kanban.rejecting = null;
  }

  function toggle() {
    // The click a swipe ends with is not a touch.
    if (moved) {
      moved = false;
      return;
    }
    if (revealed) {
      kanban.open = null;
      return;
    }
    mode = 'tap';
    kanban.open = unfolded ? null : ticket.id;
    if (!unfolded) kanban.rejecting = null;
  }

  async function run(g: Gesture) {
    if (g === 'reject') {
      mode = 'tap';
      kanban.open = ticket.id;
      kanban.rejecting = ticket.id;
      comment = '';
      await tick();
      field?.focus();
      return;
    }
    await (g === 'approve' ? approve(ticket) : g === 'start' ? start(ticket) : resolve(ticket));
  }

  async function send(e: SubmitEvent) {
    e.preventDefault();
    if (await reject(ticket, comment)) comment = '';
  }
</script>

<li class="line" class:dragging data-ticket={ticket.key}>
  <!-- The line and what its swipe uncovers: the gestures unfolded below stay out of it. -->
  <div class="swipe">
    {#if gestures.length}
      <div class="side" style:width="{width}px" aria-hidden={!revealed} inert={!revealed}>
        {#each gestures as g (g)}
          <button type="button" class="sg {g}" disabled={busy} onclick={() => run(g)}>
            <Icon name={ICONS[g]} size={20} />
            <span>{t(`mobile.kanban.${g}`)}</span>
          </button>
        {/each}
      </div>
    {/if}
    <!-- The swipe is a pointer's shortcut: its gestures are buttons of the line too (touched, or by the keyboard). -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="front"
      style:transform="translateX({offset}px)"
      onpointerdown={down}
      onpointermove={move}
      onpointerup={up}
      onpointercancel={up}
    >
      {#if gestures.length}
        <button
          type="button"
          class="main"
          aria-expanded={unfolded}
          aria-controls="gestures-{ticket.id}"
          aria-label={t('mobile.kanban.line', { key: ticket.key, title: ticket.title })}
          onclick={toggle}
        >
          {@render body()}
        </button>
      {:else}
        <div class="main">{@render body()}</div>
      {/if}
    </div>
  </div>
  {#if unfolded}
    <div class="acts" id="gestures-{ticket.id}" role="group" aria-label={t('mobile.kanban.gestures', { key: ticket.key })}>
      {#if rejecting}
        <form class="why" onsubmit={send}>
          <label for="why-{ticket.id}">{t('mobile.kanban.rejectLabel')}</label>
          <textarea
            id="why-{ticket.id}"
            rows="2"
            bind:this={field}
            bind:value={comment}
            aria-describedby="why-hint-{ticket.id}"
            enterkeyhint="send"></textarea>
          <p id="why-hint-{ticket.id}" class="hint">{t('mobile.kanban.rejectHint')}</p>
          <div class="row">
            <button type="button" class="ab" onclick={() => (kanban.rejecting = null)}>{t('mobile.kanban.cancel')}</button>
            <button type="submit" class="ab go" disabled={busy || !comment.trim()}>{t('mobile.kanban.rejectSend')}</button>
          </div>
        </form>
      {:else}
        <div class="row">
          {#each gestures as g (g)}
            <button type="button" class="ab {g}" disabled={busy} onclick={() => run(g)}>
              <Icon name={ICONS[g]} size={18} />{t(`mobile.kanban.${g}`)}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</li>

{#snippet body()}
  <span class="top">
    <span class="key">{ticket.key}</span>
    {#if ticket.criteria.length}
      <span class="sq" role="img" aria-label={t('mobile.kanban.criteria', { count: metOf(ticket), total: ticket.criteria.length })}>
        {#each ticket.criteria.slice(0, 8) as ok, i (i)}<i class:ok></i>{/each}
      </span>
    {/if}
  </span>
  <span class="tt">{ticket.title}</span>
  <span class="mt">
    {#if busy}
      <span class="st">{t('mobile.kanban.busy')}</span>
    {:else if ticket.conflict}
      <Icon name="warn" size={14} /><span class="st warn">{t('mobile.kanban.conflict')}</span>
    {:else if ticket.step}
      <span class="st">{ticket.step}</span>
    {:else if ticket.blocked}
      <Icon name="warn" size={14} /><span class="st warn">{ticket.blocked}</span>
    {:else if ticket.column === 'doing'}
      <i class="run" aria-hidden="true"></i>
    {:else}
      <span class="st">{t(`board.columns.${ticket.column}`)}</span>
    {/if}
    {#if ticket.agent}<span class="s" aria-hidden="true">·</span><span class="ag">{ticket.agent}</span>{/if}
    {#if ticket.iteration > 0 && ticket.column !== 'done'}
      <span class="s" aria-hidden="true">·</span><span>{t('mobile.kanban.loop', { n: ticket.iteration, max: ticket.maxLoops })}</span>
    {/if}
    {#if ticket.partial && ticket.column === 'review'}
      <span class="s" aria-hidden="true">·</span><span class="warn">{t('mobile.kanban.partial')}</span>
    {/if}
  </span>
{/snippet}

<style>
  .swipe {
    position: relative;
    overflow: hidden;
  }
  .line + :global(.line) {
    box-shadow: inset 0 1px 0 var(--line);
  }
  .side {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    display: flex;
  }
  .sg {
    flex: 1;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 4px;
    border: 0;
    background: var(--raise-2);
    color: var(--text);
    font-size: 14px;
    font-weight: 650;
    cursor: pointer;
  }
  .sg.approve,
  .sg.start {
    background: var(--add);
    color: var(--on-add);
  }
  .sg.resolve {
    background: var(--wait);
    color: var(--on-wait);
  }
  .front {
    position: relative;
    background: var(--panel);
    transition: transform 0.22s cubic-bezier(0.2, 0.8, 0.2, 1);
    touch-action: pan-y;
  }
  .dragging .front {
    transition: none;
  }
  .main {
    width: 100%;
    display: grid;
    gap: 3px;
    padding: 12px var(--gut);
    min-height: var(--target);
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    -webkit-user-select: none;
    user-select: none;
  }
  button.main {
    cursor: pointer;
  }
  button.main:active {
    background: var(--hover);
  }
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .key {
    font-size: 14px;
    font-weight: 650;
    color: var(--soft);
    font-variant-numeric: tabular-nums;
  }
  .sq {
    display: inline-flex;
    gap: 3px;
  }
  .sq i {
    width: 9px;
    height: 9px;
    border-radius: 2px;
    background: var(--line-3);
  }
  .sq i.ok {
    background: var(--run);
  }
  .tt {
    font-weight: 620;
    font-size: 16px;
    line-height: 1.3;
    color: var(--text);
    overflow-wrap: anywhere;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .mt {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    font-size: var(--small);
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
  }
  .mt :global(svg) {
    flex: none;
    color: var(--wait);
  }
  .warn {
    color: var(--wait);
  }
  .st {
    flex: none;
  }
  .st.warn {
    flex: 0 1 auto;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ag {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .s {
    opacity: 0.6;
  }
  .run {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--run);
  }
  .acts {
    padding: 0 var(--gut) 14px;
  }
  .row {
    display: flex;
    gap: var(--gap-target);
  }
  .ab {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: var(--target);
    padding: 0 12px;
    border: 0;
    border-radius: var(--r-m);
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font-size: var(--body);
    font-weight: 620;
    cursor: pointer;
  }
  .ab.approve,
  .ab.start,
  .ab.go {
    background: var(--add);
    box-shadow: none;
    color: var(--on-add);
  }
  .ab.resolve {
    background: var(--wait);
    box-shadow: none;
    color: var(--on-wait);
  }
  .ab:disabled,
  .sg:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .why {
    display: grid;
    gap: 8px;
  }
  .why label {
    font-size: var(--small);
    font-weight: 650;
    color: var(--soft);
  }
  .why textarea {
    width: 100%;
    box-sizing: border-box;
    min-height: calc(var(--target) + 16px);
    padding: 10px 12px;
    border: 0;
    border-radius: var(--r-m);
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font: inherit;
    font-size: 16px;
    resize: none;
  }
  .why textarea:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 1px;
  }
  .hint {
    margin: 0;
    font-size: var(--small);
    color: var(--muted);
  }
  @media (prefers-reduced-motion: reduce) {
    .front {
      transition: none;
    }
  }
</style>
