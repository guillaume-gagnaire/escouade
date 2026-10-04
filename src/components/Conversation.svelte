<script lang="ts">
  import { keyLabel } from '../lib/platform';
  import { onMount } from 'svelte';
  import { api } from '../lib/ipc';
  import { conversationOf } from '../lib/conversations.svelte';
  import { splitEscouade } from '../lib/escouade';
  import { fDur, fInt, fTok } from '../lib/format';
  import { modelLabel } from '../lib/models';
  import { ESTIMATE_HINT, fSpentUsd, spent } from '../lib/spend';
  import { injectedSource, parseAgentMessage, subagentLabels } from '../lib/events';
  import { editsByTurn } from '../lib/tools';
  import { app } from '../lib/state.svelte';
  import { canPrepare, canTest, prepareLaunch, testAgent } from '../lib/test-launch.svelte';
  import type { Agent, ConvItem, Project } from '../lib/types';
  import Composer from './Composer.svelte';
  import CriteriaReport from './conv/CriteriaReport.svelte';
  import EventMessage from './conv/EventMessage.svelte';
  import Markdown from './conv/Markdown.svelte';
  import Notice from './conv/Notice.svelte';
  import PermissionCard from './conv/PermissionCard.svelte';
  import QuestionCard from './conv/QuestionCard.svelte';
  import Thinking from './conv/Thinking.svelte';
  import ToolRow from './conv/ToolRow.svelte';
  import TurnCard from './conv/TurnCard.svelte';
  import UserMessage from './conv/UserMessage.svelte';
  import StatusDot from './StatusDot.svelte';

  let { agent, project }: { agent: Agent; project: Project } = $props();

  const SL: Record<string, string> = { running: 'En cours', waiting: 'Question', idle: 'Prêt', done: 'Terminé', error: 'Erreur' };
  const SC: Record<string, string> = {
    running: 'var(--ok)',
    waiting: 'var(--wait)',
    idle: 'var(--dim)',
    done: 'var(--ok)',
    error: 'var(--del)',
  };

  const conv = $derived(conversationOf(agent.id));
  // The ticket the agent works on, whose criteria the reports of its turns name.
  const ticket = $derived(app.ticketOf(agent.id));
  const top = $derived(conv.items.filter((i) => !i.parent));
  const children = $derived.by(() => {
    const m = new Map<string, ConvItem[]>();
    for (const i of conv.items) {
      if (!i.parent) continue;
      let list = m.get(i.parent);
      if (!list) m.set(i.parent, (list = []));
      list.push(i);
    }
    return m;
  });
  // What the last turn edited, for its recap: worked out once it has ended, not while streaming,
  // and not again on each update of the agent (a new object, same folder).
  const cwd = $derived(agent.cwd);
  const lastEdits = $derived.by(() => {
    const last = top.at(-1);
    return last?.kind === 'turn' ? (editsByTurn(conv.items, cwd).get(last.id) ?? []) : [];
  });
  // The conversation's latest turn, which tells when an agent stopped by the usage limit resumes.
  const latestTurn = $derived.by(() => {
    for (let i = top.length - 1; i >= 0; i--) if (top[i].kind === 'turn') return top[i].id;
    return null;
  });
  // Subagents named after their tasks, for their messages.
  const labels = $derived(subagentLabels(conv.items));
  const branch = $derived(agent.worktree?.branch ?? app.git[project.id]?.branch ?? '');
  const files = $derived(app.git[project.id]?.agents[agent.id] ?? 0);
  const used = $derived(spent(agent));
  // How full the context is, out of the window of the conversation's model (once a turn told it).
  const context = $derived.by(() => {
    const used = agent.contextTokens;
    const size = agent.contextWindow;
    if (!size) return { shown: used ? fTok(used) : '—', title: 'Contexte actuel', full: false };
    const pct = Math.round((used / size) * 100);
    const window = size >= 1e6 ? `${size / 1e6} M` : `${Math.round(size / 1e3)} k`;
    return {
      shown: `${fTok(used)} / ${window}`,
      title: `Contexte : ${pct} % de la fenêtre du modèle (${fInt(used)} tokens sur ${fInt(size)})`,
      full: pct >= 80,
    };
  });
  // The editor shows the agent's own checkout: its worktree, else the project's.
  const editorSource = $derived(agent.worktree ? agent.id : 'project');
  const openFile = (abs: string, line: number | null) =>
    app.openEditor({ projectId: project.id, source: editorSource, abs, line: line ?? undefined });
  const duration = $derived(fDur(agent.activeMs + (agent.activeSince ? app.now - agent.activeSince : 0)));
  const running = $derived(agent.status === 'running');

  let scroller = $state<HTMLDivElement>();
  let content = $state<HTMLDivElement>();
  let stick = true;
  let showJump = $state(false);
  let lastTop = 0;

  function atBottom() {
    return !scroller || scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 24;
  }

  function toBottom() {
    if (scroller) {
      scroller.scrollTop = scroller.scrollHeight;
      lastTop = scroller.scrollTop;
    }
    stick = true;
    showJump = false;
  }

  // The reader scrolling up leaves the bottom, however close to it: messages rendered as they come
  // into view (content-visibility) resize the content, which must not pull them back. Only the
  // reader does: content shrinking under the view, or the message field shrinking back once a
  // message is sent, moves the view up too, and the conversation must go on following.
  let readerAt = 0;
  let dragging = false;
  const reading = () => dragging || performance.now() - readerAt < 500;
  const byReader = () => (readerAt = performance.now());

  function onScroll() {
    if (!scroller) return;
    const top = scroller.scrollTop;
    if (top < lastTop - 1 && reading()) stick = false;
    else if (atBottom()) stick = true;
    lastTop = top;
    if (stick) showJump = false;
  }

  // The view is re-created for each agent ({#key}): scroll down once, not on every agent update.
  onMount(() => {
    requestAnimationFrame(toBottom);
  });

  $effect(() => {
    if (!content) return;
    const ro = new ResizeObserver(() => {
      if (stick) toBottom();
      else showJump = true;
    });
    ro.observe(content);
    return () => ro.disconnect();
  });

  function rename() {
    app.modal = {
      kind: 'rename',
      title: "Renommer l'agent",
      value: agent.name,
      onSubmit: (name) => app.run(api.renameAgent(agent.id, name)),
    };
  }

  function prevIsText(i: number) {
    return i > 0 && top[i - 1].kind === 'text';
  }
