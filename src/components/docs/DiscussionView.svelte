<script lang="ts">
  import Icon from '../Icon.svelte';
  import Conversation from '../Conversation.svelte';
  import PlanResult from './PlanResult.svelte';
  import SpaceHead from './SpaceHead.svelte';
  import { decisionNo } from '../../lib/decisions';
  import { docsStore } from '../../lib/docs.svelte';
  import { pageLines, proposedCount } from '../../lib/discussions';
  import { canPlan } from '../../lib/planning';
  import { fAgo } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { Agent, Project } from '../../lib/types';

  // A discussion open in the Docs tab (ESC-261, piste B, design/factory/1-brainstorm.png): its conversation on the left,
  // the same as any agent's; on the right « Ce qui ira dans la spec », the changes it proposes page by page and its
  // decisions, which « Verser dans la spec » writes on the docs branch (nothing shared: the status bar says so).
  let { project, agent }: { project: Project; agent: Agent } = $props();

  const d = $derived(agent.discussion!);
  const count = $derived(proposedCount(d));
  let pouring = $state(false);
  /** « Planifier » (ESC-262): offered once changes were poured, none planned since. */
  const plannable = $derived(canPlan(d));
  const planning = $derived(!!docsStore.planning[project.id]);

  async function pour() {
    pouring = true;
    const done = await app.run(api.discussionPour(agent.id));
    pouring = false;
    if (!done) return;
    app.toast(t('docs.discussion.poured', { count: done.pages.length }), 'ok');
    void docsStore.load(project.id);
  }

  /** The page of the spec a change is to, once poured: it opens in the Specification. */
  function openPage(path: string) {
    app.openDocs(project.id, 'spec', path);
  }
</script>

<div class="discussion" data-discussion={agent.id} data-kind={d.kind}>
  <SpaceHead title={d.subject}>
    {#snippet line()}
      <span class="tag {d.kind}">{t(`docs.discussion.kinds.${d.kind}`)}</span>
      <span>{t(`docs.discussion.with.${d.kind}`)}</span>
      {#if d.poured}
        <span>{t('docs.discussion.pouredLine', { when: fAgo(d.poured.at / 1000, app.now) })}</span>
      {/if}
    {/snippet}
    {#snippet actions()}
      {#if d.poured?.pages.length}
        <button class="btn ghost" onclick={() => openPage(d.poured!.pages[0])}
          ><Icon name="file" size={14} />{t('docs.discussion.openPage')}</button
        >
      {/if}
      {#if plannable && !count}
        <button
          class="btn primary"
          data-plan-discussion
          disabled={planning}
          use:tip={t('docs.plan.tip')}
          onclick={() => docsStore.plan(project.id, agent.id)}
          ><Icon name="kanban" size={14} />{planning ? t('docs.plan.running') : t('docs.plan.action')}</button
        >
      {:else}
        {#if plannable}
          <button
            class="btn"
            data-plan-discussion
            disabled={planning}
            use:tip={t('docs.plan.tip')}
            onclick={() => docsStore.plan(project.id, agent.id)}
            ><Icon name="kanban" size={14} />{planning ? t('docs.plan.running') : t('docs.plan.action')}</button
          >
        {/if}
        <button class="btn primary" disabled={!count || pouring} use:tip={t('docs.discussion.pourTip')} onclick={pour}
          >{t('docs.discussion.pour')}</button
        >
      {/if}
    {/snippet}
  </SpaceHead>

  <div class="split">
    <div class="conv">
      {#key agent.id}
        <Conversation {agent} {project} />
      {/key}
    </div>
    <aside class="pour" aria-label={t('docs.discussion.pourPanel')}>
      {#if d.planned && !docsStore.closedPlans.includes(d.planned.id)}
        <h3 class="sh">{t('docs.plan.heading')}</h3>
        <PlanResult {project} plan={d.planned} />
      {/if}
      <h3 class="sh">{t('docs.discussion.pourPanel')}</h3>
      {#if !count}
        <p class="nothing">{t('docs.discussion.nothing')}</p>
      {/if}
      {#each d.pages as c (c.path)}
        {@const lines = pageLines(c.before, c.page.body)}
        <section class="change" data-page={c.path}>
          <header>
            <b
              >{c.page.subject && c.page.subject.toLowerCase() !== c.page.title.toLowerCase() && c.page.subject !== c.path.split('/')[0]
                ? `${c.page.subject} › `
                : ''}{c.page.title}</b
            >
            <span class="what"
              >{c.before === null ? t('docs.discussion.newSection') : t('docs.discussion.changed', { count: c.added + c.removed })}</span
            >
            <button
              class="drop"
              aria-label={t('docs.discussion.dropPage', { name: c.page.title })}
              use:tip={t('docs.discussion.dropPage', { name: c.page.title })}
              onclick={() => app.run(api.discussionDrop(agent.id, c.path, null))}><Icon name="close" size={12} /></button
            >
          </header>
          <div class="lines">
            {#each lines as l, i (i)}
              <p class={l.kind}>{l.kind === 'add' && c.before === null ? '+ ' : ''}{l.text}</p>
            {/each}
          </div>
        </section>
      {/each}
      {#if d.decisions.length}
        <h3 class="sh">{t('docs.discussion.proposedDecisions')}</h3>
        {#each d.decisions as dec, i (i)}
          <section class="change dec" data-decision={i + 1}>
            <header>
              <b>{dec.title}</b>
              <span class="what"
                >{dec.supersedes?.length
                  ? t('docs.discussion.replaces', { n: decisionNo(dec.supersedes[0]) })
                  : t('docs.discussion.proposed')}</span
              >
              <button
                class="drop"
                aria-label={t('docs.discussion.dropDecision', { name: dec.title })}
                use:tip={t('docs.discussion.dropDecision', { name: dec.title })}
                onclick={() => app.run(api.discussionDrop(agent.id, null, i + 1))}><Icon name="close" size={12} /></button
              >
            </header>
            {#if dec.decision || dec.consequences}
              <p class="text">{dec.decision}{dec.decision && dec.consequences ? ' ' : ''}{dec.consequences}</p>
            {/if}
          </section>
        {/each}
      {/if}
    </aside>
  </div>
</div>

<style>
  .discussion {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .split {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(280px, 0.85fr);
  }
  .conv {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    border-right: 1px solid var(--line);
  }
  .pour {
    min-height: 0;
    overflow: auto;
    padding: 18px 20px 24px;
  }
  .sh {
    margin: 4px 0 10px;
    color: var(--dim);
    font-size: 11.5px;
    font-weight: 560;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .sh:not(:first-child) {
    margin-top: 20px;
  }
  .nothing {
    color: var(--dim);
    font-size: 13px;
    line-height: 1.5;
  }
  .change {
    margin-bottom: 10px;
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-radius: 10px;
    background: var(--raise);
  }
  .change header {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin-bottom: 6px;
  }
  .change b {
    flex: 1;
    min-width: 0;
    font-size: 13.5px;
  }
  .what {
    flex: none;
    color: var(--dim);
    font-size: 12px;
  }
  .drop {
    flex: none;
    align-self: center;
    display: inline-flex;
    padding: 3px;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--dim);
    cursor: pointer;
  }
  .drop:hover {
    background: var(--hover);
    color: var(--text);
  }
  .lines p,
  .text {
    margin: 2px 0;
    font-size: 13px;
    line-height: 1.5;
  }
  .lines .add {
    color: var(--add);
  }
  .lines .del {
    color: var(--del);
    text-decoration: line-through;
  }
  .tag {
    padding: 0 7px;
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
  @container (max-width: 900px) {
    .split {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
