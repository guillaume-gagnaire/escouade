<script lang="ts">
  import Modal from '../modals/Modal.svelte';
  import { t } from '../../lib/i18n';
  import { decisionLabel, decisionNo } from '../../lib/decisions';
  import { docsStore } from '../../lib/docs.svelte';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import type { DocsDecision, Project } from '../../lib/types';

  // « Proposer une décision » (ESC-259): a proposal written by hand, or one that replaces an accepted decision (an
  // accepted one is never rewritten). It is a decision only once accepted, in the register.
  let {
    project,
    replaces = null,
    onclose,
    ondone,
  }: {
    project: Project;
    /** The accepted decision it replaces, chosen from its detail; the field lets the user choose one otherwise. */
    replaces?: number | null;
    onclose: () => void;
    ondone: (d: DocsDecision) => void;
  } = $props();

  const accepted = $derived(
    (docsStore.decisions[project.id] ?? [])
      .filter((d): d is DocsDecision & { number: number } => d.state === 'accepted' && d.number != null)
      .sort((a, b) => b.number - a.number),
  );

  let title = $state('');
  let context = $state('');
  let decision = $state('');
  let consequences = $state('');
  // svelte-ignore state_referenced_locally
  let replacing = $state(replaces == null ? '' : String(replaces));
  let busy = $state(false);

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!title.trim() || busy) return;
    busy = true;
    const fields = { title: title.trim(), context, decision, consequences };
    const made = await app.run(
      replacing ? api.docsDecisionReplace(project.id, Number(replacing), fields) : api.docsDecisionPropose(project.id, null, fields),
    );
    busy = false;
    if (!made) return;
    app.toast(t('docs.decisions.form.done', { n: decisionLabel(made) }));
    await docsStore.load(project.id);
    ondone(made);
  }
</script>

<Modal
  title={replaces == null ? t('docs.decisions.form.title') : t('docs.decisions.form.replaceTitle', { n: decisionNo(replaces) })}
  width={600}
  {onclose}
>
  <form id="decision-form" class="form" onsubmit={submit}>
    <p class="intro">{t('docs.decisions.form.intro')}</p>
    <label
      ><span>{t('docs.decisions.form.name')}</span>
      <!-- svelte-ignore a11y_autofocus -->
      <input class="field input" bind:value={title} placeholder={t('docs.decisions.form.nameHint')} required autofocus /></label
    >
    <label
      ><span>{t('docs.decisions.form.context')}</span><textarea
        class="field"
        rows="3"
        bind:value={context}
        placeholder={t('docs.decisions.form.contextHint')}
      ></textarea></label
    >
    <label
      ><span>{t('docs.decisions.form.decision')}</span><textarea
        class="field"
        rows="3"
        bind:value={decision}
        placeholder={t('docs.decisions.form.decisionHint')}
      ></textarea></label
    >
    <label
      ><span>{t('docs.decisions.form.consequences')}</span><textarea
        class="field"
        rows="3"
        bind:value={consequences}
        placeholder={t('docs.decisions.form.consequencesHint')}
      ></textarea></label
    >
    {#if accepted.length}
      <label
        ><span>{t('docs.decisions.form.replaces')}</span>
        <select class="field input" bind:value={replacing}>
          <option value="">{t('docs.decisions.form.replacesNone')}</option>
          {#each accepted as d (d.number)}
            <option value={String(d.number)}>{decisionNo(d.number)} · {d.title}</option>
          {/each}
        </select></label
      >
    {/if}
  </form>
  {#snippet footer()}
    <button class="btn ghost" type="button" onclick={onclose}>{t('docs.page.cancel')}</button>
    <button class="btn primary" type="submit" form="decision-form" disabled={busy || !title.trim()}>{t('docs.decisions.form.go')}</button>
  {/snippet}
</Modal>

<style>
  .form {
    display: grid;
    gap: 12px;
  }
  .intro {
    margin: 0;
    color: var(--muted);
    line-height: 1.5;
  }
  label {
    display: grid;
    gap: 5px;
    color: var(--muted);
    font-size: 12.5px;
  }
  textarea {
    resize: vertical;
    min-height: 64px;
  }
</style>
