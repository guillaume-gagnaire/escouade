<script lang="ts">
  import { t } from '../../lib/i18n';
  import { firstChangedLine, patchLines } from '../../lib/diff';
  import { callTitle, isSubagentCall } from '../../lib/subagents';
  import { hasDiff, isQuietTool, toolArg, toolIcon, toolLabel, toolResultSummary } from '../../lib/tools';
  import type { ConvItem, ToolItem } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import Markdown from './Markdown.svelte';
  import LongText from './LongText.svelte';
  import PatchView from './PatchView.svelte';
  import Self from './ToolRow.svelte';
  import { tip } from '../../lib/tip';

  // `onOpenFile`: opens a file the tool edited (its path) in the editor, at a line. `onOpenSub`: opens the conversation
  // of the subagent a call launched (by the call's id), in place of the agent's. `linked`: the next item of the conversation
  // is a call too, which the thread ties to this one.
  let {
    item,
    cwd,
    linked = false,
    childrenOf = () => [],
    onOpenFile,
    onOpenSub,
  }: {
    item: ToolItem;
    cwd: string;
    linked?: boolean;
    childrenOf?: (id: string) => ConvItem[];
    onOpenFile?: (abs: string, line: number | null) => void;
    onOpenSub?: (call: string) => void;
  } = $props();
  let open = $state(false);

  const FILE_TOOLS = new Set(['Edit', 'Write', 'MultiEdit']);

  const children = $derived(childrenOf(item.id));
  const arg = $derived(toolArg(item, cwd));
  const diff = $derived(hasDiff(item));
  const res = $derived(toolResultSummary(item));
  const subTools = $derived(children.filter((c) => c.kind === 'tool') as ToolItem[]);
  const subText = $derived(
    children.filter((c) => c.kind === 'text' && c.text.trim()).at(-1) as { text: string; streaming: boolean } | undefined,
  );
  const expandable = $derived(item.status !== 'running' || children.length > 0);
  const file = $derived(
    onOpenFile && FILE_TOOLS.has(item.name) && typeof item.input?.file_path === 'string' ? (item.input.file_path as string) : null,
  );
  const firstLine = $derived(firstChangedLine(item.result?.patch));
  const sub = $derived(!!onOpenSub && isSubagentCall(item));
  const toggle = () => expandable && (open = !open);
</script>

