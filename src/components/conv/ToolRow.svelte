<script lang="ts">
  import { t } from '../../lib/i18n';
  import { firstChangedLine, patchLines } from '../../lib/diff';
  import { hasDiff, isQuietTool, toolArg, toolLabel, toolResultSummary } from '../../lib/tools';
  import type { ConvItem, ToolItem } from '../../lib/types';
  import Markdown from './Markdown.svelte';
  import PatchView from './PatchView.svelte';
  import Self from './ToolRow.svelte';

  // `onOpenFile`: opens a file the tool edited (its path) in the editor, at a line.
  let {
    item,
    cwd,
    childrenOf = () => [],
    onOpenFile,
  }: {
    item: ToolItem;
    cwd: string;
    childrenOf?: (id: string) => ConvItem[];
    onOpenFile?: (abs: string, line: number | null) => void;
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
  const toggle = () => expandable && (open = !open);
</script>

<!-- Reading the list of tasks is no change to the plan: a step back, in softer colours. -->
<div class="tool" class:err={item.status === 'error'} class:quiet={isQuietTool(item.name)}>
  <!-- Not a <button>: the edited file's path inside it is one. -->
  <div
    class="row"
    role="button"
    tabindex="0"
    aria-expanded={open}
    title={arg}
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
    <span class="badge">{toolLabel(item.name)}</span>
    {#if file}
      <button
        class="arg link"
        aria-label={t('conv.tool.openFile', { path: arg })}
        title={t('common.openInEditor')}
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
      <span class="spin" aria-label={t('conv.tool.running')}></span>
    {:else if diff}
      <span class="add">+{item.result?.add}</span><span class="del">−{item.result?.del}</span>
    {:else if res}
      <span class="res">{res}</span>
    {/if}
    {#if subTools.length}<span class="res">{t('conv.tool.subTools', { count: subTools.length })}</span>{/if}
  </div>
  {#if open}
    <div class="detail">
      {#if diff && item.result?.patch?.length}
        <PatchView lines={patchLines(item.result.patch)} />
      {:else if item.name === 'TodoWrite' && Array.isArray(item.input.todos)}
        <ul class="todos">
          {#each item.input.todos as todo, i (i)}
            <li class={todo.status}>
              <span>{todo.status === 'completed' ? '☑' : todo.status === 'in_progress' ? '◐' : '☐'}</span>
              {todo.content ?? todo.activeForm}
            </li>
          {/each}
        </ul>
      {:else if children.length}
        <div class="sub">
          {#if item.input.prompt}<div class="prompt">{item.input.prompt}</div>{/if}
          {#each subTools as c (c.id)}
            <Self item={c} {cwd} {childrenOf} {onOpenFile} />
          {/each}
          {#if subText}<div class="subtext"><Markdown text={subText.text} streaming={subText.streaming} /></div>{/if}
        </div>
      {:else}
        {#if item.name === 'Bash' || item.name === 'PowerShell'}
          <pre class="out cmd">$ {item.input.command}</pre>
        {/if}
        {#if item.result?.text}
          <pre class="out">{item.result.text}</pre>
        {:else if item.name !== 'Bash'}
          <pre class="out">{JSON.stringify(item.input, null, 2)}</pre>
        {/if}
      {/if}
    </div>
  {/if}
</div>

<style>
  .tool {
    margin-left: 34px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line);
    background: var(--panel);
    overflow: hidden;
    min-width: 0;
  }
  .tool.err {
    border-color: color-mix(in oklch, var(--del) 45%, transparent);
  }
  .tool.quiet {
    margin-left: 52px;
    color: var(--muted);
  }
  .quiet .badge {
    background: transparent;
    color: var(--muted);
  }
  .row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    border: none;
    background: transparent;
    font-family: var(--mono);
    font-size: 12px;
    text-align: left;
    cursor: pointer;
    min-width: 0;
  }
  .row:hover {
    background: color-mix(in oklch, var(--elev) 60%, transparent);
  }
  /* The tool's frame clips an outline drawn outside the row. */
  .row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .badge {
    padding: 2px 6px;
    border-radius: 3px;
    background: var(--elev2);
    font-weight: 600;
    font-size: 11px;
    flex: none;
  }
  .err .badge {
    color: var(--del);
  }
  .arg {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fill {
    flex: 1;
    min-width: 0;
  }
  .arg.link {
    flex: 0 1 auto;
    border: none;
    background: none;
    padding: 0;
    font: inherit;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .arg.link:hover {
    color: var(--accent);
    text-decoration: underline;
  }
  .add {
    color: var(--add);
    flex: none;
  }
  .del {
    color: var(--del);
    flex: none;
  }
  .res {
    color: var(--muted);
    flex: none;
    max-width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .err .res {
    color: var(--del);
  }
  .spin {
    width: 11px;
    height: 11px;
    border-radius: 50%;
    border: 1.5px solid var(--line2);
    border-top-color: var(--accent);
    animation: ccSpin 0.8s linear infinite;
    flex: none;
  }
  .detail {
    border-top: 1px solid var(--line);
  }
  .out {
    margin: 0;
    padding: 10px 12px;
    max-height: 360px;
    overflow: auto;
    font-family: var(--mono);
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    background: var(--term);
    color: #cfc7bb;
  }
  .out.cmd {
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
    gap: 8px;
  }
  .todos .completed {
    color: var(--dim);
    text-decoration: line-through;
  }
  .todos .in_progress {
    color: var(--accent);
  }
  .sub {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px 10px 10px 0;
  }
  .sub :global(.tool) {
    margin-left: 12px;
  }
  .prompt {
    margin-left: 12px;
    padding: 8px 12px;
    border-left: 2px solid var(--line2);
    color: var(--muted);
    font-size: 12.5px;
    white-space: pre-wrap;
    max-height: 140px;
    overflow: auto;
  }
  .subtext {
    margin-left: 12px;
    padding: 4px 4px 0 12px;
  }
</style>
