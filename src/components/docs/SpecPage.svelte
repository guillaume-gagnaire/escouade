<script lang="ts">
  import { untrack } from 'svelte';
  import Icon from '../Icon.svelte';
  import Markdown from '../conv/Markdown.svelte';
  import { t } from '../../lib/i18n';
  import { fAgo } from '../../lib/format';
  import { docsStore } from '../../lib/docs.svelte';
  import { comparedLine } from '../../lib/docs';
  import { refLabel } from '../../lib/decisions';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { DocsGap, DocsPage, Project, TicketDraft } from '../../lib/types';

  // A page of the spec (ESC-258, docs/FACTORY.md « La spécification »): on top its gaps with the code and what settles
  // them, then its Markdown rendered, or written in a simple editor (or the file in Escouade's editor); beside it, its
  // sources, tickets and decisions. « Comparer au code » asks the short agent that reads the code (ESC-263); « Mettre la spec
  // à jour » opens a discussion with the spec writer on the gap (ESC-261).
  let { project, path, onback }: { project: Project; path: string; onback: () => void } = $props();

  let page = $state<DocsPage | null>(null);
  let missing = $state(false);
  let editing = $state(untrack(() => app.docsEdit === path));
  let title = $state('');
  let body = $state('');
  let saving = $state(false);

  async function load() {
    try {
      page = await api.docsSpecPage(project.id, path);
      missing = false;
    } catch {
      missing = true;
    }
  }
  $effect(() => {
    void path;
    void docsStore.tree[project.id];
    untrack(() => void load());
  });
  $effect(() => {
    if (app.docsEdit === path) untrack(() => (app.docsEdit = null));
  });
  $effect(() => {
    // Into the editor: the page as it is now.
    if (editing && page) untrack(() => ((title = page!.title), (body = page!.body)));
  });

  const subject = $derived(docsStore.tree[project.id]?.subjects.find((s) => s.pages.some((p) => p.path === path)));
  /** The file is in the project's folder only with the current branch: then Escouade's editor can open it. */
  const inProject = $derived(!(project.docsBranch ?? '').trim());

  async function save() {
    saving = true;
    const done = await app.run(api.docsSpecWrite(project.id, path, title, body));
    saving = false;
    if (!done) return;
    page = done;
    editing = false;
    app.toast(t('docs.page.saved'));
    void docsStore.load(project.id);
  }

  /** This page being compared to the code, here or after a ticket (ESC-263). */
  const comparing = $derived((app.docsComparing[project.id] ?? []).some((p) => p === path || p === '*'));
  async function compare() {
    app.docsComparing[project.id] = [...(app.docsComparing[project.id] ?? []), path];
    const done = await app.run(api.docsCompare(project.id, [path]));
    app.docsComparing[project.id] = (app.docsComparing[project.id] ?? []).filter((p) => p !== path);
    if (done) app.toast(comparedLine(done));
    await docsStore.load(project.id);
    await load();
  }

  function seeCode(g: DocsGap) {
    if (g.code[0]) void app.openEditor({ projectId: project.id, source: 'project', path: g.code[0] });
  }

  /** « Mettre la spec à jour »: a discussion with the spec writer, told the gap, which proposes the page from the code. */
  function updateSpec(g: DocsGap, index: number) {
    if (!page) return;
    app.newDiscussion(project.id, {
      kind: 'discussion',
      gap: { page: path, index },
      subject: t('docs.discussion.updateSubject', { page: page.title }),
      message: t('docs.discussion.updateMessage', {
        page: page.title,
        path: `.escouade/spec/${path}.md`,
        gap: g.text,
        files: g.code.length ? g.code.join(', ') : t('docs.page.noFiles'),
      }),
    });
  }

  /** « Ouvrir un ticket pour le code »: the ticket's window, its title and description written from the gap. */
  function openTicket(g: DocsGap) {
    if (!page) return;
    const projectId = project.id;
    app.modal = {
      kind: 'ticket',
      projectId,
      onSubmit: async (d: TicketDraft) => {
        const made = await app.run(api.ticketCreate(projectId, d));
        if (!made) return false;
        app.tickets[made.id] ??= made;
        return true;
      },
      resume: {
        title: t('docs.page.ticketTitle', { page: page.title }),
        description: t('docs.page.ticketDescription', {
          page: page.title,
          path,
          gap: g.text,
          files: g.code.length ? g.code.join(', ') : t('docs.page.noFiles'),
        }),
        criteria: '',
        maxLoops: 5,
        after: [],
        branch: '',
      },
    };
  }
</script>