<!-- Reading the list of tasks is no change to the plan: a step back, in softer colours. -->
<div class="tool" class:err={item.status === 'error'} class:quiet={isQuietTool(item.name)} class:linked={linked && !open}>
  <!-- Not a <button>: the edited file's path inside it is one. -->
  <div
    class="row"
    role="button"
    tabindex="0"
    aria-expanded={open}
    use:tip={{ text: arg, cut: true }}
    onclick={toggle}
    onkeydown={(e) => {
      // The keys of the path's button are its own.
      if (e.target !== e.currentTarget) return;
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        toggle();
      }
    }}
  >
    <span class="ic" aria-hidden="true"><Icon name={toolIcon(item.name)} size={14} /></span>
    <span class="badge">{toolLabel(item.name)}</span>
    {#if file}
      <button
        class="arg link"
        aria-label={t('conv.tool.openFile', { path: arg })}
        use:tip={t('common.openInEditor')}
        onclick={(e) => {
          e.stopPropagation();
          onOpenFile?.(file, firstLine);
        }}>{arg}</button
      >
      <!-- The link is as wide as its text: the rest of the row expands it. -->
      <span class="fill"></span>
    {:else}
      <span class="arg">{arg}</span>
    {/if}
    {#if item.status === 'running'}
      <span class="spin" role="img" aria-label={t('conv.tool.running')}></span>
    {:else if diff}
      <span class="res"><span class="add">+{item.result?.add}</span> <span class="del">−{item.result?.del}</span></span>
    {:else if res}
      <span class="res"
        >{#if item.status === 'ok'}<span class="ok"><Icon name="check" size={14} /></span>{/if}{res}</span
      >
    {/if}
    {#if subTools.length}<span class="res">{t('conv.tool.subTools', { count: subTools.length })}</span>{/if}
    {#if sub}
      <button
        class="open-sub"
        data-open-sub={item.id}
        data-open-from="call"
        aria-label={t('subagents.openRow', { name: callTitle(item) })}
        use:tip={t('subagents.openRow', { name: callTitle(item) })}
        onclick={(e) => {
          e.stopPropagation();
          onOpenSub?.(item.id);
        }}><Icon name="ext" size={14} /></button
      >
    {/if}
    {#if expandable}<span class="chev" aria-hidden="true"><Icon name="right" size={14} /></span>{/if}
  </div>
  {#if open}
    <div class="detail">
      {#if diff && item.result?.patch?.length}
        <PatchView lines={patchLines(item.result.patch)} />
      {:else if item.name === 'TodoWrite' && Array.isArray(item.input.todos)}
        <ul class="todos">
          {#each item.input.todos as todo, i (i)}
            <li class={todo.status}>
              <Icon name={todo.status === 'completed' ? 'check-c' : 'circle'} size={14} />
              {todo.content ?? todo.activeForm}
            </li>
          {/each}
        </ul>
      {:else if children.length}
        <div class="sub">
          {#if item.input.prompt}<div class="prompt">{item.input.prompt}</div>{/if}
          {#each subTools as c (c.id)}
            <Self item={c} {cwd} {childrenOf} {onOpenFile} {onOpenSub} />
          {/each}
          {#if subText}<div class="subtext"><Markdown text={subText.text} streaming={subText.streaming} /></div>{/if}
        </div>
      {:else}
        {#if item.name === 'Bash' || item.name === 'PowerShell'}
          <LongText class="out cmd" text={'$ ' + item.input.command} />
        {/if}
        {#if item.result?.text}
          <LongText class="out" text={item.result.text} />
        {:else if item.name !== 'Bash'}
          <LongText class="out" text={JSON.stringify(item.input, null, 2)} />
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  /* A call: one line of the turn's thread, not a box (2.0 design). Its detail, once open, keeps a frame. */
  .tool {
    position: relative;
    min-width: 0;
  }
  .row {
    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 30px;
    min-width: 0;
    padding: 0 8px 0 4px;
    border-radius: 7px;
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.62;
    text-align: left;
    cursor: pointer;
  }
  .row:hover {
    background: var(--hover);
  }
  .row:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: -2px;
  }
  .ic {
    flex: none;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 6px;
    background: rgba(var(--fg-rgb), 0.05);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--muted);
  }
  /* The thread that ties a turn's calls together, from this one's icon to the next one's. */
  .linked > .row .ic::after {
    content: '';
    position: absolute;
    left: 14.5px;
    top: 26px;
    width: 1px;
    height: 12px;
    background: var(--line-2);
  }
  .badge {
    flex: none;
    font-weight: 600;
    color: var(--text);
  }
  .err .badge,
  .err .ic {
    color: var(--del);
  }
  /* Reading the list of tasks is no change to the plan: a step back. */
  .quiet .badge {
    font-weight: 500;
    color: var(--muted);
  }
  .arg {
    flex: 0 1 auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
    font-family: var(--mono);
    font-size: 0.92em;
    letter-spacing: -0.01em;
  }
  .fill {
    flex: 1;
    min-width: 0;
  }
  .arg.link {
    border: none;
    background: none;
    padding: 0;
    text-align: left;
    cursor: pointer;
  }
  .arg.link:hover {
    color: var(--p-ink);
    text-decoration: underline;
  }
  .res {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    max-width: 40%;
    margin-left: auto;
    padding-left: 12px;
    overflow: hidden;
    color: var(--muted);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .res + .res {
    margin-left: 0;
  }
  .res .ok {
    display: flex;
    color: var(--add);
  }
  .err .res {
    color: var(--del);
  }
  .add {
    color: var(--add);
  }
  .del {
    color: var(--del);
  }
  .open-sub {
    flex: none;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--p-ink);
    cursor: pointer;
  }
  .open-sub:hover {
    background: var(--hover);
  }
  .open-sub:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 1px;
  }
  .chev {
    flex: none;
    display: flex;
    color: var(--dim);
    transition: transform 0.2s var(--ease);
  }
  .res ~ .chev,
  .spin ~ .chev,
  .open-sub ~ .chev {
    margin-left: 0;
  }
  .row[aria-expanded='true'] .chev {
    transform: rotate(90deg);
  }
  .spin {
    flex: none;
    width: 11px;
    height: 11px;
    margin-left: auto;
    border-radius: 50%;
    border: 1.5px solid var(--line-2);
    border-top-color: var(--p-ink);
    animation: ccSpin 0.8s linear infinite;
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
    .chev {
      transition: none;
    }
  }
  /* The detail of a call: its output, its diff, its subagent's calls, in a frame. */
  .detail {
    margin: 4px 0 6px 36px;
    overflow: hidden;
    border-radius: 10px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .detail :global(.out) {
    margin: 0;
    padding: 10px 12px;
    max-height: 360px;
    overflow: auto;
    font-family: var(--mono);
    font-size: 12px;
    line-height: 1.6;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    color: var(--code-fg);
  }
  .detail :global(.out.cmd) {
    color: var(--text);
    border-bottom: 1px solid var(--line);
    max-height: 120px;
  }
  .todos {
    list-style: none;
    margin: 0;
    padding: 10px 14px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
  }
  .todos li {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .todos .completed {
    color: var(--muted);
    text-decoration: line-through;
  }
  .todos .in_progress {
    color: var(--p-ink);
  }
  .sub {
    display: flex;
    flex-direction: column;
    gap: 0;
    padding: 8px 8px 8px 4px;
  }
  .prompt {
    margin: 0 0 6px 8px;
    padding: 4px 12px;
    border-left: 1px solid var(--line-3);
    color: var(--muted);
    font-size: 12.5px;
    white-space: pre-wrap;
    max-height: 140px;
    overflow: auto;
  }
  .subtext {
    margin: 6px 0 0 8px;
    padding: 0 4px 0 12px;
  }
</style>
