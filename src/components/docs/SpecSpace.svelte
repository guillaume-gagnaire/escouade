<script lang="ts">
  import Icon from '../Icon.svelte';
  import { t } from '../../lib/i18n';
  import { fAgo, fSize } from '../../lib/format';
  import { docsStore } from '../../lib/docs.svelte';
  import {
    comparedLine,
    latestCompared,
    movedLines,
    newPagePath,
    pageState,
    specSummary,
    subjectAbout,
    subjectCounts,
    subjectState,
    unsharedPages,
  } from '../../lib/docs';
  import { api } from '../../lib/ipc';
  import { subjectTickets, unplannedCount } from '../../lib/planning';
  import { app } from '../../lib/state.svelte';
  import type { DocsCandidate, DocsSubject, Project } from '../../lib/types';
  import Rich from '../../lib/i18n/Rich.svelte';
  import { tip } from '../../lib/tip';
  import PlanResult from './PlanResult.svelte';
  import SpaceHead from './SpaceHead.svelte';
  import SpecImport from './SpecImport.svelte';
  import SpecPage from './SpecPage.svelte';

  // The specification (ESC-258, docs/FACTORY.md « La spécification (piste C) », design/factory/2-specification.png): the
  // map of the subjects, each with where it stands against the code; on the left the tree of the pages, as a Confluence,
  // by subject; a page opened in place of the map. A project without a spec starts from an empty space that offers to
  // import the spec it already has.
  let { project }: { project: Project } = $props();

  const tree = $derived(docsStore.tree[project.id]);
  const error = $derived(docsStore.error[project.id]);
  const summary = $derived(tree ? specSummary(tree) : null);
  const page = $derived(app.docsPage[project.id] ?? null);
  const unshared = $derived(unsharedPages(docsStore.pending[project.id]?.files ?? []));
  const moved = $derived(tree ? movedLines(docsStore.moved[project.id] ?? [], tree) : []);

  /** The subjects the user folded or unfolded; the others follow `openByDefault`. */
  let folded = $state<Record<string, boolean>>({});
  /** Unfolded at first: every subject of a small spec; of a larger one, those with a gap or the page open. */
  function isOpen(s: DocsSubject): boolean {
    if (s.name in folded) return !folded[s.name];
    if (s.pages.some((p) => p.path === page)) return true;
    return (summary?.pages ?? 0) <= 12 || subjectState(s) === 'gap';
  }
  function toggle(s: DocsSubject) {
    folded[s.name] = isOpen(s);
  }
  function openPage(path: string | null) {
    app.openDocs(project.id, 'spec', path);
  }
  /** A subject's card opens its first page (its own when it has one of its name). */
  function openSubject(s: DocsSubject) {
    folded[s.name] = false;
    openPage(s.pages[0]?.path ?? null);
  }

  // « Nouvelle page »: a subject and a title, then the page opens to be written.
  let creating = $state(false);
  let newSubject = $state('');
  let newTitle = $state('');
  let newError = $state('');
  function startNew(subject = '') {
    creating = true;
    newSubject = subject;
    newTitle = '';
    newError = '';
  }
  async function createPage(e: SubmitEvent) {
    e.preventDefault();
    const path = newPagePath(newSubject, newTitle);
    const known = tree?.subjects.some((s) => s.pages.some((p) => p.path === path));
    if (known) {
      newError = t('docs.newPage.exists', { path });
      return;
    }
    // Its subject as typed (« Espace adhérent »), its folder the subject's slug.
    const made = await app.run(api.docsSpecWrite(project.id, path, newTitle.trim(), '', newSubject.trim()));
    if (!made) return;
    creating = false;
    await docsStore.load(project.id);
    app.docsEdit = path;
    openPage(path);
  }

  /** « Comparer au code » (ESC-263): every page, by the short agent that reads the code; their gaps come back on the
   * map as each one is done. */
  const comparing = $derived(app.docsComparing[project.id] ?? []);
  async function compareAll() {
    app.docsComparing[project.id] = ['*'];
    const done = await app.run(api.docsCompare(project.id, null));
    if (app.docsComparing[project.id]?.[0] === '*') app.docsComparing[project.id] = [];
    if (done) app.toast(comparedLine(done));
    await docsStore.load(project.id);
  }

  /** « Planifier » (ESC-262): the changes without tickets, the planner running, and the latest plan of the last day, unless closed. */
  const unplanned = $derived(unplannedCount(tree?.planning));
  const planRunning = $derived(!!docsStore.planning[project.id] || !!tree?.planning?.running);
  const lastPlan = $derived.by(() => {
    const last = tree?.planning?.last;
    return last && app.now - last.at < 24 * 3600 * 1000 && !docsStore.closedPlans.includes(last.id) ? last : null;
  });

  /** The spec the project has, whose cut is being looked at before an import. */
  let importing = $state<DocsCandidate | null>(null);
