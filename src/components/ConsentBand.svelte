<script lang="ts">
  // The band « veut lire » of a browser (ESC-669, docs/AGENT-BROWSERS.md § 3 bis, choice C4): an agent asked to read a
  // page that is not on its project's launch (an artifact, a site where the user is signed in). Native, in Escouade's
  // window above the page, never in it: the page can neither draw nor click it. One request at a time, the oldest
  // first; « Permettre » lets that agent read that origin in this browser until it closes, « Refuser » tells it no.
  // Either way the agent is told by a message (`devpage_consent`).
  import { t } from '../lib/i18n';
  import { app } from '../lib/state.svelte';
  import type { PageConsent } from '../lib/types';
  import Icon from './Icon.svelte';

  let {
    asks,
    onanswer,
  }: {
    /** The requests waiting, the oldest first. */
    asks: PageConsent[];
    onanswer: (ask: PageConsent, allow: boolean) => void;
  } = $props();

  const ask = $derived(asks[0]);
  const agent = $derived(ask ? (app.agents[ask.agent]?.name ?? ask.agent) : '');
  const host = $derived.by(() => {
    if (!ask) return '';
    try {
      return new URL(ask.origin).host;
    } catch {
      return ask.origin;
    }
  });
  let busy = $state(false);

  async function answer(allow: boolean) {
    if (!ask || busy) return;
    busy = true;
    try {
      await onanswer(ask, allow);
    } finally {
      busy = false;
    }
  }
</script>

{#if ask}
  <div class="band" role="alert" data-testid="consent-band">
    <span class="ic" aria-hidden="true"><Icon name="info" size={14} /></span>
    <p class="say">
      <b>{t('devpage.consent.wants', { agent })}</b>
      <span class="host mono">{host}</span>
      <span class="why">{t('devpage.consent.session')}</span>
      {#if asks.length > 1}<span class="more">{t('devpage.consent.more', { count: asks.length - 1 })}</span>{/if}
    </p>
    <div class="acts">
      <button class="btn" disabled={busy} onclick={() => answer(false)}>{t('devpage.consent.refuse')}</button>
      <button class="btn primary" disabled={busy} onclick={() => answer(true)}>{t('devpage.consent.allow')}</button>
    </div>
  </div>
{/if}

<style>
  /* A strip of the window, under the browser's bar: the same height as a toolbar row, the warning's ink. */
  .band {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 40px;
    padding: 6px 12px 6px 14px;
    border-bottom: 1px solid var(--line);
    background: var(--wait-bg);
    flex: none;
  }
  .ic {
    display: inline-flex;
    flex: none;
    color: var(--warn-ink);
  }
  .say {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: 12.5px;
    line-height: 1.4;
    color: var(--text);
  }
  .say b {
    font-weight: 600;
  }
  .host {
    margin: 0 2px;
    padding: 1px 5px;
    border-radius: 5px;
    background: rgba(var(--fg-rgb), 0.06);
    font-size: 12px;
  }
  .why,
  .more {
    color: var(--muted);
  }
  .more {
    margin-left: 6px;
  }
  .acts {
    display: flex;
    flex: none;
    gap: 6px;
  }
  @container devpage (max-width: 560px) {
    .why {
      display: none;
    }
  }
</style>
