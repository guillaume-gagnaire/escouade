<script lang="ts">
  // The agent's page a request comes with, in its panel of « À toi » (AP-4; docs/AGENT-PAGES.md § 5): a thumbnail of its
  // template (a sketch drawn by the window, never the page itself), its title, its template and version, and « Ouvrir la
  // page », which opens its row beside its agent's conversation. One answers in the box or in the page: the same request.
  import { openFromCard, templateName } from '../../lib/agentpages.svelte';
  import { t } from '../../lib/i18n';
  import Icon from '../Icon.svelte';

  let { page }: { page: { page: string; title: string; template: string; version: number } } = $props();

  const sketch = $derived(['review', 'diagram', 'compare'].includes(page.template) ? page.template : 'html');
</script>

<div class="pthumb" data-testid="ask-page" data-page={page.page}>
  <span class="mini {sketch}" aria-hidden="true">
    {#if sketch === 'review'}
      <i class="ln w8"></i><i class="ck"></i><i class="ln w6"></i><i class="ck"></i><i class="ln w7"></i>
    {:else if sketch === 'diagram'}
      <i class="bx a"></i><i class="bx b"></i><i class="bx c"></i><i class="ar"></i>
    {:else if sketch === 'compare'}
      <i class="col"></i><i class="col"></i>
    {:else}
      <i class="ln w8"></i><i class="ln w5"></i><i class="ln w7"></i>
    {/if}
  </span>
  <span class="txt">
    <span class="lbl">{t('inbox.ask.page')}</span>
    <span class="nm">{page.title}</span>
    <span class="meta">{t('inbox.ask.pageMeta', { template: templateName(page.template), n: page.version })}</span>
  </span>
  <button class="btn" onclick={() => openFromCard(page.page)} data-testid="ask-open-page"
    ><Icon name="page" size={13} />{t('inbox.ask.openPage')}</button
  >
</div>

<style>
  .pthumb {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 10px 0 2px;
    padding: 8px 10px 8px 8px;
    border-radius: 10px;
    background: rgba(var(--fg-rgb), 0.025);
    box-shadow: inset 0 0 0 1px var(--line);
    min-width: 0;
  }
  /* The template's sketch: a little sheet, in the project's ink. */
  .mini {
    flex: none;
    position: relative;
    width: 76px;
    height: 50px;
    padding: 7px 8px;
    border-radius: 6px;
    background: var(--panel, var(--bg));
    box-shadow: inset 0 0 0 1px var(--line-2);
    display: flex;
    flex-wrap: wrap;
    align-content: flex-start;
    gap: 4px;
    overflow: hidden;
  }
  .mini i {
    display: block;
  }
  .ln {
    height: 4px;
    border-radius: 2px;
    background: rgba(var(--fg-rgb), 0.16);
  }
  .w5 {
    width: 50%;
  }
  .w6 {
    width: 60%;
  }
  .w7 {
    width: 70%;
  }
  .w8 {
    width: 82%;
  }
  .ck {
    width: 4px;
    height: 4px;
    border-radius: 2px;
    background: var(--ok, var(--p-ink));
  }
  .bx {
    position: absolute;
    width: 20px;
    height: 12px;
    border-radius: 3px;
    box-shadow: inset 0 0 0 1.5px var(--p-ink);
  }
  .bx.a {
    left: 8px;
    top: 8px;
  }
  .bx.b {
    right: 8px;
    top: 8px;
  }
  .bx.c {
    left: 28px;
    bottom: 7px;
  }
  .ar {
    position: absolute;
    left: 28px;
    right: 28px;
    top: 14px;
    height: 1.5px;
    background: var(--p-ink);
  }
  .col {
    width: calc(50% - 2px);
    height: 100%;
    border-radius: 3px;
    background: rgba(var(--fg-rgb), 0.08);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .txt {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .lbl,
  .meta {
    color: var(--muted);
    font-size: 11.5px;
  }
  .nm {
    font-size: 13px;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .btn {
    flex: none;
  }
</style>
