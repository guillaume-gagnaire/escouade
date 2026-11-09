<script lang="ts">
  import { captureKeys } from '../../lib/answer-keys.svelte';
  import { fInt } from '../../lib/format';
  import { t, tIn } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { keyLabel } from '../../lib/platform';
  import { revealHidden } from '../../lib/recipe';
  import { answersHere, ariaEnter, enterAnswer } from '../../lib/shortcuts';
  import { speakerOf } from '../../lib/engines';
  import { tip } from '../../lib/tip';
  import { app } from '../../lib/state.svelte';
  import { escouadeArgs, isEscouadeTool, toolArg, toolLabel } from '../../lib/tools';
  import type { PermissionItem, ToolItem } from '../../lib/types';
  import { patchLines } from '../../lib/diff';
  import { modeLabel, planResume } from '../../lib/models';
  import Markdown from './Markdown.svelte';
  import PatchView from './PatchView.svelte';
  import Icon from '../Icon.svelte';
  import Via from './Via.svelte';

  // `current`: several requests can wait at once, and the keyboard answers the first (as the message field does).
  let {
    item,
    agentId,
    pending,
    cwd,
    current = true,
  }: { item: PermissionItem; agentId: string; pending: boolean; cwd: string; current?: boolean } = $props();
  let busy = $state(false);
  let card = $state<HTMLElement>();

  const isPlan = $derived(item.toolName === 'ExitPlanMode');
  // The mode its work resumes in once the plan is approved (ESC-277), as the backend puts it in.
  const resume = $derived.by(() => {
    const a = app.agents[agentId];
    return isPlan && a ? modeLabel(planResume(a)) : null;
  });
  // A request Claude Code would refuse by default: the refusal is its main button, and only a click allows it.
  const keys = $derived(pending && current && !item.defaultNo);
  // Its agent named as its harness names it (« Claude demande une autorisation »).
  const engine = $derived(speakerOf(app.agents[agentId]?.engine));
  const title = $derived(isPlan ? t('composer.permission.titlePlan', { engine }) : t('composer.permission.title', { engine }));
  const summary = $derived(
    toolArg({ kind: 'tool', id: item.id, name: item.toolName, input: item.input, status: 'running', ts: 0 } as ToolItem, cwd),
  );
  // A tool of Escouade's own server creates tickets, starts agents…: all it is given is read here, not a summary.
  const escouade = $derived(isEscouadeTool(item.toolName));
  const args = $derived(escouade ? escouadeArgs(item.input) : []);
  // What was decided; answered through the MCP server and not by the user in the window, the line says by whom too.
  const verdict = $derived.by(() => {
    if (item.decision === 'deny') return t('composer.permission.denied');
    if (item.decision === 'always')
      return item.alwaysScope === 'session'
        ? t('composer.permission.sessionAllowed')
        : item.alwaysScope === 'agent'
          ? t('composer.permission.agentAllowed')
          : t('composer.permission.alwaysAllowed');
    return t('composer.permission.allowed');
  });

  async function decide(decision: 'allow' | 'always' | 'deny') {
    busy = true;
    // The card is about to go: the focus it holds would be lost with it.
    const held = !!card?.contains(document.activeElement);
    // Told to Claude: in the language of the texts for Claude, not the interface's.
    const message = decision === 'deny' && isPlan ? tIn(app.lang.claude, 'composer.permission.keepPlanningMessage') : null;
    await app.run(api.answerPermission(agentId, item.id, decision, message));
    busy = false;
    if (held) app.focusComposer++;
  }

  function onKeydown(e: KeyboardEvent) {
    if (!keys || !answersHere(e.target)) return;
    const answer = enterAnswer(e);
    if (answer === 'plain' || (answer === 'shift' && item.canAlways)) {
      e.preventDefault();
      e.stopPropagation();
      if (!busy) decide(answer === 'plain' ? 'allow' : 'always');
    }
  }

  // Listened to without its own keys too: the message field waits the same moment before refusing it.
  // In the split screen (ESC-392), only the active pane's request takes the keys.
  captureKeys(() => pending && current && app.keysFor(agentId), onKeydown);
</script>

