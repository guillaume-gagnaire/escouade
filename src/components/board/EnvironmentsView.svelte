<script lang="ts">
  // The view « Environnements » of a project's board (ESC-67; direction A of design/kanban-envs-2-2, ESC-390): the tickets
  // merged and deployed nowhere yet, then one lane per environment shown, in the order the user gave, each headed by
  // what runs there (commit, when, who, its address), a deployment under way and a failed one with its job's link. Its
  // line, its menu of the environments and « Rafraîchir » are in the board's header.
  import { onDestroy, onMount } from 'svelte';
  import { arrange, envState, lanes, shortSha } from '../../lib/environments';
  import { envsView } from '../../lib/envs-view.svelte';
  import { fAgo, fList } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { openUrl } from '../../lib/opener';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { Deployment, Project, Ticket } from '../../lib/types';
  import Icon from '../Icon.svelte';

  let { project }: { project: Project } = $props();

  /** The tickets a lane shows before « + N autres ». */
  const SHOWN = 8;
  /** How often the project is said followed while the view is open (the backend forgets it after 15 minutes). */
  const KEEP_EVERY = 5 * 60 * 1000;

  const data = $derived(app.environments[project.id]);
  const all = $derived(Object.values(app.tickets).filter((x) => x.projectId === project.id));
  const phase = $derived(envState(data));
  const set = $derived(arrange(data?.environments ?? [], envsView.of(project.id)));
  /** The lanes of the environments shown: a ticket deployed only where the user hid goes back among those that wait. */
  const placed = $derived(lanes(all, set.shown, app.now));

  let keep: ReturnType<typeof setInterval> | undefined;
  onMount(async () => {
    const p = await app.run(api.projectEnvironments(project.id));
    if (p) app.environments[project.id] = p;
    // Read when never read, or read more than a minute ago.
    if (!p?.readAt || Date.now() - p.readAt > 60_000) void envsView.refresh(project.id);
    keep = setInterval(() => void api.projectEnvironments(project.id).catch(() => {}), KEEP_EVERY);
  });
  onDestroy(() => clearInterval(keep));

  const when = (d: Deployment | null | undefined) => (d?.created ? fAgo(d.created, app.now) : '');

  function open(x: Ticket) {
    if (x.outcomeUrl) void app.run(openUrl(x.outcomeUrl));
  }
</script>

