<script lang="ts">
  // An agent's conversation on the phone (MO-5, `docs/MOBILE.md` § 3.5): its messages read (Markdown, the tool calls in
  // rows, the last ones), the request it stopped on unfolded at its end, as in the thread; at the bottom, a message to
  // write or dictate (M9, the system's dictation), a photo of the phone joined to it. Read again every few seconds while
  // it is open; held in memory alone (M13). A space that is cut keeps what was read, dated and dimmed; sending waits for
  // the connection, the draft kept.
  import { onMount, tick, untrack } from 'svelte';
  import Icon from '$shared/components/Icon.svelte';
  import { fTime } from '$shared/lib/format';
  import { t } from '$shared/lib/i18n';
  import { renderMarkdown } from '$shared/lib/markdown';
  import { toolArg, toolIcon, toolLabel } from '$shared/lib/tools';
  import type { Attachment, ConvItem, ToolItem } from '$shared/lib/types';
  import { conversationOf, failText, sendMessage } from '../lib/act';
  import { currentThread } from '../lib/feed.svelte';
  import { draftOf, keepDraft } from '../lib/kept';
  import { closeTalk, type Talk } from '../lib/nav.svelte';
  import { phone, refreshSpace, stateOf } from '../lib/phone-spaces.svelte';
  import type { StoppedLine } from '../lib/thread';
  import { photoAttachment } from '../lib/voice';
  import Mic from './Mic.svelte';
  import Screen from './Screen.svelte';
  import StopFold from './StopFold.svelte';

  /** The most items shown: the rest is read at the computer. */
  const SHOWN = 120;
  const EVERY = 2500;

  let { talk }: { talk: Talk } = $props();

  const space = $derived(phone.spaces.find((s) => s.id === talk.space));
  const cut = $derived(!space || stateOf(space) !== 'open');
  const agent = $derived(space?.glance?.agents.find((a) => a.id === talk.agent) ?? null);
  /** Its name and project, kept once known: an agent that stops waiting leaves the glance. */
  let known = $state<{ name: string; project: string }>({ name: '', project: '' });
  $effect(() => {
    const p = space?.glance?.projects.find((x) => x.id === agent?.projectId);
    if (agent) known = { name: agent.name, project: p?.name ?? untrack(() => known.project) };
  });
  const stopped = $derived.by(() => {
    const th = currentThread();
    return th.stopped.find((l): l is StoppedLine => l.kind === 'stopped' && l.space.id === talk.space && l.agentId === talk.agent) ?? null;
  });

  let items = $state<ConvItem[] | null>(null);
  let readAt = $state(0);
  let error = $state<string | null>(null);
  let text = $state(untrack(() => draftOf(`${talk.space}:talk:${talk.agent}`)));
  let photos = $state<(Attachment & { url: string })[]>([]);
  let sending = $state(false);
  let said = $state<string | null>(null);
  let list = $state<HTMLElement>();
  let picker = $state<HTMLInputElement>();

  const shown = $derived.by(() => {
    const all = (items ?? []).filter((i) => !i.parent && ['user', 'text', 'tool'].includes(i.kind));
    return { earlier: Math.max(0, all.length - SHOWN), items: all.slice(-SHOWN) };
  });

  async function read(scroll = false) {
    try {
      const got = await conversationOf(talk.space, talk.agent);
      const grew = !items || got.length !== items.length;
      items = got;
      readAt = Date.now();
      error = null;
      if (scroll || grew) {
        await tick();
        const box = list?.parentElement;
        box?.scrollTo({ top: box.scrollHeight });
      }
    } catch (e) {
      if (!items) error = failText(e);
    }
  }

  onMount(() => {
    void read(true);
    const timer = setInterval(() => {
      if (!cut) void read();
    }, EVERY);
    return () => clearInterval(timer);
  });

  function write(v: string) {
    text = v;
    keepDraft(`${talk.space}:talk:${talk.agent}`, v);
  }

  async function pick(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    said = null;
    for (const f of Array.from(input.files ?? [])) {
      const a = await photoAttachment(f);
      if (a) photos = [...photos, a];
      else said = t('mobile.talk.photoTooBig');
    }
    input.value = '';
  }

  async function send() {
    if (cut || sending || (!text.trim() && !photos.length)) return;
    sending = true;
    said = null;
    const o = await sendMessage(
      { id: talk.space, name: space?.name ?? '', here: space?.here ?? false },
      talk.agent,
      text.trim(),
      photos.map(({ name, mediaType, data }) => ({ name, mediaType, data })),
    );
    sending = false;
    if (o.ok) {
      write('');
      photos = [];
      said = t('mobile.talk.sent');
      void read(true);
      void refreshSpace(talk.space);
    } else if ('error' in o) said = o.error;
  }

  const status = $derived(
    agent?.status === 'waiting'
      ? t('mobile.talk.waits')
      : agent && agent.status !== 'idle'
        ? t('mobile.talk.working')
        : t('mobile.talk.idle'),
  );
  const toolLine = (i: ToolItem) => toolArg(i, '');
</script>

<Screen
  title={known.name || stopped?.agentName || ''}
  sub={[space?.name, known.project, status].filter(Boolean).join(' · ')}
  label={known.name || stopped?.agentName || ''}
  onClose={closeTalk}