</script>

<svelte:window onpointerup={() => (dragging = false)} onpointercancel={() => (dragging = false)} />

<main class="conv">
  <header class="head">
    <div class="who">
      <div class="line1">
        <span class="name" ondblclick={rename} role="presentation" title="Double-clic pour renommer">{agent.name}</span>
        <span class="st" style:color={SC[agent.status]}><StatusDot status={agent.status} size={7} />{SL[agent.status]}</span>
      </div>
      <span class="sub mono">{project.name} / {branch || '—'}</span>
    </div>
    <div style="flex:1"></div>
    <button
      class="btn edit"
      aria-label="Éditeur"
      title="Parcourir et éditer les fichiers de cet agent"
      onclick={() => app.openEditor({ projectId: project.id, source: editorSource })}
    >
      <span class="mono glyph">&lt;/&gt;</span><span class="lbl">Éditeur</span>
    </button>
    {#if agent.recipe}
      {#if canTest(agent)}
        <button
          class="btn test"
          aria-label="▶ Tester"
          title="Lance le worktree de cet agent et ouvre la fonctionnalité dans le navigateur"
          onclick={() => testAgent(agent, project)}
        >
          <span class="glyph">▶</span><span class="lbl">Tester</span>
        </button>
      {/if}
    {:else if canPrepare(agent)}
      <button
        class="btn test"
        aria-label="Préparer le lancement"
        title="Demande à l'agent comment lancer son worktree, sur ses propres ports"
        onclick={() => prepareLaunch(agent)}
      >
        <span class="glyph">▷</span><span class="lbl">Préparer le lancement</span>
      </button>
    {/if}
    <div class="metrics">
      <span class="model mono">{modelLabel(agent.model, app.models)}</span>
      <div class="m" title={context.title}>
        <span class="k">Contexte</span><span class="v mono" class:full={context.full}>{context.shown}</span>
      </div>
      <div class="m opt"><span class="k">Tokens</span><span class="v mono">{fTok(used.tokens)}</span></div>
      <div class="m opt2" title={used.estimated ? ESTIMATE_HINT : undefined}>
        <span class="k">Coût</span><span class="v mono">{fSpentUsd(used)}</span>
      </div>
      {#if app.split}
        <div class="m opt2"><span class="k">Fichiers</span><span class="v mono">{files}</span></div>
      {:else}
        <button
          class="m files opt2"
          class:open={app.filesOpen}
          title="Voir les fichiers non commités"
          onclick={() => (app.filesOpen = !app.filesOpen)}
        >
          <span class="k">Fichiers ▸</span><span class="v mono">{files}</span>
        </button>
      {/if}
      <div class="m opt"><span class="k">Durée</span><span class="v mono">{duration}</span></div>
    </div>
    <div class="segmented layout" role="group" aria-label={`Disposition (${keyLabel('Ctrl+Maj+L')})`}>
      <button
        class:on={!app.split}
        aria-pressed={!app.split}
        aria-label="Disposition classique"
        title="Disposition classique"
        onclick={() => app.split && app.toggleLayout()}
      >
        <svg width="16" height="12" viewBox="0 0 16 12" aria-hidden="true"
          ><rect x="0.5" y="0.5" width="15" height="11" rx="1.5" fill="none" stroke="currentColor" /><rect
            x="11"
            y="1"
            width="4"
            height="10"
            fill="currentColor"
            opacity="0.45"
          /></svg
        >
      </button>
      <button
        class:on={app.split}
        aria-pressed={app.split}
        aria-label="Conversation et fichiers côte à côte"
        title="Conversation et fichiers côte à côte"
        onclick={() => !app.split && app.toggleLayout()}
      >
        <svg width="16" height="12" viewBox="0 0 16 12" aria-hidden="true"
          ><rect x="0.5" y="0.5" width="15" height="11" rx="1.5" fill="none" stroke="currentColor" /><rect
            x="8"
            y="1"
            width="7"
            height="10"
            fill="currentColor"
            opacity="0.45"
          /></svg
        >
      </button>
    </div>
  </header>

  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="scroll"
    bind:this={scroller}
    onscroll={onScroll}
    onwheel={byReader}
    onkeydown={byReader}
    onpointerdown={() => (dragging = true)}
  >
    <div class="msgs" bind:this={content}>
      {#if conv.error}
        <div class="load-error">
          Impossible de charger la conversation : {conv.error}
          <button class="btn" onclick={() => conv.load()}>Réessayer</button>
        </div>
      {:else if conv.loaded && top.length === 0}
        <div class="empty">
          <span class="t">Agent prêt</span>
          <span class="s">Décris la tâche à confier à Claude. L'agent travaille dans <span class="mono">{agent.cwd}</span>.</span>
          {#if !app.claudeFound}
            <span class="warn">Claude Code est introuvable sur ce poste : installe-le ou indique son chemin dans les réglages (⚙).</span>
          {/if}
        </div>
      {/if}
      {#each top as item, i (item.id)}
        {#if item.kind === 'user'}
          {@const injected = injectedSource(item)}
          {#if injected}
            <EventMessage source={injected} text={item.text} label={labels.get(parseAgentMessage(item.text).from ?? '')} />
          {:else}
            <UserMessage {item} />
          {/if}
        {:else if item.kind === 'event'}
          <EventMessage source={item.source} text={item.text} label={labels.get(item.from ?? '')} />
        {:else if item.kind === 'text'}
          {#if item.text.trim() || item.streaming}
            <div class="assistant">
              {#if !prevIsText(i)}<span class="avatar">C</span>{:else}<span class="avatar ghost"></span>{/if}
              <div class="blocks">
                {#each splitEscouade(item.text) as seg, si (si)}
                  {#if seg.kind === 'report'}
                    <CriteriaReport report={seg.report} criteria={ticket?.criteria ?? []} {agent} />
                  {:else}
                    <Markdown text={seg.text} streaming={item.streaming} />
                  {/if}
                {/each}
              </div>
            </div>
          {/if}
        {:else if item.kind === 'thinking'}
          <Thinking {item} />
        {:else if item.kind === 'tool'}
          {#if item.name !== 'AskUserQuestion' && item.name !== 'ExitPlanMode'}
            <ToolRow {item} cwd={agent.cwd} childrenOf={(id) => children.get(id) ?? []} onOpenFile={openFile} />
          {/if}
        {:else if item.kind === 'question'}
          <QuestionCard {item} agentId={agent.id} pending={agent.pending.includes(item.id)} />
        {:else if item.kind === 'permission'}
          <PermissionCard {item} agentId={agent.id} cwd={agent.cwd} pending={agent.pending.includes(item.id)} />
        {:else if item.kind === 'turn'}
          <TurnCard
            {item}
            {agent}
            last={i === top.length - 1}
            latest={item.id === latestTurn}
            edits={i === top.length - 1 ? lastEdits : []}
          />
        {:else if item.kind === 'notice'}
          <Notice {item} />
        {/if}
      {/each}
      {#if running}
        <div class="working">
          <span class="dots"><span></span><span></span><span></span></span>Claude travaille…
        </div>
      {/if}
    </div>
  </div>

  {#if showJump}
    <button class="jump" onclick={toBottom}>↓ Nouveaux messages</button>
  {/if}

  {#if agent.archived}
    <div class="banner">
      Agent archivé.
      <button class="btn" onclick={() => app.run(api.archiveAgent(agent.id, false))}>Restaurer</button>
    </div>
  {:else}
    {#if agent.status === 'error'}
      <div class="banner err">L'agent s'est arrêté. Envoie un message pour relancer Claude sur la même session.</div>
    {/if}
    <Composer {agent} />
  {/if}
</main>

<style>
  .conv {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .head {
    container-type: inline-size;
    height: 60px;
    flex: none;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 0 24px;
    border-bottom: 1px solid var(--line);
  }
  .who {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
    white-space: nowrap;
  }
  .line1 {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .name {
    font-size: 15px;
    font-weight: 700;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .st {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }
  .sub {
    font-size: 11px;
    color: var(--dim);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .edit,
  .test {
    height: 28px;
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 11px;
    font-size: 12px;
    font-weight: 600;
    flex: none;
  }
  .edit .glyph,
  .test .glyph {
    font-size: 11px;
    color: var(--accent);
  }
  .metrics {
    display: flex;
    align-items: center;
    gap: 22px;
    flex: none;
    white-space: nowrap;
  }
  .model {
    font-size: 11px;
    padding: 4px 8px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line2);
  }
  .m {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .k {
    font-size: 10.5px;
    color: var(--dim);
  }
  .v {
    font-size: 12.5px;
  }
  .v.full {
    color: var(--wait);
  }
  .files {
    align-items: flex-start;
    padding: 4px 8px;
    margin: -4px -8px;
    border: 1px solid transparent;
    border-radius: var(--r-sm);
    background: transparent;
    cursor: pointer;
    text-align: left;
  }
  .files:hover {
    border-color: var(--line2);
  }
  .files.open {
    background: var(--elev);
    border-color: var(--line2);
  }
  .layout {
    flex: none;
  }
  /* Half-width conversation (split layout): the agent's card in the sidebar shows these too. */
  @container (max-width: 680px) {
    .opt {
      display: none;
    }
    .edit .lbl,
    .test .lbl {
      display: none;
    }
  }
  @container (max-width: 440px) {
    .opt2 {
      display: none;
    }
  }
  .layout button {
    display: flex;
    align-items: center;
    padding: 0 7px;
  }
  .scroll {
    flex: 1;
    overflow: auto;
  }
  .msgs {
    max-width: 780px;
    margin: 0 auto;
    padding: 28px 28px 24px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }
  .msgs > :global(*) {
    content-visibility: auto;
    contain-intrinsic-size: auto 60px;
  }
  .empty {
    padding: 90px 0 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    text-align: center;
  }
  .empty .t {
    font-size: 18px;
    font-weight: 600;
  }
  .empty .s {
    color: var(--muted);
    max-width: 420px;
    text-wrap: pretty;
    line-height: 1.5;
  }
  .empty .warn {
    margin-top: 10px;
    max-width: 420px;
    color: var(--wait);
    font-size: 12.5px;
  }
  .load-error {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 14px;
    border-radius: var(--r);
    border: 1px solid color-mix(in oklch, var(--del) 50%, transparent);
    font-size: 12.5px;
    color: var(--muted);
  }
  .assistant {
    display: flex;
    gap: 12px;
    min-width: 0;
  }
  .blocks {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .avatar {
    width: 22px;
    height: 22px;
    flex: none;
    margin-top: 1px;
    border-radius: var(--r-sm);
    background: var(--accent);
    color: var(--accent-ink);
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .avatar.ghost {
    background: transparent;
  }
  .working {
    margin-left: 34px;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12.5px;
    color: var(--muted);
  }
  .dots {
    display: inline-flex;
    gap: 3px;
  }
  .dots span {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--accent);
    animation: ccBlink 1.2s infinite;
  }
  .dots span:nth-child(2) {
    animation-delay: 0.15s;
  }
  .dots span:nth-child(3) {
    animation-delay: 0.3s;
  }
  .jump {
    position: absolute;
    left: 50%;
    bottom: 150px;
    transform: translateX(-50%);
    height: 30px;
    padding: 0 14px;
    border-radius: 99px;
    border: 1px solid var(--line2);
    background: var(--elev);
    font-size: 12px;
    cursor: pointer;
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.35);
    z-index: 5;
  }
  .banner {
    max-width: 780px;
    width: calc(100% - 56px);
    margin: 0 auto 10px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px 14px;
    border-radius: var(--r);
    border: 1px solid var(--line2);
    background: var(--panel);
    font-size: 12.5px;
    color: var(--muted);
  }
  .banner.err {
    border-color: color-mix(in oklch, var(--del) 50%, transparent);
  }
</style>
