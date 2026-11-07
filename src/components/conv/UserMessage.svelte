<script lang="ts">
  import type { Snippet } from 'svelte';
  import { t } from '../../lib/i18n';
  import { engineName, speakerOf } from '../../lib/engines';
  import type { EngineId, UserItem } from '../../lib/types';
  import { tip } from '../../lib/tip';
  import { withoutInstruction } from '../../lib/conversation';
  import EngineMark from '../EngineMark.svelte';
  import Icon from '../Icon.svelte';

  // `engine`: its agent's harness, which the note names; `extra`: more notes (a held message's).
  let {
    item,
    waiting = false,
    engine = null,
    extra,
  }: { item: UserItem; waiting?: boolean; engine?: EngineId | null; extra?: Snippet } = $props();
  // Said in a voice conversation: shown without the instruction Escouade added for the agent (ESC-97).
  const shown = $derived(withoutInstruction(item.text));
  const files = $derived(item.files ?? []);
  const attached = $derived(item.images > 0 || files.length > 0);
  let open = $state(false);
</script>

{#if item.origin === 'handover'}
  <!-- The handover file (ESC-88): written by Escouade, folded under its title. -->
  <div class="handover">
    <button class="fold" aria-expanded={open} onclick={() => (open = !open)}>
      <span class="chev" aria-hidden="true"><Icon name="right" size={14} /></span>
      <EngineMark {engine} />
      <b>{t('handover.file.title')}</b>
      <span class="grow d">{t('handover.file.hint', { harness: engineName(engine) })}</span>
    </button>
    {#if open}<div class="file-text">{item.text}</div>{/if}
  </div>
{:else}
  <div class="msg">
    <div class="bubble" class:with={attached}>
      <span class="text">{shown.text}</span>
      {#if attached}
        <div class="atts">
          {#if item.images > 0}
            <span class="att"
              ><span class="th"><Icon name="image" size={14} /></span><span class="nm">{t('conv.user.images', { count: item.images })}</span
              ></span
            >
          {/if}
          {#each files as name, i (i)}
            <span class="att" use:tip={{ text: name, cut: true }}
              ><span class="th"><Icon name="file" size={14} /></span><span class="nm">{name}</span></span
            >
          {/each}
        </div>
      {/if}
    </div>
    {#if shown.spoken || waiting || item.queued || item.origin === 'remote' || extra}
      <div class="meta">
        {#if shown.spoken}<span class="note aloud" data-said-aloud><Icon name="wave" size={14} />{t('conv.user.saidAloud')}</span>{/if}
        <!-- Sent while the worktree is prepared: the backend takes it once that is over. -->
        {#if waiting}<span class="note">{t('conv.user.waiting')}</span>{/if}
        <!-- Sent while Claude worked: the CLI takes it at the turn's next step (after the running tool). -->
        {#if item.queued}<span class="note" use:tip={t('conv.user.queuedHint', { engine: speakerOf(engine) })}>{t('conv.user.queued')}</span
          >{/if}
        {#if item.origin === 'remote'}<span class="note" use:tip={t('conv.user.remoteHint')}>{t('conv.user.remote')}</span>{/if}
        {@render extra?.()}
      </div>
    {/if}
  </div>
{/if}

<style>
  .handover {
    display: grid;
    gap: 6px;
    min-width: 0;
  }
  .fold {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-width: 0;
    padding: 9px 12px;
    border: none;
    border-radius: 9px;
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .fold:hover {
    background: var(--hover);
  }
  .fold b {
    flex: none;
    font-weight: 600;
  }
  .chev {
    display: flex;
    color: var(--muted);
    transition: transform 0.2s var(--ease);
  }
  .fold[aria-expanded='true'] .chev {
    transform: rotate(90deg);
  }
  .grow {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .d {
    color: var(--muted);
    font-size: 12px;
  }
  .file-text {
    max-height: 360px;
    overflow: auto;
    padding: 10px 12px;
    border-radius: 9px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--code-fg);
    font-family: var(--mono);
    font-size: 11.5px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .msg {
    display: grid;
    justify-items: end;
    gap: 6px;
  }
  .bubble {
    max-width: 82%;
    padding: 10px 14px;
    border-radius: 14px 14px 4px 14px;
    background: var(--raise-2);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 13.5px;
    line-height: 1.5;
    text-wrap: pretty;
    overflow-wrap: anywhere;
  }
  .bubble.with {
    display: grid;
    gap: 10px;
  }
  /* Only the message keeps its line breaks and spaces, not the template's. */
  .text {
    white-space: pre-wrap;
  }
  .atts {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .att {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    max-width: 260px;
    height: 40px;
    padding: 0 10px 0 6px;
    border-radius: 9px;
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line-2);
    font-size: 12px;
  }
  .th {
    flex: none;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    background: rgba(var(--fg-rgb), 0.06);
    color: var(--soft);
  }
  .nm {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 550;
  }
  /* Under the message: said aloud, waiting, passed on during the turn, sent from claude.ai. */
  .meta {
    display: flex;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 10px;
  }
  .note {
    color: var(--muted);
    font-size: 11.5px;
  }
  .aloud {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  @media (prefers-reduced-motion: reduce) {
    .chev {
      transition: none;
    }
  }
</style>