</script>

<SpaceHead title={t('docs.spaces.spec')}>
  {#snippet line()}
    {#if summary && summary.pages}
      {@const s = summary}
      <span class="stat" data-stat="ok"><Rich k="docs.spec.upToDate" count={s.ok}>{#snippet n()}<b>{s.ok}</b>{/snippet}</Rich></span>
      {#if s.gaps}<span class="stat gap" data-stat="gaps"
          ><Rich k="docs.spec.gaps" count={s.gaps}>{#snippet n()}<b>{s.gaps}</b>{/snippet}</Rich></span
        >{/if}
      {#if s.drafts}<span class="stat" data-stat="drafts"
          ><Rich k="docs.spec.drafts" count={s.drafts}>{#snippet n()}<b>{s.drafts}</b>{/snippet}</Rich></span
        >{/if}
      <span class="when"
        >{summary.compared ? t('docs.spec.compared', { when: fAgo(summary.compared / 1000, app.now) }) : t('docs.spec.neverCompared')}</span
      >
    {/if}
  {/snippet}
  {#snippet actions()}
    {#if tree}
      <button class="btn" onclick={() => app.newDiscussion(project.id, { kind: 'discussion', subject: '' })}
        ><Icon name="search" size={14} />{t('docs.discussion.ask')}</button
      >
      <!-- « Comparer au code » here is the whole spec; an open page has its own. -->
      {#if tree.pages && !page}
        <button class="btn" data-compare="all" disabled={comparing.length > 0} use:tip={t('docs.compare.allTip')} onclick={compareAll}
          ><Icon name="swap" size={14} />{comparing.length
            ? t('docs.compare.running', { count: comparing.length })
            : t('docs.compare.all')}</button
        >
      {/if}
      <button class="btn primary" onclick={() => startNew()}><Icon name="plus" size={14} />{t('docs.spec.newPage')}</button>
    {/if}
  {/snippet}
</SpaceHead>

{#if tree?.pages && (unplanned || planRunning)}
  <div class="plan-bar" data-plan-bar>
    <Icon name="kanban" size={14} />
    <span class="grow">{planRunning ? t('docs.plan.running') : t('docs.plan.unplanned', { count: unplanned })}</span>
    {#if unplanned}
      <button
        class="btn primary"
        data-plan="all"
        disabled={planRunning}
        use:tip={t('docs.plan.tip')}
        onclick={() => docsStore.plan(project.id)}>{t('docs.plan.action')}</button
      >
    {/if}
  </div>
{/if}

{#if creating}
  <form class="new" onsubmit={createPage} aria-label={t('docs.newPage.title')}>
    <label
      ><span>{t('docs.newPage.subject')}</span><input
        class="field input"
        bind:value={newSubject}
        placeholder={t('docs.newPage.subjectHint')}
        required
      /></label
    >
    <label
      ><span>{t('docs.newPage.name')}</span><input
        class="field input"
        bind:value={newTitle}
        placeholder={t('docs.newPage.nameHint')}
        required
      /></label
    >
    <button class="btn primary" type="submit">{t('docs.newPage.create')}</button>
    <button class="btn ghost" type="button" onclick={() => (creating = false)}>{t('docs.page.cancel')}</button>
    {#if newError}<p class="error" role="alert">{newError}</p>{/if}
  </form>
{/if}

{#if error && !tree}
  <p class="problem" role="alert">{error}</p>
{:else if tree && !tree.pages}
  <!-- A project without a spec: an empty space, the spec it has offered for import. -->
  <div class="empty">
    <div class="empty-in">
      <h2>{t('docs.empty.title')}</h2>
      <p>{t('docs.empty.text')}</p>
      {#if tree.existingSpec.length}
        <p class="found">{t('docs.empty.found')}</p>
        <ul class="cands">
          {#each tree.existingSpec as c (c.path)}
            <li class="cand" data-candidate={c.path}>
              <Icon name={c.kind === 'folder' ? 'folder' : 'file'} size={14} />
              <span class="mono">
                {c.kind === 'folder'
                  ? t('docs.empty.folder', { count: c.files, path: c.path })
                  : t('docs.empty.file', { path: c.path, size: fSize(c.bytes) })}
              </span>
              <button class="btn" onclick={() => (importing = c)}><Icon name="import" size={14} />{t('docs.empty.look')}</button>
            </li>
          {/each}
        </ul>
      {/if}
      <div class="empty-acts">
        <!-- The spec writer writes a first version from the code (ESC-261): a gesture of the user, it costs tokens. -->
        <button
          class="btn primary"
          use:tip={t('docs.discussion.firstSpecTip')}
          onclick={() =>
            app.newDiscussion(project.id, {
              kind: 'discussion',
              subject: t('docs.discussion.firstSpecSubject'),
              message: t('docs.discussion.firstSpecMessage'),
            })}><Icon name="sparkle" size={14} />{t('docs.discussion.firstSpec')}</button
        >
        <button class="btn" onclick={() => startNew()}><Icon name="edit" size={14} />{t('docs.empty.firstPage')}</button>
      </div>
    </div>
  </div>
{:else if tree}
  <div class="split">
    <nav class="tree" aria-label={t('docs.spec.treeLabel')}>
      <ul>
        {#each tree.subjects as s (s.name)}
          {@const open = isOpen(s)}
          <li data-subject={s.name}>
            <button
              class="subj"
              aria-label={open ? t('docs.spec.fold', { name: s.title }) : t('docs.spec.unfold', { name: s.title })}
              onclick={() => toggle(s)}
            >
              <Icon name={open ? 'down' : 'right'} size={12} /><span class="label">{s.title}</span>
              {#if !open && subjectState(s) !== 'ok'}<span class="mark"
                  >{t(subjectState(s) === 'gap' ? 'docs.spec.gapMark' : 'docs.spec.draftMark')}</span
                >{/if}
            </button>
            {#if open}
              <ul>
                {#each s.pages as p (p.path)}
                  {@const st = pageState(p)}
                  <li data-page={p.path}>
                    <button
                      class="pg"
                      class:on={p.path === page}
                      aria-current={p.path === page ? 'page' : undefined}
                      onclick={() => openPage(p.path)}
                    >
                      <span class="dot {st}" aria-hidden="true"></span><span class="label">{p.title}</span>
                      {#if st === 'gap'}<span class="mark">{t('docs.spec.gapMark')}</span>
                      {:else if st === 'draft'}<span class="mark">{t('docs.spec.draftMark')}</span>
                      {:else if unshared.has(p.path)}<span class="mark" use:tip={t('docs.spec.unsharedTip')}
                          >{t('docs.spec.unsharedMark')}</span
                        >{/if}
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
          </li>
        {/each}
      </ul>
    </nav>

    <div class="main">
      {#if page}
        {#key page}
          <SpecPage {project} path={page} onback={() => openPage(null)} />
        {/key}
      {:else}
        {#if lastPlan}
          <div class="last-plan"><PlanResult {project} plan={lastPlan} /></div>
        {/if}
        <ul class="map" aria-label={t('docs.spec.mapLabel')}>
          {#each tree.subjects as s (s.name)}
            {@const st = subjectState(s)}
            {@const n = subjectCounts(s)}
            {@const at = latestCompared(s.pages)}
            <li>
              <button
                class="card {st}"
                data-subject-card={s.name}
                aria-label={t('docs.spec.openSubject', { name: s.title })}
                onclick={() => openSubject(s)}
              >
                <span class="c-head">
                  <span class="c-title">{s.title}</span>
                  <span class="pill {st}" data-state={st}
                    >{st === 'gap' ? t('docs.count.gaps', { count: n.gaps }) : t(`docs.states.${st}`)}</span
                  >
                </span>
                {#if subjectAbout(s)}<span class="c-about">{subjectAbout(s)}</span>{/if}
                <span class="c-counts">
                  <span>{t('docs.count.pages', { count: n.pages })}</span>
                  <span>{t('docs.count.tickets', { count: n.tickets })}</span>
                  <span>{t('docs.count.decisions', { count: n.decisions })}</span>
                </span>
                {#if n.tickets}
                  {@const keys = subjectTickets(s.pages)}
                  <!-- Its tickets (ESC-262): the keys, the state of those of this board. -->
                  <span class="c-tickets" data-subject-tickets>
                    {#each keys.slice(0, 4) as k (k)}
                      {@const tk = Object.values(app.tickets).find((x) => x.projectId === project.id && x.key === k)}
                      <span class="tkey {tk?.column ?? ''}" use:tip={tk ? `${k} · ${tk.title}` : k}>{k}</span>
                    {/each}
                    {#if keys.length > 4}<span class="tkey more">+{keys.length - 4}</span>{/if}
                  </span>
                {/if}
                <span class="c-when" data-compared={at ?? null}
                  >{s.pages.some((p) => comparing.includes(p.path))
                    ? t('docs.compare.runningOne')
                    : at
                      ? t('docs.spec.subjectCompared', { when: fAgo(at / 1000, app.now) })
                      : t('docs.spec.subjectNeverCompared')}</span
                >
              </button>
            </li>
          {/each}
          <li>
            <div class="card moved" data-moved>
              <span class="m-title">{t('docs.moved.title')}</span>
              {#if unshared.size}<span class="m-line">{t('docs.moved.unshared', { count: unshared.size })}</span>{/if}
              {#each moved as m, i (i)}
                <span class="m-line">{t('docs.moved.line', { who: m.author, what: m.what, when: fAgo(m.when, app.now) })}</span>
              {/each}
              {#if !moved.length && !unshared.size}<span class="m-line">{t('docs.moved.none')}</span>{/if}
            </div>
          </li>
        </ul>
      {/if}
    </div>
  </div>
{/if}

{#if importing}
  <SpecImport {project} candidate={importing} onclose={() => (importing = null)} />
{/if}

<style>
  .plan-bar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 22px;
    border-bottom: 1px solid var(--line);
    background: color-mix(in oklch, var(--accent) 7%, transparent);
    color: var(--muted);
    font-size: 13px;
  }
  .plan-bar .grow {
    flex: 1;
    min-width: 0;
  }
  .last-plan {
    flex: none;
    padding: 20px 20px 0;
  }
  .last-plan + .map {
    padding-top: 14px;
  }
  .c-tickets {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .tkey {
    padding: 0 6px;
    border-radius: 5px;
    background: var(--hover);
    color: var(--muted);
    font-family: var(--mono);
    font-size: 11px;
    line-height: 18px;
  }
  .tkey.done {
    color: var(--ok);
  }
  .tkey.doing,
  .tkey.review {
    color: var(--accent);
  }
  .stat b {
    color: var(--text);
    font-weight: 620;
  }
  .stat.gap,
  .stat.gap b {
    color: var(--wait);
  }
  .when {
    color: var(--dim);
  }
  .new {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    gap: 10px;
    padding: 12px 22px;
    border-bottom: 1px solid var(--line);
  }
  .new label {
    display: grid;
    gap: 4px;
    color: var(--muted);
    font-size: 12px;
  }
  .new input {
    width: 220px;
  }
  .error,
  .problem {
    flex-basis: 100%;
    margin: 0;
    color: var(--del);
    font-size: 12.5px;
  }
  .problem {
    padding: 22px;
  }

  .empty {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: grid;
    place-items: center;
    padding: 30px;
  }
  .empty-in {
    max-width: 560px;
    display: grid;
    gap: 10px;
  }
  .empty h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 620;
  }
  .empty p {
    margin: 0;
    color: var(--muted);
    line-height: 1.5;
  }
  .empty .found {
    margin-top: 8px;
    color: var(--soft);
  }
  .cands {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 6px;
  }
  .cand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 8px 8px 12px;
    border: 1px solid var(--line-2);
    border-radius: 10px;
    background: var(--raise);
  }
  .cand .mono {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12.5px;
  }
  .empty-acts {
    margin-top: 8px;
  }

  .split {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(200px, 300px) minmax(0, 1fr);
  }
  .tree {
    min-height: 0;
    overflow: auto;
    padding: 14px 10px;
    border-right: 1px solid var(--line);
  }
  .tree ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .tree ul ul {
    padding-left: 14px;
  }
  .subj,
  .pg {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 32px;
    padding: 0 8px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--soft);
    font-size: 13.5px;
    text-align: left;
    cursor: pointer;
  }
  .subj {
    color: var(--text);
  }
  .subj:hover,
  .pg:hover {
    background: var(--hover);
  }
  .pg.on {
    background: var(--raise-2);
    color: var(--text);
  }
  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .mark {
    flex: none;
    color: var(--dim);
    font-size: 12px;
  }
  .dot {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--run);
  }
  .dot.gap {
    background: var(--wait);
  }
  .dot.draft {
    background: var(--idle);
  }

  .main {
    container-type: inline-size;
    min-width: 0;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
  }
  .map {
    margin: 0;
    padding: 20px;
    list-style: none;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 14px;
    align-content: start;
  }
  .map li {
    display: flex;
  }
  .card {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 132px;
    padding: 16px 16px 14px;
    border: 1px solid var(--line-2);
    border-radius: 12px;
    background: var(--raise);
    color: var(--soft);
    font-size: 13px;
    text-align: left;
  }
  button.card {
    cursor: pointer;
  }
  button.card:hover {
    border-color: var(--line-3);
  }
  .card.gap {
    border-color: var(--wait-line);
  }
  .c-head {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .c-title {
    flex: 1;
    min-width: 0;
    color: var(--text);
    font-size: 14.5px;
    font-weight: 640;
  }
  .pill {
    flex: none;
    padding: 2px 9px;
    border-radius: 999px;
    font-size: 12px;
    font-weight: 560;
    background: oklch(from var(--run) l c h / 0.12);
    color: var(--run);
  }
  .pill.gap {
    background: oklch(from var(--wait) l c h / 0.14);
    color: var(--wait);
  }
  .pill.draft {
    background: var(--hover);
    color: var(--muted);
  }
  .c-about {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
    line-height: 1.45;
  }
  .c-counts {
    margin-top: auto;
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    color: var(--dim);
    font-size: 12.5px;
  }
  .c-when {
    color: var(--dim);
    font-size: 12px;
  }
  .card.moved {
    align-items: center;
    justify-content: center;
    gap: 6px;
    border-style: dashed;
    background: transparent;
    text-align: center;
  }
  .m-title {
    color: var(--muted);
    font-size: 14px;
  }
  .m-line {
    color: var(--dim);
    font-size: 12.5px;
  }
</style>
