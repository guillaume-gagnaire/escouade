<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { engineOrder, hasAccount } from '../../lib/accounts';
  import { conversationOf } from '../../lib/conversations.svelte';
  import { engineName } from '../../lib/engines';
  import { firstTurn, starters, type Starter } from '../../lib/first-agent';
  import { samePath } from '../../lib/first-project';
  import { fTime } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import Rich from '../../lib/i18n/Rich.svelte';
  import { api } from '../../lib/ipc';
  import { keyLabel } from '../../lib/platform';
  import { onboarding, type Primary } from '../../lib/onboarding.svelte';
  import { app } from '../../lib/state.svelte';
  import Conversation from '../Conversation.svelte';
  import Icon from '../Icon.svelte';
  import Choices from './Choices.svelte';
  import StationHead from './StationHead.svelte';

  // Station 5, the first agent (ESC-505, docs/ONBOARDING.md § 4.5, direction C): the real conversation framed beside
  // its explanation. The agent is made with what the stations before set (its harness, its account's model and effort,
  // the mode of the new agents); a message is proposed (several, after the project), put in its composer, editable,
  // and sent by the user's own click on the composer's button: nothing goes out by itself, and the quota it uses is
  // said. Then the explanation follows its turn: its answer live, its first permission beside the real card (what
  // Autoriser, Toujours and Refuser mean), its diff once it is done, where to find it and how to stop it; and when the
  // turn stops short (the usage limit, offline, not signed in), what to do, with the way back.
  let { primary = $bindable() }: { primary: Primary | null } = $props();

  const project = $derived((onboarding.project ? app.projects.find((p) => p.id === onboarding.project) : null) ?? app.projects[0] ?? null);
  const engine = $derived(
    onboarding.engine && hasAccount(onboarding.engine) ? onboarding.engine : (engineOrder().find((e) => hasAccount(e)) ?? null),
  );
  let agentId = $state<string | null>(null);
  let failed = $state(false);
  const agent = $derived(agentId ? (app.agents[agentId] ?? null) : null);
  /** Its conversation, taken once the agent is known (taking it loads it: never inside a derived value). */
  let conv = $state<ReturnType<typeof conversationOf> | null>(null);
  const items = $derived(conv?.items ?? []);
  const turn = $derived(agent ? firstTurn(agent, items, conv?.waiting ?? []) : null);
  const harness = $derived(engineName(agent?.engine ?? engine ?? 'claude'));
  const isDemo = $derived(!!project && samePath(project.path, onboarding.view?.demo ?? ''));

  let tests = $state(false);
  const list = $derived(starters(tests, isDemo));
  let starter = $state<string | null>(null);

  onMount(() => {
    const p = untrack(() => project);
    if (!p || !untrack(() => engine)) return;
    void (async () => {
      tests = (await api.onboardingFolder(p.path).catch(() => null))?.tests ?? false;
      // The first message proposed, once the project is read (the demonstration starts with its tests).
      starter = untrack(() => list[0]) ?? null;
      app.selectProject(p.id);
      // The project's agent there already (the station shown again), else a new one, with the stations' settings.
      let a = Object.values(app.agents).find((x) => x.projectId === p.id && !x.archived);
      if (!a) {
        await app.newAgent(
          p.id,
          untrack(() => engine),
        );
        a = app.agent && app.agent.projectId === p.id ? app.agent : undefined;
      }
      if (!a) {
        failed = true;
        return;
      }
      app.selectAgent(a.id);
      conv = conversationOf(a.id);
      agentId = a.id;
    })();
  });

  // The message chosen put in the composer while nothing is sent: never sent from here.
  $effect(() => {
    const s = starter as Starter | null;
    const id = agentId;
    if (!s || !id || untrack(() => turn?.state) !== 'compose') return;
    app.composerPut = { agentId: id, text: t(`onboarding.agent.starters.${s}`), replace: true };
  });

  $effect(() => {
    primary = { label: t('onboarding.continue'), disabled: turn?.state !== 'done', run: async () => true };
  });

  function diff() {
    if (!agent || !project) return;
    app.modal = {
      kind: 'diff',
      projectId: project.id,
      agentId: agent.id,
      paths: agent.worktree ? [] : agent.touchedFiles,
      title: t('onboarding.agent.done.diffTitle', { agent: agent.name }),
    };
  }
  function again() {
    if (agent && turn?.sent) app.composerPut = { agentId: agent.id, text: turn.sent, replace: true };
  }
  const modeLabel = $derived(agent ? t(`composer.mode.${(agent.mode || 'default') as 'default'}.label`) : '');
