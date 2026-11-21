<script lang="ts">
  // « À découvrir » (ESC-506, docs/ONBOARDING.md § 7, direction C « La ligne »): what Escouade can do, as the first
  // launch's line goes on into five lanes of the cycle (Planifier, Coder, Relire, Livrer, Piloter). Each station is a
  // discovery, ticked by what the user did for real; the one chosen says what it is, what it asks before, and leads to the
  // real gesture. Nothing is done from here but showing the way: the gesture stays the user's, on the real control.
  import { onMount } from 'svelte';
  import { act, closeMap, discover, dismiss, go, LANES, load } from '../../lib/discover.svelte';
  import { t } from '../../lib/i18n';
  import type { IconName } from '../../lib/icons';
  import type { DiscoverItem, Discovery } from '../../lib/types';
  import Icon from '../Icon.svelte';

  onMount(() => void load());

  /** The icon of each discovery. */
  const ICON: Record<Discovery, IconName> = {
    ticket: 'kanban',
    autopilot: 'flow',
    docs: 'page',
    sources: 'kanban',
    dictation: 'mic',
    voice: 'wave',
    terminal: 'term',
    editor: 'code',
    secondAgent: 'agents',
    launch: 'play',
    forge: 'link',
    prView: 'pr',
    prReview: 'sparkle',
    approve: 'check',
    openPr: 'push',
    plugin: 'puzzle',
    mcp: 'plug',
    account: 'user',
    remote: 'phone',
    team: 'team',
  };

  const view = $derived(discover.view);
  const items = $derived(view?.items ?? []);
  const shown = $derived(discover.onlyNew ? items.filter((i) => i.new) : items);
  const lanes = $derived(LANES.map((lane) => ({ lane, stops: shown.filter((i) => i.lane === lane) })).filter((l) => l.stops.length));
  /** The station chosen: the user's, else the first to try, else the first. */
  const sel = $derived(
    shown.find((i) => i.id === discover.selected) ??
      shown.find((i) => i.status === 'todo' && !i.after) ??
      shown.find((i) => i.status === 'todo') ??
      shown[0],
  );

  const name = (id: Discovery) => t(`discover.items.${id}.name`);

  /** The line under a station: done, dismissed, why it is not here, what it needs first, or what it asks. */
  function meta(i: DiscoverItem): string {
    if (i.status === 'done') return t('discover.state.done');
    if (i.status === 'dismissed') return t('discover.state.dismissed');
    if (i.status === 'absent') return t(`discover.absent.${i.absent?.kind ?? 'policy'}`);
    if (i.after) return t('discover.state.after', { name: name(i.after) });
    return t(`discover.items.${i.id}.meta`);
  }

  /** Why it is not on this device, at length. */
  function absentLong(i: DiscoverItem): string {
    if (i.absent?.kind === 'policy') return i.absent.why;
    if (i.absent?.kind === 'noTeamServer') return t('discover.absent.noTeamServerLong');
    return t('discover.absent.noMicrophoneLong');
  }

  function choose(id: Discovery) {
    discover.selected = id;
  }

  /** What `to` needs first, chosen and tried. */
  function lead(to: Discovery | null) {
    if (!to) return;
    discover.selected = to;
    go(to);
  }

  const LANE_VAR: Record<string, string> = { plan: '--lane-1', code: '--lane-2', review: '--lane-3', ship: '--lane-4', drive: '--lane-5' };
</script>

