<script lang="ts">
  import { untrack } from 'svelte';
  import Icon from '../Icon.svelte';
  import EngineMark from '../EngineMark.svelte';
  import Modal from '../modals/Modal.svelte';
  import { accountDefaults, accountName, newAgentAccount } from '../../lib/accounts';
  import { dropDocuments } from '../../lib/docs-drop';
  import { docsStore } from '../../lib/docs.svelte';
  import { engineName, engineOptions } from '../../lib/engines';
  import { fSize } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { menu } from '../../lib/menu.svelte';
  import { modelLabel } from '../../lib/models';
  import { app } from '../../lib/state.svelte';
  import { accountChoices, labelOf, modelChoices } from '../../lib/ticket-options';
  import { holds } from '../../lib/discussions';
  import { errorText, missingAccountOf } from '../../lib/refusal';
  import type { DiscussionKind, DiscussionPreset, DocsCheck, EngineId } from '../../lib/types';

  // « Nouvelle discussion » of the Docs tab (ESC-261, design/factory/3-nouvelle-discussion.png): a discussion with the
  // spec writer or a brainstorm with the product agent, a subject, and new files to join (filed and indexed by
  // themselves in the Documents; the agent has all the others). The footer says, and lets change, the harness, the
  // account and the model it starts on: the project's by default.
  let { projectId, preset }: { projectId: string; preset?: DiscussionPreset } = $props();

  const project = $derived(app.projects.find((p) => p.id === projectId));
  let kind = $state<DiscussionKind>(untrack(() => preset?.kind ?? 'discussion'));
  let subject = $state(untrack(() => preset?.subject ?? ''));
  let files = $state<File[]>([]);
  let engine = $state<EngineId | null>(null);
  let account = $state('');
  let model = $state('');
  let starting = $state(false);
  /** Why « Commencer » was refused (ESC-289): no discussion was left, the window keeps what was chosen. */
  let refused = $state('');
  /** The files over 50 MB no LFS rule takes, asked about before they go in the repository (as the drive does). */
  let large = $state<{ files: DocsCheck[]; answer: (yes: boolean) => void } | null>(null);
  function answerLarge(yes: boolean) {
    large?.answer(yes);
    large = null;
  }
  let picker = $state<HTMLInputElement>();
  let engineButton = $state<HTMLButtonElement>();
  let accountButton = $state<HTMLButtonElement>();
  let modelButton = $state<HTMLButtonElement>();

  const projectEngine = $derived(project?.engine ?? 'claude');
  /** The harnesses a discussion runs on (`holds`), in the usual menu's order. */
  const engines = $derived(engineOptions(engine).filter((e) => holds(e.value)));
  /** Its harness when none is chosen: the project's, else the first that keeps a discussion from writing. */
  const baseEngine = $derived(holds(projectEngine) ? projectEngine : (engines[0]?.value ?? 'claude'));
  const optionsOf = $derived(project ? { project, engine: engine ?? baseEngine } : null);
  const accounts = $derived(optionsOf ? accountChoices(optionsOf) : []);
  const models = $derived(optionsOf ? modelChoices(optionsOf, account) : []);
  /** The account it starts on: the one chosen, else the project's for its harness. */
  const accountUsed = $derived(
    account
      ? (app.settings.accounts ?? []).find((a) => a.id === account)
      : project
        ? newAgentAccount(project, engine ?? baseEngine)
        : undefined,
  );
  const modelUsed = $derived(model || accountDefaults(accountUsed, engine ?? baseEngine).model);
  const branch = $derived((project?.docsBranch ?? '').trim());

  const KINDS: DiscussionKind[] = ['discussion', 'brainstorm'];

  function engineMenu() {
    if (!engineButton) return;
    menu.showAt(
      engineButton,
      engines.map((e) => ({
        label: e.label,
        engine: e.value,
        checked: (engine ?? baseEngine) === e.value,
        onClick: () => {
          engine = e.value === baseEngine ? null : e.value;
          // Another harness: its accounts and models are others.
          account = '';
          model = '';
        },
      })),
    );
  }

  function accountMenu() {
    if (!accountButton) return;
    menu.showAt(
      accountButton,
      accounts.map((a) => ({
        label: a.label,
        checked: accountUsed?.id === a.value,
        onClick: () => {
          account = a.value;
          model = '';
        },
      })),
    );
  }

  function modelMenu() {
    if (!modelButton) return;
    menu.showAt(
      modelButton,
      models.map((m) => ({ label: m.label, checked: modelUsed === m.value, onClick: () => (model = m.value) })),
    );
  }

  function addFiles(list: FileList | null) {
    if (!list) return;
    const known = new Set(files.map((f) => `${f.name}:${f.size}`));
    files = [...files, ...Array.from(list).filter((f) => !known.has(`${f.name}:${f.size}`))];
  }

  async function start() {
    if (!subject.trim() || starting) return;
    starting = true;
    refused = '';
    try {
      // The files first: filed by Escouade in the drive, then named to the agent.
      const dropped = await dropDocuments(projectId, files, null, {
        ask: (f) => new Promise((answer) => (large = { files: f, answer })),
        ended: (name, d, error) => {
          if (!d && error) app.toast(t('docs.discussion.notFiled', { name, error }), 'error');
        },
      });
      let agent;
      try {
        agent = await api.discussionStart(projectId, {
          kind,
          subject: subject.trim(),
          documents: dropped.map((d) => d.file.path),
          message: preset?.message ?? null,
          gap: preset?.gap ?? null,
          engine: engine ?? baseEngine,
          account: account || null,
          model: model || null,
        });
      } catch (e) {
        // Said here, where it can start again or on another account; for want of an account, the toast offers to add one.
        refused = errorText(e);
        if (missingAccountOf(e)) app.refused(e);
        return;
      }
      app.agents[agent.id] = agent;
      // Read before the window closes: its props come from it.
      const pid = projectId;
      app.openDiscussion(pid, agent.id);
      app.modal = null;
      if (dropped.length) void docsStore.load(pid);
    } finally {
      starting = false;
    }
  }
