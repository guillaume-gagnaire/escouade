<script lang="ts">
  import { t } from '../lib/i18n';
  import { app } from '../lib/state.svelte';
  import { tip } from '../lib/tip';
  import {
    layout,
    scopeTo,
    type DgAgent,
    type DgEntry,
    type DgLayout,
    type DgModel,
    type DgPass,
    type DgProgress,
    type Measure,
  } from '../lib/workflow-diagram';
  import Icon from './Icon.svelte';

  // The Diagram view of the Workflow tab (ESC-245, choices 196 to 212; `#loops-diagram*` of the preview), loaded only when
  // it opens: the workflow drawn down the column, and enlarged over the whole work panel (`big`), across when it fits.
  // Every name, command and condition comes from the repository's files: drawn as text, never as HTML.
  // It always starts from an entry (ESC-406, direction A of design/workflow-path): the door the agent read came in by
  // (`entry`), else one of `entries` to choose from. Only what that entry reaches is drawn, light and dashed as what may
  // happen; the agent's `path` is traced over it in full lines, each passage numbered on its step, and said in words in
  // an ordered list for a screen reader. The steps the entry does not reach fold under the diagram.
  let {
    model,
    progress,
    entries = [],
    entry = null,
    path = [],
    reader = '',
    agents,
    big = false,
    onclose,
    oncopy,
  }: {
    model: DgModel;
    progress: DgProgress;
    entries?: DgEntry[];
    entry?: DgEntry | null;
    path?: DgPass[];
    /** The name of the agent read, for the list of its path. */
    reader?: string;
    agents: Record<string, DgAgent[]>;
    big?: boolean;
    onclose?: () => void;
    oncopy?: () => void;
  } = $props();

  // The widths of the texts, from the fonts of the app (a canvas); without one (the tests), an average width.
  let ctx: CanvasRenderingContext2D | null | undefined;
  const fonts = () => {
    const css = getComputedStyle(document.documentElement);
    const sans = css.getPropertyValue('--font').trim() || 'Inter, system-ui, sans-serif';
    const mono = css.getPropertyValue('--mono').trim() || 'ui-monospace, monospace';
    return {
      name: `600 12.5px ${sans}`,
      chip: `500 10.5px ${sans}`,
      exit: `600 12px ${sans}`,
      cmd: `500 10.5px ${mono}`,
      door: `500 11px ${mono}`,
    } as const;
  };
  let font: ReturnType<typeof fonts> | null = null;
  const measure: Measure = (text, f) => {
    if (ctx === undefined) {
      try {
        // jsdom (the tests) has no canvas, and says so on every call.
        ctx = /jsdom/.test(navigator.userAgent) ? null : document.createElement('canvas').getContext('2d');
      } catch {
        ctx = null;
      }
    }
    if (!ctx) return text.length * (f === 'name' ? 7 : 6.4);
    font ??= fonts();
    ctx.font = font[f];
    return ctx.measureText(text).width;
  };
  const rounds = (n: number) => `×${n}`;

  // ---------- from an entry (ESC-406) ----------
  /** The door chosen when no agent is read: the first until another is picked. */
  let picked = $state<string | null>(null);
  const chosen = $derived(entry ?? entries.find((e) => e.door === picked) ?? entries[0] ?? null);
  // Where the agent went stays in its diagram, even off the paths of its door.
  const passed = $derived(entry ? [...path.map((p) => p.step), ...(progress.on ? [progress.on] : [])] : []);
  const scope = $derived(chosen ? scopeTo(model, chosen, passed) : { model, off: [] });
  const scoped = $derived(scope.model);
  /** « n étapes hors de ce chemin », opened. */
  let open = $state(false);
  const choosing = $derived(!entry && entries.length > 1);
  const words = $derived({ entryLabel: t('workflow.dg.entry'), tags: { next: t('workflow.dg.keyNext'), on: t('workflow.dg.onTag') } });
  function pickKey(e: KeyboardEvent, i: number) {
    const d = e.key === 'ArrowRight' || e.key === 'ArrowDown' ? 1 : e.key === 'ArrowLeft' || e.key === 'ArrowUp' ? -1 : 0;
    if (!d) return;
    e.preventDefault();
    const to = entries[(i + d + entries.length) % entries.length];
    picked = to.door;
    const group = (e.currentTarget as HTMLElement).parentElement;
    queueMicrotask(() => group?.querySelector<HTMLElement>('[aria-checked="true"]')?.focus());
  }
  /** One passage in words: where, how it came there and why, which stay, whether the agent is there. */
  function say(p: DgPass): string {
    const from = model.steps.find((s) => s.id === p.from)?.name ?? '';
    let out =
      p.via === 'entry'
        ? p.door
          ? t('workflow.dg.passEntry', { name: p.name, door: p.door })
          : t('workflow.dg.passGo', { name: p.name })
        : p.via === 'back'
          ? p.reason
            ? t('workflow.dg.passBack', { name: p.name, from, reason: p.reason })
            : t('workflow.dg.passBackPlain', { name: p.name, from })
          : p.via === 'loop'
            ? p.reason
              ? t('workflow.dg.passLoop', { name: p.name, reason: p.reason })
              : t('workflow.dg.passLoopPlain', { name: p.name })
            : t('workflow.dg.passGo', { name: p.name });
    if (p.pass > 1) out = t('workflow.dg.passNth', { say: out, n: p.pass });
    return p.on ? t('workflow.dg.passOn', { say: out }) : out;
  }
  const selfSays = (s: NonNullable<DgLayout['nodes'][number]['self']>) =>
    s.turns ? (s.max ? `${s.turns}/${s.max}` : `×${s.turns}`) : s.max ? rounds(s.max) : '';

  // ---------- in the column: at the width of the tab ----------
  let wrap = $state<HTMLElement>();
  let width = $state(312);
  $effect(() => {
    const el = wrap;
    if (!el) return;
    const ro = new ResizeObserver(() => (width = Math.max(Math.round(el.clientWidth), 200)));
    ro.observe(el);
    width = Math.max(Math.round(el.clientWidth), 200);
    return () => ro.disconnect();
  });
  const narrow = $derived(layout(scoped, { width, progress, agents, measure, maxRounds: rounds, ...words }));

  // ---------- enlarged: over the work panel, zoom and scroll ----------
  let canvas = $state<HTMLElement>();
  let room = $state({ w: 0, h: 0 });
  $effect(() => {
    const el = canvas;
    if (!el) return;
    const ro = new ResizeObserver(() => (room = { w: el.clientWidth, h: el.clientHeight }));
    ro.observe(el);
    room = { w: el.clientWidth, h: el.clientHeight };
    return () => ro.disconnect();
  });
  const fitOf = (l: DgLayout) => Math.min((room.w - 88) / l.width, (room.h - 72) / l.height);
  // Across when it holds at 85 % or more; else down, as in the column but wider (choice 208).
  const wide = $derived(big ? layout(scoped, { dir: 'LR', names: true, progress, agents, measure, maxRounds: rounds, ...words }) : null);
  const tall = $derived(
    big && room.w
      ? layout(scoped, { width: Math.min(Math.max(room.w - 88, 320), 560), progress, agents, measure, maxRounds: rounds, ...words })
      : null,
  );
  const shown = $derived(wide && tall && fitOf(wide) < 0.85 && fitOf(tall) > fitOf(wide) ? tall : (wide ?? narrow));
  let zoom = $state<number | null>(null);
  // Opened at 100 %, or at the size that shows it all (never under 50 %).
  const z = $derived(zoom ?? (room.w ? clamp(Math.min(1, fitOf(shown))) : 1));
  function clamp(v: number) {
    return Math.min(2, Math.max(0.5, Math.round(v * 10) / 10));
  }
  const setZoom = (v: number) => (zoom = clamp(v));

  /** Moved to the work panel, so as to cover the conversation and the panel (choice 207). */
  function portal(node: HTMLElement) {
    const target = node.parentElement?.closest('.work') ?? document.body;
    target.append(node);
    node.querySelector<HTMLElement>('.dg-canvas')?.focus();
    return { destroy: () => node.remove() };
  }
  function keys(e: KeyboardEvent) {
    if (e.key === 'Escape') (e.stopPropagation(), onclose?.());
    else if (e.ctrlKey && (e.key === '+' || e.key === '=')) (e.preventDefault(), setZoom(z + 0.1));
    else if (e.ctrlKey && e.key === '-') (e.preventDefault(), setZoom(z - 0.1));
  }
  function wheel(e: WheelEvent) {
    if (!e.ctrlKey) return;
    e.preventDefault();
    setZoom(z - Math.sign(e.deltaY) * 0.1);
  }
  // The canvas drags by hand.
  let drag: { x: number; y: number; l: number; t: number } | null = null;
  function down(e: PointerEvent) {
    if (e.button !== 0 || (e.target as Element).closest('.dg-node, .dg-dot, button')) return;
    drag = { x: e.clientX, y: e.clientY, l: canvas!.scrollLeft, t: canvas!.scrollTop };
    canvas!.setPointerCapture(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (!drag || !canvas) return;
    canvas.scrollLeft = drag.l - (e.clientX - drag.x);
    canvas.scrollTop = drag.t - (e.clientY - drag.y);
  }

  const ariaOf = (n: DgLayout['nodes'][number]) =>
    n.state === 'on'
      ? t('workflow.dg.stepOn', { name: n.full })
      : n.state === 'done'
        ? t('workflow.dg.stepDone', { name: n.full })
        : n.state === 'next'
          ? t('workflow.dg.stepNext', { name: n.full })
          : n.state === 'seen'
            ? t('workflow.dg.stepSeen', { name: n.full })
            : n.full;
  /** The numbers of the agent's passages in a step, whole (the beads show the last ones). */
  const passesOf = (id: string) => {
    const all = progress.seen[id] ?? [];
    return all.length ? t('workflow.dg.passesTip', { count: all.length, list: all.join(', ') }) : '';
  };
  const reading = $derived(Object.values(agents).flat());
</script>

{#snippet svg(l: DgLayout, scale: number)}
  <svg
    class="dg"
    viewBox="0 0 {l.width} {l.height}"
    width={l.width * scale}
    height={l.height * scale}
    role="img"
    aria-label={t('workflow.dg.label', { name: model.name })}
  >
    {#each l.edges as e, i (i)}<path class="e {e.cls}" d={e.d} />{/each}
    {#each l.doors as d, i (i)}
      <g class="dg-door" class:taken={d.taken} data-door={d.full}>
        <title>{d.label ? `${d.label} · ${d.full}` : d.full}</title>
        <rect x={d.x} y={d.y} width={d.w} height="20" rx="10" />
        {#if d.label}<text class="lb" x={d.x + 7} y={d.y + 13.5}>{d.label}</text>{/if}
        <text x={d.x + d.w - 7} y={d.y + 13.5} text-anchor="end">{d.text}</text>
      </g>
    {/each}
    {#if l.exit}
      {@const e = l.exit}
      <g class="dg-exit" class:next={e.next}>
        <rect x={e.x} y={e.y} width={e.w} height={e.h} rx={Math.min(e.w, e.h) / 2} />
        <circle cx={e.x + 15} cy={e.y + e.h / 2} r="5" /><circle class="in" cx={e.x + 15} cy={e.y + e.h / 2} r="2.2" />
        <text x={e.x + 27} y={e.y + e.h / 2 + 4}>{e.label}</text>
      </g>
    {/if}
    {#each l.nodes as n (n.id)}
      <g
        class="dg-node {n.state}"
        class:guess={n.guessed}
        class:passed={n.beads.length > 0}
        data-step={n.id}
        tabindex="-1"
        role="listitem"
        aria-label={ariaOf(n)}
      >
        <title>{n.full}{n.cmd ? ` · ${n.cmd}` : ''}{n.beads.length ? ` · ${passesOf(n.id)}` : ''}</title>
        <rect x={n.x} y={n.y} width={n.w} height={n.h} rx="9" />
        {#if n.state === 'done' && !n.beads.length}<use class="ck" href="#i-check" x={n.x + 9} y={n.y + 9} width="14" height="14" />{/if}
        <text class="nm" x={n.x + 10 + (n.state === 'done' && !n.beads.length ? 16 : 0)} y={n.y + 17}>{n.name}</text>
        {#if n.beadsMore}<text class="more" x={n.beadsMore.x} y={n.beadsMore.y} text-anchor="end">{n.beadsMore.text}</text>{/if}
        {#each n.beads as b (b.n)}
          <g class="dg-bead" class:on={b.on} data-pass={b.n}>
            <circle cx={b.cx} cy={b.cy} r="8" /><text x={b.cx} y={b.cy + 3.5} text-anchor="middle">{b.n}</text>
          </g>
        {/each}
        {#if n.cmd}<text class="cmd" x={n.x + 10} y={n.y + n.h - (n.h > 50 ? 14 : 10)}>{n.cmd}</text>{/if}
        {#if n.self}
          {@const s = n.self}
          <g class="dg-self" class:took={s.took}>
            <title
              >{s.turns
                ? t('workflow.dg.turnsTip', { cond: s.cond, count: s.turns })
                : t('workflow.dg.selfTip', { cond: s.cond, count: s.max ?? 0 })}</title
            >
            <rect x={s.x} y={s.y} width="44" height="16" rx="8" />
            <use href="#i-restart" x={s.x + 5} y={s.y + 3} width="10" height="10" />
            {#if selfSays(s)}<text x={s.x + 18} y={s.y + 11.6}>{selfSays(s)}</text>{/if}
          </g>
        {/if}
        {#each n.dots as d (d.id)}
          <circle class="ag" class:cur={d.cur} class:wait={d.wait} cx={d.cx} cy={d.cy} r="4"><title>{d.name}</title></circle>
        {/each}
        {#if n.more}<text class="more" x={n.moreX} y={n.dots[0].cy + 3.6} text-anchor="end">{n.more}</text>{/if}
        {#if n.tag}<text class="tag" x={n.tag.x} y={n.tag.y} text-anchor="end">{n.tag.text}</text>{/if}
        {#if n.fork}<path class="dg-fork" d={n.fork} />{/if}
      </g>
      {#if n.who}
        {@const w = n.who}
        <g class="dg-who" class:cur={w.cur} class:wait={w.wait}>
          <title>{w.all}</title>
          <rect x={w.x} y={w.y} width={w.w} height="18" rx="9" /><circle cx={w.x + 9} cy={w.y + 9} r="3" />
          <text x={w.x + 16} y={w.y + 12.6}>{w.name}</text>
          {#if w.more}<text class="more" x={w.x + w.w + 5} y={w.y + 12.6}>{w.more}</text>{/if}
        </g>
      {/if}
    {/each}
    {#each l.chips as c, i (i)}
      <g class="dg-chip {c.kind}" class:took={c.took}>
        <title>{c.full}</title>
        <rect x={c.x} y={c.y} width={c.w} height="18" rx="9" />
        {#if c.kind === 'ret'}<use href="#i-restart" x={c.x + 5} y={c.y + 4} width="10" height="10" />{/if}
        <text x={c.x + 6 + (c.kind === 'ret' ? 13 : 0)} y={c.y + 12.6}>{c.text}</text>
      </g>
    {/each}
  </svg>
{/snippet}

{#snippet legend()}
  <div class="dg-key">
    <span><svg viewBox="0 0 18 10"><path class="k-taken" d="M1 5h16" /></svg>{t('workflow.dg.keyTaken')}</span>
    <span><svg viewBox="0 0 18 10"><path class="k-poss" d="M1 5h16" /></svg>{t('workflow.dg.keyPossible')}</span>
    <span><svg viewBox="0 0 18 10"><path class="k-took" d="M1 5h16" /></svg>{t('workflow.dg.keyBackTook')}</span>
    <span><svg viewBox="0 0 18 10"><path class="k-ret" d="M1 5h16" /></svg>{t('workflow.dg.keyBack')}</span>
    <span><svg viewBox="0 0 18 10"><path class="k-fork" d="M9 .5l4.5 4.5L9 9.5 4.5 5z" /></svg>{t('workflow.dg.keyChoice')}</span>
  </div>
{/snippet}

<!-- No agent read: the doors to start from, the first until another is picked. -->
{#snippet pick()}
  {#if choosing}
    <div class="dg-pick" role="radiogroup" aria-label={t('workflow.dg.entryPick')} data-testid="workflow-entries">
      <span class="lb">{t('workflow.dg.entryPick')}</span>
      {#each entries as e, i (e.door)}
        <button
          class="mono"
          role="radio"
          aria-checked={e.door === chosen?.door}
          tabindex={e.door === chosen?.door ? 0 : -1}
          onclick={() => (picked = e.door)}
          onkeydown={(ev) => pickKey(ev, i)}>{e.door}</button
        >
      {/each}
    </div>
  {/if}
{/snippet}

<!-- What the entry does not reach: folded, opened on demand. -->
{#snippet aside()}
  {#if scope.off.length && chosen}
    <div class="dg-off" data-testid="workflow-off">
      <button class="h" aria-expanded={open} onclick={() => (open = !open)}>
        <svg viewBox="0 0 10 10" class:open aria-hidden="true"><path d="M3.5 2l3 3-3 3" /></svg>
        {t('workflow.dg.off', { count: scope.off.length })}
      </button>
      {#if open}
        <ul>
          {#each scope.off as s (s.id)}
            <li>
              <b>{s.name}</b>{#if s.cmd}<span class="mono">{s.cmd}</span>{/if}
            </li>
          {/each}
        </ul>
        <p>{t('workflow.dg.offWhy', { door: chosen.door })}</p>
      {/if}
    </div>
  {/if}
{/snippet}

<!-- The path in words, in its order, for a screen reader: the drawing alone says it by lines and colours. -->
{#snippet told()}
  {#if path.length}
    <ol class="dg-told" aria-label={t('workflow.dg.path', { name: reader })} data-testid="workflow-path">
      {#each path as p (p.n)}<li>{say(p)}</li>{/each}
    </ol>
  {/if}
{/snippet}

{@render pick()}
<div class="dg-wrap" bind:this={wrap} data-testid="workflow-diagram">
  {@render svg(narrow, 1)}
</div>
{#if !big}{@render told()}{/if}
{@render aside()}
{@render legend()}

{#if big}
  <section class="dg-big" use:portal aria-label={t('workflow.dg.label', { name: model.name })} data-testid="workflow-diagram-big">
    <div class="dg-bar">
      <span class="dg-title"
        ><Icon name="flow" size={14} /><b>{model.name}</b>{#if entry}<span class="by"
            >· {t('workflow.dg.enteredBy')} <span class="mono">{entry.door}</span></span
          >{/if}</span
      >
      <span class="grow"></span>
      <div class="dg-zoom" role="group" aria-label={t('workflow.dg.zoom')}>
        <button class="icon-btn" aria-label={t('workflow.dg.zoomOut')} use:tip={t('workflow.dg.zoomOut')} onclick={() => setZoom(z - 0.1)}
          ><Icon name="min" size={14} /></button
        >
        <span class="v" data-testid="workflow-zoom">{Math.round(z * 100)} %</span>
        <button class="icon-btn" aria-label={t('workflow.dg.zoomIn')} use:tip={t('workflow.dg.zoomIn')} onclick={() => setZoom(z + 0.1)}
          ><Icon name="plus" size={14} /></button
        >
      </div>
      <button class="btn" onclick={() => setZoom(Math.min(fitOf(shown), 2))}>{t('workflow.dg.fit')}</button>
      <button class="btn" onclick={() => oncopy?.()}><Icon name="copy" size={14} />{t('workflow.dg.copy')}</button>
      <button class="icon-btn" aria-label={t('workflow.dg.close')} use:tip={t('workflow.dg.close')} onclick={() => onclose?.()}
        ><Icon name="close" size={14} /></button
      >
    </div>
    {@render pick()}
    <!-- The canvas scrolls and zooms by the keyboard: a region that takes the focus. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
    <div
      class="dg-canvas"
      bind:this={canvas}
      tabindex="0"
      role="region"
      aria-label={t('workflow.dg.label', { name: model.name })}
      onkeydown={keys}
      onwheel={wheel}
      onpointerdown={down}
      onpointermove={move}
      onpointerup={() => (drag = null)}
    >
      {@render svg(shown, z)}
    </div>
    {@render told()}
    {@render aside()}
    {#if reading.length}
      <div class="dg-side">
        <h4>{t('workflow.dg.agents')}</h4>
        {#each reading as a (a.id)}
          <button class="r" class:cur={a.cur} class:wait={a.wait} onclick={() => (app.selectAgent(a.id), onclose?.())}
            ><i></i><span>{a.name}</span></button
          >
        {/each}
      </div>
    {/if}
    {@render legend()}
  </section>
{/if}

<style>
  /* The tokens of the preview's `.dg` (design/app-redesign, ESC-245): the ways back in the red of a failure, paler; the
     next round dashed and neutral; the agent read's step green. */
  .dg-wrap {
    position: relative;
    margin: 2px 14px 4px;
  }
  .dg {
    display: block;
    overflow: visible;
    font-family: var(--font);
  }
  .dg text {
    fill: var(--text);
    font-size: 12.5px;
  }
  /* What may happen is light and dashed; what the agent took is full and marked (ESC-406): told apart by the line
     itself, not by its colour alone. */
  .dg .e {
    fill: none;
    stroke: var(--line-3);
    stroke-width: 1.2;
    stroke-dasharray: 2.5 3.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .dg .e.walked {
    stroke: var(--text);
    stroke-width: 2.4;
    stroke-dasharray: none;
  }
  .dg .e.back {
    stroke: oklch(from var(--del) l c h / 0.6);
  }
  .dg .e.took {
    stroke: var(--del);
    stroke-width: 2.4;
    stroke-dasharray: none;
  }
  .dg .e.again {
    stroke-dasharray: 1 4;
  }
  .dg .e.head {
    stroke-dasharray: none;
  }
  .dg use {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  /* A step the agent has not passed is a possible: dashed, quieter. */
  .dg-node rect {
    fill: var(--panel);
    stroke: var(--line-3);
    stroke-dasharray: 3 3;
  }
  .dg-node:hover rect {
    stroke: var(--muted);
  }
  .dg-node .nm {
    fill: var(--soft);
    font-weight: 550;
    letter-spacing: -0.005em;
  }
  .dg-node:is(.done, .seen, .on, .passed) rect {
    fill: var(--raise);
    stroke: var(--muted);
    stroke-width: 1.4;
    stroke-dasharray: none;
  }
  .dg-node:is(.done, .seen, .on, .passed) .nm {
    fill: var(--text);
    font-weight: 600;
  }
  .dg-bead circle {
    fill: var(--text);
  }
  .dg-bead text {
    fill: var(--panel);
    font-size: 10px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .dg-bead.on circle {
    fill: var(--run);
  }
  .dg-node .cmd {
    fill: var(--muted);
    font: 500 10.5px var(--mono);
  }
  .dg-node .ck {
    color: var(--muted);
  }
  .dg-node.on rect {
    fill: color-mix(in oklch, var(--run) 8%, var(--raise));
    stroke: var(--run);
    stroke-width: 1.8;
  }
  .dg-node.next:not(.passed) rect {
    stroke: var(--muted);
    stroke-dasharray: 4 3;
  }
  .dg-node.guess rect {
    stroke-dasharray: 1.5 3;
  }
  .dg-node.on .tag {
    fill: var(--run);
  }
  .dg-node .tag,
  .dg-node .more {
    fill: var(--muted);
    font-size: 10.5px;
    font-weight: 550;
  }
  .dg-node .ag {
    fill: var(--panel);
    stroke: var(--muted);
    stroke-width: 1.5;
  }
  .dg-node .ag.cur {
    fill: var(--run);
    stroke: var(--run);
  }
  .dg-node .ag.wait {
    stroke: var(--wait);
  }
  .dg-fork {
    fill: var(--panel);
    stroke: var(--muted);
    stroke-width: 1.2;
  }
  .dg-self rect {
    fill: color-mix(in oklch, var(--del) 10%, var(--raise));
    stroke: oklch(from var(--del) l c h / 0.45);
    stroke-dasharray: 2.5 2.5;
  }
  .dg-self.took rect {
    stroke: var(--del);
    stroke-width: 1.4;
    stroke-dasharray: none;
  }
  .dg-self use {
    color: var(--del);
  }
  .dg-self text {
    fill: var(--del);
    font-size: 10.5px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .dg-door rect {
    fill: var(--panel);
    stroke: var(--line-3);
    stroke-dasharray: 2 2.5;
  }
  .dg-door text {
    fill: var(--soft);
    font: 500 11px var(--mono);
  }
  .dg-door text.lb {
    fill: var(--muted);
    font: 550 10.5px var(--font);
  }
  .dg-door.taken rect {
    fill: var(--raise-2);
    stroke: var(--text);
    stroke-width: 1.3;
    stroke-dasharray: none;
  }
  .dg-door.taken text:not(.lb) {
    fill: var(--text);
  }
  .dg-exit rect {
    fill: var(--panel);
    stroke: var(--line-3);
    stroke-dasharray: 3 3;
  }
  .dg-exit.next rect {
    stroke: var(--muted);
    stroke-dasharray: 4 3;
  }
  .dg-exit circle {
    fill: none;
    stroke: var(--soft);
    stroke-width: 1.3;
  }
  .dg-exit circle.in {
    fill: var(--soft);
    stroke: none;
  }
  .dg-exit text {
    fill: var(--soft);
    font-size: 12px;
    font-weight: 600;
  }
  .dg-chip rect {
    fill: var(--panel);
    stroke: var(--line-2);
  }
  .dg-chip text {
    fill: var(--soft);
    font-size: 10.5px;
    font-weight: 500;
  }
  .dg-chip.ret rect {
    fill: color-mix(in oklch, var(--del) 9%, var(--panel));
    stroke: oklch(from var(--del) l c h / 0.45);
    stroke-dasharray: 2.5 2.5;
  }
  .dg-chip.ret.took rect {
    stroke: var(--del);
    stroke-width: 1.3;
    stroke-dasharray: none;
  }
  .dg-chip.ret text,
  .dg-chip.ret use {
    fill: var(--del);
    color: var(--del);
  }
  .dg-chip.ret use {
    fill: none;
  }
  .dg-chip.again rect {
    stroke: var(--line-3);
    stroke-dasharray: 3 3;
  }
  .dg-chip.again text {
    fill: var(--muted);
  }
  .dg-who rect {
    fill: var(--panel);
    stroke: var(--line-2);
  }
  .dg-who circle {
    fill: var(--muted);
  }
  .dg-who.cur rect {
    stroke: oklch(from var(--run) l c h / 0.55);
  }
  .dg-who.cur circle {
    fill: var(--run);
  }
  .dg-who.wait circle {
    fill: var(--wait);
  }
  .dg-who text {
    fill: var(--soft);
    font-size: 10.5px;
    font-weight: 500;
  }
  .dg-who .more {
    fill: var(--muted);
  }
  @media (prefers-reduced-motion: no-preference) {
    .dg-node.on rect {
      animation: dg-breathe 2.4s ease-in-out infinite;
    }
  }
  @keyframes dg-breathe {
    50% {
      stroke: oklch(from var(--run) l c h / 0.5);
    }
  }
  .dg-key {
    display: flex;
    flex-wrap: wrap;
    gap: 4px 12px;
    padding: 6px 14px 12px;
    color: var(--muted);
    font-size: 11px;
  }
  .dg-key span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .dg-key svg {
    width: 18px;
    height: 10px;
    overflow: visible;
  }
  .dg-key path,
  .dg-key rect {
    fill: none;
    stroke-width: 1.5;
    stroke-linecap: round;
  }
  .dg-key .k-taken {
    stroke: var(--text);
    stroke-width: 2.4;
  }
  .k-poss {
    stroke: var(--line-3);
    stroke-dasharray: 2.5 3.5;
  }
  .dg-key .k-took {
    stroke: var(--del);
    stroke-width: 2.4;
  }
  .k-ret {
    stroke: oklch(from var(--del) l c h / 0.6);
    stroke-dasharray: 2.5 3.5;
  }
  .dg-key .k-fork {
    fill: var(--panel);
    stroke: var(--muted);
    stroke-width: 1.2;
  }

  /* The doors to choose from, when no agent is read. */
  .dg-pick {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 5px;
    padding: 2px 14px 8px;
  }
  .dg-pick .lb {
    flex-basis: 100%;
    color: var(--muted);
    font-size: 11px;
    font-weight: 550;
  }
  .dg-pick button {
    max-width: 100%;
    height: 22px;
    padding: 0 9px;
    overflow: hidden;
    border: 0;
    border-radius: 11px;
    background: none;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: pointer;
  }
  .dg-pick button:hover {
    background: var(--hover);
    color: var(--soft);
  }
  .dg-pick button[aria-checked='true'] {
    background: var(--raise-2);
    box-shadow: inset 0 0 0 1.3px var(--text);
    color: var(--text);
  }
  .dg-pick button:focus-visible {
    outline: 2px solid var(--focus, var(--text));
    outline-offset: 2px;
  }
  /* What the entry does not reach. */
  .dg-off {
    margin: 4px 14px 6px;
    border-radius: 8px;
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--muted);
    font-size: 11.5px;
  }
  .dg-off .h {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    height: 28px;
    padding: 0 10px;
    border: 0;
    border-radius: inherit;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .dg-off .h:hover {
    color: var(--soft);
  }
  .dg-off .h:focus-visible {
    outline: 2px solid var(--focus, var(--text));
    outline-offset: -2px;
  }
  .dg-off .h svg {
    flex: none;
    width: 10px;
    height: 10px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .dg-off .h svg.open {
    transform: rotate(90deg);
  }
  .dg-off ul {
    display: grid;
    gap: 4px;
    margin: 0;
    padding: 2px 10px 6px 26px;
    list-style: none;
  }
  .dg-off li {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }
  .dg-off li b {
    flex: none;
    color: var(--soft);
    font-weight: 550;
  }
  .dg-off li span {
    overflow: hidden;
    font-size: 10.5px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dg-off p {
    margin: 0;
    padding: 0 10px 8px 26px;
    font-size: 11px;
  }
  /* The path in words: read, not seen. */
  .dg-told {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    padding: 0;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  /* Enlarged: the whole work panel. */
  .dg-big {
    position: absolute;
    inset: 0;
    z-index: 30;
    display: flex;
    flex-direction: column;
    border-radius: inherit;
    background: var(--panel);
  }
  .dg-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 48px;
    padding: 0 12px 0 16px;
    border-bottom: 1px solid var(--line);
  }
  .dg-title {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    color: var(--text);
  }
  .dg-title b {
    flex: none;
    font-weight: 600;
  }
  .dg-title .by {
    overflow: hidden;
    color: var(--muted);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dg-big .dg-pick {
    padding: 8px 16px;
    border-bottom: 1px solid var(--line);
  }
  .dg-big .dg-pick .lb {
    flex-basis: auto;
    margin-right: 4px;
  }
  .dg-big .dg-off {
    position: absolute;
    bottom: 52px;
    left: 14px;
    max-width: min(320px, calc(100% - 28px));
    margin: 0;
    background: var(--raise);
    box-shadow:
      0 8px 24px rgba(0, 0, 0, calc(0.35 * var(--shade, 1))),
      inset 0 0 0 1px var(--line-2);
  }
  .grow {
    flex: 1;
  }
  .dg-zoom {
    display: inline-flex;
    align-items: center;
    height: 28px;
    padding: 0 2px;
    border-radius: 7px;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .dg-zoom .v {
    min-width: 44px;
    color: var(--soft);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    text-align: center;
  }
  .dg-canvas {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: auto;
    background:
      radial-gradient(circle at 1px 1px, rgba(var(--fg-rgb), 0.07) 1px, transparent 1.5px) 0 0 / 18px 18px,
      var(--well);
    cursor: grab;
    outline: none;
    scrollbar-width: thin;
  }
  .dg-canvas .dg {
    margin: 36px 44px;
  }
  .dg-side {
    position: absolute;
    right: 14px;
    bottom: 52px;
    display: grid;
    gap: 4px;
    width: 260px;
    padding: 12px 10px 10px;
    border-radius: 11px;
    background: var(--raise);
    box-shadow:
      0 8px 24px rgba(0, 0, 0, calc(0.35 * var(--shade, 1))),
      inset 0 0 0 1px var(--line-2);
  }
  .dg-side h4 {
    margin: 0 4px 4px;
    font-size: 12.5px;
    font-weight: 600;
  }
  .dg-side .r {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 26px;
    padding: 0 6px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--soft);
    font: inherit;
    font-size: 12px;
    text-align: left;
    cursor: pointer;
  }
  .dg-side .r:hover {
    background: var(--hover);
  }
  .dg-side .r i {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--muted);
  }
  .dg-side .r.cur i {
    background: var(--run);
  }
  .dg-side .r.wait i {
    background: var(--wait);
  }
  .dg-side .r span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dg-big .dg-key {
    padding: 10px 16px;
    border-top: 1px solid var(--line);
  }
</style>
