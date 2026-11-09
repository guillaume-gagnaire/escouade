<script lang="ts">
  import { fHeat, heatClock, heatDetail, heatLabel, heatTone, temperature } from '../lib/heat.svelte';
  import { t } from '../lib/i18n';
  import { tip } from '../lib/tip';
  import type { Agent } from '../lib/types';

  // The temperature of an agent's cache (ESC-398): « 72 °C » in the colour of its warmth, its meaning in words for a
  // screen reader and in the tooltip. In the conversation's header (`detail`), a figure of the header whose hover or
  // focus shows the detail of the last turn (maquette design/features/temperature.png).
  let {
    agent,
    detail = false,
    now = undefined,
  }: {
    agent: Pick<Agent, 'id' | 'heat' | 'engine'>;
    detail?: boolean;
    /** The clock (ms); the common one when left out. */
    now?: number;
  } = $props();

  const at = $derived(now ?? heatClock.now);
  const deg = $derived(temperature(agent.heat, at));
  const tone = $derived(heatTone(deg));
  const label = $derived(heatLabel(agent, at));
  let open = $state(false);
  const info = $derived(open ? heatDetail(agent, at) : null);
  const popId = $derived(`heat-pop-${agent.id}`);
  const bar = $derived.by(() => {
    const h = agent.heat;
    if (!h) return null;
    const total = h.cached + h.fresh;
    if (!total) return null;
    const read = h.read ?? h.cached;
    const written = h.written ?? 0;
    return { read: (read / total) * 100, written: (written / total) * 100 };
  });
</script>

{#if detail}
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="fig fig-heat"
    role="group"
    tabindex="0"
    aria-label={label}
    aria-describedby={open ? popId : undefined}
    data-heat={agent.id}
    onmouseenter={() => (open = true)}
    onmouseleave={() => (open = false)}
    onfocus={() => (open = true)}
    onblur={() => (open = false)}
    onkeydown={(e) => e.key === 'Escape' && (open = false)}
  >
    <dt aria-hidden="true">{t('heat.label')}</dt>
    <dd class="heat {tone}" aria-hidden="true" data-tone={tone}>{fHeat(deg)}</dd>
    {#if info}
      <div class="heat-pop" id={popId} role="tooltip">
        <p class="hp-title heat {tone}">{info.title}</p>
        <p class="hp-line">{info.line}</p>
        {#if bar}
          <div class="hp-bar" aria-hidden="true">
            <i class="r" style:width="{bar.read}%"></i><i class="w" style:width="{bar.written}%"></i>
          </div>
        {/if}
        {#if info.rows.length}
          <dl class="hp-rows">
            {#each info.rows as r (r.label)}
              <div>
                <dt>{r.label}</dt>
                <dd>{r.value}</dd>
              </div>
            {/each}
          </dl>
        {/if}
        {#if info.next}<p class="hp-next">{info.next}</p>{/if}
      </div>
    {/if}
  </div>
{:else}
  <span
    class="heat on-card {tone}"
    role="img"
    aria-label={label}
    use:tip={{ text: label, aria: false }}
    data-heat={agent.id}
    data-tone={tone}>{fHeat(deg)}</span
  >
{/if}

<style>
  .heat {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .heat.hot {
    color: var(--heat-hot);
  }
  .heat.mid {
    color: var(--heat-mid);
  }
  .heat.cool,
  .heat.cold {
    color: var(--heat-cool);
  }
  .heat.none {
    color: var(--muted);
  }
  .heat.on-card {
    margin-left: auto;
    flex: none;
    padding-left: 8px;
    font-size: 12px;
  }
  /* A figure of the header, as its others (Conversation.svelte's `.fig`). */
  .fig-heat {
    position: relative;
    display: grid;
    gap: 1px;
    /* Closer to « Coût », which it goes with (the maquette's « Coût 0,61 $ 27 °C »): the header's name keeps its room. */
    margin-left: -8px;
    text-align: left;
    outline: none;
    cursor: default;
  }
  .fig-heat > dt {
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 500;
  }
  .fig-heat > dd {
    margin: 0;
    /* The header's figures are in the text's face; the card keeps the mono of the maquette. */
    font-family: inherit;
    font-size: 12.5px;
    font-weight: 550;
  }
  .fig-heat:focus-visible {
    box-shadow: 0 0 0 2px var(--ring);
    border-radius: 4px;
  }
  .heat-pop {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    z-index: 30;
    width: min(360px, 80vw);
    padding: 12px 14px;
    border: 1px solid var(--line-2);
    border-radius: 10px;
    background: var(--raise-2);
    box-shadow: 0 10px 30px rgba(0, 0, 0, calc(0.35 * var(--shade)));
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.45;
    text-align: left;
    white-space: normal;
  }
  .hp-title {
    margin: 0 0 6px;
    font-family: inherit;
    font-weight: 600;
  }
  .hp-line,
  .hp-next {
    margin: 0;
    color: var(--muted);
  }
  .hp-next {
    margin-top: 8px;
  }
  .hp-bar {
    display: flex;
    height: 6px;
    margin: 10px 0;
    overflow: hidden;
    border-radius: 3px;
    background: var(--line-3);
  }
  .hp-bar .r {
    background: var(--heat-hot);
  }
  .hp-bar .w {
    background: var(--heat-cool);
  }
  .hp-rows {
    margin: 0;
  }
  .hp-rows div {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    padding: 3px 0;
  }
  .hp-rows dt {
    color: var(--soft);
  }
  .hp-rows dd {
    margin: 0;
    color: var(--text);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
</style>
