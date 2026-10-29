<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { fDate, tildify } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { tip } from '../../lib/tip';
  import { repoLabel } from '../../lib/repos';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import { knowLogo, logoUrl, pickLogo } from '../../lib/project-logo.svelte';
  import { cleanLabel, cleanLetters, initials, tileText } from '../../lib/project-rail';
  import TileFace from '../TileFace.svelte';
  import { projectPalette } from '../../lib/plugin-theme.svelte';
  import type { Start, StepId } from '../../lib/wizard';
  import { WizardForm } from '../../lib/wizard-form.svelte';
  import AccountStep from '../wizard/AccountStep.svelte';
  import Checks from '../wizard/Checks.svelte';
  import CopiedStep from '../wizard/CopiedStep.svelte';
  import IntegrationsStep from '../wizard/IntegrationsStep.svelte';
  import KanbanStep from '../wizard/KanbanStep.svelte';
  import LaunchStep from '../wizard/LaunchStep.svelte';
  import SummaryStep from '../wizard/SummaryStep.svelte';
  import WorktreesStep from '../wizard/WorktreesStep.svelte';
  import Icon from '../Icon.svelte';
  import Modal from './Modal.svelte';

  // The new project's assistant, in steps: the folder, its name and color and where its settings start from, then its
  // worktrees, launch, Kanban, account and integrations, each to skip, and a summary that says what would keep the way
  // of working from working. Opened again on an existing project (Réglages › Projets), it sets that one up anew.
  const from = app.modal?.kind === 'newProject' ? app.modal : null;
  const existing = from?.projectId ? (app.projects.find((p) => p.id === from.projectId) ?? null) : null;
  const form = new WizardForm(existing ? $state.snapshot(existing) : null);
  // Opened by « Rouvrir » on a closed project, the window starts from its folder, and from that history of it.
  const startPath = existing ? existing.path : (from?.path?.trim() ?? '');
  /** The history asked for holds for the folder it was asked for: another path typed, it is forgotten. */
  const historyFor = (p: string) => (p === startPath ? from?.historyId : undefined);

  const used = new Set(app.projects.map((p) => p.color));
  form.path = startPath;
  let name = $state(existing?.name ?? '');
  let nameTouched = !!existing;
  /** The project colours offered: those of the plugin's theme shown (PLG-7), else Escouade's. */
  const palette = projectPalette();
  const freeColor = palette.find((c) => !used.has(c)) ?? palette[0];
  let color = $state(existing?.color ?? freeColor);
  let colorTouched = !!existing;
  /** « Lettres de la tuile » (ESC-167): empty is the initials of the name. */
  let letters = $state(existing?.tileLetters ?? '');
  /** « Libellé sous la tuile » (ESC-168): off by default; its text, empty for the name. */
  let labelOn = $state(!!existing?.tileLabel);
  let labelText = $state(existing?.tileLabelText ?? '');
  /** « Logo »: an image chosen (copied once the project is saved), or the existing one's removed. */
  let logoPick = $state<{ path: string; url: string } | null>(null);
  let logoRemoved = $state(false);
  const logo = $derived(logoPick?.url ?? (existing && !logoRemoved ? logoUrl(existing) : null));
  const hasLogo = $derived(!!logoPick || (!!existing?.logo && !logoRemoved));

  async function chooseLogo() {
    const picked = await pickLogo();
    if (picked) logoPick = picked;
  }

  /** The tile's letters and logo set in the window, saved into the project made or edited; the project as it is then. */
  async function saveTile(p: import('../../lib/types').Project) {
    let out = p;
    const tile = { tileLetters: letters.trim(), tileLabel: labelOn, tileLabelText: labelText.trim() };
    if (
      (out.tileLetters ?? '') !== tile.tileLetters ||
      !!out.tileLabel !== tile.tileLabel ||
      (out.tileLabelText ?? '') !== tile.tileLabelText
    ) {
      const next = { ...out, ...tile };
      if (await app.run(api.updateProject(next).then(() => true))) out = next;
    }
    if (logoPick || (logoRemoved && out.logo)) {
      const saved = await app.run(api.setProjectLogo(out.id, logoPick?.path ?? null));
      if (saved) {
        if (logoPick) knowLogo(saved, logoPick.url);
        out = { ...out, logo: saved.logo };
      }
    }
    return out;
  }
  /** The folder `info` was read for: until the one typed is, nothing is created or reopened with another's history. */
  let infoPath = $state('');
  /** « Partir d'un projet vierge »: the folder's history stays where it is, the project is a new one. */
  let fresh = $state(false);
  let busy = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  /** Until the first look at the path: a folder the window opened on is read at once, a typed one after a pause. */
  let opening = true;

  $effect(() => {
    const p = form.path.trim();
    const now = opening;
    opening = false;
    clearTimeout(timer);
    if (!p) {
      form.info = null;
      form.setup = null;
      return;
    }
    const read = async () => {
      const i = await api.inspectFolder(p, historyFor(p)).catch(() => null);
      if (form.path.trim() !== p) return;
      // Another folder, another choice.
      if (i?.history?.id !== form.info?.history?.id) fresh = false;
      form.info = i;
      infoPath = p;
      if (i) prefill();
      // Its .escouade.yml, if it has one: the settings the project starts from.
      await form.readShared();
      // Another folder: what is copied by default is checked again in it.
      if (form.start === 'copy' && form.sourceId) form.chooseSource(form.sourceId);
      else await form.check();
    };
    if (now) void read();
    else timer = setTimeout(read, 200);
  });

  const info = $derived(form.info);
  const history = $derived(existing ? null : (info?.history ?? null));
  /** The folder gets its history back (same project, same agents and tickets): it is reopened, not created. */
  const restoring = $derived(!!history && !fresh);
  /** The folder typed is read, and is one: the steps may go on. */
  const ready = $derived((!!existing || (!!info?.exists && infoPath === form.path.trim())) && !form.copying);

  /** What the window starts from: the history's name, color and worktrees when it comes back, else the usual ones. Not what the user chose. */
  function prefill() {
    const h = restoring ? history : null;
    if (!nameTouched) name = h?.name ?? info?.name ?? name;
    if (!colorTouched) color = h?.color ?? freeColor;
  }

  const gitLine = $derived(
    !info
      ? { text: t('shell.newProject.git.choose'), color: 'var(--dim)' }
      : !info.exists
        ? { text: t('shell.newProject.git.missing'), color: 'var(--del)' }
        : info.repos?.length
          ? // Of several repositories (ESC-169): « 4 dépôts git : front, api, infra, docs ».
            {
              text: t('repos.wizard.found', { count: info.repos.length, repos: info.repos.map(repoLabel).join(', ') }),
              color: 'var(--run)',
            }
          : info.isRepo
            ? {
                text: info.dirty
                  ? t('shell.newProject.git.repoDirty', { branch: info.branch || '—', count: info.dirty })
                  : t('shell.newProject.git.repoClean', { branch: info.branch || '—' }),
                color: 'var(--run)',
              }
            : { text: t('shell.newProject.git.none'), color: 'var(--muted)' },
  );

  const STARTS = $derived<{ id: Start; label: string; desc: string }[]>(
    (existing ? (['current', 'copy', 'recommended', 'blank'] as const) : (['file', 'copy', 'recommended', 'blank'] as const))
      .filter((id) => (id !== 'copy' || form.sources.length) && (id !== 'file' || form.shared))
      .map((id) => ({ id, label: t(`wizard.start.${id}`), desc: id === 'file' ? fileDesc : t(`wizard.start.${id}Desc`) })),
  );

  /** What the folder's .escouade.yml sets, and whether its commands will wait for the user. */
  const fileDesc = $derived.by(() => {
    const file = form.shared?.shared?.file;
    if (!file) return '';
    const read = t('wizard.start.fileDesc', { count: file.keys.length });
    return file.pending ? `${read} ${t('wizard.start.filePending')}` : read;
  });

  const at = $derived(form.steps.indexOf(form.step));
  const last = $derived(form.step === 'summary');
  /** The steps « Passer » leaves as they are: not the start, the copy or the summary. */
  const skippable = $derived(!['start', 'copied', 'summary'].includes(form.step));

  async function browse() {
    const dir = await open({ directory: true, multiple: false, title: t('shell.newProject.folderTitle') });
    if (typeof dir === 'string') form.path = dir;
  }

  /** Shows the project made and closes the window, its first agent opened if asked for. */
  async function show(p: import('../../lib/types').Project, agent: string | null) {
    if (app.projects.some((x) => x.id === p.id)) app.replaceProject(p);
    else app.projects.push(p);
    app.selectProject(p.id);
    app.modal = null;
    if (!agent) return;
    const a = await app.run(api.createAgent(p.id, agent));
    if (a) {
      app.agents[a.id] = a;
      app.selectAgent(a.id);
    }
  }

  /**
   * « Créer » (`withSettings`: with what the steps set; else, « Créer sans l'assistant », as a project is made with
   * Escouade's defaults), « Rouvrir le projet » for a folder that gets its history back, « Enregistrer » for an
   * existing project.
   */
  async function create(withSettings: boolean) {
    if (!ready || busy) return;
    busy = true;
    try {
      if (existing) {
        const saved = await form.save({
          ...$state.snapshot(existing),
          name: name.trim() || existing.name,
          color,
          tileLetters: letters.trim(),
          tileLabel: labelOn,
          tileLabelText: labelText.trim(),
        });
        if (saved) {
          app.replaceProject(await saveTile(saved));
          app.modal = null;
        }
        return;
      }
      const path = form.path.trim();
      const historyId = historyFor(path);
      const agent = !restoring && form.firstAgent ? form.model : null;
      const p = await app.run(
        api.createProject({
          path,
          name: name.trim(),
          color,
          worktreePerAgent: restoring ? !!history?.worktreePerAgent : withSettings && form.config.worktreePerAgent,
          // The first agent is made once the settings are saved: its worktree is set up as they say.
          firstAgent: restoring || withSettings ? null : agent,
          restore: restoring,
          // Nothing sent for a folder that was not asked for with a history.
          ...(historyId ? { historyId } : {}),
        }),
      );
      if (!p) return;
      // Its history's letters, logo and label come back unless others were set here.
      if (restoring) return void show(letters.trim() || logoPick || labelOn || labelText.trim() ? await saveTile(p) : p, null);
      if (!withSettings) return void show(await saveTile(p), null);
      // Made: shown even if its settings were refused (said by a toast), to be set in Réglages › Projets.
      const saved = await form.save(p);
      // « Indexer le code » changed from the app's setting: the project's choice, the reading in the background, the project
      // shown at once. Left as it was, the project follows the app's setting (read by the backend when it is on).
      if (form.codeIndex !== !!app.settings?.codeIndexAll) void app.run(api.setCodeIndex(p.id, form.codeIndex));
      await show(await saveTile(saved ?? p), agent);
    } finally {
      busy = false;
    }
  }

  function goTo(step: StepId) {
    if (ready && !restoring) form.go(step);
  }
