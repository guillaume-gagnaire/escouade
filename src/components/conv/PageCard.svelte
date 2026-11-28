<script lang="ts">
  // The card of a page an agent showed (`show_page`, AP-3; docs/AGENT-PAGES.md § 4.4, direction A), in place of the
  // row of its call: the page's icon, its title, « Page de pagination · modèle Revue · v2 · à côté », and « Afficher ».
  // Its states follow the page, as the backend says them: shown (its first version) or updated (a later one), its row
  // beside the conversation or waiting in « Navigateurs »; closed (by its agent, its ×, or a restart: « Rouvrir »); no
  // longer kept (its agent or project gone, room made); the pages off. A card always opens the last version.
  import { agentpages, know, latestOf, openFromCard, pageCall, pageKey, rowOnScreen, templateName } from '../../lib/agentpages.svelte';
  import { driveOf } from '../../lib/drives.svelte';
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { Agent, ToolItem } from '../../lib/types';
  import Icon from '../Icon.svelte';

  let { item, agent }: { item: ToolItem; agent: Agent } = $props();

  const call = $derived(pageCall(item));
  $effect(() => {
    if (call?.page) know(call.page);
  });
  const info = $derived(call?.page ? agentpages.info[call.page] : undefined);
  const off = $derived(app.settings.agentPages === false);

  /** What became of the page. */
  const phase = $derived.by((): 'running' | 'off' | 'erased' | 'beside' | 'waiting' | 'closed' => {
    if (!call?.page) return 'running';
    if (off) return 'off';
    if (info === null) return 'erased';
    if (info === undefined) return call.opened ?? 'waiting';
    if (!info.open) return 'closed';
    return rowOnScreen(pageKey(call.page)) ? 'beside' : 'waiting';
  });

  const version = $derived(call?.version ?? 1);
  const newer = $derived(info ? latestOf(info) : version);
  const name = $derived(agent.name || info?.agentName || '');
  const marks = $derived(call?.page ? (driveOf(pageKey(call.page))?.pins.length ?? 0) : 0);
  const meta = $derived(
    [
      t('devpage.card.by', { agent: name }),
      t('devpage.card.template', { name: templateName(call?.template ?? 'html') }),
      t('devpage.agentPage.version', { n: version }),
      ...(phase === 'running' || phase === 'off' ? [] : [t(`devpage.card.where.${phase}`)]),
      // Its agent's gestures on it (AB-8, direction A): « 3 repères », its rows under the card and its pins on the rail.
      ...(marks ? [t('devpage.drive.pins', { count: marks })] : []),
    ].join(' · '),
  );

  let opening = $state(false);
  async function open() {
    if (!call?.page || opening) return;
    opening = true;
    try {
      await openFromCard(call.page);
    } finally {
      opening = false;
    }
  }
</script>

{#if call}
  <div
    class="pagecard"
    class:dim={phase === 'erased' || phase === 'off'}
    role="group"
    aria-label={t('devpage.card.label', { agent: name })}
    data-page-card={call.page ?? ''}
    data-state={phase}
  >
    <div class="top">
      <span class="ic" aria-hidden="true"><Icon name="page" size={15} /></span>
      <span class="tt">
        <b class="ell">{call.title}</b>
        <span class="ell" use:tip={{ text: meta, aria: false }}>{phase === 'running' ? t('devpage.card.opening') : meta}</span>
      </span>
      {#if phase !== 'running'}
        <span class="tag" class:up={version > 1}>{version > 1 ? t('devpage.card.updated') : t('devpage.card.shown')}</span>
      {/if}
      {#if phase === 'beside' || phase === 'waiting'}
        <button class="btn" disabled={opening} onclick={open}><Icon name="pane-r" size={13} />{t('devpage.card.show')}</button>
      {:else if phase === 'closed'}
        <button class="btn" disabled={opening} onclick={open}><Icon name="restart" size={13} />{t('devpage.card.reopen')}</button>
      {/if}
    </div>
    {#if phase === 'erased' || phase === 'off' || (newer > version && phase !== 'running')}
      <div class="foot">
        {#if phase === 'erased'}
          <Icon name="info" size={13} /><span class="ell">{t('devpage.card.erased')}</span>
        {:else if phase === 'off'}
          <Icon name="info" size={13} /><span class="ell">{t('devpage.card.off')}</span>
        {:else}
          <Icon name="sync" size={13} /><span class="ell">{t('devpage.card.newer', { n: newer })}</span>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  /* The design's card (direction A): a raised tile on a hairline, the page's icon in the project's ink. */
  .pagecard {
    display: flex;
    flex-direction: column;
    min-width: 0;
    margin: 4px 0;
    overflow: hidden;
    border-radius: 10px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .top {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    padding: 10px 10px 10px 12px;
  }
  .ic {
    flex: none;
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: 8px;
    color: var(--p-ink);
    background: color-mix(in oklch, var(--p) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--p) 30%, transparent);
  }
  .tt {
    flex: 1 1 auto;
    display: grid;
    gap: 1px;
    min-width: 0;
  }
  .tt b {
    font-size: 13px;
    font-weight: 600;
  }
  .tt span {
    color: var(--muted);
    font-size: 11.5px;
  }
  .ell {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* « Montrée » or « Mise à jour »: a quiet word, not a colour. */
  .tag {
    flex: none;
    height: 20px;
    padding: 0 7px;
    border-radius: 5px;
    display: inline-flex;
    align-items: center;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
    font-size: 11px;
    font-weight: 550;
    white-space: nowrap;
  }
  .tag.up {
    color: var(--p-ink);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--p) 32%, transparent);
  }
  .btn {
    flex: none;
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    padding: 8px 12px;
    box-shadow: inset 0 1px 0 var(--line);
    color: var(--muted);
    font-size: 12px;
  }
  .foot :global(.icon) {
    flex: none;
  }
  .dim .ic {
    color: var(--muted);
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  /* Narrow conversation (a pane at 1024): the tag goes, the title and its line wrap, the button goes under them. */
  @container (max-width: 420px) {
    .tag {
      display: none;
    }
    .top {
      flex-wrap: wrap;
      row-gap: 8px;
    }
    .tt {
      flex: 1 1 calc(100% - 40px);
    }
    .tt .ell {
      white-space: normal;
    }
    .top .btn {
      margin-left: 40px;
    }
  }
</style>
