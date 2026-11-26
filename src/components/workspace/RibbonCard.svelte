<script lang="ts">
  // A card with a ribbon on the « Workspace » (spec 2026-11-25-vue-workspace-design.md, « Au sud : la zone
  // d'atterrissage » and « Choix validés »): a test run stuck under its card, a remote CI or a deployment in the landing
  // zone. Green stripes for tests and CI, blue dashes for a deployment, moving only while it runs, still and red when it
  // failed. A branch, never a commit.
  import { conversationOf } from '../../lib/conversations.svelte';
  import { fDur } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { openUrl } from '../../lib/opener';
  import { tip } from '../../lib/tip';
  import { app } from '../../lib/state.svelte';
  import type { Ribbon } from '../../lib/workspace-landing';

  let { agentId, item, side }: { agentId: string; item: Ribbon; side: 'centre' | 'east' | 'south' } = $props();

  const kind = $derived(t(`workspace.ribbon.kind.${item.kind}`));
  const state = $derived(t(`workspace.ribbon.state.${item.state}`));
  /** The card as its button names it for a screen reader: « Tests · npm test », « CI #12 · feat/x ». */
  const title = $derived([item.number === null ? kind : `${kind} #${item.number}`, item.line].filter(Boolean).join(' · '));
  const ms = $derived(
    item.since === null ? null : item.state === 'running' ? app.now - item.since : item.until === null ? null : item.until - item.since,
  );

  function open() {
    const o = item.opens;
    if (!o) return;
    if (o.kind === 'task') app.openTask(agentId, o.task);
    else if (o.kind === 'call') {
      app.selectAgent(agentId);
      conversationOf(agentId).reveal(o.call);
    } else void openUrl(o.url);
  }
</script>

<article class="card {item.kind} {item.state}" data-side={side} aria-label={t('workspace.ribbon.label', { kind, state })}>
  <div
    class="ribbon {item.state}"
    class:stripes={item.kind !== 'deploy'}
    class:dashes={item.kind === 'deploy'}
    class:running={item.state === 'running'}
  ></div>
  <header>
    <b class="kind">{kind}</b>
    {#if item.number !== null}<span class="num">#{item.number}</span>{/if}
    <span class="st">{state}</span>
    {#if ms !== null}<span class="dur">{fDur(ms)}</span>{/if}
  </header>
  <p class="line mono">{item.line}</p>
  {#if item.steps?.length}
    <ol class="steps">
      {#each item.steps as s, i (i)}<li class={s.state} use:tip={s.name}><span class="sr">{s.name}</span></li>{/each}
    </ol>
  {:else if item.total !== null}
    <p class="checks">{t('workspace.ribbon.checks', { count: item.total, failed: item.failed ?? 0 })}</p>
  {/if}
  {#if item.where || item.opens}
    <footer>
      <span class="where">{item.where}</span>
      {#if item.opens}
        <button
          type="button"
          class="btn"
          aria-label={item.opens.kind === 'url' ? t('workspace.land.openPage', { title }) : t('workspace.ribbon.outputOf', { title })}
          onclick={open}>{item.opens.kind === 'url' ? t('workspace.land.open') : t('workspace.ribbon.output')}</button
        >
      {/if}
    </footer>
  {/if}
</article>

<style>
  .card {
    --tone: var(--run);
    display: flex;
    flex-direction: column;
    gap: 3px;
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    padding: 8px 10px 7px;
    overflow: hidden;
    position: relative;
    border-radius: 9px;
    background: var(--ws-card);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font-size: 12.5px;
  }
  .card.deploy {
    --tone: var(--info);
  }
  .card.failure {
    --tone: var(--del);
  }
  .card.stopped {
    --tone: var(--muted);
  }
  .ribbon {
    position: absolute;
    inset: 0 0 auto;
    height: 4px;
  }
  .ribbon.stripes {
    background: repeating-linear-gradient(-45deg, var(--tone) 0 6px, transparent 6px 12px);
    background-size: 17px 4px;
  }
  .ribbon.dashes {
    background: repeating-linear-gradient(90deg, var(--tone) 0 10px, transparent 10px 16px);
    background-size: 16px 4px;
  }
  .ribbon.failure {
    background: var(--del);
  }
  .ribbon.running {
    animation: slide 0.8s linear infinite;
  }
  @keyframes slide {
    to {
      background-position-x: 17px;
    }
  }
  .ribbon.dashes.running {
    animation-name: slide-dash;
  }
  @keyframes slide-dash {
    to {
      background-position-x: 16px;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .ribbon.running {
      animation: none;
    }
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 6px;
    min-width: 0;
    margin-top: 2px;
  }
  .kind {
    font-weight: 590;
    letter-spacing: -0.01em;
  }
  .num,
  .dur {
    color: var(--muted);
    font-size: 11.5px;
  }
  .st {
    color: var(--tone);
    font-size: 11.5px;
    font-weight: 550;
  }
  .dur {
    margin-left: auto;
    font-family: var(--mono);
  }
  .line {
    margin: 0;
    overflow: hidden;
    color: var(--soft);
    font: 11.5px var(--mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .steps {
    display: flex;
    align-items: center;
    margin: 2px 0 0;
    padding: 0;
    list-style: none;
  }
  .steps li {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--run);
  }
  .steps li + li {
    margin-left: 10px;
    position: relative;
  }
  .steps li + li::before {
    content: '';
    position: absolute;
    top: 3.5px;
    left: -10px;
    width: 10px;
    height: 1px;
    background: var(--line-3);
  }
  .steps li.failure {
    background: var(--del);
  }
  .steps li.running {
    background: transparent;
    box-shadow: inset 0 0 0 1.5px var(--run);
  }
  .checks {
    margin: 0;
    color: var(--muted);
    font-size: 11.5px;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: auto;
  }
  .where {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    color: var(--muted);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .btn {
    padding: 2px 8px;
    font-size: 11.5px;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }
</style>
