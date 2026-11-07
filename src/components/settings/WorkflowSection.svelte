<script lang="ts">
  import Icon from '../Icon.svelte';
  import Rich from '../../lib/i18n/Rich.svelte';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { fDate, fTime, fTok, fUsd } from '../../lib/format';
  import { settingsForm } from '../../lib/settings.svelte';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { Project, Workflow, WorkflowRefinePlan, WorkflowScan, WorkflowSignal } from '../../lib/types';
  import {
    agentsOf,
    fixFrom,
    fixOf,
    merge,
    move,
    removedBy,
    signalKind,
    signalOf,
    signalValue,
    sourceGroups,
    stepRows,
    withFix,
    workflowName,
    type StepRow,
  } from '../../lib/workflow-fixes';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';

  // Réglages › Projets › Workflow (ESC-195, `#loops-settings` and `#loops-settings-steps` of the 2.0 design): what the
  // reading found and from what, « Relire », the reading again by itself, the workflows found, and the steps of the one
  // chosen as the user corrects them (the draft's `workflowFixes`), kept in the files of the workflows (in the repository, or on this machine).
  let { project }: { project: Project } = $props();

  const draft = $derived(settingsForm.project!);

  let scan = $state<WorkflowScan | null>(null);
  let scanOf = $state('');
  let reading = $state(false);
  let failed = $state('');

  async function load(id: string, force: boolean) {
    reading = true;
    failed = '';
    try {
      const s = await (force ? api.refreshWorkflows(id) : api.getWorkflows(id));
      if (id !== project.id) return;
      scan = s;
      if (force) app.toast(t('workflowSettings.rereadDone'));
    } catch (e) {
      if (id === project.id) failed = String(e);
    } finally {
      if (id === project.id) reading = false;
    }
  }

  // Read again when the project changes, or when « Pas de workflow pour ce projet » is saved or undone (ESC-309).
  const readKey = $derived(`${project.id}:${project.workflowsEnabled !== false}`);
  $effect(() => {
    const key = readKey;
    if (scanOf !== key) {
      scan = null;
      scanOf = key;
      void load(project.id, false);
    }
  });

  const shownScan = $derived(scanOf === readKey ? scan : null);
  const sources = $derived(shownScan ? sourceGroups(shownScan) : []);
  const workflows = $derived(shownScan?.workflows ?? []);
  const dismissed = (w: Workflow) => !!fixOf(draft.workflowFixes, w)?.dismissed;
  /** The workflow whose steps are corrected: the one chosen, else the first that is one. */
  let chosen = $state('');
  const current = $derived(workflows.find((w) => w.id === chosen && !dismissed(w)) ?? workflows.find((w) => !dismissed(w)) ?? null);
  const hasTickets = $derived(Object.values(app.tickets).some((x) => x.projectId === project.id));

  const readLabel = $derived.by(() => {
    if (reading && !shownScan) return t('workflowSettings.reading');
    const at = shownScan?.readAt;
    if (!at) return t('workflowSettings.reading');
    const today = new Date().toDateString() === new Date(at).toDateString();
    return today ? t('workflowSettings.readAt', { time: fTime(at) }) : t('workflowSettings.readOn', { date: fDate(at) });
  });

  // ---------- the steps of the workflow chosen ----------

  const fix = $derived(current ? fixOf(draft.workflowFixes, current) : undefined);
  const rows = $derived(current ? stepRows(current, fix) : []);

  /** The rows as they become, the steps read they no longer have taken out. */
  function setRows(next: StepRow[], w = current) {
    if (!w) return;
    const removed = removedBy(rows, next, fix?.removed ?? []);
    draft.workflowFixes = withFix(draft.workflowFixes, w, fixFrom(w, next, removed, !!fix?.dismissed));
  }
  const rename = (i: number, name: string) => setRows(rows.map((r, j) => (j === i ? { ...r, name } : r)));
  const keep = (i: number) => setRows(rows.map((r, j) => (j === i ? { ...r, guessed: false } : r)));
  const removeAt = (i: number) => setRows(rows.filter((_, j) => j !== i));
  function moveBy(i: number, by: number, input?: HTMLInputElement) {
    const j = i + by;
    if (j < 0 || j >= rows.length) return;
    setRows(move(rows, i, j));
    // The focus follows the step to its new place.
    if (input) queueMicrotask(() => document.querySelectorAll<HTMLInputElement>('.wf-er .step-name')[j]?.focus());
  }
  function add() {
    setRows([...rows, { key: '', name: '', from: [], signals: [{ command: '' }], marks: [], guessed: false, stale: false }]);
    queueMicrotask(() => {
      const names = document.querySelectorAll<HTMLInputElement>('.wf-er .step-name');
      names[names.length - 1]?.focus();
    });
  }
  function setSignal(i: number, s: WorkflowSignal) {
    setRows(rows.map((r, j) => (j === i ? { ...r, signals: [s, ...r.signals.slice(1)] } : r)));
  }
  function setDismissed(w: Workflow, on: boolean) {
    const f = fixOf(draft.workflowFixes, w);
    const next = on ? { ...(f ?? { workflow: w.id }), dismissed: true } : f && { ...f, dismissed: undefined };
    const empty = next && !next.dismissed && !next.steps?.length && !next.removed?.length;
    draft.workflowFixes = withFix(draft.workflowFixes, w, empty ? undefined : next);
  }

  let dragged = $state<number | null>(null);
  function drop(i: number) {
    if (dragged !== null) setRows(move(rows, dragged, i));
    dragged = null;
  }

  /** The ways back of the workflow chosen, said with the names of its steps as corrected. */
  const loopsLine = $derived.by(() => {
    if (!current) return '';
    const nameOf = (n: string) => rows.find((r) => r.from.includes(n))?.name || null;
    const seen = new Set<string>();
    const out: string[] = [];
    for (const tr of current.transitions.filter((x) => x.back)) {
      const [from, to] = [nameOf(tr.from), nameOf(tr.to)];
      if (!from || !to || seen.has(`${from}→${to}`)) continue;
      seen.add(`${from}→${to}`);
      const text = from === to ? t('workflowSettings.again', { step: from }) : t('workflowSettings.back', { from, to });
      out.push(tr.max ? t('workflowSettings.upTo', { count: tr.max, text }) : text);
    }
    return out.join(' · ');
  });

  const SIGNALS = ['command', 'agent', 'file'] as const;

  /** The file of a workflow, as the backend says it. */
  const fileOf = (w: Workflow) => shownScan?.files?.find((f) => f.id === w.id);

  // « Affiner… » (ESC-196): the plan is said first (model, account, cost), the agent asked on « Lancer » only.
  let plan = $state<WorkflowRefinePlan | null>(null);
  let planFailed = $state('');
  let asking = $state(false);
  let rejected = $state<string[]>([]);
  let refineMsg = $state('');
  let refineBad = $state(false);

  async function offerRefine() {
    plan = null;
    planFailed = '';
    rejected = [];
    refineMsg = '';
    refineBad = false;
    try {
      const id = project.id;
      const p = await api.workflowRefinePlan(id);
      if (id === project.id) plan = p;
    } catch (e) {
      planFailed = String(e);
    }
  }

  async function refine() {
    const id = project.id;
    asking = true;
    refineMsg = '';
    refineBad = false;
    try {
      const out = await api.refineWorkflows(id);
      if (id !== project.id) return;
      scan = out.scan;
      rejected = out.rejected;
      const added = out.refined.reduce((n, r) => n + r.added.length, 0);
      const described = out.refined.reduce((n, r) => n + r.described, 0);
      const ways = out.refined.reduce((n, r) => n + r.transitions, 0);
      const parts = [
        added && t('workflowSettings.refineAdded', { count: added }),
        described && t('workflowSettings.refineDescribed', { count: described }),
        ways && t('workflowSettings.refineWays', { count: ways }),
      ].filter(Boolean);
      refineMsg = parts.length ? t('workflowSettings.refineDone', { summary: parts.join(', ') }) : t('workflowSettings.refineNothing');
      app.toast(refineMsg);
      plan = null;
    } catch (e) {
      refineMsg = t('workflowSettings.refineFailed', { error: String(e) });
      refineBad = true;
    } finally {
      asking = false;
    }
  }

  /** « Retirer du dépôt »: the file of a workflow this machine does not read goes, at once. */
  async function removeFile(w: Workflow) {
    reading = true;
    const s = await app.run(api.removeWorkflowFile(project.id, w.id));
    if (s && scanOf === readKey) scan = s;
    reading = false;
  }

  /**
   * « Supprimer » a workflow (its file goes at once, no reading writes it again), « Laisser supprimé » one proposed again, or
   * « Rétablir » it (ESC-309).
   */
  async function setRemoved(id: string, name: string, removed: boolean) {
    reading = true;
    const key = readKey;
    const s = await app.run(api.setWorkflowRemoved(project.id, id, removed));
    if (s && scanOf === key) {
      scan = s;
      if (removed && chosen === id) chosen = '';
      app.toast(t(removed ? 'workflowSettings.removedDone' : 'workflowSettings.restoredDone', { name }));
    }
    reading = false;
  }