<main class="disc-c" data-testid="discover" aria-labelledby="disc-h">
  <div class="disc">
    <section class="map">
      <header class="mh">
        <h1 id="disc-h">{t('discover.title')}</h1>
        <span class="sub">{t('discover.progress', { done: view?.done ?? 0, total: view?.total ?? 0 })}</span>
        {#if discover.onlyNew && discover.release}
          <span class="chip">
            {t('discover.onlyNew', { release: discover.release })}
            <button class="link" onclick={() => (discover.onlyNew = false)}>{t('discover.seeAll')}</button>
          </span>
        {/if}
        <span class="sp"></span>
        {#if view}
          <button class="btn small ghost" onclick={() => void act({ kind: 'hide', hidden: !view.hidden })}
            >{view.hidden ? t('discover.showAgain') : t('discover.hideAll')}</button
          >
        {/if}
        <button class="btn small ghost" onclick={closeMap}><Icon name="left" size={13} />{t('discover.close')}</button>
      </header>
      {#if view?.hidden}
        <p class="note" role="status">{t('discover.hiddenNote')}</p>
      {/if}
      <div class="trunk" aria-hidden="true"><span></span><span class="o"><i></i></span><span class="tx">{t('discover.trunk')}</span></div>
      <ul class="lanes" aria-label={t('discover.mapLabel')}>
        {#each lanes as l, k (l.lane)}
          <li class="lane" style:--lc="var({LANE_VAR[l.lane]})">
            <span class="ln"><i aria-hidden="true"></i>{t(`discover.lanes.${l.lane}`)}</span>
            <span class="elbow" aria-hidden="true"
              ><svg width="30" height="96"
                >{#if k < lanes.length - 1}<path class="tr" d="M0 0 V96" />{/if}<path class="br" d="M0 0 C0 15, 6 15, 30 15" /></svg
              ></span
            >
            <ul class="track" aria-label={t(`discover.lanes.${l.lane}`)}>
              {#each l.stops as i (i.id)}
                <li class="stop-li">
                  <button
                    class="stop {i.status}"
                    class:sel={sel?.id === i.id}
                    aria-pressed={sel?.id === i.id}
                    data-stop={i.id}
                    data-status={i.status}
                    onclick={() => choose(i.id)}
                  >
                    <span class="o" aria-hidden="true"><i></i></span>
                    <span class="t">{name(i.id)}</span>
                    <span class="m">{meta(i)}</span>
                  </button>
                </li>
              {/each}
            </ul>
          </li>
        {/each}
      </ul>
      <div class="legend" aria-hidden="true">
        <span><i class="f"></i>{t('discover.legend.done')}</span>
        <span><i></i>{t('discover.legend.todo')}</span>
        <span><i class="d"></i>{t('discover.legend.dismissed')}</span>
        <span><i class="a"></i>{t('discover.legend.absent')}</span>
      </div>
    </section>
    {#if sel}
      {@const facts = t(`discover.items.${sel.id}.facts`)}
      <aside class="detail" aria-live="polite" data-testid="discover-detail" data-detail={sel.id}>
        <h2><Icon name={ICON[sel.id]} size={16} />{name(sel.id)}</h2>
        <p class="line">{t(`discover.items.${sel.id}.line`)}</p>
        {#if sel.status === 'absent'}
          <p class="state absent"><Icon name="lock" size={13} />{absentLong(sel)}</p>
        {:else if sel.status === 'done'}
          <p class="state ok"><Icon name="check" size={13} />{t('discover.detail.done')}</p>
        {:else if sel.status === 'dismissed'}
          <p class="state">{t('discover.detail.dismissed')}</p>
        {/if}
        {#if facts && sel.status !== 'absent'}
          <div class="facts">
            <span class="fh">{t('discover.detail.before')}</span>
            <span>{facts}</span>
          </div>
        {/if}
        <div class="acts">
          {#if sel.status === 'todo' && sel.after}
            <button class="btn primary" onclick={() => lead(sel.after)}
              ><Icon name={ICON[sel.after]} size={13} />{t('discover.detail.first', { name: name(sel.after) })}</button
            >
            <button class="btn ghost" onclick={() => void dismiss(sel.id)}>{t('discover.detail.dismiss')}</button>
          {:else if sel.status === 'todo'}
            <button class="btn primary" onclick={() => go(sel.id)}><Icon name={ICON[sel.id]} size={13} />{t('discover.detail.try')}</button>
            <button class="btn ghost" onclick={() => void dismiss(sel.id)}>{t('discover.detail.dismiss')}</button>
          {:else if sel.status === 'done'}
            <button class="btn" onclick={() => go(sel.id)}>{t('discover.detail.again')}</button>
          {:else if sel.status === 'dismissed'}
            <button class="btn" onclick={() => void act({ kind: 'restore', id: sel.id })}>{t('discover.detail.restore')}</button>
          {/if}
        </div>
      </aside>
    {/if}
  </div>
</main>

<style>
  /* The container whose width chooses the layout (the window's, less the rail). */
  .disc-c {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    container-type: inline-size;
  }
  .disc {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 320px;
    overflow: hidden;
  }
  .map {
    padding: 26px 28px;
    display: grid;
    align-content: start;
    overflow: auto;
    min-height: 0;
  }
  .mh {
    display: flex;
    align-items: baseline;
    gap: 12px;
    margin-bottom: 18px;
    flex-wrap: wrap;
  }
  .mh h1 {
    font-size: 20px;
    font-weight: 650;
    letter-spacing: -0.015em;
  }
  .sub {
    color: var(--muted);
    font-size: 12.5px;
  }
  .sp {
    flex: 1;
  }
  .chip {
    display: inline-flex;
    gap: 8px;
    align-items: baseline;
    padding: 2px 10px;
    border-radius: 999px;
    background: var(--p-soft);
    color: var(--p-ink);
    font-size: 12px;
  }
  .link {
    color: inherit;
    text-decoration: underline;
    font-size: 12px;
  }
  .note {
    margin: -8px 0 14px;
    color: var(--muted);
    font-size: 12.5px;
  }
  .trunk {
    display: grid;
    grid-template-columns: 112px 30px minmax(0, 1fr);
    align-items: center;
    height: 30px;
  }
  .trunk .o {
    width: 18px;
    height: 18px;
    display: grid;
    place-items: center;
    margin-left: -8px;
  }
  .trunk .o i {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--lane-main);
  }
  .trunk .tx {
    color: var(--muted);
    font-size: 12px;
  }
  .lanes {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .lane {
    position: relative;
    display: grid;
    grid-template-columns: 112px 30px minmax(0, 1fr);
    align-items: start;
    min-height: 96px;
  }
  .ln {
    font-weight: 620;
    font-size: 12.5px;
    color: var(--soft);
    display: flex;
    align-items: center;
    gap: 8px;
    padding-top: 6px;
  }
  .ln i {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--lc);
  }
  .elbow {
    position: relative;
    height: 100%;
  }
  .elbow svg {
    position: absolute;
    left: 0;
    top: 0;
    overflow: visible;
  }
  .elbow path {
    fill: none;
    stroke-width: 2;
  }
  .elbow .tr {
    stroke: var(--lane-main);
    opacity: 0.7;
  }
  .elbow .br {
    stroke: var(--lc);
  }
  .track {
    position: relative;
    display: flex;
    align-items: flex-start;
    list-style: none;
    margin: 0;
    padding: 6px 0 0;
  }
  .track::before {
    content: '';
    position: absolute;
    left: 0;
    right: 8px;
    top: 15px;
    height: 2px;
    background: var(--lc);
    opacity: 0.8;
  }
  .stop-li {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
  }
  .stop {
    position: relative;
    min-width: 0;
    display: grid;
    justify-items: start;
    gap: 7px;
    padding: 0 8px 8px 0;
    margin: 0;
    border: 0;
    box-shadow: none;
    text-align: left;
    border-radius: 6px;
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .stop:focus-visible {
    outline: 2px solid var(--teal-ink);
    outline-offset: 2px;
  }
  .o {
    width: 18px;
    height: 18px;
    display: grid;
    place-items: center;
    z-index: 1;
  }
  .o i {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--panel);
    box-shadow: inset 0 0 0 2px var(--lc);
  }
  .stop.done .o i {
    background: var(--lc);
  }
  .stop.dismissed .o i {
    box-shadow: inset 0 0 0 2px var(--line-3);
    outline: 1px dashed var(--lc);
    outline-offset: 1px;
  }
  .stop.absent .o i {
    box-shadow: inset 0 0 0 2px var(--line-3);
  }
  .stop.sel .o i {
    width: 16px;
    height: 16px;
    box-shadow:
      inset 0 0 0 2px var(--lc),
      0 0 0 4px color-mix(in oklch, var(--lc) 25%, transparent);
  }
  .stop.done.sel .o i {
    background: var(--lc);
  }
  .t {
    font-size: 12px;
    color: var(--soft);
    line-height: 1.3;
  }
  .m {
    font-size: 11.5px;
    color: var(--dim);
    line-height: 1.3;
    margin-top: -4px;
  }
  .stop.done .t {
    color: var(--muted);
  }
  .stop.done .m {
    color: var(--teal-ink);
  }
  .stop.absent .t,
  .stop.dismissed .t {
    color: var(--dim);
  }
  .stop.sel .t {
    color: var(--text);
    font-weight: 600;
  }
  .stop:hover .t {
    color: var(--text);
  }
  .legend {
    display: flex;
    gap: 16px;
    color: var(--muted);
    font-size: 12px;
    margin-top: 16px;
    flex-wrap: wrap;
  }
  .legend span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .legend i {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 2px var(--lane-main);
  }
  .legend i.f {
    background: var(--lane-main);
  }
  .legend i.d {
    outline: 1px dashed var(--lane-main);
    outline-offset: 1px;
    box-shadow: inset 0 0 0 2px var(--line-3);
  }
  .legend i.a {
    box-shadow: inset 0 0 0 2px var(--line-3);
  }
  .detail {
    border-left: 1px solid var(--line);
    padding: 26px 22px;
    display: grid;
    align-content: start;
    gap: 14px;
    overflow: auto;
    min-height: 0;
  }
  .detail h2 {
    font-size: 16px;
    font-weight: 640;
    display: flex;
    gap: 9px;
    align-items: center;
  }
  .line {
    color: var(--soft);
    line-height: 1.55;
  }
  .state {
    display: flex;
    gap: 7px;
    align-items: baseline;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .state.ok {
    color: var(--teal-ink);
  }
  .facts {
    display: grid;
    gap: 4px;
    font-size: 12.5px;
    color: var(--muted);
    line-height: 1.5;
  }
  .fh {
    color: var(--soft);
    font-weight: 600;
    font-size: 12px;
  }
  .acts {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  /* 1024 × 728: the map on one column, the detail at the bottom (the design's narrow window). */
  @container (max-width: 1100px) {
    .disc {
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: minmax(0, 1fr) auto;
    }
    .map {
      padding: 16px 20px 4px;
    }
    .lane {
      grid-template-columns: 76px 26px minmax(0, 1fr);
      min-height: 62px;
    }
    .mh {
      margin-bottom: 10px;
    }
    .trunk {
      grid-template-columns: 76px 26px minmax(0, 1fr);
    }
    .m,
    .legend {
      display: none;
    }
    .detail {
      border-left: none;
      border-top: 1px solid var(--line);
      padding: 14px 20px;
      grid-template-columns: minmax(0, 1fr) auto;
      gap: 6px 16px;
      align-items: center;
    }
    .detail > :not(.acts) {
      grid-column: 1;
    }
    .detail .acts {
      grid-column: 2;
      grid-row: 1 / span 4;
    }
    .facts {
      grid-template-columns: auto minmax(0, 1fr);
      gap: 8px;
    }
  }
  @media (prefers-reduced-motion: no-preference) {
    .o i {
      transition:
        background 160ms var(--ease),
        box-shadow 160ms var(--ease);
    }
  }
</style>
