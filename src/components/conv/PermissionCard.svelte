<script lang="ts">
  import { captureKeys } from '../../lib/answer-keys.svelte';
  import { api } from '../../lib/ipc';
  import { keyLabel } from '../../lib/platform';
  import { answersHere, ariaEnter, enterAnswer } from '../../lib/shortcuts';
  import { app } from '../../lib/state.svelte';
  import { toolArg, toolLabel } from '../../lib/tools';
  import type { PermissionItem, ToolItem } from '../../lib/types';
  import Markdown from './Markdown.svelte';

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
  // A request Claude Code would refuse by default: the refusal is its main button, and only a click allows it.
  const keys = $derived(pending && current && !item.defaultNo);
  const title = $derived(isPlan ? 'Claude propose un plan' : 'Claude demande une autorisation');
  const summary = $derived(
    toolArg({ kind: 'tool', id: item.id, name: item.toolName, input: item.input, status: 'running', ts: 0 } as ToolItem, cwd),
  );

  async function decide(decision: 'allow' | 'always' | 'deny') {
    busy = true;
    // The card is about to go: the focus it holds would be lost with it.
    const held = !!card?.contains(document.activeElement);
    const message = decision === 'deny' && isPlan ? 'Continue à planifier : le plan ne me convient pas encore.' : null;
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
  captureKeys(() => pending && current, onKeydown);
</script>

{#if pending}
  <div class="card pending" data-testid="permission-pending" role="group" aria-label={title} bind:this={card}>
    <div class="title">
      <span class="pulse" style="width:8px;height:8px"></span>
      {title}
    </div>
    {#if isPlan && typeof item.input.plan === 'string'}
      <div class="plan"><Markdown text={item.input.plan} /></div>
    {:else}
      <div class="what">
        <span class="badge">{toolLabel(item.toolName)}</span>
        <span class="arg mono">{item.title || summary}</span>
      </div>
      {#if item.description}<div class="desc">{item.description}</div>{/if}
    {/if}
    {#if item.reason}<div class="reason">{item.reason}</div>{/if}
    <div class="opts">
      <button
        class="opt"
        class:primary={!item.defaultNo}
        disabled={busy}
        aria-keyshortcuts={keys ? ariaEnter() : undefined}
        onclick={() => decide('allow')}
      >
        {isPlan ? 'Approuver le plan' : 'Autoriser'}
        {#if keys}<kbd class="kbd" aria-hidden="true">{keyLabel('Ctrl+Entrée')}</kbd>{/if}
      </button>
      {#if item.canAlways}
        <button class="opt" disabled={busy} aria-keyshortcuts={keys ? ariaEnter(true) : undefined} onclick={() => decide('always')}>
          {isPlan ? 'Approuver et accepter les édits' : 'Toujours autoriser'}
          {#if keys}<kbd class="kbd" aria-hidden="true">{keyLabel('Ctrl+Maj+Entrée')}</kbd>{/if}
        </button>
      {/if}
      <button class="opt" class:primary={item.defaultNo} disabled={busy} onclick={() => decide('deny')}
        >{isPlan ? 'Continuer à planifier' : 'Refuser'}</button
      >
    </div>
    <span class="hint">Pour refuser en expliquant quoi faire à la place, écris-le dans le champ ci-dessous.</span>
  </div>
{:else}
  <div class="line mono" class:denied={item.decision === 'deny'}>
    {#if item.decision === 'deny'}✕ Refusé{:else if item.decision}✓ {item.decision === 'always' ? 'Toujours autorisé' : 'Autorisé'}{:else}·
      Demande annulée{/if}
    · {toolLabel(item.toolName)}
    {isPlan ? '' : summary}
    {#if item.message}<span class="msg">— {item.message}</span>{/if}
  </div>
{/if}

<style>
  .card {
    margin-left: 34px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 18px;
    border-radius: var(--r);
    border: 1px solid var(--wait);
    background: var(--wait-soft);
    animation: ccFadeIn 0.2s ease-out;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 12px;
    font-weight: 700;
    color: var(--wait);
    letter-spacing: 0.02em;
  }
  .what {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }
  .badge {
    padding: 2px 6px;
    border-radius: 3px;
    background: var(--elev2);
    font-family: var(--mono);
    font-weight: 600;
    font-size: 11px;
    flex: none;
  }
  .arg {
    font-size: 12.5px;
    overflow-wrap: anywhere;
  }
  .desc,
  .reason {
    font-size: 13px;
    color: var(--muted);
    line-height: 1.5;
  }
  .plan {
    max-height: 420px;
    overflow: auto;
    padding: 12px 14px;
    border-radius: var(--r-sm);
    background: var(--panel);
    border: 1px solid var(--line);
  }
  .opts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .opt {
    height: 32px;
    padding: 0 14px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line2);
    background: transparent;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .opt:hover:not(:disabled) {
    border-color: var(--wait);
  }
  .opt.primary {
    background: var(--wait);
    border-color: var(--wait);
    color: #2a1f05;
  }
  /* The shortcut, in the button's own colors: the amber of the main button is no place for the dim gray. */
  .opt .kbd {
    margin-left: 8px;
    color: inherit;
    opacity: 0.7;
  }
  .hint {
    font-size: 11.5px;
    color: var(--dim);
  }
  .line {
    margin-left: 34px;
    font-size: 11.5px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .line.denied {
    color: var(--del);
  }
  .msg {
    color: var(--dim);
  }
</style>
