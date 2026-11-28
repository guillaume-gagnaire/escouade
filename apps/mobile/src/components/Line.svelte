<script lang="ts">
  // A line of the thread (MO-1, MO-5, direction B): its sort in a square (amber for an agent that stopped), what it is,
  // since when and its dot until it was seen; where it is (its sort, the space's plate and name, the project's dot and
  // name); a choice shows its renders. Touched, it unfolds on the spot, with what answers it.
  import Icon from '$shared/components/Icon.svelte';
  import SpacePlate from '$shared/components/servers/SpacePlate.svelte';
  import { fSince } from '$shared/lib/format';
  import { imagesOf, isLight } from '$shared/lib/inbox/rules';
  import type { ViewSource } from '$shared/lib/inbox/views';
  import { t } from '$shared/lib/i18n';
  import type { IconName } from '$shared/lib/icons';
  import type { AskKind, WindowAsk } from '$shared/lib/types';
  import type { Line } from '../lib/thread';
  import AskFold from './AskFold.svelte';
  import AskImg from './AskImg.svelte';
  import StopFold from './StopFold.svelte';
  import ViewFold from './ViewFold.svelte';

  let {
    line,
    open,
    dot,
    cut,
    now,
    onToggle,
    onSettled,
    onAnswered,
  }: {
    line: Line;
    open: boolean;
    dot: boolean;
    cut: boolean;
    now: number;
    onToggle: () => void;
    onSettled: () => void;
    onAnswered: (a: WindowAsk) => void;
  } = $props();

  const ASK_ICON: Record<AskKind, IconName> = { choice: 'choice', confirm: 'check-c', answer: 'edit', task: 'check' };
  const VIEW_ICON: Record<ViewSource, IconName> = {
    permission: 'shield',
    question: 'question',
    plan: 'sheet',
    ticketReview: 'kanban',
    ticketBlocked: 'kanban',
    decision: 'diagram',
    docs: 'read',
    review: 'pr',
    handoff: 'swap',
    teamAccept: 'team',
    scheduled: 'clock',
  };

  const icon = $derived.by((): IconName => {
    if (line.kind === 'stopped')
      return line.stop === 'question' ? 'question' : line.stop === 'plan' ? 'sheet' : line.stop === 'delete' ? 'trash' : 'shield';
    if (line.kind === 'ask') return line.ask ? ASK_ICON[line.ask.kind] : 'inbox';
    return VIEW_ICON[line.view.source];
  });

  const kind = $derived.by(() => {
    if (line.kind === 'stopped') return line.stop === 'delete' ? t('inbox.queue.stoppedWhat.delete') : t(`inboxViews.kind.${line.stop}`);
    if (line.kind === 'ask') return line.ask ? t(`inbox.kind.${line.ask.kind}`) : t('spaces.kind.ask');
    return t(`inboxViews.kind.${line.view.source}`);
  });

  const waits = $derived(line.kind === 'stopped');
  const thumbs = $derived.by(() => {
    if (line.kind !== 'ask' || !line.ask || line.ask.kind !== 'choice' || open) return [];
    const a = line.ask;
    const light = typeof document !== 'undefined' && document.documentElement.dataset.theme === 'light';
    return a.options
      .map((o) => imagesOf(a, o.id).find((f) => isLight(f) === light) ?? imagesOf(a, o.id)[0])
      .filter((f) => f !== undefined)
      .slice(0, 4);
  });
  const id = $derived(`line-${line.key.replace(/[^\w-]/g, '_')}`);
</script>

<li class="line" class:waits class:open data-key={line.key} data-testid="line">
  <button type="button" class="hd" data-testid="line-head" aria-expanded={open} aria-controls={`${id}-fold`} onclick={onToggle}>
    <span class="k"><Icon name={icon} size={16} /></span>
    <span class="tt">{line.title}</span>
    <span class="when">
      {#if dot}<i class="dot" aria-label={t('mobile.thread.unseen')}></i>{/if}
      {fSince(line.since, now)}
    </span>
    <span class="mt">
      <span class="kind">{kind}</span>
      <span class="s" aria-hidden="true">·</span>
      <SpacePlate name={line.space.name} here={line.space.here} size={20} />
      <span>{line.space.name}</span>
      {#if line.project}
        <span class="s" aria-hidden="true">·</span>
        <i class="pdot" style:background={line.project.color} aria-hidden="true"></i>
        <span class="pj">{line.project.name}</span>
      {/if}
    </span>
  </button>
  {#if thumbs.length && line.kind === 'ask' && line.ask}
    <span class="thumbs" aria-label={t('inbox.queue.thumbs')}>
      {#each thumbs as f (f.file)}<span class="th"><AskImg space={line.space.id} ask={line.ask} file={f} /></span>{/each}
    </span>
  {/if}
  {#if open}
    <div class="body" id={`${id}-fold`}>
      {#if line.kind === 'stopped'}
        <StopFold {line} {cut} {onSettled} />
      {:else if line.kind === 'ask' && line.ask}
        <AskFold space={line.space} ask={line.ask} {cut} {onSettled} {onAnswered} />
      {:else if line.kind === 'view'}
        <ViewFold space={line.space} view={line.view} {cut} {onSettled} />
      {:else}
        <p class="note">{t('mobile.answer.offline')}</p>
      {/if}
    </div>
  {/if}
</li>

<style>
  .line {
    position: relative;
  }
  .line + :global(.line) {
    box-shadow: inset 0 1px 0 var(--line);
  }
  .line.open {
    margin: 4px 8px;
    border-radius: var(--r-l);
    background: rgba(var(--fg-rgb), 0.035);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .line.open.waits {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .line.open + :global(.line) {
    box-shadow: none;
  }
  .hd {
    display: grid;
    grid-template-columns: 30px minmax(0, 1fr) auto;
    gap: 4px 12px;
    width: 100%;
    padding: 12px var(--gut);
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    align-items: start;
    min-height: var(--target);
  }
  .open .hd {
    padding: 12px calc(var(--gut) - 8px) 8px;
  }
  .k {
    grid-row: span 2;
    width: 30px;
    height: 30px;
    border-radius: 9px;
    display: grid;
    place-items: center;
    color: var(--soft);
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .waits .k {
    color: var(--wait);
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .tt {
    font-weight: 620;
    font-size: var(--body);
    line-height: 1.32;
    color: var(--text);
    overflow-wrap: anywhere;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .when {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--small);
    color: var(--muted);
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--teal-ink);
  }
  .mt {
    grid-column: 2 / 4;
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    font-size: var(--small);
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
  }
  .s {
    opacity: 0.6;
  }
  .pdot {
    width: 8px;
    height: 8px;
    border-radius: 3px;
    flex: none;
  }
  .pj {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .thumbs {
    display: flex;
    gap: 6px;
    padding: 0 var(--gut) 12px calc(var(--gut) + 42px);
    margin-top: -4px;
  }
  .th {
    width: 64px;
    height: 42px;
    border-radius: 6px;
    overflow: hidden;
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .body {
    padding: 0 calc(var(--gut) - 8px) 12px;
  }
  .note {
    margin: 0;
    font-size: var(--small);
    color: var(--muted);
  }
</style>
