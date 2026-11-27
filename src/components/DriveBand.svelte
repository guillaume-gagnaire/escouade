<script lang="ts" module>
  /** Who has the hand on a driven row (docs/AGENT-PAGES.md § 10): its agent, or the user who took it back. */
  export type Hand = 'agent' | 'user';
</script>

<script lang="ts">
  // The driving band, common to every row of « Navigateurs » (AP-3; docs/AGENT-PAGES.md § 10, « Ce que voit
  // l'utilisateur »): native, above the row and outside the rectangle of its webview, so that no page can draw it. On an
  // agent's page it carries the trust mention (« Écrite par <agent> · hors réseau, sans accès à Escouade ni à tes
  // fichiers »). When an agent drives the row (AB-8, ESC-680: wired for the pages and the tabs alike): « <agent> pilote »,
  // the caption of its last gesture with its number, « Reprendre la main », or « Tu as la main · La rendre », and
  // « Continuer » while the agent waits for it; « Tu ne laisses plus piloter » once the user ended it. Always there while
  // a row is driven: the user always sees that an agent drives. Nobody drives: the trust mention alone on an agent's
  // page, nothing at all on a tab.
  import { t } from '../lib/i18n';
  import { tip } from '../lib/tip';
  import Icon from './Icon.svelte';

  let {
    agent,
    trust = false,
    hand = null,
    step = null,
    last = '',
    waiting = false,
    stopped = false,
    ontake,
    ongive,
    oncontinue,
  }: {
    /** The agent the row is about: the author of the page, the one that drives it. */
    agent: string;
    /** An agent's page: the trust mention is said. */
    trust?: boolean;
    /** Who has the hand while the row is driven; null: nobody drives it. */
    hand?: Hand | null;
    /** The number of the agent's last gesture (its row in the conversation, its pin on the rail). */
    step?: number | null;
    /** What that gesture did, as the core says it (« Met en évidence le critère 3 »). */
    last?: string;
    /** The agent waits for « Continuer » (`wait_user`). */
    waiting?: boolean;
    /** « Ne plus laisser piloter »: the user ended the driving until the row closes. */
    stopped?: boolean;
    ontake?: () => void;
    ongive?: () => void;
    oncontinue?: () => void;
  } = $props();

  const driven = $derived(hand !== null);
  const trustText = $derived(t('devpage.drive.trust', { agent }));
</script>