{#if pending}
  <div class="req-card pending" data-testid="permission-pending" role="group" aria-label={title} bind:this={card}>
    <div class="rt"><Icon name="shield" size={14} />{title}</div>
    {#if isPlan && typeof item.input.plan === 'string'}
      <div class="plan"><Markdown text={item.input.plan} /></div>
    {:else if escouade}
      <div class="cmdl"><span class="badge">{toolLabel(item.toolName)}</span></div>
      {#if args.length}
        <!-- What is read is what it gets: an invisible or direction character is spelled out, not drawn. -->
        <dl class="args" role="group" aria-label={t('mcp.permission.args')}>
          {#each args as a (a.name)}
            <div class="arg-line">
              <dt class="mono">{t('mcp.permission.argName', { name: revealHidden(a.name) })}</dt>
              <dd class="mono">{revealHidden(a.value)}</dd>
              <!-- What is not shown is told, and where the whole request is: the conversation. -->
              {#if a.hidden}<dd class="cut">{t('mcp.permission.cut', { count: a.hidden, n: fInt(a.hidden) })}</dd>{/if}
            </div>
          {/each}
        </dl>
      {/if}
      {#if item.description}<div class="desc">{item.description}</div>{/if}
    {:else}
      <div class="cmdl">
        <span class="badge">{toolLabel(item.toolName)}</span>
        <span class="arg mono">{item.title || summary}</span>
      </div>
      <!-- What « Toujours autoriser » keeps, when it is not the request itself (OpenCode's pattern), is said here. -->
      {#if item.description}<div class="desc">{item.description}</div>{/if}
      {#if item.patch?.length}<div class="frame"><PatchView lines={patchLines(item.patch)} /></div>{/if}
    {/if}
    {#if item.reason}<div class="desc">{item.reason}</div>{/if}
    <div class="row-btns">
      <button
        class="btn"
        class:primary={!item.defaultNo}
        disabled={busy}
        aria-keyshortcuts={keys ? ariaEnter() : undefined}
        onclick={() => decide('allow')}
      >
        {isPlan ? t('composer.permission.approvePlan') : t('composer.permission.allow')}
        {#if resume}<span class="then" use:tip={t('composer.permission.resumesIn', { mode: resume })}>· {resume}</span>{/if}
        {#if keys}<kbd aria-hidden="true">{keyLabel('Ctrl+Enter')}</kbd>{/if}
      </button>
      {#if item.canAlways}
        <button class="btn" disabled={busy} aria-keyshortcuts={keys ? ariaEnter(true) : undefined} onclick={() => decide('always')}>
          {isPlan
            ? t('composer.permission.approvePlanAndEdits')
            : item.alwaysScope === 'session'
              ? t('composer.permission.forSession')
              : item.alwaysScope === 'agent'
                ? t('composer.permission.forAgent')
                : t('composer.permission.always')}
          {#if keys}<kbd aria-hidden="true">{keyLabel('Ctrl+Shift+Enter')}</kbd>{/if}
        </button>
      {/if}
      <button class="btn" class:primary={item.defaultNo} disabled={busy} onclick={() => decide('deny')}
        >{isPlan ? t('composer.permission.keepPlanning') : t('composer.permission.deny')}</button
      >
    </div>
    <span class="hint">{t('composer.permission.denyHint')}</span>
  </div>
{:else}
  <div class="req-card done">
    {#if !isPlan}
      <div class="cmdl"><span class="badge">{toolLabel(item.toolName)}</span><span class="arg mono">{summary}</span></div>
    {/if}
    <div class="by line" class:denied={item.decision === 'deny'}>
      <Icon name={item.decision === 'deny' ? 'close' : item.decision ? 'check' : 'circle'} size={14} />
      <span
        ><b class="verdict">{item.decision ? verdict : t('composer.permission.cancelled')}</b>{#if isPlan}
          · {toolLabel(item.toolName)}{/if}</span
      >
      {#if item.by}<Via author={item.by} />{/if}
      {#if item.message}<span class="msg">— {item.message}</span>{/if}
    </div>
  </div>
{/if}

<style>
  /* A request of the agent (2.0 design): amber while it waits for the user, a hairline once answered. */
  .req-card {
    display: grid;
    gap: 9px;
    min-width: 0;
    padding: 12px 14px;
    border-radius: 11px;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .req-card.pending {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    animation: ccFadeIn 0.2s ease-out;
  }
  .rt {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
  }
  .rt :global(.icon) {
    color: var(--wait);
  }
  .cmdl {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .badge {
    flex: none;
    color: var(--muted);
    font-size: 12px;
  }
  .arg {
    min-width: 0;
    color: var(--soft);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  .done .arg {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .desc {
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-line;
  }
  .frame,
  .plan,
  .args {
    overflow: hidden;
    border-radius: 10px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .plan {
    max-height: 420px;
    overflow: auto;
    padding: 12px 14px;
  }
  /* One argument per line, its name before it; a value of several lines keeps them. */
  .args {
    margin: 0;
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 6px 12px;
    max-height: 420px;
    overflow: auto;
    padding: 10px 14px;
    color: var(--code-fg);
    font-size: 12px;
    line-height: 1.5;
  }
  .arg-line {
    display: contents;
  }
  .args dt {
    color: var(--code-dim);
  }
  .args dd {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  /* Under its value, in the value's column. */
  .args dd.cut {
    grid-column: 2;
    color: var(--warn-ink);
    font-size: 11.5px;
    white-space: normal;
  }
  .row-btns {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  /* The mode the work resumes in, after « Approuver le plan »: said, not shouted. */
  .then {
    opacity: 0.72;
    font-weight: 450;
  }
  .hint {
    color: var(--muted);
    font-size: 11.5px;
  }
  .by {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 7px;
    min-width: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .by :global(.icon) {
    color: var(--add);
  }
  .by.denied :global(.icon),
  .by.denied .verdict {
    color: var(--del);
  }
  .verdict {
    color: var(--soft);
    font-weight: 550;
  }
  .msg {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  @media (prefers-reduced-motion: reduce) {
    .req-card.pending {
      animation: none;
    }
  }
</style>
