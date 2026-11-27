<script lang="ts">
  // « <agent> veut remplir un mot de passe » (choice C2; ESC-679 asks, ESC-680 draws it; docs/AGENT-BROWSERS.md § 5.5):
  // an agent driving the project's launch met a password field. Native, in Escouade's window above the page, never in
  // it. One request at a time; « Permettre » lets that agent fill passwords in this browser until it closes (the value
  // is never read back), « Refuser » tells it no for good. Either way the agent is told by a message.
  import { t } from '../lib/i18n';
  import { app } from '../lib/state.svelte';
  import Icon from './Icon.svelte';

  let {
    asks,
    onanswer,
  }: {
    /** The agents asking, the oldest first. */
    asks: string[];
    onanswer: (agent: string, allow: boolean) => void | Promise<void>;
  } = $props();

  const ask = $derived(asks[0]);
  const agent = $derived(ask ? (app.agents[ask]?.name ?? ask) : '');
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
  <div class="band" role="alert" data-testid="password-band">
    <span class="ic" aria-hidden="true"><Icon name="lock" size={14} /></span>
    <p class="say">
      <b>{t('devpage.drive.password', { agent })}</b>
      <span class="why">{t('devpage.drive.passwordWhy')}</span>
    </p>
    <div class="acts">
      <button class="btn" disabled={busy} onclick={() => answer(false)}>{t('devpage.consent.refuse')}</button>
      <button class="btn primary" disabled={busy} onclick={() => answer(true)}>{t('devpage.consent.allow')}</button>
    </div>
  </div>
{/if}

<style>
  /* The strip of « veut lire » (ConsentBand): under the browser's bar, the warning's ink. */
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
  .why {
    color: var(--muted);
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
