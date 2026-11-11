<script lang="ts">
  import { fileCategory } from '../../lib/attachments';
  import { speakerOf } from '../../lib/engines';
  import { fList, fSize } from '../../lib/format';
  import { galleryOf } from '../../lib/gallery';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { EngineId, SentView, ToolItem } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import ImageViewer from './ImageViewer.svelte';
  import Markdown from './Markdown.svelte';

  // What an agent sent the user (Claude Code's SendUserFile, ESC-171): its caption, its images as thumbnails that open the
  // viewer (`display: "render"`; `"attach"` lists them as files), its other files with « Ouvrir » and « Afficher dans le
  // dossier ». The core reads them all, by the agent and the call, never by a path the window would give: an image copied
  // when it was sent shows still once its folder is gone; a file gone since, or of a private project, says so.
  let { item, agentId, engine = null }: { item: ToolItem; agentId: string; engine?: EngineId | null } = $props();

  const caption = $derived(typeof item.input?.caption === 'string' ? item.input.caption.trim() : '');
  const named = $derived<string[]>(Array.isArray(item.input?.files) ? item.input.files.filter((f: unknown) => typeof f === 'string') : []);
  const attach = $derived(item.input?.display === 'attach');

  let views = $state<SentView[] | null>(null);
  // Read again when the core says how it delivered them (`sent`), or once the call ends.
  $effect(() => {
    void item.sent;
    void item.status;
    const id = agentId;
    let gone = false;
    api.sentFiles(id, item.id).then(
      (v) => !gone && (views = v),
      () => !gone && (views = []),
    );
    return () => (gone = true);
  });

  /** Each file, by its rank in the call: what the core says of it, else (still read) its name alone. */
  const files = $derived(
    named.map((path, i) => ({ index: i, view: views?.[i] ?? null, name: views?.[i]?.name ?? path.split(/[\\/]/).pop() ?? path })),
  );
  const images = $derived(files.filter((f) => f.view?.kind === 'image' && f.view.url));
  const thumbs = $derived(attach ? [] : images);
  const rows = $derived(files.filter((f) => !thumbs.includes(f)));
  const imageCount = $derived(files.filter((f) => f.view?.kind === 'image').length);
  const fileCount = $derived(files.length - imageCount);
  const counted = $derived(
    fList([
      ...(imageCount ? [t('conv.sent.images', { count: imageCount })] : []),
      ...(fileCount ? [t('conv.sent.files', { count: fileCount })] : []),
    ]),
  );

  // The viewer pages through the images of the whole conversation, from the one clicked.
  let shown = $state<ReturnType<typeof galleryOf> | null>(null);
  const view = (e: MouseEvent) => {
    const img = (e.currentTarget as HTMLElement).querySelector('img');
    if (img) shown = galleryOf(img);
  };

  const act = (index: number, reveal: boolean) => app.run(api.sentFileOpen(agentId, item.id, index, reveal));
</script>

