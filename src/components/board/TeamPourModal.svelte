<script lang="ts">
  // « Verser à l'équipe » (TS-7, `src-tauri/src/teampour.rs`): the tickets a project had before its Kanban became its
  // team's. First the preview: the tickets of the files of `.escouade/tickets` (their ids kept, never doubled), those of
  // this machine to choose (all can be checked), the docs of `.escouade/` when the team has none on its server yet (TS-9:
  // they go by this gesture alone), what leaves and what stays. Nothing leaves before « Verser ». Then what
  // was done, and the files frozen that may be removed by hand (Escouade removes and commits nothing).
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { app } from '../../lib/state.svelte';
  import { docsCount, filesToPour, teamBoardApi, type PourPreview, type Poured } from '../../lib/teamboard';
  import { errorText } from '../../lib/teams.svelte';
  import Modal from '../modals/Modal.svelte';

  let { projectId }: { projectId: string } = $props();

  let preview = $state<PourPreview | null>(null);
  let failure = $state('');
  let busy = $state(false);
  let done = $state<Poured | null>(null);
  let takeFiles = $state(true);
  let chosen = $state<string[]>([]);
  let takeDocs = $state(true);

  onMount(async () => {
    try {
      preview = await teamBoardApi.pourPreview(projectId);
    } catch (e) {
      failure = errorText(e);
    }
  });

  const files = $derived(preview ? filesToPour(preview) : []);
  const count = $derived((takeFiles ? files.length : 0) + chosen.length);
  /** The docs' files that go with « Verser ». */
  const docs = $derived(preview && takeDocs ? docsCount(preview.docs) : 0);
  const submitLabel = $derived(
    count === 0 && docs === 0
      ? t('teamBoard.pour.nothing')
      : count === 0
        ? t('teamBoard.pour.docsOnly', { count: docs })
        : t('teamBoard.pour.submit', { count }) + (docs ? t('teamBoard.pour.withDocs', { count: docs }) : ''),
  );
  const allChosen = $derived(!!preview && preview.locals.length > 0 && chosen.length === preview.locals.length);
  const command = $derived(done ? t('teamBoard.pour.command', { folder: done.folder }) : '');

  function toggle(id: string, on: boolean) {
    chosen = on ? [...chosen.filter((x) => x !== id), id] : chosen.filter((x) => x !== id);
  }

  async function pour() {
    busy = true;
    failure = '';
    try {
      done = await teamBoardApi.pour(projectId, takeFiles && files.length > 0, chosen, docs > 0);
    } catch (e) {
      failure = errorText(e);
    } finally {
      busy = false;
    }
  }

  const close = () => (app.modal = null);
</script>

