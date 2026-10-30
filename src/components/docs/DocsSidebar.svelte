<script lang="ts">
  import { t } from '../../lib/i18n';
  import { DOCS_SPACES, specSummary, type DocsSpaceId } from '../../lib/docs';
  import { docsStore } from '../../lib/docs.svelte';
  import { discussionState, discussionsOf, proposedCount } from '../../lib/discussions';
  import { modelLabel } from '../../lib/models';
  import { keyLabel } from '../../lib/platform';
  import { tip } from '../../lib/tip';
  import { fAgo } from '../../lib/format';
  import { refLabel } from '../../lib/decisions';
  import Icon from '../Icon.svelte';
  import StatusDot from '../StatusDot.svelte';
  import { app } from '../../lib/state.svelte';
  import type { Project } from '../../lib/types';

  // Under « Agents · Kanban · Docs » when Docs is on (ESC-258, design/factory/2-specification.png): the three spaces of
  // the docs, the spec with its gaps. Above them, « Nouvelle discussion » and the discussions (ESC-261), in cards as the
  // agents, each marked Brainstorm or Discussion, with its state.
  let { project }: { project: Project } = $props();

  const space = $derived(app.docsSpace[project.id] ?? 'spec');
  const open = $derived(app.docsDiscussion[project.id] ?? null);
  const discussions = $derived(discussionsOf(app.agents, project.id));
  const tree = $derived(docsStore.tree[project.id]);
  const summary = $derived(tree ? specSummary(tree) : null);

  function count(id: DocsSpaceId): string {
    if (id === 'spec') {
      if (!summary?.pages) return '';
      return summary.gaps ? t('docs.count.gaps', { count: summary.gaps }) : t('docs.count.pages', { count: summary.pages });
    }
    const n = id === 'decisions' ? docsStore.decisions[project.id]?.length : docsStore.documents[project.id]?.length;
    return n === undefined ? '' : String(n);
  }

  /** The dot of a space: the spec in the colour of a gap when it has one, the decisions when one waits to be accepted. */
  function tone(id: DocsSpaceId) {
    if (id === 'spec') return summary?.gaps ? 'gap' : summary?.pages ? 'ok' : 'none';
    if (id === 'decisions') {
      const list = docsStore.decisions[project.id] ?? [];
      return list.some((d) => d.state === 'proposed') ? 'gap' : list.length ? 'ok' : 'none';
    }
    return 'none';
  }
</script>

<div class="docs-side">
  <button
    class="new-disc"
    aria-keyshortcuts="Control+B"
    use:tip={t('docs.side.newDiscussionTip', { shortcut: keyLabel('Ctrl+B') })}
    onclick={() => app.newDiscussion(project.id)}
    ><span class="plus"><Icon name="plus" size={14} /></span><span class="nl">{t('docs.side.newDiscussion')}</span><kbd
      >{keyLabel('Ctrl+B')}</kbd
    ></button
  >
  {#if discussions.length}
    <section class="discs" aria-label={t('docs.side.discussions')}>
      {#each discussions as a (a.id)}
        {@const d = a.discussion!}
        {@const state = discussionState(a)}
        <button
          class="disc card"
          class:sel={open === a.id}
          aria-current={open === a.id ? 'true' : undefined}
          data-discussion={a.id}
          data-kind={d.kind}
          data-state={state}
          onclick={() => app.openDiscussion(project.id, a.id)}
        >
          <span class="d-top">
            <StatusDot status={a.status} />
            <span class="d-name">{d.subject}</span>
            <span class="d-state {state}">{t(`docs.discussion.states.${state}`)}</span>
          </span>
          <span class="d-line">
            <span class="tag {d.kind}">{t(`docs.discussion.kinds.${d.kind}`)}</span>
            {#if state === 'poured' && d.poured}
              <span
                >{fAgo(d.poured.at / 1000, app.now)} · {t('docs.discussion.pages', {
                  count: d.poured.pages.length,
                })}{#each d.poured.decisions as n (n)}{' · '}{typeof n === 'number'
                    ? t('docs.discussion.decisionNo', { n: refLabel(n) })
                    : n}{/each}</span
              >
            {:else}
              <span
                >{modelLabel(a.model, app.models)}{#if proposedCount(d)}{' · '}{t('docs.discussion.proposals', {
                    count: proposedCount(d),
                  })}{/if}</span
              >
            {/if}
          </span>
        </button>
      {/each}
    </section>
  {/if}
  <section class="spaces" aria-label={t('docs.side.spacesLabel')}>
    <h3 class="sh">{t('docs.side.spaces')}</h3>
    {#each DOCS_SPACES as id (id)}
      <button
        class="space"
        class:sel={space === id}
        aria-current={space === id ? 'page' : undefined}
        data-space={id}
        onclick={() => {
          app.openDocs(project.id, id, null);
          // A click on a space reads it again: what an agent or a comparison wrote meanwhile.
          void docsStore.load(project.id);
        }}
      >
        <span class="dot {tone(id)}" aria-hidden="true"></span>
        <span class="name">{t(`docs.spaces.${id}`)}</span>
        <span class="n" class:gap={id === 'spec' && !!summary?.gaps}>{count(id)}</span>
      </button>
    {/each}
  </section>
</div>

<style>
  .docs-side {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    padding-top: 14px;
  }
  .new-disc {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 34px;
    margin: 0 0 8px;
    padding: 0 10px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: 13.5px;
    cursor: pointer;
  }
  .new-disc:hover {
    background: var(--hover);
  }
  .new-disc .nl {
    flex: 1;
    text-align: left;
  }
  .new-disc kbd {
    color: var(--dim);
    font-size: 11.5px;
    font-family: inherit;
  }
  .discs {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 6px;
    margin-bottom: 18px;
  }
  .disc {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 5px;
    padding: 9px 11px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--raise);
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .disc:hover {
    background: var(--hover);
  }
  .disc.sel {
    border-color: var(--line-strong, var(--soft));
    background: var(--raise-2);
  }
  .d-top {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .d-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 600;
    font-size: 13.5px;
  }
  .d-state {
    flex: none;
    color: var(--dim);
    font-size: 12px;
  }
  .d-state.question {
    color: var(--wait);
  }
  .d-state.answering {
    color: var(--run);
  }
  .d-line {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    color: var(--dim);
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
  }
  .d-line > span:last-child {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tag {
    flex: none;
    padding: 0 6px;
    border-radius: 4px;
    font-size: 11.5px;
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
  .sh {
    margin: 0 0 6px;
    padding: 0 10px;
    color: var(--dim);
    font-size: 11px;
    font-weight: 560;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .spaces {
    display: grid;
    gap: 2px;
  }
  .space {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 36px;
    padding: 0 10px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--soft);
    font-size: 13.5px;
    text-align: left;
    cursor: pointer;
  }
  .space:hover {
    background: var(--hover);
  }
  .space.sel {
    background: var(--raise-2);
    color: var(--text);
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .n {
    flex: none;
    color: var(--dim);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .dot {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--idle);
  }
  .dot.ok {
    background: var(--run);
  }
  .dot.gap {
    background: var(--wait);
  }
</style>