</script>

<div class="q" data-testid="onboarding-agent">
  <div class="side">
    {#if !project}
      <StationHead step="agent" question={t('onboarding.agent.question')} />
      <p class="note" role="status">{t('onboarding.agent.noProject')}</p>
      <button class="btn" onclick={() => (onboarding.at = 'project')}
        ><Icon name="left" size={13} />{t('onboarding.agent.toProject')}</button
      >
    {:else if !engine}
      <StationHead step="agent" question={t('onboarding.agent.question')} />
      <p class="note" role="status">{t('onboarding.agent.noAccount')}</p>
      <button class="btn" onclick={() => (onboarding.at = 'account')}
        ><Icon name="left" size={13} />{t('onboarding.agent.signedOut.back')}</button
      >
    {:else if !turn || turn.state === 'compose'}
      <StationHead step="agent" question={t('onboarding.agent.question')}>
        {#snippet why()}{t('onboarding.agent.why', { harness })}{/snippet}
      </StationHead>
      {#if failed}<p class="bad" role="alert">{t('onboarding.agent.noAccount')}</p>{/if}
      <Choices items={list.map((id) => ({ id }))} bind:value={starter} label={t('onboarding.agent.startersLabel')}>
        {#snippet head(it: { id: string })}<span class="st">{t(`onboarding.agent.starters.${it.id as Starter}`)}</span>{/snippet}
      </Choices>
      <p class="note" data-testid="onboarding-agent-ready">
        {agent ? t('onboarding.agent.ready', { agent: agent.name }) : t('onboarding.agent.preparing')}
      </p>
    {:else if turn.state === 'preparing'}
      <StationHead step="agent" question={t('onboarding.agent.preparing2.title')}>
        {#snippet why()}{t('onboarding.agent.preparing2.body')}{/snippet}
      </StationHead>
      <p class="note">{t('onboarding.agent.quota', { harness })}</p>
    {:else if turn.state === 'running'}
      <StationHead step="agent" question={t('onboarding.agent.running.title')}>
        {#snippet why()}{t('onboarding.agent.running.body')}{/snippet}
      </StationHead>
      <p class="note">{t('onboarding.agent.running.stop')}</p>
      <p class="note">{t('onboarding.agent.quota', { harness })}</p>
    {:else if turn.state === 'permission'}
      <StationHead step="agent" question={t('onboarding.agent.permission.title')}>
        {#snippet why()}
          <Rich k="onboarding.agent.permission.body" mode={modeLabel}>
            {#snippet allow()}<b>{t('onboarding.agent.permission.allow')}</b>{/snippet}
            {#snippet always()}<b>{t('onboarding.agent.permission.always')}</b>{/snippet}
            {#snippet deny()}<b>{t('onboarding.agent.permission.deny')}</b>{/snippet}
          </Rich>
        {/snippet}
      </StationHead>
      <p class="note">
        {t('onboarding.agent.permission.keys', {
          allow: keyLabel('Ctrl+Enter'),
          always: keyLabel('Ctrl+Shift+Enter'),
          next: keyLabel('Ctrl+J'),
        })}
      </p>
      <p class="note">{t('onboarding.agent.quota', { harness })} {t('onboarding.agent.permission.later')}</p>
    {:else if turn.state === 'asking'}
      <StationHead step="agent" question={t('onboarding.agent.asking.title')}>
        {#snippet why()}{t('onboarding.agent.asking.body')}{/snippet}
      </StationHead>
    {:else if turn.state === 'done' && agent}
      <StationHead step="agent" question={t('onboarding.agent.done.title')}>
        {#snippet why()}{t('onboarding.agent.done.body')}{/snippet}
      </StationHead>
      {#if agent.touchedFiles.length}
        <div class="row">
          <button class="btn" data-testid="onboarding-diff" onclick={diff}
            ><Icon name="diagram" size={13} />{t('onboarding.agent.done.diff')}</button
          >
          <span class="note">{t('onboarding.agent.done.files', { count: agent.touchedFiles.length })}</span>
        </div>
      {:else}
        <p class="note">{t('onboarding.agent.done.noDiff')}</p>
      {/if}
      <p class="note">{t('onboarding.agent.done.where', { agent: agent.name })}</p>
      <p class="note">{t('onboarding.agent.running.stop')}</p>
    {:else if turn.state === 'limit'}
      <StationHead step="agent" question={t('onboarding.agent.limit.title')} />
      <p class="warn" role="alert">
        {turn.until ? t('onboarding.agent.limit.body', { time: fTime(turn.until) }) : t('onboarding.agent.limit.bodyNoTime')}
      </p>
      <div class="row">
        <button class="btn" onclick={() => (app.modal = { kind: 'account', engine: agent?.engine ?? engine ?? 'claude', onboarding: true })}
          >{t('onboarding.agent.limit.add')}</button
        >
      </div>
    {:else if turn.state === 'offline'}
      <StationHead step="agent" question={t('onboarding.agent.offline.title')} />
      <p class="warn" role="alert">{t('onboarding.agent.offline.body', { harness })}</p>
      {#if turn.sent}<div class="row"><button class="btn" onclick={again}>{t('onboarding.agent.offline.again')}</button></div>{/if}
    {:else if turn.state === 'signedOut'}
      <StationHead step="agent" question={t('onboarding.agent.signedOut.title', { harness })} />
      <p class="warn" role="alert">{t('onboarding.agent.signedOut.body', { harness })}</p>
      <div class="row">
        <button class="btn" onclick={() => (onboarding.at = 'account')}
          ><Icon name="left" size={13} />{t('onboarding.agent.signedOut.back')}</button
        >
      </div>
    {:else}
      <StationHead step="agent" question={t('onboarding.agent.failed.title')} />
      <p class="warn" role="alert">{t('onboarding.agent.failed.body', { error: (turn.error ?? '—').replace(/[.\s]+$/, '') })}</p>
      {#if turn.sent}<div class="row"><button class="btn" onclick={again}>{t('onboarding.agent.offline.again')}</button></div>{/if}
    {/if}
  </div>
  {#if agent && project}
    <!-- The real conversation, framed: its composer sends, its permission card answers, as anywhere in the app. -->
    <section class="frame" aria-label={t('onboarding.agent.frame')} data-onb-frame data-testid="onboarding-frame">
      {#key agent.id}<Conversation {agent} {project} />{/key}
    </section>
  {/if}
</div>

<style>
  .q {
    display: grid;
    grid-template-columns: minmax(0, 380px) minmax(0, 1fr);
    gap: 32px;
    align-items: start;
  }
  .side {
    display: grid;
    align-content: start;
    gap: 16px;
  }
  .frame {
    display: grid;
    min-width: 0;
    height: clamp(420px, calc(100vh - 250px), 680px);
    overflow: hidden;
    border-radius: 12px;
    background: var(--frame);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .frame > :global(*) {
    min-height: 0;
  }
  .st {
    color: var(--text);
    font-size: 13px;
  }
  .note,
  .warn,
  .bad {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .warn {
    color: var(--warn-ink);
  }
  .bad {
    color: var(--del);
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
  }
  /* 1024: the explanation over the conversation, as the design draws it. */
  @media (max-width: 1100px) {
    .q {
      grid-template-columns: minmax(0, 1fr);
      gap: 16px;
    }
    .frame {
      height: clamp(360px, calc(100vh - 330px), 520px);
    }
  }
</style>