<Modal title={t('teamBoard.pour.title')} width={600} onclose={close}>
  <div class="pour" data-testid="team-pour">
    {#if done}
      <p data-testid="team-pour-done">
        {t('teamBoard.pour.done', { imported: done.imported, locals: done.locals, already: done.already })}
      </p>
      {#if done.files.length}
        <p>{t('teamBoard.pour.frozen', { count: done.files.length })}</p>
        <ul class="names mono" data-testid="team-pour-frozen">
          {#each done.files as f (f)}<li>{done.folder}/{f}</li>{/each}
        </ul>
        <div class="row">
          <code class="mono">{command}</code>
          <button class="btn ghost" onclick={() => navigator.clipboard?.writeText(command)}>{t('teamBoard.pour.copy')}</button>
        </div>
      {/if}
      {#if done.docs.length}
        <p data-testid="team-pour-docs-done">{t('teamBoard.pour.docsDone', { count: done.docs.length })}</p>
        <ul class="names mono">
          {#each done.docs as f (f)}<li>{f}</li>{/each}
        </ul>
      {/if}
    {:else if !preview}
      <p class="muted">{failure || t('teamBoard.pour.loading')}</p>
    {:else}
      <p class="lead">{t('teamBoard.pour.lead')}</p>
      <section>
        <h4>{t('teamBoard.pour.files', { folder: preview.folder })}</h4>
        {#if preview.files.length === 0}
          <p class="muted">{t('teamBoard.pour.filesNone')}</p>
        {:else}
          <label class="check">
            <input type="checkbox" bind:checked={takeFiles} disabled={files.length === 0} data-testid="team-pour-files" />
            {t('teamBoard.pour.filesTake')}
          </label>
          <p class="muted small">{t('teamBoard.pour.filesHint')}</p>
          <ul class="items">
            {#each preview.files as f (f.id)}
              <li class:off={f.poured || !!f.problem} data-testid="team-pour-file">
                <span class="mono key">{f.key}</span>
                <span class="title">{f.title}</span>
                {#if f.poured}<span class="tag">{t('teamBoard.pour.already')}</span>{/if}
                {#if f.external}<span class="tag">{t('teamBoard.pour.keepsKey')}</span>{/if}
                {#if f.problem}<span class="tag warn">{f.problem}</span>{/if}
              </li>
            {/each}
          </ul>
        {/if}
      </section>
      <section>
        <h4>{t('teamBoard.pour.locals')}</h4>
        {#if preview.locals.length === 0}
          <p class="muted">{t('teamBoard.pour.localsNone')}</p>
        {:else}
          <button
            class="btn ghost small"
            data-testid="team-pour-all"
            onclick={() => (chosen = allChosen ? [] : preview!.locals.map((l) => l.id))}
            >{allChosen ? t('teamBoard.pour.none') : t('teamBoard.pour.all')}</button
          >
          <ul class="items">
            {#each preview.locals as l (l.id)}
              <li>
                <label class="check">
                  <input
                    type="checkbox"
                    checked={chosen.includes(l.id)}
                    onchange={(e) => toggle(l.id, e.currentTarget.checked)}
                    data-testid="team-pour-local"
                  />
                  <span class="mono key">{l.key}</span>
                  <span class="title">{l.title}</span>
                  {#if l.external}<span class="tag">{t('teamBoard.pour.keepsKey')}</span>{/if}
                </label>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
      <section data-testid="team-pour-docs" data-state={preview.docs.state}>
        <h4>{t('teamBoard.pour.docs')}</h4>
        {#if preview.docs.state === 'pending'}
          <label class="check">
            <input type="checkbox" bind:checked={takeDocs} data-testid="team-pour-docs-take" />
            {t('teamBoard.pour.docsTake')}
          </label>
          <p class="muted small">
            {[
              t('teamBoard.pour.docsPages', { count: preview.docs.pages.length }),
              t('teamBoard.pour.docsDecisions', { count: preview.docs.decisions.length }),
              t('teamBoard.pour.docsDocuments', { count: preview.docs.documents.length }),
            ].join(', ')} · {t('teamBoard.pour.docsHint')}
          </p>
          <ul class="items mono small">
            {#each [...preview.docs.pages, ...preview.docs.decisions, ...preview.docs.documents] as f (f)}<li>{f}</li>{/each}
          </ul>
        {:else}
          <p class="muted">
            {preview.docs.state === 'server'
              ? t('teamBoard.pour.docsServer')
              : preview.docs.state === 'repo'
                ? t('teamBoard.pour.docsRepo')
                : preview.docs.state === 'private'
                  ? t('teamBoard.pour.docsPrivate')
                  : t('teamBoard.pour.docsNone')}
          </p>
        {/if}
      </section>
      <p class="muted small">{t('teamBoard.pour.goes')}</p>
      <p class="muted small">{t('teamBoard.pour.stays')}</p>
      {#if failure}<p class="fail" role="alert">{failure}</p>{/if}
    {/if}
  </div>
  {#snippet footer()}
    {#if done || !preview}
      <button class="btn" onclick={close}>{t('teamBoard.pour.close')}</button>
    {:else}
      <button class="btn ghost" onclick={close}>{t('teamBoard.pour.cancel')}</button>
      <button class="btn primary" disabled={busy || (count === 0 && docs === 0)} onclick={pour} data-testid="team-pour-submit"
        >{busy ? t('teamBoard.pour.working') : submitLabel}</button
      >
    {/if}
  {/snippet}
</Modal>

<style>
  .pour {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  h4 {
    margin: 4px 0;
    font-size: 13px;
  }
  .items {
    list-style: none;
    margin: 6px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 220px;
    overflow: auto;
  }
  .items li {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .items li.off {
    opacity: 0.6;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .key {
    flex: none;
    color: var(--muted);
  }
  .title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag {
    flex: none;
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 6px;
    background: var(--p-soft);
    color: var(--p-ink);
  }
  .tag.warn {
    background: color-mix(in oklch, var(--warn, #c80) 18%, transparent);
    color: inherit;
  }
  .small {
    font-size: 12px;
  }
  .muted {
    color: var(--muted);
  }
  .names {
    margin: 0;
    padding-left: 18px;
    font-size: 12px;
  }
  .row {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .fail {
    color: var(--danger, #c33);
  }
</style>
