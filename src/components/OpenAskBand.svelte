<script lang="ts">
  // The band « veut ouvrir » of an agent's conversation (ESC-678, docs/AGENT-BROWSERS.md § 3.2, choice C3): the agent
  // asked to open an address outside its project's launch (`open_browser`). Nothing is open until the user answers:
  // « Permettre une fois » opens it in a browser of the project, beside the conversation, « Refuser » tells the agent no
  // and it does not ask again. Either way the agent is told by a message (`devpage_open_answer`). One request at a time,
  // the oldest first. Native, in Escouade's window: no page can draw or click it.
  import { t } from '../lib/i18n';
  import { app } from '../lib/state.svelte';
  import type { BrowserOpenAsk } from '../lib/types';
  import { tip } from '../lib/tip';
  import Icon from './Icon.svelte';

  let {
    asks,
    onanswer,
  }: {
    /** The agent's requests waiting, the oldest first. */
    asks: BrowserOpenAsk[];
    onanswer: (ask: BrowserOpenAsk, allow: boolean) => void | Promise<void>;
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
  <div class="band" role="alert" data-testid="open-ask-band">
    <span class="ic" aria-hidden="true"><Icon name="globe" size={14} /></span>
    <p class="say">
      <b>{t('devpage.open.wants', { agent })}</b>
      <span class="host mono" use:tip={{ text: ask.url, aria: false }}>{host}</span>
      <span class="why">{t('devpage.open.where')}</span>
      {#if asks.length > 1}<span class="more">{t('devpage.open.more', { count: asks.length - 1 })}</span>{/if}
    </p>
    <div class="acts">
      <button class="btn" disabled={busy} onclick={() => answer(false)}>{t('devpage.open.refuse')}</button>
      <button class="btn primary" disabled={busy} onclick={() => answer(true)}>{t('devpage.open.allow')}</button>
    </div>
  </div>
{/if}

<style>
  /* A strip above the composer, as the conversation's other bands: the warning's ink, one line that wraps when narrow. */
  .band {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 10px;
    margin: 0 16px 8px;
    padding: 8px 10px 8px 12px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--wait-bg);
  }
  .ic {
    display: inline-flex;
    flex: none;
    color: var(--warn-ink);
  }
  .say {
    flex: 1 1 240px;
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
    overflow-wrap: anywhere;
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
    margin-left: auto;
  }
</style>
