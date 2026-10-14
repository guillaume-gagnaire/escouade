<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { t } from '../lib/i18n';
  import { pullTip, shownPulls } from '../lib/pulls';
  import { app } from '../lib/state.svelte';
  import { tip } from '../lib/tip';
  import type { PullRequestLink } from '../lib/types';

  // The pull requests linked to an agent: the number of each, which opens its page, its state and, while it is to
  // merge, its checks and its reviews. One under the other on an agent's card, in a row in the header of its
  // conversation (`inline`); `max` of them, the others counted, and named when the count is pointed at.
  let { pulls, max = Infinity, inline = false }: { pulls: PullRequestLink[]; max?: number; inline?: boolean } = $props();

  const all = $derived(shownPulls(pulls));
  const shown = $derived(all.slice(0, max));
  const others = $derived(all.slice(max));
  /** What the checks say, at a glance; their words are in the tip, and read to a screen reader. */
  const CHECKS = { pending: '●', passing: '✓', failing: '✗' } as const;

  function open(e: Event, pr: PullRequestLink) {
    // On a card: the click is the link's, not the card's.
    e.stopPropagation();
    void app.run(openUrl(pr.url));
  }

  /** The keys that press the number are its own, not the card's (Enter selects its agent); every other one goes on, to the window's shortcuts. */
  function ownKeys(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') e.stopPropagation();
  }
</script>

{#if shown.length}
  <ul class="pulls mono" class:inline aria-label={t('git.pulls.label')}>
    {#each shown as pr (pr.url)}
      <li class="pr" data-state={pr.state || 'unknown'}>
        <span class="glyph" aria-hidden="true">⇡</span>
        <button
          class="num"
          aria-label={t('git.pulls.open', { n: pr.number })}
          use:tip={pullTip(pr)}
          onclick={(e) => open(e, pr)}
          onkeydown={ownKeys}>#{pr.number}</button
        >
        {#if pr.state}<span class="state">{t(`git.pulls.state.${pr.state}`)}</span>{/if}
        {#if pr.checks}
          <span class="checks" data-checks={pr.checks} use:tip={t(`git.pulls.checks.${pr.checks}`)}
            ><span aria-hidden="true">{CHECKS[pr.checks]}</span><span class="sr">{t(`git.pulls.checks.${pr.checks}`)}</span></span
          >
        {/if}
        {#if pr.review}<span class="review" data-review={pr.review}>{t(`git.pulls.review.${pr.review}`)}</span>{/if}
      </li>
    {/each}
    {#if others.length}
      <li class="more" use:tip={others.map(pullTip).join('\n')}>{t('git.pulls.more', { count: others.length })}</li>
    {/if}
  </ul>
{/if}

<style>
  .pulls {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
    font-size: 11px;
    color: var(--muted);
  }
  /* On one line, whatever their number: the header has one line for them. */
  .pulls.inline {
    flex-direction: row;
    gap: 12px;
    overflow: hidden;
  }
  .pr,
  .more {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
    white-space: nowrap;
  }
  .glyph,
  .more {
    color: var(--dim);
  }
  .num {
    padding: 0;
    border: none;
    background: none;
    color: var(--text);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
  }
  .num:hover {
    text-decoration: underline;
  }
  .state {
    color: var(--muted);
  }
  .pr[data-state='open'] .state {
    color: var(--info);
  }
  .pr[data-state='merged'] .state {
    color: var(--ok);
  }
  .pr[data-state='closed'] .state {
    color: var(--dim);
  }
  .checks[data-checks='passing'] {
    color: var(--ok);
  }
  .checks[data-checks='pending'] {
    color: var(--wait);
  }
  .checks[data-checks='failing'] {
    color: var(--del);
  }
  .review {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .review[data-review='approved'] {
    color: var(--ok);
  }
  .review[data-review='changes'] {
    color: var(--wait);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