</script>

{#snippet gate(text: string)}<span class="gate">{text}</span>{/snippet}

<Group title={t('workflowSettings.title')} anchor="workflow" id="sg-workflow">
  <Row label={t('workflowSettings.enabled')} desc={t('workflowSettings.enabledDesc')} shared="workflows.enabled">
    <div class="segmented" role="radiogroup" aria-label={t('workflowSettings.enabled')}>
      {#each [true, false] as on (on)}
        <button
          role="radio"
          aria-checked={draft.workflowsEnabled === on}
          class:on={draft.workflowsEnabled === on}
          onclick={() => (draft.workflowsEnabled = on)}>{t(on ? 'workflowSettings.on' : 'workflowSettings.off')}</button
        >
      {/each}
    </div>
  </Row>
  {#if !draft.workflowsEnabled}
    <p class="off-note" data-testid="workflows-off">
      <Rich k="workflowSettings.offNote">{#snippet dir()}<span class="mono">.escouade/workflows/</span>{/snippet}</Rich>
    </p>
  {:else if shownScan?.off}
    <p class="off-note" data-testid="workflows-off">{t('workflowSettings.onNote')}</p>
  {:else}
    <Row label={readLabel}>
      {#snippet extra()}
        <span class="more"
          ><Rich k="workflowSettings.readDesc"
            >{#snippet prompt()}<span class="mono">PROMPT.md</span>{/snippet}{#snippet at()}<span class="mono">@</span>{/snippet}</Rich
          ></span
        >
        {#each shownScan?.unreadable ?? [] as u (u.path)}
          <span class="bad">{t('workflowSettings.unreadable', { path: u.path, why: u.why })}</span>
        {/each}
        {#if failed}
          <span class="bad">{t('workflowSettings.readFailed', { error: failed })}</span>
        {:else if shownScan && !sources.length}
          <span class="more">{t('workflowSettings.noSource')}</span>
        {:else if sources.length}
          <span class="wf-src">
            {#each sources as s (s.path + s.included)}
              <span class="gate"
                >{s.path}{#if s.count > 1 || s.included}&nbsp;<span class="n"
                    >{s.included ? t('workflowSettings.included', { count: s.count }) : s.count}</span
                  >{/if}</span
              >
            {/each}
          </span>
        {/if}
      {/snippet}
      <button class="btn" disabled={reading} onclick={() => load(project.id, true)}
        ><Icon name="restart" size={14} />{t('workflowSettings.reread')}</button
      >
    </Row>
    <Row label={t('workflowSettings.refine')} desc={t('workflowSettings.refineDesc')}>
      {#snippet extra()}
        {#if plan}
          <span class="more wf-plan" data-testid="refine-plan">
            <span>{t('workflowSettings.refineWho', { model: plan.model, account: plan.account })}</span>
            <span
              >{plan.cost === null
                ? t('workflowSettings.refineCostUnknown', { tokens: fTok(plan.inputTokens) })
                : t('workflowSettings.refineCost', { tokens: fTok(plan.inputTokens), cost: fUsd(plan.cost) })}</span
            >
            <span
              >{t('workflowSettings.refineWeighs', {
                workflows: t('workflowSettings.refineWf', { count: plan.workflows }),
                files: t('workflowSettings.refineFiles', { count: plan.files }),
              })}</span
            >
            {#if plan.private}<span>{t('workflowSettings.refinePrivate')}</span>{/if}
            <span class="wf-plan-acts">
              <button class="btn primary small" disabled={asking} onclick={refine}>{t('workflowSettings.refineRun')}</button>
              <button class="btn small" disabled={asking} onclick={() => (plan = null)}>{t('workflowSettings.refineCancel')}</button>
            </span>
          </span>
        {/if}
        {#if asking}<span class="more">{t('workflowSettings.refineAsking')}</span>{/if}
        {#if planFailed}<span class="bad">{t('workflowSettings.refineFailed', { error: planFailed })}</span>{/if}
        {#if refineMsg}<span class={refineBad ? 'bad' : 'more'}>{refineMsg}</span>{/if}
        {#if rejected.length}
          <span class="more wf-plan"
            ><span>{t('workflowSettings.refineRejected')}</span>{#each rejected as r (r)}<span class="bad">{r}</span>{/each}</span
          >
        {/if}
      {/snippet}
      <button class="btn" disabled={reading || asking} onclick={offerRefine}
        ><Icon name="sparkle" size={14} />{t('workflowSettings.refineButton')}</button
      >
    </Row>
    <Row label={t('workflowSettings.auto')} desc={t('workflowSettings.autoDesc')}>
      <Switch label={t('workflowSettings.auto')} bind:on={draft.workflowsAutoRead} />
    </Row>

    <div class="sub-h"><b>{t('workflowSettings.found')}</b><span>{t('workflowSettings.foundDesc')}</span></div>
    <div class="rlist">
      {#each workflows as w (w.id)}
        {@const off = dismissed(w)}
        {#if off}
          <div class="rrow idle">
            <Icon name="flow" size={14} /><span class="rn plain">{workflowName(w)}</span><span class="rb plain"
              >{t('workflowSettings.steps', { count: w.steps.length })}</span
            ><span class="st"
              >{t('workflowSettings.stateDismissed')} ·
              <button class="link" onclick={() => setDismissed(w, false)}>{t('workflowSettings.restore')}</button></span
            >
          </div>
        {:else}
          <button
            class="rrow ok pick"
            aria-pressed={current?.id === w.id}
            aria-label={t('workflowSettings.choose', { name: workflowName(w) })}
            onclick={() => (chosen = w.id)}
          >
            <Icon name="flow" size={14} /><span class="rn plain">{workflowName(w)}</span><span class="rb plain"
              >{[
                t('workflowSettings.steps', { count: stepRows(w, fixOf(draft.workflowFixes, w)).length }),
                t('workflowSettings.doors', { count: w.doors.length }),
                t('workflowSettings.agents', { count: agentsOf(w) }),
                ...(fileOf(w) ? [fileOf(w)!.inRepo ? t('workflowSettings.placeRepo') : t('workflowSettings.placeLocal')] : []),
              ].join(' · ')}</span
            ><span class="st" use:tip={fileOf(w)?.updated ?? ''}
              >{fileOf(w)?.updated
                ? t('workflowSettings.stateUpdated')
                : fileOf(w) && !fileOf(w)!.local
                  ? t('workflowSettings.stateTeam')
                  : fixOf(draft.workflowFixes, w)
                    ? t('workflowSettings.stateFixed')
                    : t('workflowSettings.stateRead')}</span
            >
          </button>
          {#if fileOf(w) && !fileOf(w)!.local}
            <p class="team">
              {t('workflowSettings.teamOnly', { path: fileOf(w)!.path })}
              <button class="link" disabled={reading} onclick={() => removeFile(w)}>{t('workflowSettings.removeFile')}</button>
            </p>
          {/if}
        {/if}
      {/each}
      {#if hasTickets}
        <div class="rrow">
          <Icon name="kanban" size={14} /><span class="rn plain">{t('workflowSettings.tickets')}</span><span class="rb plain"
            >{t('workflowSettings.ticketsSteps')}</span
          ><span class="st">{t('workflowSettings.stateEscouade')}</span>
        </div>
      {/if}
      {#each shownScan?.alone ?? [] as a (a.repo + a.harness + a.command)}
        <div class="rrow idle">
          <Icon name="circle" size={14} /><span class="rn">{a.repo ? `${a.repo} · ${a.command}` : a.command}</span><span class="rb plain"
            >{t('workflowSettings.alone')}</span
          ><span class="st">{t('workflowSettings.stateDismissed')}</span>
        </div>
      {/each}
    </div>
    {#if shownScan && !workflows.length}<p class="none">{t('workflowSettings.none')}</p>{/if}

    {#if shownScan?.removed?.length}
      <div class="sub-h"><b>{t('workflowSettings.removedTitle')}</b><span>{t('workflowSettings.removedDesc')}</span></div>
      <div class="rlist">
        {#each shownScan.removed as r (r.id)}
          <div class="rrow idle" data-testid="wf-removed">
            <Icon name="trash" size={14} /><span class="rn plain">{r.name || r.id}</span><span class="rb plain"
              >{t(r.changed ? 'workflowSettings.removedChanged' : 'workflowSettings.removedKept')}</span
            ><span class="st"
              >{#if r.changed}<button class="link" disabled={reading} onclick={() => setRemoved(r.id, r.name || r.id, true)}
                  >{t('workflowSettings.keepRemoved')}</button
                >
                ·
              {/if}<button class="link" disabled={reading} onclick={() => setRemoved(r.id, r.name || r.id, false)}
                >{t(r.changed ? 'workflowSettings.takeBack' : 'workflowSettings.restore')}</button
              ></span
            >
          </div>
        {/each}
      </div>
    {/if}

    {#if current}
      <div class="sub-h">
        <b>{t('workflowSettings.stepsTitle')}</b><span>{t('workflowSettings.stepsDesc', { name: workflowName(current) })}</span>
      </div>
      <div class="wf-edit">
        {#each rows as r, i (r.key || `new-${i}`)}
          {@const n = i + 1}
          <div
            class="wf-er"
            class:guess={r.guessed}
            class:stale={r.stale}
            class:dragged={dragged === i}
            role="group"
            aria-label={t('workflowSettings.stepName', { n })}
            ondragover={(e) => dragged !== null && e.preventDefault()}
            ondrop={(e) => (e.preventDefault(), drop(i))}
          >
            <span
              class="no"
              draggable="true"
              role="img"
              aria-label={t('workflowSettings.move', { n })}
              use:tip={t('workflowSettings.move', { n })}
              ondragstart={(e) => {
                dragged = i;
                e.dataTransfer?.setData('text/plain', String(i));
              }}
              ondragend={() => (dragged = null)}>{n}</span
            >
            <input
              class="inp field step-name"
              value={r.name}
              placeholder={t('workflowSettings.newStep')}
              aria-label={t('workflowSettings.stepName', { n })}
              spellcheck="false"
              oninput={(e) => rename(i, e.currentTarget.value)}
              onkeydown={(e) => {
                if (!e.altKey || (e.key !== 'ArrowUp' && e.key !== 'ArrowDown')) return;
                e.preventDefault();
                moveBy(i, e.key === 'ArrowUp' ? -1 : 1, e.currentTarget);
              }}
            />
            <span class="by">
              {#if r.stale}
                <span class="bad">{t('workflowSettings.stale', { names: r.from.join(', ') })}</span>
              {:else if !r.from.length}
                {@const s = r.signals[0] ?? { command: '' }}
                <select
                  class="field sig-kind"
                  aria-label={t('workflowSettings.signalKind', { n })}
                  value={signalKind(s)}
                  onchange={(e) => setSignal(i, signalOf(e.currentTarget.value as (typeof SIGNALS)[number], signalValue(s)))}
                >
                  {#each SIGNALS as k (k)}<option value={k}>{t(`workflowSettings.signal.${k}`)}</option>{/each}
                </select>
                <input
                  class="inp field mono sig-value"
                  value={signalValue(s)}
                  placeholder={t(`workflowSettings.signalHint.${signalKind(s)}`)}
                  aria-label={t('workflowSettings.signalValue', { n })}
                  spellcheck="false"
                  oninput={(e) => setSignal(i, signalOf(signalKind(s), e.currentTarget.value))}
                />
              {:else}
                {#if r.guessed}<span>{t('workflowSettings.guessed')} ·</span>{/if}
                {#each r.marks as m (m)}{@render gate(m)}{/each}
              {/if}
            </span>
            <span class="acts">
              {#if r.guessed}
                <button class="btn small" onclick={() => keep(i)}>{t('workflowSettings.keep')}</button>
                <button class="btn small" onclick={() => removeAt(i)}>{t('workflowSettings.drop')}</button>
              {:else}
                <button
                  class="icon-btn"
                  aria-label={t('workflowSettings.merge', { n })}
                  use:tip={t('workflowSettings.merge', { n })}
                  disabled={i === rows.length - 1}
                  onclick={() => setRows(merge(rows, i))}><Icon name="merge" size={14} /></button
                >
                <button
                  class="icon-btn"
                  aria-label={t('workflowSettings.remove', { n })}
                  use:tip={t('workflowSettings.remove', { n })}
                  onclick={() => removeAt(i)}><Icon name="trash" size={14} /></button
                >
              {/if}
            </span>
          </div>
        {/each}
        {#if loopsLine}
          <div class="wf-er loop"><Icon name="restart" size={14} /><span>{loopsLine}</span></div>
        {/if}
      </div>
      <div class="edit-foot">
        <button class="add-line" onclick={add}><Icon name="plus" size={14} />{t('workflowSettings.add')}</button>
        <span class="foot-acts">
          <button class="link dismiss" onclick={() => current && setDismissed(current, true)}>{t('workflowSettings.dismiss')}</button>
          <button
            class="link dismiss"
            disabled={reading}
            use:tip={t('workflowSettings.deleteTip')}
            onclick={() => current && setRemoved(current.id, workflowName(current), true)}>{t('workflowSettings.delete')}</button
          >
        </span>
      </div>
    {/if}

    <Row label={t('workflowSettings.where')}>
      {#snippet extra()}
        <span class="more"
          ><Rich k="workflowSettings.whereDesc">{#snippet dir()}<span class="mono">.escouade/workflows/</span>{/snippet}</Rich></span
        >
      {/snippet}
      <div class="segmented" role="radiogroup" aria-label={t('workflowSettings.where')}>
        {#each [true, false] as inRepo (inRepo)}
          <button
            role="radio"
            aria-checked={draft.workflowsInRepo === inRepo}
            class:on={draft.workflowsInRepo === inRepo}
            onclick={() => (draft.workflowsInRepo = inRepo)}>{t(inRepo ? 'workflowSettings.repo' : 'workflowSettings.machine')}</button
          >
        {/each}
      </div>
    </Row>
  {/if}
</Group>

<style>
  /* What the reading looked at, as the doors of the workflow tab show them. */
  .wf-src {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    min-width: 0;
    max-width: 100%;
    margin-top: 6px;
  }
  .gate {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    height: 20px;
    padding: 0 6px;
    overflow: hidden;
    border-radius: 5px;
    background: rgba(var(--fg-rgb), 0.05);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--soft);
    font: 500 11px var(--mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .gate .n {
    color: var(--muted);
    font-family: var(--font);
  }
  /* Under a row's label, as the 2.0 design draws them: in the muted ink of what the row says. */
  .wf-src .gate {
    color: var(--muted);
    font-size: 12px;
  }
  .wf-src .gate .n {
    font-size: 12px;
    line-height: 1.5;
  }
  .wf-plan {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-top: 6px;
  }
  .wf-plan-acts {
    display: flex;
    gap: 6px;
    margin-top: 6px;
  }
  .sub-h {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 10px;
    margin-top: 18px;
  }
  .sub-h b {
    font-weight: 600;
  }
  .sub-h span {
    color: var(--muted);
    font-size: 12px;
  }
  /* The workflows found. */
  .rlist {
    display: grid;
    gap: 6px;
    margin: 4px 0 12px;
  }
  .rrow {
    display: grid;
    grid-template-columns: 16px minmax(72px, auto) minmax(0, 1fr) auto;
    align-items: center;
    gap: 4px 10px;
    padding: 9px 12px;
    border-radius: 9px;
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 12.5px;
  }
  button.rrow {
    width: 100%;
    border: none;
    background: transparent;
    color: inherit;
    font-family: inherit;
    text-align: left;
    cursor: pointer;
    transition: box-shadow 0.15s;
  }
  button.rrow:hover {
    box-shadow: inset 0 0 0 1px var(--line-3);
  }
  .rrow > :global(svg) {
    color: var(--muted);
  }
  .rrow.ok > :global(svg) {
    color: var(--add);
  }
  .rrow .rn {
    color: var(--text);
    font: 600 12px var(--mono);
  }
  .rrow .rb {
    overflow: hidden;
    min-width: 0;
    color: var(--muted);
    font: 11px var(--mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rrow .rb.plain,
  .rrow .rn.plain {
    font-family: var(--font);
  }
  .rrow.idle {
    color: var(--muted);
    box-shadow: inset 0 0 0 1px rgba(var(--fg-rgb), 0.05);
  }
  .rrow.idle .rn {
    color: var(--muted);
  }
  .rrow .st {
    color: var(--muted);
    font-size: 11.5px;
  }
  .link {
    padding: 0;
    border: none;
    background: none;
    color: var(--p-ink);
    font: inherit;
    font-size: 12px;
    cursor: pointer;
  }
  .none {
    margin: 0 0 10px;
    color: var(--muted);
    font-size: 12px;
  }
  /* The steps, as corrected. */
  .wf-edit {
    display: grid;
    gap: 4px;
    margin: 4px 0 10px;
  }
  .wf-er {
    display: grid;
    grid-template-columns: 22px minmax(0, 170px) minmax(0, 1fr) auto;
    align-items: center;
    gap: 10px;
    padding: 5px 6px 5px 10px;
    border-radius: 9px;
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 12px;
  }
  .wf-er .no {
    color: var(--dim);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
    cursor: grab;
  }
  .wf-er.dragged {
    opacity: 0.5;
  }
  .wf-er .inp {
    height: 26px;
    min-width: 0;
  }
  .wf-er .by {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px;
    min-width: 0;
    color: var(--muted);
    font-size: 11.5px;
  }
  .wf-er .by .bad {
    color: var(--del);
  }
  .wf-er .sig-kind {
    height: 26px;
    padding: 0 6px;
  }
  .wf-er .sig-value {
    flex: 1 1 120px;
    min-width: 0;
  }
  .wf-er .acts {
    display: inline-flex;
    gap: 2px;
  }
  .wf-er .icon-btn {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border-radius: 6px;
  }
  .wf-er .btn.small {
    height: 26px;
  }
  .wf-er.guess {
    background-image: repeating-linear-gradient(135deg, rgba(var(--fg-rgb), 0.03) 0 6px, transparent 6px 12px);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .wf-er.stale {
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.35);
  }
  .wf-er.loop {
    grid-template-columns: 22px minmax(0, 1fr) auto;
    background: rgba(var(--fg-rgb), 0.03);
    color: var(--soft);
  }
  .wf-er.loop > :global(svg) {
    color: var(--muted);
  }
  .edit-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .add-line {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 28px;
    padding: 0 8px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    font-size: 12.5px;
    font-weight: 550;
    cursor: pointer;
  }
  .add-line:hover {
    background: var(--hover);
    color: var(--text);
  }
  .dismiss {
    color: var(--muted);
  }
  /* A workflow of the team's files that this machine does not read. */
  .team {
    margin: -2px 0 2px 38px;
    color: var(--muted);
    font-size: 12px;
  }
  .dismiss:hover {
    color: var(--text);
  }
  .foot-acts {
    display: inline-flex;
    gap: 14px;
  }
  /* « Pas de workflow pour ce projet » (ESC-309): what it does and how to undo it. */
  .off-note {
    margin: 2px 0 10px;
    color: var(--muted);
    font-size: 12px;
  }
</style>
