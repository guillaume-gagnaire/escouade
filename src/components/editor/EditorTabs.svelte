<script lang="ts">
  import { tabHints } from '../../lib/editor/tabs';
  import type { FileStatus } from '../../lib/editor/tree';
  import { t } from '../../lib/i18n';

  let {
    tabs,
    onselect,
    onclose,
  }: {
    tabs: { path: string; name: string; dirty: boolean; status: FileStatus | null; active: boolean }[];
    onselect: (path: string) => void;
    onclose: (path: string) => void;
  } = $props();

  const SC: Record<string, string> = { M: 'var(--wait)', A: 'var(--add)', D: 'var(--del)' };
  /** The folder that tells a tab from the open ones of the same name; '' for a name that is alone. */
  const hints = $derived(tabHints(tabs.map((tab) => tab.path)));
</script>

<div class="tabs" role="tablist" aria-label={t('editor.tabs.label')}>
  {#each tabs as tab (tab.path)}
    {@const hint = hints[tab.path]}
    <div
      class="tab"
      class:on={tab.active}
      role="tab"
      tabindex="0"
      aria-selected={tab.active}
      title={tab.path}
      onclick={() => onselect(tab.path)}
      onkeydown={(e) => e.target === e.currentTarget && e.key === 'Enter' && onselect(tab.path)}
      onauxclick={(e) => e.button === 1 && onclose(tab.path)}
    >
      <span class="label"
        ><span class="name" style:color={tab.status ? SC[tab.status] : undefined}>{tab.name}</span>{#if hint}<span class="hint"
            >{' · ' + hint}</span
          >{/if}</span
      >
      <button
        class="close"
        class:dirty={tab.dirty}
        aria-label={hint ? t('editor.tabs.closeIn', { name: tab.name, folder: hint }) : t('editor.tabs.close', { name: tab.name })}
        title={tab.dirty ? t('editor.tabs.closeUnsaved') : t('common.close')}
        onclick={(e) => {
          e.stopPropagation();
          onclose(tab.path);
        }}><span class="dot"></span><span class="x">×</span></button
      >
    </div>
  {/each}
</div>

<style>
  .tabs {
    height: 36px;
    flex: none;
    display: flex;
    align-items: stretch;
    background: var(--bg);
    border-bottom: 1px solid var(--line);
    overflow-x: auto;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 6px 0 14px;
    border-right: 1px solid var(--line);
    border-top: 2px solid transparent;
    white-space: nowrap;
    cursor: pointer;
    color: var(--muted);
    font-size: 12.5px;
  }
  .tab.on {
    border-top-color: var(--accent);
    background: var(--term);
    color: var(--text);
  }
  .hint {
    color: var(--dim);
  }
  .close {
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 3px;
    background: transparent;
    color: var(--dim);
    font-size: 13px;
    cursor: pointer;
  }
  .close:hover {
    background: var(--elev2);
    color: var(--text);
  }
  .dot {
    display: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text);
  }
  .close.dirty .dot {
    display: block;
  }
  .close.dirty .x {
    display: none;
  }
  .close.dirty:hover .dot,
  .close.dirty:focus-visible .dot {
    display: none;
  }
  .close.dirty:hover .x,
  .close.dirty:focus-visible .x {
    display: inline;
  }
</style>