<article class="page" data-page-open={path}>
  <div class="crumbs">
    <button class="back" onclick={onback}><Icon name="right" size={12} />{t('docs.page.back')}</button>
    {#if subject}<span class="sep" aria-hidden="true">›</span><span>{subject.title}</span>{/if}
  </div>

  {#if missing}
    <p class="gone">{t('docs.page.notFound')}</p>
  {:else if page}
    <header class="p-head">
      {#if editing}
        <input class="field input p-title-in" aria-label={t('docs.page.titleField')} bind:value={title} />
      {:else}
        <h2 class="p-title">{page.title}</h2>
      {/if}
      <div class="p-acts">
        {#if editing}
          <button class="btn ghost" onclick={() => (editing = false)}>{t('docs.page.cancel')}</button>
          <button class="btn primary" disabled={saving} onclick={save}>{t('docs.page.save')}</button>
        {:else}
          {#if inProject}
            <button
              class="btn ghost"
              use:tip={t('docs.page.inEditorTip')}
              onclick={() => app.openEditor({ projectId: project.id, source: 'project', path: `.escouade/spec/${path}.md` })}
              ><Icon name="code" size={14} />{t('docs.page.inEditor')}</button
            >
          {/if}
          <button class="btn ghost" data-compare="page" disabled={comparing} use:tip={t('docs.compare.pageTip')} onclick={compare}
            ><Icon name="swap" size={14} />{comparing ? t('docs.compare.runningOne') : t('docs.compare.page')}</button
          >
          <button class="btn" onclick={() => (editing = true)}><Icon name="edit" size={14} />{t('docs.page.edit')}</button>
        {/if}
      </div>
    </header>

    {#if page.gaps.length}
      <section class="gaps" aria-label={t('docs.page.gaps', { count: page.gaps.length })}>
        <h3>{t('docs.page.gaps', { count: page.gaps.length })}</h3>
        {#each page.gaps as g, i (i)}
          <div class="gap" data-gap={i}>
            <p>{g.text}</p>
            {#if g.code.length}<p class="files mono">{g.code.join(' · ')}</p>{/if}
            <div class="g-acts">
              <button class="btn primary" onclick={() => updateSpec(g, i)}
                ><Icon name="edit" size={14} />{t('docs.discussion.updateSpec')}</button
              >
              <button class="btn" onclick={() => openTicket(g)}><Icon name="kanban" size={14} />{t('docs.page.openTicket')}</button>
              <button
                class="btn ghost"
                disabled={!g.code.length}
                use:tip={g.code.length ? null : t('docs.page.noCode')}
                onclick={() => seeCode(g)}><Icon name="code" size={14} />{t('docs.page.seeCode')}</button
              >
            </div>
          </div>
        {/each}
      </section>
    {/if}

    <div class="p-body">
      <div class="text">
        {#if editing}
          <textarea class="field mono editor" aria-label={t('docs.page.bodyField')} bind:value={body} spellcheck="true"></textarea>
        {:else if page.body.trim()}
          <Markdown text={page.body} />
        {:else}
          <p class="blank">{t('docs.page.empty')}</p>
        {/if}
      </div>
      <aside class="meta">
        {#if page.draft}<p class="draft">{t('docs.page.draft')}</p>{/if}
        <p class="when" data-compared={page.compared || null}>
          {comparing
            ? t('docs.compare.runningOne')
            : page.compared && !Number.isNaN(Date.parse(page.compared))
              ? t('docs.page.compared', { when: fAgo(Date.parse(page.compared) / 1000, app.now) })
              : t('docs.page.neverCompared')}
        </p>
        <h4>{t('docs.page.sources')}</h4>
        {#if page.sources.length}
          <ul class="sources">
            {#each page.sources as s (s)}<li class="mono">{s}</li>{/each}
          </ul>
        {:else}
          <p class="none">{t('docs.page.noSources')}</p>
        {/if}
        {#if page.tickets.length}
          <h4>{t('docs.page.tickets')}</h4>
          <ul class="chips">
            {#each page.tickets as k (k)}<li class="mono">{k}</li>{/each}
          </ul>
        {/if}
        {#if page.decisions.length}
          <h4>{t('docs.page.decisions')}</h4>
          <ul class="chips">
            {#each page.decisions as n (n)}<li class="mono">{refLabel(n)}</li>{/each}
          </ul>
        {/if}
      </aside>
    </div>
  {/if}
</article>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px 24px 24px;
    min-height: 100%;
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--muted);
    font-size: 12.5px;
  }
  .back {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }
  .back :global(.icon) {
    transform: rotate(180deg);
  }
  .back:hover {
    color: var(--text);
  }
  .sep {
    color: var(--dim);
  }
  .gone {
    color: var(--muted);
  }
  .p-head {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .p-title {
    flex: 1;
    min-width: 0;
    margin: 0;
    font-size: 20px;
    font-weight: 650;
    letter-spacing: -0.015em;
  }
  .p-title-in {
    flex: 1;
    min-width: 0;
    font-size: 16px;
  }
  .p-acts {
    flex: none;
    display: flex;
    gap: 8px;
  }
  .gaps {
    display: grid;
    gap: 8px;
    padding: 12px 14px;
    border: 1px solid var(--wait-line);
    border-radius: 12px;
    background: var(--wait-bg);
  }
  .gaps h3 {
    margin: 0;
    color: var(--wait);
    font-size: 13px;
    font-weight: 620;
  }
  .gap {
    display: grid;
    gap: 6px;
  }
  .gap p {
    margin: 0;
    color: var(--soft);
    line-height: 1.45;
  }
  .gap .files {
    color: var(--dim);
    font-size: 12px;
  }
  .g-acts {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .p-body {
    flex: 1;
    display: grid;
    grid-template-columns: minmax(0, 1fr) 220px;
    gap: 28px;
  }
  @container (max-width: 760px) {
    .p-body {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .text {
    min-width: 0;
    color: var(--text-ai);
    font-size: 14px;
    line-height: 1.6;
  }
  .editor {
    width: 100%;
    min-height: 420px;
    height: 100%;
    resize: vertical;
    font-size: 13px;
    line-height: 1.55;
  }
  .blank,
  .none {
    margin: 0;
    color: var(--muted);
  }
  .meta {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 12.5px;
    color: var(--muted);
  }
  .meta p {
    margin: 0;
  }
  .meta h4 {
    margin: 8px 0 0;
    color: var(--dim);
    font-size: 11px;
    font-weight: 560;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .draft {
    color: var(--wait);
  }
  .sources,
  .chips {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .sources {
    flex-direction: column;
    word-break: break-all;
  }
  .chips li {
    padding: 1px 7px;
    border: 1px solid var(--line-2);
    border-radius: 6px;
    color: var(--soft);
  }
</style>