{#if trust || driven}
  <div class="band" class:driven role="group" aria-label={t('devpage.drive.label')} data-testid="drive-band">
    {#if trust}
      <span class="trust" use:tip={{ text: `${trustText}. ${t('devpage.drive.trustTip')}`, aria: false }}
        ><Icon name="shield" size={12} /><span class="ell">{trustText}</span></span
      >
    {/if}
    {#if driven}
      <span class="sp"></span>
      {#if hand === 'agent' && last}
        <span class="last" data-testid="drive-last" use:tip={{ text: last, aria: false }}
          >{#if step !== null}<span class="pin" aria-hidden="true">{step}</span>{/if}<span class="ell"
            >{#if step !== null}<span class="sr">{t('devpage.drive.lastSr', { n: step, what: '' })}</span>{/if}{last}</span
          ></span
        >
      {/if}
      <span class="drv" class:mine={hand === 'user'} class:off={stopped} data-testid="drive-hand" data-hand={stopped ? 'stopped' : hand}>
        <span class="pulse" aria-hidden="true"></span>
        <span class="lbl" role="status"
          >{stopped
            ? t('devpage.drive.stopped')
            : hand === 'agent'
              ? t('devpage.drive.drives', { agent })
              : t('devpage.drive.youHave')}</span
        >
        {#if stopped}
          <span class="why">{t('devpage.drive.stoppedTip')}</span>
        {:else if hand === 'agent'}
          <button class="take" use:tip={t('devpage.drive.takeTip')} onclick={() => ontake?.()}>{t('devpage.drive.take')}</button>
        {:else}
          <button class="take" use:tip={t('devpage.drive.giveTip', { agent })} onclick={() => ongive?.()}>{t('devpage.drive.give')}</button>
        {/if}
      </span>
      {#if waiting && !stopped}
        <button class="btn primary cont" use:tip={t('devpage.drive.continueTip', { agent })} onclick={() => oncontinue?.()}
          >{t('devpage.drive.continue')}</button
        >
      {/if}
    {/if}
  </div>
{/if}

<style>
  /* A thin native band under the row's bar: a whisper of ink, a hairline under it (the design's « bande de confiance »). */
  .band {
    flex: none;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 28px;
    padding: 0 8px 0 12px;
    background: rgba(var(--fg-rgb), 0.015);
    box-shadow: inset 0 -1px 0 var(--line);
    font-size: 11.5px;
    container: driveband / inline-size;
  }
  .band.driven {
    height: 36px;
  }
  .trust {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    flex: 0 1 auto;
    color: var(--muted);
  }
  .trust :global(.icon) {
    flex: none;
    color: var(--muted);
  }
  .ell {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .sp {
    flex: 1 1 0;
    min-width: 4px;
  }
  .last {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    flex: 0 1 auto;
    color: var(--soft);
  }
  /* While an agent drives, what it just did comes before who wrote the page: the mention yields first. */
  .band.driven .trust {
    flex-shrink: 6;
    min-width: 12px;
  }
  .band.driven .last {
    min-width: 72px;
  }
  /* The gesture's number: the same ③ as its row in the conversation and its pin on the rail. */
  .pin {
    flex: none;
    display: grid;
    place-items: center;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: 8px;
    background: color-mix(in oklch, var(--p) 22%, transparent);
    color: var(--p-ink);
    font-size: 10.5px;
    font-weight: 650;
    font-variant-numeric: tabular-nums;
  }
  /* Who has the hand: the agent in the project's colour, the user in ink. */
  .drv {
    flex: 0 1 auto;
    min-width: 0;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 26px;
    padding: 0 3px 0 9px;
    border-radius: 8px;
    color: var(--p-ink);
    background: color-mix(in oklch, var(--p) 11%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--p) 32%, transparent);
    font-size: 12px;
    font-weight: 550;
    white-space: nowrap;
  }
  /* Who drives yields before the buttons: « Reprendre la main » and « Continuer » stay whole. */
  .drv .lbl {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .drv .pulse,
  .take {
    flex: none;
  }
  .drv .pulse {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--p);
    box-shadow: 0 0 0 3px color-mix(in oklch, var(--p) 22%, transparent);
    animation: drive-pulse 1.6s var(--ease) infinite;
  }
  .drv.mine {
    color: var(--soft);
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .drv.mine .pulse {
    background: var(--muted);
    box-shadow: none;
    animation: none;
  }
  .drv.off {
    padding-right: 9px;
  }
  .why {
    color: var(--muted);
    font-weight: 450;
    font-size: 11.5px;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  .take {
    height: 20px;
    padding: 0 7px;
    border: 0;
    border-radius: 6px;
    background: rgba(var(--fg-rgb), 0.07);
    color: var(--text);
    font: inherit;
    font-size: 11.5px;
    cursor: pointer;
  }
  .take:hover {
    background: rgba(var(--fg-rgb), 0.12);
  }
  .take:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 1px;
  }
  .cont {
    flex: none;
    height: 26px;
    padding: 0 10px;
    font-size: 12px;
  }
  @keyframes drive-pulse {
    50% {
      box-shadow: 0 0 0 5px color-mix(in oklch, var(--p) 8%, transparent);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .drv .pulse {
      animation: none;
    }
  }
  /* A pane beside the conversation: while driven, the trust mention keeps its shield (its words in its tooltip). */
  @container driveband (max-width: 620px) {
    .band.driven .trust .ell {
      display: none;
    }
  }
  /* Narrow (a pane at 1024): the caption of the last gesture goes too, the band keeps who has the hand. */
  @container driveband (max-width: 460px) {
    .last,
    .why {
      display: none;
    }
  }
</style>