{#snippet card(x: Ticket, merged: boolean)}
  <!-- Its key and title; merged and deployed nowhere yet, when and its commit. -->
  {#snippet inner()}
    <span class="key">{x.key}</span>
    <span class="title">{x.title}</span>
    {#if merged && x.doneAt}
      <span class="meta"
        >{t('envs.view.merged', { since: fAgo(x.doneAt / 1000, app.now) })}{#if x.outcomeCommit}{' · '}<span class="mono"
            >{shortSha(x.outcomeCommit)}</span
          >{/if}</span
      >
    {/if}
  {/snippet}
  {#if x.outcomeUrl}
    <button class="tk" data-env-ticket={x.key} onclick={() => open(x)}>{@render inner()}</button>
  {:else}
    <div class="tk" data-env-ticket={x.key}>{@render inner()}</div>
  {/if}
{/snippet}

{#snippet list(tickets: Ticket[], merged: boolean)}
  {#each tickets.slice(0, SHOWN) as x (x.id)}
    {@render card(x, merged)}
  {:else}
    <p class="empty">{merged ? t('envs.view.waitingEmpty') : t('envs.view.empty')}</p>
  {/each}
  {#if tickets.length > SHOWN}<span class="more">{t('envs.view.more', { count: tickets.length - SHOWN })}</span>{/if}
{/snippet}

<section class="envs" aria-label={t('envs.view.title')}>
  {#if data?.error && phase === 'ready'}
    <p class="note err" role="status">{t('envs.view.error', { error: data.error })}</p>
  {/if}

  {#if phase === 'ready'}
    <div class="lanes" style:--n={set.shown.length + 1}>
      <section class="lane" data-env-lane="waiting">
        <div class="lh">
          <h2><i class="dot wt"></i>{t('envs.view.waiting')}<span class="n">{placed.waiting.length}</span></h2>
        </div>
        {@render list(placed.waiting, true)}
      </section>
      {#each placed.lanes as l (l.env.name)}
        {@const e = l.env}
        <section class="lane" data-env-lane={e.name}>
          <div class="lh" class:bad={e.failed} data-env={e.name}>
            <h2>
              <i class="dot" class:ok={!!e.running && !e.failed} class:ko={e.failed}></i><span class="name">{e.name}</span><span class="n"
                >{l.tickets.length}</span
              >{#if e.production}<span class="tag">{t('envs.view.production')}</span>{/if}
            </h2>
            <span class="m">
              {#if e.running}
                <span class="mono">{shortSha(e.running.sha)}</span>{#if when(e.running)}{' · '}{when(
                    e.running,
                  )}{/if}{#if e.running.author}{' · '}{t('envs.view.by', { author: e.running.author })}{/if}
              {:else}
                {t('envs.view.nothingRuns')}
              {/if}
            </span>
            {#if e.url}
              {@const url = e.url}
              <button class="link" use:tip={url} onclick={() => app.run(openUrl(url))}
                ><span class="u">{url.replace(/^https?:\/\//, '')}</span><Icon name="ext" size={12} /></button
              >
            {/if}
            {#if e.failed}
              <span class="fail"
                ><Icon name="close" size={12} /><span class="ft" role="status" data-env-failed
                  >{e.latest?.sha ? t('envs.view.failed', { sha: shortSha(e.latest.sha) }) : t('envs.view.failedNoSha')}</span
                >{#if e.latest?.log}{@const log = e.latest.log}<button class="link job" onclick={() => app.run(openUrl(log))}
                    >{t('envs.view.job')}<Icon name="ext" size={12} /></button
                  >{/if}</span
              >
            {:else if e.latest}
              <span class="wip" data-env-underway
                ><i class="spin" aria-hidden="true"></i>{t('envs.view.underWay', { sha: shortSha(e.latest.sha) })}</span
              >
            {/if}
          </div>
          {@render list(l.tickets, false)}
        </section>
      {/each}
    </div>
    {#if !set.shown.length}
      <p class="note" role="status">{t('envs.view.noneShown')}</p>
    {:else if set.neverRan.length}
      <p class="note" data-never-ran>
        <Icon name="read" size={14} />{t('envs.view.neverRan', {
          count: set.neverRan.length,
          names: fList(set.neverRan.map((e) => e.name)),
        })}
      </p>
    {/if}
  {:else}
    <p class="note" class:err={phase === 'error'} role="status">
      {#if phase === 'loading'}{t('envs.view.loading')}
      {:else if phase === 'unsupported'}{t('envs.view.unsupported', { host: data?.host ?? '' })}
      {:else if phase === 'error'}{t('envs.view.error', { error: data?.error ?? '' })}
      {:else}{t('envs.view.none', { host: data?.host ?? '' })}{/if}
    </p>
  {/if}
</section>

<style>
  .envs {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: auto;
  }
  .note {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 4px 22px 16px;
    color: var(--muted);
    font-size: 12.5px;
  }
  .note.err {
    margin-top: 16px;
    color: var(--del);
  }
  /* One lane per environment shown, side by side as the board's columns; under 250 pixels each, it scrolls sideways. */
  .lanes {
    display: grid;
    grid-template-columns: repeat(var(--n, 3), minmax(250px, 1fr));
    align-items: start;
    gap: 14px;
    padding: 16px 16px 12px 18px;
  }
  .lane {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  /* The head of a lane: no box, a line under it; what runs there, under way, failed. */
  .lh {
    display: grid;
    justify-items: start;
    gap: 4px;
    margin-bottom: 4px;
    padding: 2px 2px 10px;
    border-bottom: 1px solid var(--line);
    font-size: 12px;
  }
  .lh.bad {
    border-bottom-color: oklch(from var(--del) l c h / 0.6);
  }
  h2 {
    justify-self: stretch;
    margin: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    color: var(--text);
    font-size: 13px;
    font-weight: 600;
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .n {
    color: var(--muted);
    font-size: 12px;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .tag {
    margin-left: auto;
    padding: 1px 7px;
    border-radius: 6px;
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--muted);
    font-size: 11px;
    font-weight: 550;
  }
  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--idle);
  }
  .dot.ok {
    background: var(--run);
  }
  .dot.wt {
    background: var(--wait);
  }
  .dot.ko {
    background: var(--del);
  }
  .m {
    min-width: 0;
    color: var(--muted);
  }
  .m .mono {
    color: var(--soft);
  }
  .mono {
    font-family: var(--mono, ui-monospace, Consolas, monospace);
    font-size: 11.5px;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    padding: 0;
    border: none;
    background: none;
    color: var(--info);
    font: inherit;
    cursor: pointer;
  }
  .link .u {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .link:hover {
    text-decoration: underline;
  }
  .wip {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    color: var(--wait);
  }
  .spin {
    flex: none;
    width: 10px;
    height: 10px;
    border: 1.6px solid currentColor;
    border-right-color: transparent;
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(1turn);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }
  .fail {
    justify-self: stretch;
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--del);
    font-weight: 550;
  }
  .fail .ft {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fail .job {
    margin-left: auto;
    flex: none;
    font-weight: 500;
  }
  /* A ticket that reached the lane: as a small card of the board. */
  .tk {
    display: grid;
    gap: 3px;
    padding: 10px 12px;
    border: none;
    border-radius: 10px;
    background: var(--raise-2);
    box-shadow: inset 0 0 0 1px var(--line);
    color: inherit;
    font: inherit;
    text-align: left;
  }
  button.tk {
    cursor: pointer;
  }
  button.tk:hover {
    box-shadow: inset 0 0 0 1px var(--line-3);
  }
  .key {
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 600;
  }
  .title {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
    color: var(--soft);
    font-size: 12.5px;
  }
  .meta {
    color: var(--muted);
    font-size: 11.5px;
  }
  /* A lane without a ticket: said in its place, hatched lightly. */
  .empty {
    margin: 0;
    padding: 14px 12px;
    border-radius: 10px;
    background: repeating-linear-gradient(135deg, transparent 0 6px, rgba(var(--fg-rgb), 0.02) 6px 12px);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--muted);
    font-size: 12px;
    text-align: center;
  }
  .more {
    padding: 2px 4px;
    color: var(--muted);
    font-size: 12px;
  }
</style>
