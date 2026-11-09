<script lang="ts">
  // A pane of the split screen (ESC-392): its own panel on the frame (direction B), ringed in the project's colour
  // when it is the active one. It draws its content as the work area draws it alone: an agent (its conversation, or
  // its subagent's, or its background task's), a terminal, a launch log, the editor. A press anywhere in it makes it
  // active; its × (on hover, and always on the active one) closes it.
  import { t } from '../../lib/i18n';
  import { setPaneCtx } from '../../lib/pane-context';
  import { itemKey, type PaneItem } from '../../lib/panes';
  import { keyLabel } from '../../lib/platform';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { Project } from '../../lib/types';
  import Conversation from '../Conversation.svelte';
  import EditorView from '../editor/EditorView.svelte';
  import Icon from '../Icon.svelte';
  import RunView from '../RunView.svelte';
  import SidePanel from '../SidePanel.svelte';
  import SubagentView from '../SubagentView.svelte';
  import TaskView from '../TaskView.svelte';
  import TerminalView from '../TerminalView.svelte';

  let { item, project }: { item: PaneItem; project: Project } = $props();

  const key = $derived(itemKey(item));
  const active = $derived(app.panes?.active === key);
  setPaneCtx({
    get key() {
      return key;
    },
    get active() {
      return active;
    },
  });

  const agent = $derived(item.kind === 'agent' ? app.agents[item.id] : undefined);
  const term = $derived(item.kind === 'term' ? app.terminals.find((x) => x.id === item.id) : undefined);
  const cmd = $derived(item.kind === 'launch' ? app.launchOf(project, item.id) : undefined);
  const name = $derived(agent?.name ?? term?.name ?? cmd?.name ?? t('shell.panes.editor'));
  const closeKey = keyLabel('Ctrl+Shift+W');
</script>

<section
  class="pane"
  class:active
  data-pane-key={key}
  data-pane-active={active}
  data-testid="pane"
  aria-label={t('shell.panes.region', { name })}
  aria-current={active ? 'true' : undefined}
  onpointerdowncapture={() => {
    if (!active) app.activatePane(key);
  }}
>
  {#if agent}
    {#if app.subView[agent.id]}
      {#key app.subView[agent.id]}
        <SubagentView {agent} {project} call={app.subView[agent.id] ?? ''} />
      {/key}
    {:else if app.taskView[agent.id]}
      {#key app.taskView[agent.id]}
        <TaskView {agent} task={app.taskView[agent.id] ?? ''} />
      {/key}
    {:else}
      <Conversation {agent} {project} />
    {/if}
    {#if active && app.filesOpen}
      <SidePanel {project} {agent} />
    {/if}
  {:else if term}
    <TerminalView {term} {project} />
  {:else if cmd}
    <RunView {cmd} {project} />
  {:else if item.kind === 'editor'}
    <EditorView {project} />
  {/if}
  <button
    class="pane-x"
    aria-label={t('shell.panes.close')}
    use:tip={{ text: t('shell.panes.closeTip', { shortcut: closeKey }), aria: false }}
    data-testid="pane-close"
    onclick={() => app.closePaneOf(key)}><Icon name="close" size={12} /></button
  >
</section>

<style>
  .pane {
    position: relative;
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
    border-radius: 12px;
    background: var(--panel);
    box-shadow:
      0 0 0 1px var(--line),
      0 8px 24px -6px rgba(0, 0, 0, calc(0.5 * var(--shade)));
    overflow: hidden;
    transition: box-shadow 0.15s var(--ease);
  }
  /* The active pane: a ring in the project's colour, where the keyboard, the voice and the answers go. */
  .pane.active {
    box-shadow:
      0 0 0 1px color-mix(in oklch, var(--p-ink) 65%, transparent),
      0 0 0 4px color-mix(in oklch, var(--p) 12%, transparent),
      0 8px 24px -6px rgba(0, 0, 0, calc(0.5 * var(--shade)));
  }
  /* The others, a shade quieter. */
  .pane:not(.active)::after {
    content: '';
    position: absolute;
    inset: 0;
    background: var(--frame);
    opacity: 0.12;
    pointer-events: none;
  }
  .pane-x {
    position: absolute;
    z-index: 5;
    top: 0;
    right: 0;
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
    border: 0;
    border-bottom-left-radius: 8px;
    background: var(--raise);
    box-shadow:
      -1px 1px 0 var(--line),
      0 2px 6px -2px rgba(0, 0, 0, calc(0.4 * var(--shade)));
    color: var(--muted);
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.12s;
  }
  .pane:hover > .pane-x,
  .pane-x:focus-visible {
    opacity: 1;
  }
  .pane-x:hover {
    color: var(--text);
  }
</style>