>
  <div class="conv" class:dim={cut} bind:this={list} data-testid="conversation">
    {#if cut && readAt}<p class="note warn" role="status">{t('mobile.talk.dated', { time: fTime(readAt) })}</p>{/if}
    {#if error}
      <p class="note" role="alert">{t('mobile.talk.failed', { error })}</p>
    {:else if !items}
      <p class="note" role="status">{t('mobile.talk.loading')}</p>
    {:else if !shown.items.length}
      <p class="note">{t('mobile.talk.empty')}</p>
    {:else}
      {#if shown.earlier}<p class="note">{t('mobile.talk.earlier', { count: shown.earlier })}</p>{/if}
      {#each shown.items as i (i.id)}
        {#if i.kind === 'user'}
          <div class="me">
            <p>{i.text}</p>
            {#if i.images}<span class="att"><Icon name="image" size={13} />{i.images}</span>{/if}
          </div>
        {:else if i.kind === 'text'}
          <div class="md ai">
            <!-- eslint-disable-next-line svelte/no-at-html-tags -- renderMarkdown sanitizes (DOMPurify) -->
            {@html renderMarkdown(i.text)}
          </div>
        {:else if i.kind === 'tool'}
          <p class="tool" class:err={i.status === 'error'}>
            <Icon name={toolIcon(i.name)} size={14} />
            <b>{toolLabel(i.name)}</b>
            <code>{toolLine(i)}</code>
            {#if i.status === 'ok'}<Icon name="check" size={14} />{/if}
          </p>
        {/if}
      {/each}
    {/if}
    {#if stopped}
      <div class="card">
        <p class="cardh">
          <Icon name="shield" size={15} />{stopped.stop === 'question'
            ? t('inboxViews.kind.question')
            : stopped.stop === 'plan'
              ? t('inboxViews.kind.plan')
              : t('inboxViews.kind.permission')} · {t('inboxViews.stopped')}
        </p>
        <StopFold line={stopped} {cut} inTalk onSettled={() => void read(true)} />
      </div>
    {/if}
  </div>
  {#snippet foot()}
    {#if cut}<p class="note warn" role="status">{t('mobile.talk.offline')}</p>{/if}
    {#if said}<p class="note" role="status">{said}</p>{/if}
    {#if photos.length}
      <div class="photos">
        {#each photos as p, n (n)}
          <span class="ph">
            <img src={p.url} alt={p.name} />
            <button type="button" aria-label={t('mobile.talk.removePhoto')} onclick={() => (photos = photos.filter((_, k) => k !== n))}>
              <Icon name="close" size={12} />
            </button>
          </span>
        {/each}
      </div>
    {/if}
    <div class="compose">
      <button type="button" class="ic" aria-label={t('mobile.talk.photo')} onclick={() => picker?.click()}
        ><Icon name="image" size={20} /></button
      >
      <input bind:this={picker} type="file" accept="image/*" hidden onchange={pick} data-testid="photo-input" />
      <textarea
        rows="1"
        placeholder={t('mobile.talk.placeholder', { agent: known.name })}
        value={text}
        oninput={(e) => write(e.currentTarget.value)}
        aria-label={t('mobile.talk.placeholder', { agent: known.name })}></textarea>
      <Mic onHeard={(h) => write(text ? `${text.trimEnd()} ${h}` : h)} />
      <button
        type="button"
        class="send"
        aria-label={t('mobile.talk.send')}
        disabled={cut || sending || (!text.trim() && !photos.length)}
        onclick={send}
      >
        <Icon name="up" size={20} />
      </button>
    </div>
  {/snippet}
</Screen>

<style>
  .conv {
    display: grid;
    gap: 12px;
    padding: 8px 0 4px;
    align-content: end;
    min-height: 100%;
  }
  .dim {
    opacity: 0.6;
  }
  .me {
    justify-self: end;
    max-width: 85%;
    padding: 10px 14px;
    border-radius: 16px;
    background: rgba(var(--fg-rgb), 0.07);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .me p {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .att {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-top: 4px;
    font-size: 12px;
    color: var(--muted);
  }
  .ai {
    overflow-wrap: anywhere;
    color: var(--text-ai);
  }
  .ai :global(p) {
    margin: 0 0 8px;
  }
  .ai :global(pre) {
    overflow-x: auto;
  }
  .tool {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 14px;
    color: var(--muted);
    min-width: 0;
  }
  .tool b {
    color: var(--soft);
    font-weight: 600;
  }
  .tool code {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--mono);
    font-size: 13px;
  }
  .tool.err b {
    color: var(--del);
  }
  .card {
    padding: 12px;
    border-radius: var(--r-l);
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .cardh {
    display: flex;
    align-items: center;
    gap: 7px;
    margin: 0 0 8px;
    color: var(--wait);
    font-weight: 650;
  }
  .note {
    margin: 0 0 6px;
    font-size: var(--small);
    color: var(--muted);
  }
  .warn {
    color: var(--wait);
  }
  .photos {
    display: flex;
    gap: 8px;
    margin-bottom: 8px;
  }
  .ph {
    position: relative;
    width: 56px;
    height: 56px;
    border-radius: 10px;
    overflow: hidden;
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .ph img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .ph button {
    position: absolute;
    top: 2px;
    right: 2px;
    width: 22px;
    height: 22px;
    border: 0;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.6);
    color: #fff;
  }
  .compose {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    padding: 4px;
    border-radius: 22px;
    background: rgba(var(--fg-rgb), 0.04);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  textarea {
    flex: 1;
    min-width: 0;
    min-height: var(--target);
    max-height: 140px;
    padding: 11px 4px;
    border: 0;
    outline: none;
    resize: none;
    field-sizing: content;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--body);
  }
  .ic,
  .send {
    flex: none;
    width: var(--target);
    height: var(--target);
    border: 0;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: transparent;
    color: var(--soft);
  }
  .send {
    background: var(--text);
    color: var(--panel);
  }
  .send:disabled {
    opacity: 0.35;
  }
</style>