<div class="sent" data-sent={item.id}>
  <div class="by">
    <span class="mark"><Icon name={imageCount && !fileCount ? 'image' : 'clip'} size={13} /></span>
    <span>{t('conv.sent.by', { engine: speakerOf(engine) })}</span>
    {#if views}
      <span class="dot" aria-hidden="true">·</span>
      <span class="count">{counted}</span>
    {/if}
  </div>
  {#if caption}
    <div class="caption"><Markdown text={caption} {agentId} /></div>
  {/if}
  {#if thumbs.length}
    <div class="thumbs">
      {#each thumbs as f (f.index)}
        <button class="thumb" aria-label={t('conv.sent.view', { name: f.name })} use:tip={{ text: f.name, cut: true }} onclick={view}>
          <img
            src={f.view!.url}
            alt={f.name}
            draggable="false"
            data-view
            data-agent={agentId}
            data-item={item.id}
            data-index={f.index}
            data-name={f.name}
          />
        </button>
      {/each}
    </div>
  {/if}
  {#if rows.length}
    <div class="rows">
      {#each rows as f (f.index)}
        {@const v = f.view}
        {@const there = v?.state === 'ok'}
        <div class="row" class:off={v && !there}>
          {#if v?.kind === 'image' && v.url}
            <button class="th pic" aria-label={t('conv.sent.view', { name: f.name })} onclick={view}
              ><img
                src={v.url}
                alt=""
                draggable="false"
                data-view
                data-agent={agentId}
                data-item={item.id}
                data-index={f.index}
                data-name={f.name}
              /></button
            >
          {:else}
            <span class="th" aria-hidden="true"
              ><Icon name={v?.kind === 'image' ? 'image' : fileCategory(f.name) === 'code' ? 'code' : 'file'} size={14} /></span
            >
          {/if}
          <span class="nm">
            <b class="fname" use:tip={{ text: f.name, cut: true }}>{f.name}</b>
            <span class="sub">
              {#if !v}
                {t('conv.sent.loading')}
              {:else if v.state === 'missing'}
                <span class="warn" use:tip={t('conv.sent.missingHint')}>{t('conv.sent.missing')}</span>
              {:else if v.state === 'fenced'}
                <span class="warn"><Icon name="lock" size={11} />{t('conv.sent.fenced')}</span>
              {:else}
                {#if v.size != null}<span class="size">{fSize(v.size)}</span>{/if}
                {#if v.big}<span>· {t('conv.sent.big')}</span>{/if}
              {/if}
            </span>
          </span>
          {#if there}
            <span class="acts">
              <!-- A program or a script the agent sent is never run from here: shown in its folder only. -->
              {#if !v.runnable}
                <button class="act" aria-label={t('conv.sent.openName', { name: f.name })} onclick={() => act(f.index, false)}
                  ><Icon name="ext" size={13} /><span class="lbl">{t('common.open')}</span></button
                >
              {/if}
              <button
                class="act"
                aria-label={t('conv.sent.revealName', { name: f.name })}
                use:tip={t('conv.sent.reveal')}
                onclick={() => act(f.index, true)}><Icon name="folder" size={13} /></button
              >
            </span>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

{#if shown}
  <ImageViewer images={shown.images} bind:index={shown.index} onclose={() => (shown = null)} />
{/if}

<style>
  /* What the agent sent (2.0 design): on the assistant's side, its caption as its text, the images in a row that wraps,
     the files as the attachments of a message are (their icon, their name over their size). */
  .sent {
    display: grid;
    gap: 10px;
    min-width: 0;
  }
  .by {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .mark {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 5px;
    background: rgba(var(--fg-rgb), 0.06);
    color: var(--soft);
  }
  .count {
    font-variant-numeric: tabular-nums;
  }
  .caption {
    min-width: 0;
  }
  .thumbs {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .thumb {
    display: block;
    height: 140px;
    max-width: min(260px, 100%);
    padding: 0;
    overflow: hidden;
    border: none;
    border-radius: 9px;
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line-2);
    cursor: zoom-in;
    transition: box-shadow 0.15s var(--ease);
  }
  .thumb:hover {
    box-shadow:
      inset 0 0 0 1px var(--line-3, var(--line-2)),
      0 0 0 3px rgba(var(--fg-rgb), 0.06);
  }
  .thumb:focus-visible,
  .act:focus-visible,
  .pic:focus-visible {
    outline: 2px solid var(--focus, var(--p-ink));
    outline-offset: 2px;
  }
  .thumb img {
    display: block;
    height: 100%;
    width: auto;
    max-width: 100%;
    min-width: 60px;
    object-fit: cover;
  }
  .rows {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .row {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    max-width: 100%;
    min-width: 0;
    height: 40px;
    padding: 0 4px 0 6px;
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
    padding: 0;
    overflow: hidden;
    border: none;
    border-radius: 6px;
    background: rgba(var(--fg-rgb), 0.06);
    color: var(--soft);
  }
  .pic {
    cursor: zoom-in;
  }
  .pic img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .nm {
    display: grid;
    min-width: 0;
    line-height: 1.25;
  }
  .fname {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 550;
  }
  .sub {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .warn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--wait);
  }
  .off .fname {
    color: var(--soft);
  }
  .acts {
    display: flex;
    gap: 2px;
    margin-left: 4px;
  }
  .act {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 28px;
    padding: 0 8px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--soft);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .act:hover {
    background: var(--hover);
    color: var(--text);
  }
  /* A narrow conversation (the side panel open, a small window): smaller thumbnails, the actions by their icon. */
  @container conv (max-width: 560px) {
    .thumb {
      height: 96px;
    }
    .act .lbl {
      display: none;
    }
  }
</style>