</script>

<Modal title={t('docs.discussion.title')} width={720} onclose={() => (app.modal = null)}>
  <div class="kinds" role="radiogroup" aria-label={t('docs.discussion.title')}>
    {#each KINDS as k (k)}
      <button
        type="button"
        class="kind {k}"
        class:sel={kind === k}
        role="radio"
        aria-checked={kind === k}
        data-kind={k}
        onclick={() => (kind = k)}
      >
        <span class="tag {k}">{t(`docs.discussion.kinds.${k}`)}</span>
        <span class="what">{t(k === 'discussion' ? 'docs.discussion.discussionText' : 'docs.discussion.brainstormText')}</span>
        {#if k === 'discussion'}
          <span class="ex">{t('docs.discussion.discussionExample1')}</span>
          <span class="ex">{t('docs.discussion.discussionExample2')}</span>
        {:else}
          <span class="ex">{t('docs.discussion.brainstormExample')}</span>
        {/if}
      </button>
    {/each}
  </div>

  <label class="row">
    <span class="lab">{t('docs.discussion.subject')}</span>
    <!-- svelte-ignore a11y_autofocus -->
    <input
      class="field input"
      bind:value={subject}
      placeholder={t('docs.discussion.subjectHint')}
      autofocus
      onkeydown={(e) => e.key === 'Enter' && start()}
    />
  </label>

  <div class="row">
    <span class="lab" id="nd-files">{t('docs.discussion.files')}</span>
    <div class="files field" aria-labelledby="nd-files">
      {#each files as f, i (f.name + f.size)}
        <span class="file" data-file={f.name}
          ><Icon name="file" size={13} />{f.name}<span class="size">{fSize(f.size)}</span><button
            class="x"
            aria-label={t('docs.discussion.removeFile', { name: f.name })}
            onclick={() => (files = files.filter((_, j) => j !== i))}><Icon name="close" size={12} /></button
          ></span
        >
      {/each}
      <button class="add" onclick={() => picker?.click()}><Icon name="plus" size={13} />{t('docs.discussion.addFiles')}</button>
      <input
        bind:this={picker}
        type="file"
        multiple
        hidden
        data-testid="discussion-files"
        onchange={(e) => {
          addFiles(e.currentTarget.files);
          e.currentTarget.value = '';
        }}
      />
    </div>
    <span class="hint">{t('docs.discussion.filesHint')}</span>
    {#if !holds(projectEngine)}
      <p class="hint not-on">
        {t('docs.discussion.notOn', { name: engineName(projectEngine), other: engineName(baseEngine) })}
      </p>
    {/if}
  </div>

  {#if refused}
    <p class="refused" role="alert">{refused}</p>
  {/if}

  {#if large}
    <div class="large" role="alert">
      <p>{t('docs.documents.large.intro')}</p>
      <ul>
        {#each large.files as f (f.name)}<li><b>{f.name}</b> · {fSize(f.size)}</li>{/each}
      </ul>
      <div class="large-acts">
        <button class="btn ghost" onclick={() => answerLarge(false)}>{t('docs.documents.large.skip')}</button>
        <button class="btn primary" onclick={() => answerLarge(true)}>{t('docs.documents.large.add')}</button>
      </div>
    </div>
  {/if}

  {#snippet footer()}
    <div class="runs">
      <button
        class="pick"
        bind:this={engineButton}
        aria-haspopup="menu"
        aria-label={t('docs.discussion.harness', { name: engineName(engine ?? baseEngine) })}
        onclick={engineMenu}><EngineMark engine={engine ?? baseEngine} />{engineName(engine ?? baseEngine)}</button
      >
      <span class="sep" aria-hidden="true">·</span>
      <button
        class="pick"
        bind:this={accountButton}
        aria-haspopup="menu"
        aria-label={t('docs.discussion.account', { name: accountUsed ? accountName(accountUsed) : '' })}
        onclick={accountMenu}>{accountUsed ? accountName(accountUsed) : '—'}</button
      >
      <span class="sep" aria-hidden="true">·</span>
      <button
        class="pick"
        bind:this={modelButton}
        aria-haspopup="menu"
        aria-label={t('docs.discussion.model', { name: modelLabel(modelUsed, app.models) })}
        onclick={modelMenu}>{modelUsed ? labelOf(models, modelUsed, modelLabel(modelUsed, app.models)) : '—'}</button
      >
      <span class="sep" aria-hidden="true">·</span>
      <span class="branch">{branch ? t('docs.discussion.branch', { name: branch }) : t('docs.discussion.currentBranch')}</span>
    </div>
    <button class="btn ghost" onclick={() => (app.modal = null)}>{t('docs.discussion.cancel')}</button>
    <button class="btn primary" disabled={!subject.trim() || starting} onclick={start}>{t('docs.discussion.start')}</button>
  {/snippet}
</Modal>

<style>
  .refused {
    margin: 0 0 14px;
    font-size: 12.5px;
    color: var(--del);
    overflow-wrap: anywhere;
  }
  .large {
    padding: 12px 14px;
    border: 1px solid color-mix(in oklch, var(--wait) 45%, transparent);
    border-radius: 8px;
    font-size: 13px;
  }
  .large p {
    margin: 0 0 6px;
  }
  .large ul {
    margin: 0 0 8px;
    padding-left: 18px;
  }
  .large-acts {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .kinds {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
    margin-bottom: 18px;
  }
  .kind {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    padding: 16px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: transparent;
    color: var(--text);
    text-align: left;
    font: inherit;
    cursor: pointer;
  }
  .kind:hover {
    background: var(--hover);
  }
  .kind.sel.discussion {
    border-color: var(--disc);
    background: var(--raise-2);
  }
  .kind.sel.brainstorm {
    border-color: var(--brain);
    background: var(--raise-2);
  }
  .what {
    font-weight: 400;
    font-size: 13.5px;
    line-height: 1.5;
  }
  .ex {
    font-weight: 400;
    color: var(--dim);
    font-size: 12.5px;
    font-style: italic;
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-bottom: 14px;
  }
  .lab {
    color: var(--soft);
    font-size: 12.5px;
  }
  .hint {
    color: var(--dim);
    font-size: 12px;
  }
  .files {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    min-height: 38px;
    padding: 5px 8px;
  }
  .file {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 4px 2px 8px;
    border-radius: 6px;
    background: var(--raise-2);
    font-size: 12.5px;
  }
  .size {
    color: var(--dim);
  }
  .x,
  .add {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: none;
    background: transparent;
    color: var(--soft);
    font: inherit;
    font-size: 12.5px;
    cursor: pointer;
    border-radius: 5px;
    padding: 2px 4px;
  }
  .x:hover,
  .add:hover {
    background: var(--hover);
    color: var(--text);
  }
  .runs {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px;
    color: var(--dim);
    font-size: 12.5px;
  }
  .pick {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    border: none;
    background: transparent;
    color: var(--soft);
    font: inherit;
    padding: 3px 5px;
    border-radius: 5px;
    cursor: pointer;
  }
  .pick:hover {
    background: var(--hover);
    color: var(--text);
  }
  .sep {
    padding: 0 1px;
  }
  .tag {
    padding: 1px 7px;
    border-radius: 5px;
    font-size: 12px;
    font-weight: 600;
  }
  .tag.discussion {
    background: color-mix(in oklch, var(--disc) 22%, transparent);
    color: var(--disc);
  }
  .tag.brainstorm {
    background: color-mix(in oklch, var(--brain) 22%, transparent);
    color: var(--brain);
  }
</style>