</script>

<Modal
  title={existing ? t('wizard.titleEdit', { name: existing.name }) : t('wizard.title')}
  width={900}
  height="min(640px, 100vh - 80px)"
  flush
  onclose={() => (app.modal = null)}
>
  {#snippet badge()}
    {#if form.path.trim()}<span class="chip-sm mono path">{tildify(form.path.trim())}</span>{/if}
    <span class="chip-sm" data-step-of>{t('wizard.stepOf', { n: at + 1, count: form.steps.length })}</span>
  {/snippet}
  <div class="wiz">
    <nav class="rail" aria-label={t('wizard.stepsLabel')}>
      <ol>
        {#each form.steps as s, i (s)}
          {@const done = form.done.includes(s) && form.step !== s}
          {@const skipped = form.skipped.includes(s) && form.step !== s}
          <li>
            <button
              class="st"
              class:done
              class:skip={skipped}
              aria-current={form.step === s ? 'step' : undefined}
              disabled={!ready || restoring}
              onclick={() => goTo(s)}
            >
              <span class="k"
                >{#if done}<Icon name="check" size={11} />{:else}{i + 1}{/if}</span
              >
              <span class="l"
                >{t(`wizard.steps.${s}`)}{#if skipped}{' '}<span class="tag">{t('wizard.skippedTag')}</span>
                {:else if s === 'integrations'}{' '}<span class="tag">{t('wizard.optionalTag')}</span>{/if}</span
              >
            </button>
          </li>
        {/each}
      </ol>
    </nav>
    <div class="page" data-step={form.step}>
      {#if form.step === 'start'}
        <h3 class="h">{t('wizard.start.label')}</h3>
        <p class="lead">{t('wizard.leads.start', { name: name.trim() || t('shell.newProject.untitled') })}</p>
        {#if !existing}
          <div class="w-fld">
            <span class="lab">{t('common.folder')}</span>
            <div class="row">
              <!-- svelte-ignore a11y_autofocus -->
              <input
                class="field mono path-in"
                bind:value={form.path}
                placeholder={t('shell.newProject.pathPlaceholder')}
                spellcheck="false"
                autofocus
              />
              <button class="btn" onclick={browse}><Icon name="folder" size={14} />{t('shell.newProject.browse')}</button>
            </div>
            <span class="git" style:color={gitLine.color} use:tip={info?.repos?.length ? t('repos.wizard.foundHint') : undefined}
              ><span class="gd" style:background={gitLine.color}></span>{gitLine.text}</span
            >
            {#if info?.reposWithoutCommit?.length}
              <span class="git" style:color="var(--wait)" use:tip={t('repos.wizard.noCommitHint')}
                ><span class="gd" style:background="var(--wait)"></span>{t('repos.wizard.noCommit', {
                  count: info.reposWithoutCommit.length,
                  repos: info.reposWithoutCommit.map(repoLabel).join(', '),
                })}</span
              >
            {/if}
          </div>
        {/if}
        {#if history}
          <div class="history">
            <p class="found">
              {t('shell.newProject.history.found', {
                agents: t('common.count.agents', { count: history.agents }),
                tickets: t('common.count.tickets', { count: history.tickets }),
                date: fDate(history.lastActivity),
              })}
            </p>
            <div class="toggle">
              <div class="tx">
                <span class="tt">{t('shell.newProject.history.fresh')}</span>
                <span class="ts">{t('shell.newProject.history.freshDesc')}</span>
              </div>
              <button
                class="switch"
                role="switch"
                aria-checked={fresh}
                class:on={fresh}
                aria-label={t('shell.newProject.history.fresh')}
                onclick={() => {
                  fresh = !fresh;
                  prefill();
                }}
              ></button>
            </div>
          </div>
        {/if}
        <div class="two">
          <div class="w-fld">
            <span class="lab">{t('common.name')}</span>
            <div class="row">
              <span class="tile" style:--c={color} aria-hidden="true"
                ><TileFace text={tileText({ name: name || t('shell.newProject.untitled'), tileLetters: letters })} {logo} /></span
              >
              <input class="field name" bind:value={name} oninput={() => (nameTouched = true)} />
            </div>
          </div>
          <div class="w-fld">
            <span class="lab">{t('common.color')}</span>
            <div class="swatches">
              {#each projectPalette(color) as c, i (c)}
                <button
                  aria-label={t('shell.newProject.colorN', { n: i + 1 })}
                  aria-pressed={color === c}
                  style:--c={c}
                  onclick={() => {
                    color = c;
                    colorTouched = true;
                  }}
                ></button>
              {/each}
            </div>
          </div>
        </div>
        <div class="two">
          <div class="w-fld">
            <span class="lab">{t('settings.project.lettersLabel')}</span>
            <input
              class="field letters"
              aria-label={t('settings.project.lettersLabel')}
              spellcheck="false"
              autocomplete="off"
              placeholder={initials(name || t('shell.newProject.untitled'))}
              value={letters}
              oninput={(e) => {
                const v = cleanLetters(e.currentTarget.value);
                e.currentTarget.value = v;
                letters = v;
              }}
            />
          </div>
          <div class="w-fld">
            <span class="lab">{t('settings.project.logoLabel')}</span>
            <div class="row">
              <button class="btn" onclick={chooseLogo}><Icon name="image" size={14} />{t('settings.project.logoChoose')}</button>
              {#if hasLogo}<button
                  class="btn"
                  onclick={() => {
                    logoPick = null;
                    logoRemoved = true;
                  }}>{t('settings.project.logoRemove')}</button
                >{/if}
            </div>
          </div>
        </div>
        <div class="w-fld">
          <span class="lab">{t('settings.project.labelLabel')}</span>
          <div class="row">
            <input
              class="field label-text"
              aria-label={t('settings.project.labelText')}
              spellcheck="false"
              autocomplete="off"
              placeholder={name || t('shell.newProject.untitled')}
              value={labelText}
              oninput={(e) => {
                const v = cleanLabel(e.currentTarget.value);
                if (v !== e.currentTarget.value) e.currentTarget.value = v;
                labelText = v;
              }}
            />
            <button
              class="switch"
              role="switch"
              aria-checked={labelOn}
              class:on={labelOn}
              aria-label={t('settings.project.labelLabel')}
              onclick={() => (labelOn = !labelOn)}
            ></button>
          </div>
        </div>
        {#if !restoring}
          <Checks {form} step="start" />
          <div class="w-radios" role="radiogroup" aria-label={t('wizard.start.label')}>
            {#each STARTS as s (s.id)}
              <button class="w-radio" role="radio" aria-checked={form.start === s.id} onclick={() => form.chooseStart(s.id)}>
                <i class="dot"></i>
                <span><b>{s.label}</b><span class="d">{s.desc}</span></span>
              </button>
              {#if s.id === 'copy' && form.start === 'copy'}
                <div class="sub">
                  <div class="w-chips" role="group" aria-label={t('wizard.start.copyFrom')}>
                    {#each form.sources as p (p.id)}
                      <button class="w-chip" aria-pressed={form.sourceId === p.id} onclick={() => form.chooseSource(p.id)}
                        ><i class="sw" style:background={p.color}></i>{p.name}</button
                      >
                    {/each}
                  </div>
                  {#if form.privateLeftOut}
                    <span class="private"
                      ><Icon name="lock" size={14} />{t('wizard.start.privateLeftOut', { count: form.privateLeftOut })}</span
                    >
                  {/if}
                </div>
              {/if}
            {/each}
            {#if form.privateLeftOut && form.start !== 'copy'}
              <span class="private alone"
                ><Icon name="lock" size={14} />{t('wizard.start.privateLeftOut', { count: form.privateLeftOut })}</span
              >
            {/if}
          </div>
        {/if}
      {:else}
        <h3 class="h">
          {t(`wizard.steps.${form.step}`)}{#if form.step === 'copied' && form.source}<span class="from">{` · ${form.source.name}`}</span
            >{/if}
        </h3>
        {#if form.step === 'copied'}
          <p class="lead">{t('wizard.leads.copied', { name: name.trim() || t('shell.newProject.untitled') })}</p>
          <CopiedStep {form} />
        {:else}
          <p class="lead">{t(`wizard.leads.${form.step}`)}</p>
          {#if form.step === 'worktrees'}<WorktreesStep {form} />
          {:else if form.step === 'launch'}<LaunchStep {form} />
          {:else if form.step === 'kanban'}<KanbanStep {form} />
          {:else if form.step === 'account'}<AccountStep {form} />
          {:else if form.step === 'integrations'}<IntegrationsStep {form} />
          {:else}<SummaryStep {form} />{/if}
        {/if}
      {/if}
    </div>
  </div>
  {#snippet footer()}
    {#if !existing && !restoring}
      <button class="btn without" disabled={!ready || busy} onclick={() => create(false)}>{t('wizard.buttons.withoutWizard')}</button>
    {:else}
      <span class="sp"></span>
    {/if}
    {#if at > 0}
      <button class="btn" onclick={() => form.back()}>{t('wizard.buttons.back')}</button>
    {:else}
      <button class="btn" onclick={() => (app.modal = null)}>{t('common.cancel')}</button>
    {/if}
    {#if restoring}
      <button class="btn primary" disabled={!ready || busy} onclick={() => create(false)}>
        {busy ? t('shell.newProject.history.reopening') : t('shell.newProject.history.reopen')}
      </button>
    {:else if last}
      <button class="btn primary" disabled={!ready || busy} onclick={() => create(true)}>
        {#if existing}{busy ? t('wizard.buttons.saving') : t('wizard.buttons.save')}
        {:else}{busy ? t('wizard.buttons.creating') : t('wizard.buttons.create')}{/if}
      </button>
    {:else}
      {#if skippable}<button class="btn" onclick={() => form.next(true)}>{t('wizard.buttons.skip')}</button>{/if}
      <button class="btn primary" disabled={!ready} onclick={() => form.next()}>{t('wizard.buttons.next')}</button>
    {/if}
  {/snippet}
</Modal>

<style>
  /* The assistant, form A (the preview's `#wizard`): the steps on a rail of 200 px, the step's page beside it. */
  .wiz {
    display: grid;
    grid-template-columns: 200px minmax(0, 1fr);
    height: 100%;
    min-height: 0;
  }
  .rail {
    min-height: 0;
    overflow: auto;
    padding: 14px 10px;
    border-right: 1px solid var(--line);
  }
  .rail ol {
    display: grid;
    align-content: start;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .st {
    display: grid;
    grid-template-columns: 18px minmax(0, 1fr);
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 30px;
    padding: 0 8px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--muted);
    font: inherit;
    font-size: 12.5px;
    text-align: left;
    cursor: pointer;
  }
  .st:disabled {
    cursor: default;
  }
  .st:hover:not(:disabled, [aria-current='step']) {
    background: var(--hover);
  }
  .st:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: -2px;
  }
  .k {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    box-shadow: inset 0 0 0 1px var(--line-3);
    font-size: 10.5px;
    font-variant-numeric: tabular-nums;
  }
  .st.done {
    color: var(--soft);
  }
  .st.done .k {
    background: color-mix(in oklch, var(--p) 30%, transparent);
    box-shadow: none;
    color: var(--text);
  }
  .st.skip {
    color: var(--dim);
  }
  .st[aria-current='step'] {
    background: rgba(var(--fg-rgb), 0.06);
    color: var(--text);
    font-weight: 550;
  }
  .st[aria-current='step'] .k {
    box-shadow: inset 0 0 0 1.5px var(--p-ink);
    color: var(--text);
  }
  .l {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag {
    margin-left: 1px;
    color: var(--dim);
    font-size: 11px;
    font-weight: 450;
  }
  .page {
    display: grid;
    align-content: start;
    gap: 18px;
    min-height: 0;
    min-width: 0;
    padding: 18px 20px 20px;
    overflow: auto;
  }
  .h {
    margin: 0;
    font-size: 15px;
    font-weight: 700;
  }
  .from {
    color: var(--muted);
    font-weight: 550;
    white-space: pre;
  }
  .lead {
    margin: -6px 0 0;
    color: var(--muted);
    line-height: 1.5;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .path-in {
    flex: 1;
    height: 32px;
    font-size: 12px;
  }
  .git {
    display: flex;
    align-items: center;
    gap: 7px;
    font-family: var(--mono);
    font-size: 11.5px;
  }
  .gd {
    width: 6px;
    height: 6px;
    flex: none;
    border-radius: 50%;
  }
  .history {
    display: grid;
    gap: 12px;
    padding: 11px 13px;
    border-radius: 10px;
    background: rgba(var(--fg-rgb), 0.035);
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .found {
    margin: 0;
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  .tx {
    flex: 1;
    display: grid;
    gap: 2px;
  }
  .tt {
    font-weight: 550;
  }
  .ts {
    color: var(--muted);
    font-size: 12px;
    text-wrap: pretty;
  }
  .two {
    display: grid;
    grid-template-columns: minmax(200px, 1fr) minmax(0, max-content);
    gap: 18px;
  }
  /* The folder beside the title, cut short when it is long. */
  .chip-sm.path {
    flex: 0 1 auto;
    min-width: 0;
    max-width: 380px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* The project's tile as the rail will show it: its initials on its colour. */
  .tile {
    position: relative;
    display: grid;
    place-items: center;
    flex: none;
    width: 32px;
    height: 32px;
    border-radius: 9px;
    background: var(--c);
    color: var(--on-tile);
    font: 650 12px/1 var(--font);
  }
  .letters {
    width: 96px;
    height: 32px;
    text-align: center;
  }
  .label-text {
    width: 150px;
    height: 32px;
  }
  .name {
    flex: 1;
    min-width: 0;
    height: 32px;
    font-size: 13px;
    font-weight: 550;
  }
  .swatches {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    min-height: 32px;
  }
  .swatches button {
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    border-radius: 7px;
    background: var(--c);
    cursor: pointer;
    transition: transform 0.15s var(--ease);
  }
  .swatches button:hover {
    transform: scale(1.1);
  }
  .swatches button[aria-pressed='true'] {
    box-shadow:
      0 0 0 2px var(--raise),
      0 0 0 4px var(--c);
  }
  .sub {
    display: grid;
    gap: 8px;
    margin: 4px 0 4px 28px;
  }
  .sw {
    width: 7px;
    height: 7px;
    border-radius: 2px;
  }
  .private {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 11.5px;
  }
  .private.alone {
    margin: 4px 0 0;
  }
  .without {
    margin-right: auto;
  }
  .sp {
    flex: 1;
  }
</style>
