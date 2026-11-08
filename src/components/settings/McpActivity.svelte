<script lang="ts">
  import { onMount } from 'svelte';
  import { fDateTime, fTime } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { mcp } from '../../lib/mcp.svelte';
  import { tip } from '../../lib/tip';

  // « Activité MCP », in Agents › Avancé: what the server of « Piloter Escouade » was asked, the most recent first, with
  // « Effacer ».
  onMount(() => {
    void mcp.load().catch(() => {});
  });
  const calls = $derived([...mcp.activity].reverse());
</script>

<div class="act">
  <h3>{t('mcp.activity.title')}</h3>
  <button class="btn" disabled={!calls.length} onclick={() => mcp.clear()}>{t('common.clear')}</button>
</div>
{#if calls.length}
  <!-- A list that scrolls is reached with the keyboard to be read. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <ul class="log" tabindex="0" aria-label={t('mcp.activity.list')}>
    {#each calls as e}
      <li class={e.outcome} class:bad={e.outcome !== 'ok'}>
        <span class="tm" use:tip={fDateTime(e.at)}>{fTime(e.at)}</span>
        <span class="who">{e.caller}</span>
        <span class="wh"
          ><b class="tool mono">{e.tool || '—'}</b>{#if e.summary}<span class="sum"> · {e.summary}</span>{/if}</span
        >
        <span class="res">{t(`mcp.activity.outcome.${e.outcome}`)}</span>
        {#if e.message}<span class="msg">{e.message}</span>{/if}
      </li>
    {/each}
  </ul>
{:else}
  <p class="empty">{t('mcp.activity.empty')}</p>
{/if}

<style>
  p {
    margin: 0;
    font-size: 12px;
    line-height: 1.5;
  }
  .act {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding-top: 14px;
  }
  .act h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
  }
  .bad {
    color: var(--del);
  }
  /* The calls, a line each under a hairline (`.log` of the 2.0 design). */
  .log {
    display: grid;
    max-height: 320px;
    margin: 8px 0 0;
    padding: 0;
    overflow-y: auto;
    list-style: none;
    border-top: 1px solid rgba(var(--fg-rgb), 0.05);
  }
  .log:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 1px;
  }
  .log li {
    display: grid;
    grid-template-columns: 52px 150px minmax(0, 1fr) auto;
    align-items: center;
    gap: 2px 12px;
    padding: 8px 0;
    border-bottom: 1px solid rgba(var(--fg-rgb), 0.05);
    font-size: 12px;
  }
  .tm {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .who,
  .wh {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .who {
    color: var(--soft);
  }
  .wh {
    color: var(--muted);
  }
  .wh b {
    color: var(--text);
    font-size: 11.5px;
    font-weight: 550;
  }
  .res {
    color: var(--add);
    font-size: 11.5px;
  }
  .refused .res {
    color: var(--warn-ink);
  }
  .error .res,
  .msg {
    color: var(--del);
  }
  .msg {
    grid-column: 3 / -1;
    min-width: 0;
    font-size: 11.5px;
    overflow-wrap: anywhere;
  }
  .empty {
    margin-top: 8px;
    color: var(--muted);
  }
  @container settings (max-width: 520px) {
    .log li {
      grid-template-columns: 44px minmax(0, 1fr) auto;
    }
    .log .who {
      display: none;
    }
  }
</style>
