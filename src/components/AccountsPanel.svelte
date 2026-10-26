<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { onMount } from 'svelte';
  import { accountName, metersShown, projectedDayCost, projectionLevel, projectionMode, quotaGroups } from '../lib/accounts';
  import { capsOf, usagePage } from '../lib/engines';
  import { trapFocus } from '../lib/focus';
  import { fUsd } from '../lib/format';
  import { t } from '../lib/i18n';
  import { estimatedBy, estimateHint, fSpentUsd } from '../lib/spend';
  import { app } from '../lib/state.svelte';
  import Icon from './Icon.svelte';
  import QuotaMeter from './QuotaMeter.svelte';
  import { tip } from '../lib/tip';

  // The panel the quota gauges of the status bar open, above them: a group per harness, in « Ordre des harness », each
  // account with its windows (the percentage, the projection, the reset), its cost of the day, and what concerns it
  // (current, past the threshold, extra usage, a daily budget); the current account of the harness first.
  // `anchor`: that button, to open over it and to give the focus back to. `id`: what the button's `aria-controls` names.
  let { anchor, id, onclose }: { anchor?: HTMLElement; id?: string; onclose: () => void } = $props();

  /** The width of the panel. */
  const WIDTH = 440;

  const groups = $derived(quotaGroups(app.now));
  /** Some cost is estimated (Codex): the panel says how. */
  const estimated = $derived(groups.some((g) => estimatedBy(g.engine)));
  let place = $state({ left: 16, bottom: 36 });

  onMount(() => {
    if (anchor) {
      const r = anchor.getBoundingClientRect();
      place = {
        left: Math.max(8, Math.min(r.left, window.innerWidth - WIDTH - 8)),
        bottom: Math.max(8, window.innerHeight - r.top + 6),
      };
    }
    // The focus goes back to the button, which a click may not have focused (Safari's): the keyboard goes on from there.
    return () => anchor?.focus();
  });

  /** Escape closes the panel before it can reach what is under it (the composer, where it interrupts Claude). */
  function onWindowKeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    e.preventDefault();
    e.stopPropagation();
    onclose();
  }

  /** What the account answering as extra usage does, under it (choice 22). */
  const overage = (id: string, name: string, engine: string) =>
    t(
      id === app.usage.current[engine as keyof typeof app.usage.current]
        ? 'accounts.quota.overageCurrent'
        : (app.settings.switchAt ?? 100) < 100
          ? 'accounts.quota.overageMoving'
          : 'accounts.quota.overage',
      { names: name },
    );

  /** A harness without a quota (OpenCode): its cost of the day out of its budget, and where it would end at midnight. */
  function budgetOf(cost: number, budget: number) {
    const end = projectionMode() === 'off' ? null : projectedDayCost(cost, app.now);
    return { pct: Math.min(100, (cost / budget) * 100), end, level: end === null ? null : projectionLevel((end / budget) * 100) };
  }
</script>

<svelte:window onkeydowncapture={onWindowKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onclose}></div>
<div
  class="panel"
  {id}
  use:trapFocus
  role="dialog"
  tabindex="-1"
  aria-modal="true"
  aria-label={t('accounts.panel.title')}
  style:left="{place.left}px"
  style:bottom="{place.bottom}px"
  style:width="{WIDTH}px"
