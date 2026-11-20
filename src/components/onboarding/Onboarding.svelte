<script lang="ts">
  import { accountName, accountsOf, engineOrder, hasAccount } from '../../lib/accounts';
  import { engineName } from '../../lib/engines';
  import { RECOMMENDED_MODE } from '../../lib/first-mode';
  import { openMap } from '../../lib/discover.svelte';
  import { t } from '../../lib/i18n';
  import { before, close, end, leave, onboarding, refresh, skipAll, stations, statusOf, type Primary } from '../../lib/onboarding.svelte';
  import { locks } from '../../lib/locks.svelte';
  import { app } from '../../lib/state.svelte';
  import type { OnboardingStep } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import AccountStation from './AccountStation.svelte';
  import DefaultsStation from './DefaultsStation.svelte';
  import AgentStation from './AgentStation.svelte';
  import HarnessStation from './HarnessStation.svelte';
  import HookupsStation from './HookupsStation.svelte';
  import ProjectStation from './ProjectStation.svelte';
  import TelemetryStation from './TelemetryStation.svelte';

  // The first launch (ESC-504, docs/ONBOARDING.md, direction C « La ligne »): the whole window but its title bar and its
  // status bar. On the left, the line: one station per question, drawn as the commits of the git graph (done: full; now:
  // teal; passed: hollow, « à reprendre »; imposed: done, « imposé »). On the right, one question at a time, its why and
  // its choices; at the foot, « Retour », « Plus tard » and the main button. All of it by the keyboard: ↑ ↓ choose, Entrée
  // goes on, Échap is « Plus tard ». The core keeps where it stands and reads the real state of the app: the view is
  // asked again after each change of what makes a station done.

  let primary = $state<Primary | null>(null);
  let busy = $state(false);
  const at = $derived(onboarding.at);
  const list = $derived(stations());
  const redoing = $derived(onboarding.view?.trigger === 'redo');

  // What makes a station done, wherever it changes (an account added in Réglages, a project made by the assistant).
  $effect(() => {
    void app.settings.accounts?.length;
    void app.settings.agentMode;
    void app.settings.telemetry;
    void app.projects.length;
    // A first turn ended (ESC-505): the station of the first agent.
    void Object.values(app.agents)
      .map((a) => a.status + a.prompts)
      .join();
    void locks.current;
    void refresh();
  });

  /** « Continuer »: the station's gesture, then the next station. */
  async function go() {
    if (!at || !primary || primary.disabled || busy) return;
    busy = true;
    try {
      if (await primary.run()) await leave(at, 'done');
    } finally {
      busy = false;
    }
  }
  async function later() {
    if (!at || busy) return;
    await leave(at, 'skipped');
  }
  function back() {
    const b = at ? before(at) : list.at(-1);
    if (b) onboarding.at = b;
  }
  /** « Tout passer », or « Fermer » when it is done again: nothing set is undone. */
  async function leaveAll() {
    if (redoing) {
      await end();
      close();
    } else await skipAll();
  }
  /** The end: the app, on its first project when there is one. */
  function start() {
    close();
    const p = app.projects[0];
    if (p) app.selectProject(p.id);
  }
  /** The end, on the map of the discovery (ESC-506): what Escouade does beyond the first agent, each to try for real. */
  function toMap() {
    start();
    openMap();
  }

  function onkeydown(e: KeyboardEvent) {
    // A window over it (« Ajouter un compte… », the assistant of a project) has its own keys.
    if (app.modal || e.defaultPrevented) return;
    const target = e.target as HTMLElement;
    // The first agent's real conversation keeps its own keys (Échap stops the agent there, its shortcuts answer its card).
    if (target.closest('[data-onb-frame]')) return;
    if (e.key === 'Escape' && at) {
      e.preventDefault();
      void later();
    } else if (e.key === 'Enter' && !e.shiftKey && !e.ctrlKey && !target.closest('button, a, input, textarea, select')) {
      e.preventDefault();
      if (at) void go();
      else start();
    } else if ((e.key === 'ArrowDown' || e.key === 'ArrowUp') && !target.closest('[role=radiogroup]')) {
      // From the question, to its choices.
      const radio = document.querySelector<HTMLElement>('.onb [role=radio][tabindex="0"], .onb [role=checkbox][data-card]');
      if (radio) {
        e.preventDefault();
        radio.focus();
      }
    }
  }

  /** What the line says under a station: now, passed, imposed, or what is set in place. */
  function sub(step: OnboardingStep): string {
    const status = statusOf(step);
    if (step === at) return t('onboarding.state.now');
    if (status === 'skipped') return t('onboarding.state.skipped');
    if (status === 'policy') return t('onboarding.state.policy');
    if (status !== 'done') return t(`onboarding.stations.${step}.todo`);
    const engine = onboarding.engine ?? engineOrder().find((e) => hasAccount(e));
    switch (step) {
      case 'harness':
        return engine ? engineName(engine) : t(`onboarding.stations.${step}.todo`);
      case 'account': {
        const a = engine ? accountsOf(engine)[0] : app.settings.accounts[0];
        return a ? accountName(a) : t(`onboarding.stations.${step}.todo`);
      }
      case 'defaults': {
        const m = (app.settings.agentMode?.trim() || RECOMMENDED_MODE) as 'acceptEdits';
        return t(`composer.mode.${m}.label`);
      }
      case 'hookups': {
        const n = onboarding.view?.hookups?.filter((c) => c.status === 'done').length ?? 0;
        return n ? t('onboarding.hookups.plugged', { count: n }) : t('onboarding.hookups.nothing');
      }
      case 'project':
        return app.projects[0]?.name ?? t(`onboarding.stations.${step}.todo`);
      case 'agent':
        return Object.values(app.agents).find((a) => a.prompts > 0)?.name ?? t(`onboarding.stations.${step}.todo`);
      case 'telemetry':
        return app.settings.telemetry ? t('shell.telemetry.yes') : t('shell.telemetry.no');
    }
  }
  /** The look of a station's node: done (and imposed) full, now teal, passed hollow, to do dotted. */
  function look(step: OnboardingStep): 'cur' | 'done' | 'skip' | 'todo' {
    if (step === at) return 'cur';
    const s = statusOf(step);
    return s === 'done' || s === 'policy' ? 'done' : s === 'skipped' ? 'skip' : 'todo';
  }
  const skipped = $derived(list.filter((s) => statusOf(s) === 'skipped').length);
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions: its keys (Entrée, Échap, ↑ ↓) are the journey's, at any element of it. -->
<div class="onb" role="region" aria-label={t('onboarding.title')} {onkeydown} data-testid="onboarding">
  <nav class="line" aria-label={t('onboarding.line')}>
    <div class="brand">
      <span class="logo" aria-hidden="true"><Icon name="agents" size={16} /></span>
      <span class="bt"><b>{t('onboarding.title')}</b><span>{t('onboarding.subtitle', { count: list.length })}</span></span>
    </div>
    <ol class="stations">
      {#each list as step (step)}
        <li class="st {look(step)}">
          <button class="go" aria-current={step === at ? 'step' : undefined} onclick={() => (onboarding.at = step)}>
            <span class="nd" aria-hidden="true"><i></i></span>
            <span class="nm">{t(`onboarding.stations.${step}.name`)}</span>
            <span class="sub">{sub(step)}</span>
          </button>
        </li>
      {/each}
    </ol>
    <div class="fan" aria-hidden="true">
      <svg width="180" height="70">
        {#each [1, 2, 3, 4, 5] as l, i (l)}
          <path d="M1 0 C 1 30, {30 + i * 36} 26, {30 + i * 36} 66" style:stroke="var(--lane-{l})" />
        {/each}
      </svg>
    </div>
    <div class="then">
      <span class="h">{t('onboarding.then')}</span>
      <span>{t('onboarding.thenLanes')}</span>
    </div>
    <div class="later">
      <span>{t('onboarding.resume')}</span>
      <button class="btn ghost all" onclick={leaveAll}>{redoing ? t('onboarding.close') : t('onboarding.skipAll')}</button>
    </div>
  </nav>
  <main class="main">
    {#key at}
      <div class="cq">
        {#if at === 'harness'}
          <HarnessStation bind:primary />
        {:else if at === 'account'}
          <AccountStation bind:primary />
        {:else if at === 'defaults'}
          <DefaultsStation bind:primary />
        {:else if at === 'hookups'}
          <HookupsStation bind:primary />
        {:else if at === 'project'}
          <ProjectStation bind:primary />
        {:else if at === 'agent'}
          <AgentStation bind:primary />
        {:else if at === 'telemetry'}
          <TelemetryStation bind:primary />
        {:else}
          <div class="end" data-testid="onboarding-end">
            <h1 tabindex="-1">{t('onboarding.end.title')}</h1>
            <p class="why">{t('onboarding.end.body')}</p>
            {#if skipped}<p class="why">{t('onboarding.end.skipped', { count: skipped })}</p>{/if}
          </div>
        {/if}
      </div>
    {/key}
    <div class="foot">
      {#if at ? before(at) : list.length}
        <button class="btn ghost" onclick={back}><Icon name="left" size={13} />{t('onboarding.back')}</button>
      {/if}
      {#if at && primary}
        <span class="keys" aria-hidden="true"
          ><kbd>↑</kbd><kbd>↓</kbd> {t('onboarding.choose')} · <kbd>{t('onboarding.enter')}</kbd> {t('onboarding.confirm')}</span
        >
      {/if}
      <span class="sp"></span>
      {#if at}
        <button class="btn ghost" disabled={busy} onclick={later}>{t('onboarding.later')}</button>
        {#if primary}
          <button class="btn primary lg" disabled={busy || primary.disabled} onclick={go}
            >{primary.label}<Icon name="right" size={13} /></button
          >
        {/if}
      {:else}
        <button class="btn ghost" onclick={toMap}>{t('onboarding.end.discover')}</button>
        <button class="btn primary lg" onclick={start}>{t('onboarding.end.start')}<Icon name="right" size={13} /></button>
      {/if}
    </div>
  </main>
</div>

<style>
  .onb {
    grid-column: 1 / -1;
    grid-row: 3;
    display: grid;
    grid-template-columns: var(--line-col, 300px) minmax(0, 1fr);
    min-height: 0;
    min-width: 0;
  }
  .line {
    display: flex;
    flex-direction: column;
    min-height: 0;
    padding: 26px 0 20px 28px;
    overflow: hidden auto;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 26px;
  }
  .logo {
    display: grid;
    flex: none;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 9px;
    background: var(--brand);
    color: #08090a;
  }
  .bt {
    display: grid;
  }
  .bt b {
    font-size: 14px;
    font-weight: 640;
  }
  .bt span {
    color: var(--muted);
    font-size: 12px;
  }
  .stations {
    display: grid;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .st {
    position: relative;
    padding-bottom: 22px;
  }
  /* The lane between two stations: drawn under a done one, dotted under one still to do. */
  .st::before {
    content: '';
    position: absolute;
    top: 14px;
    bottom: -2px;
    left: 10px;
    width: 2px;
    background: var(--lane-main);
    opacity: 0.85;
  }
  .st.todo::before,
  .st.skip::before {
    background: repeating-linear-gradient(var(--lane-main) 0 4px, transparent 4px 8px);
    opacity: 0.45;
  }
  .st:last-child::before {
    display: none;
  }
  .go {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr);
    gap: 0 14px;
    width: calc(100% - 20px);
    padding: 0;
    border: 0;
    border-radius: 6px;
    background: none;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .go:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 2px;
  }
  .nd {
    z-index: 1;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
  }
  .nd i {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: var(--frame);
    box-shadow: inset 0 0 0 2px var(--lane-main);
  }
  .st.done .nd i {
    background: var(--lane-main);
  }
  .st.cur .nd i {
    width: 16px;
    height: 16px;
    background: var(--brand);
    box-shadow: 0 0 0 4px color-mix(in oklch, var(--brand) 22%, transparent);
  }
  .st.skip .nd i {
    opacity: 0.6;
  }
  .nm {
    padding-top: 1px;
    color: var(--muted);
    font-size: 13px;
    font-weight: 600;
  }
  .st.done .nm,
  .st.cur .nm {
    color: var(--text);
  }
  .sub {
    grid-column: 2;
    margin-top: 1px;
    overflow: hidden;
    color: var(--muted);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .st.cur .sub {
    color: var(--teal-ink);
  }
  .fan {
    position: relative;
    flex: none;
    height: 70px;
    margin-left: 10px;
  }
  .fan svg {
    position: absolute;
    top: 0;
    left: 0;
    overflow: visible;
  }
  .fan path {
    fill: none;
    stroke-width: 2;
    stroke-linecap: round;
    opacity: 0.6;
  }
  .then {
    display: grid;
    gap: 4px;
    margin: 0 16px 0 2px;
    color: var(--muted);
    font-size: 12px;
  }
  .then .h {
    color: var(--soft);
    font-weight: 600;
  }
  .later {
    display: grid;
    gap: 8px;
    margin-top: auto;
    padding: 16px 20px 0 0;
    color: var(--muted);
    font-size: 12px;
  }
  .all {
    justify-self: start;
  }
  .main {
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    min-width: 0;
    min-height: 0;
    margin-right: 8px;
    border-radius: 12px 12px 0 0;
    background: var(--panel);
    box-shadow: inset 0 0 0 1px var(--line);
    overflow: hidden;
  }
  .cq {
    display: grid;
    align-content: start;
    gap: 22px;
    min-height: 0;
    padding: 56px 64px 24px;
    overflow: auto;
    animation: arrive 240ms var(--ease, ease-out);
  }
  @keyframes arrive {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
  .end {
    display: grid;
    gap: 14px;
    width: min(640px, 100%);
  }
  .end h1:focus,
  .end h1:focus-visible {
    outline: none;
  }
  .end h1 {
    margin: 0;
    font-size: 26px;
    font-weight: 650;
    letter-spacing: -0.02em;
    outline: none;
  }
  .why {
    margin: 0;
    color: var(--muted);
    font-size: 13.5px;
    line-height: 1.6;
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 64px;
    border-top: 1px solid var(--line);
  }
  .keys {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--dim);
    font-size: 12px;
  }
  .sp {
    flex: 1;
  }
  @media (max-width: 1100px) {
    .onb {
      --line-col: 248px;
    }
    .line {
      padding: 22px 0 16px 20px;
    }
    .cq {
      gap: 18px;
      padding: 34px 32px 18px;
    }
    .foot {
      padding: 12px 32px;
    }
    .keys {
      display: none;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .cq {
      animation: fade 120ms linear;
    }
    @keyframes fade {
      from {
        opacity: 0;
      }
    }
  }
</style>