>
  {#each groups as g (g.engine)}
    {@const noQuota = capsOf(g.engine)?.quota === 'none'}
    {@const page = usagePage(g.engine)}
    <section class="harness" aria-label={t('accounts.tab.group', { harness: g.name })}>
      <h3><span class="emark">{g.name.split(' ')[0]}</span>{t('accounts.tab.group', { harness: g.name })}</h3>
      {#each g.rows as r (r.account.id)}
        {@const u = r.usage}
        {@const meters = metersShown(u, g.engine)}
        <div
          class="acct"
          class:off={r.reasons.length > 0}
          role="group"
          aria-label={accountName(r.account)}
          aria-current={r.current ? 'true' : undefined}
        >
          <span class="name"
            >{accountName(r.account)}{#if r.current}<span class="tag cur">{t('accounts.panel.current')}</span>{/if}{#if r.over}<span
                class="tag over">{t('accounts.panel.over')}</span
              >{/if}</span
          >
          <!-- A harness that tells no cost (Cursor CLI) has none of the day to show. -->
          {#if !page}<span class="today" use:tip={r.today.estimated ? estimateHint(r.today) : undefined}
              >{t('accounts.panel.today', { amount: fSpentUsd(r.today) })}</span
            >{/if}
          {#if r.reasons.length}<div class="why">{r.reasons.join(' · ')}</div>{/if}
          {#if page}
            <!-- Neither quota nor cost told to Escouade (Cursor CLI, ESC-43): where its usage is seen, and no pause for it. -->
            <div class="why">
              {t('accounts.panel.usageElsewhere', { harness: g.name })}
              <button class="link" onclick={() => app.run(openUrl(page))}>{t('accounts.panel.usageLink')}</button>
            </div>
          {:else if noQuota}
            {#if u?.dailyBudget}
              {@const b = budgetOf(u.todayCost, u.dailyBudget)}
              <div class="windows">
                <span class="w wide">
                  <span class="top"
                    ><span>{t('accounts.panel.budget')}</span><span
                      ><b>{fUsd(u.todayCost)} / {fUsd(u.dailyBudget)}</b>{#if b.end !== null}<span class="p {b.level}"
                          ><span class="sr">{t('accounts.quota.projectedSr')}</span>{t('accounts.quota.projectedCost', {
                            amount: fUsd(b.end),
                          })}</span
                        >{/if}</span
                    ></span
                  >
                  <span class="gauge" class:hot={b.pct > 80}><span class="fill" style:width="{b.pct}%"></span></span>
                  <span>{t('accounts.panel.noQuota')} {t('accounts.panel.budgetReset')}</span>
                </span>
              </div>
            {:else}
              <div class="why">{t('accounts.panel.noQuota')}</div>
            {/if}
          {:else if !u || u.connected || u.meters.length}
            <!-- An account that is not signed in and was never read has no meters to show. -->
            <div class="windows">
              {#each meters as m (m.minutes ?? 'credits')}
                <QuotaMeter minutes={m.minutes} usage={m.usage} now={app.now} detail />
              {/each}
            </div>
          {/if}
          {#if u?.overage}
            <p class="bill"><Icon name="info" size={14} /><span>{overage(r.account.id, accountName(r.account), g.engine)}</span></p>
          {/if}
        </div>
      {/each}
    </section>
  {/each}
  {#if estimated}<p class="note">{t('stats.cost.estimatedHint')}</p>{/if}
  {#if projectionMode() !== 'off'}<p class="note">
      {t(projectionMode() === 'always' ? 'accounts.panel.projectionNoteAlways' : 'accounts.panel.projectionNote')}
    </p>{/if}
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
  }
  .panel {
    position: fixed;
    z-index: 41;
    max-width: calc(100vw - 16px);
    max-height: calc(100vh - 60px);
    overflow: auto;
    display: flex;
    flex-direction: column;
    padding: 6px 0 4px;
    border-radius: 13px;
    background: var(--raise);
    box-shadow:
      inset 0 0 0 1px var(--line-2),
      0 24px 64px -12px rgba(0, 0, 0, calc(0.75 * var(--shade)));
    font-size: 12.5px;
    color: var(--text);
    animation: rise 0.3s var(--ease);
    outline: none;
  }
  .panel:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: -2px;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .panel {
      animation: none;
    }
  }
  h3 {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    padding: 10px 16px 6px;
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
  /* The harness's mark, a word in a hairline (as EngineMark draws it). */
  .emark {
    display: inline-flex;
    align-items: center;
    height: 17px;
    padding: 0 5px;
    border-radius: 4px;
    box-shadow: inset 0 0 0 1px var(--line-3);
    color: var(--soft);
    font-size: 10.5px;
    font-weight: 600;
  }
  .acct {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 6px 12px;
    padding: 9px 16px;
    border-top: 1px solid rgba(var(--fg-rgb), 0.05);
  }
  h3 + .acct {
    border-top: 0;
  }
  .acct.off {
    opacity: 0.55;
  }
  .name {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    font-weight: 560;
  }
  .tag {
    font-size: 11.5px;
    font-weight: 500;
  }
  .cur {
    color: var(--run);
  }
  .over {
    color: var(--wait);
  }
  .today {
    color: var(--muted);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    text-align: right;
    white-space: nowrap;
  }
  .why,
  .windows,
  .bill {
    grid-column: 1 / -1;
  }
  .why {
    color: var(--muted);
    font-size: 12px;
  }
  /* Where the usage of a harness that tells none is seen (Cursor CLI). */
  .why .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--p-ink);
    font: inherit;
    text-decoration: underline;
    text-underline-offset: 2px;
    cursor: pointer;
  }
  .windows {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 14px;
  }
  .w {
    display: grid;
    gap: 4px;
    color: var(--muted);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
  }
  .w.wide {
    grid-column: span 2;
  }
  .w .top {
    display: flex;
    justify-content: space-between;
  }
  .w b {
    color: var(--soft);
    font-weight: 550;
  }
  .w .gauge {
    width: 100%;
  }
  .p {
    margin-left: 4px;
    font-weight: 550;
  }
  .p.ok {
    color: var(--run);
  }
  .p.warn {
    color: var(--wait);
  }
  .p.over {
    color: var(--del);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
  /* Extra usage, billed apart: its sentence under the account. */
  .bill {
    display: flex;
    gap: 7px;
    align-items: flex-start;
    margin: 0;
    color: var(--warn-ink);
    font-size: 12px;
    line-height: 1.45;
  }
  .bill :global(.icon) {
    margin-top: 1px;
  }
  .note {
    margin: 0;
    padding: 8px 16px 10px;
    border-top: 1px solid var(--line);
    color: var(--muted);
    font-size: 11.5px;
    line-height: 1.45;
  }
</style>
