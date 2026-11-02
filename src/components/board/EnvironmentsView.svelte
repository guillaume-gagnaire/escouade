<script lang="ts">
  // The view « Environnements » of a project's board (ESC-67, direction A chosen on preview): one lane per environment,
  // the way code goes (merged → staging → production), each with what runs there (commit, when, who, its address) and
  // the tickets that reached it; a failed deployment is marked on its environment with its job's link.
  import { onDestroy, onMount } from 'svelte';
  import { envState, lanes, notInProduction, shortSha } from '../../lib/environments';
  import { fAgo } from '../../lib/format';
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
  const shown = $derived(envState(data));
  const envs = $derived(data?.environments ?? []);
  const placed = $derived(lanes(all, envs, app.now));
  const late = $derived(notInProduction(all, envs, app.now));
  let reading = $state(false);

  async function refresh() {
    if (reading) return;
    reading = true;
    const p = await app.run(api.projectEnvironmentsRead(project.id));
    if (p) app.environments[project.id] = p;
    reading = false;
  }

  let keep: ReturnType<typeof setInterval> | undefined;
  onMount(async () => {
    const p = await app.run(api.projectEnvironments(project.id));
    if (p) app.environments[project.id] = p;
    // Read when never read, or read more than a minute ago.
    if (!p?.readAt || Date.now() - p.readAt > 60_000) void refresh();
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
    <span class="none">{t('envs.view.empty')}</span>
  {/each}
  {#if tickets.length > SHOWN}<span class="none">{t('envs.view.more', { count: tickets.length - SHOWN })}</span>{/if}
{/snippet}

<section class="envs" aria-label={t('envs.view.title')}>
  <div class="bar">
    <span class="sub"
      >{t('envs.view.sub', { project: project.name, host: data?.host || '—' })}{#if data?.readAt}{' · '}{t('envs.view.read', {
          since: fAgo(data.readAt / 1000, app.now),
        })}{/if}</span
    >
    {#if late.length}
      <span class="late" data-not-in-prod><Icon name="clock" size={14} />{t('envs.view.notInProd', { count: late.length })}</span>
    {/if}
    <button class="btn small" use:tip={t('envs.view.refreshTitle')} aria-busy={reading} disabled={reading} onclick={refresh}
      ><Icon name="restart" size={14} />{t('envs.view.refresh')}</button
    >
  </div>
  {#if data?.error && shown === 'ready'}
    <p class="note err" role="status">{t('envs.view.error', { error: data.error })}</p>
  {/if}

  {#if shown === 'ready'}
    <div class="lanes" style:--n={envs.length + 1}>
      <section class="lane" data-env-lane="waiting">
        <h2><i class="dot wt"></i>{t('envs.view.waiting')}<span class="n">{placed.waiting.length}</span></h2>
        {@render list(placed.waiting, true)}
      </section>
      {#each placed.lanes as l (l.env.name)}
        {@const e = l.env}
        <section class="lane" data-env-lane={e.name}>
          <h2>
            <i class="dot" class:ok={!!e.running && !e.failed} class:ko={e.failed}></i>{e.name}<span class="n">{l.tickets.length}</span>
          </h2>
          <div class="runs" class:bad={e.failed} data-env={e.name}>
            {#if e.running}
              <span class="line"
                ><span class="mono">{shortSha(e.running.sha)}</span>{#if when(e.running)}<span class="meta">{' · '}{when(e.running)}</span
                  >{/if}{#if e.running.author}<span class="meta">{' · '}{t('envs.view.by', { author: e.running.author })}</span>{/if}</span
              >
            {:else}
              <span class="meta">{t('envs.view.nothingRuns')}</span>
            {/if}
            {#if e.url}
              {@const url = e.url}
              <button class="link" use:tip={url} onclick={() => app.run(openUrl(url))}
                ><span class="u">{url.replace(/^https?:\/\//, '')}</span><Icon name="ext" size={12} /></button
              >
            {/if}
            {#if e.failed}
              <span class="fail" role="status" data-env-failed
                >{e.latest?.sha ? t('envs.view.failed', { sha: shortSha(e.latest.sha) }) : t('envs.view.failedNoSha')}</span
              >
              {#if e.latest?.log}
                {@const log = e.latest.log}
                <button class="link" onclick={() => app.run(openUrl(log))}>{t('envs.view.job')}<Icon name="ext" size={12} /></button>
              {/if}
            {:else if e.latest}
              <span class="meta">{t('envs.view.underWay', { sha: shortSha(e.latest.sha) })}</span>
            {/if}
          </div>
          {@render list(l.tickets, false)}
        </section>
      {/each}
    </div>
  {:else}
    <p class="note" class:err={shown === 'error'} role="status">
      {#if shown === 'loading'}{t('envs.view.loading')}
      {:else if shown === 'unsupported'}{t('envs.view.unsupported', { host: data?.host ?? '' })}
      {:else if shown === 'error'}{t('envs.view.error', { error: data?.error ?? '' })}
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
  }
  .bar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px 0 22px;
  }
  .sub {
    margin-right: auto;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--muted);
    font-size: 12px;
  }
  .late {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--wait);
    font-size: 12.5px;
    font-weight: 550;
    white-space: nowrap;
  }
  .note {
    margin: 16px 22px;
    color: var(--muted);
    font-size: 13px;
  }
  .note.err {
    color: var(--del);
  }
  /* One lane per environment, side by side as the board's columns; under 240 pixels each, it scrolls sideways. */
  .lanes {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: repeat(var(--n, 3), minmax(240px, 1fr));
    gap: 14px;
    padding: 16px 16px 16px 18px;
    overflow: auto;
  }
  .lane {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  h2 {
    margin: 0;
    padding: 0 2px;
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--soft);
  }
  .n {
    color: var(--muted);
    font-weight: 500;
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
  /* What runs there: raised, framed in red when its newest deployment failed. */
  .runs {
    display: grid;
    justify-items: start;
    gap: 6px;
    padding: 12px;
    border-radius: 11px;
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 12.5px;
  }
  .runs.bad {
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.45);
  }
  .line {
    min-width: 0;
  }
  .mono {
    font-family: var(--mono, ui-monospace, Consolas, monospace);
    font-size: 12px;
  }
  .meta {
    color: var(--muted);
    font-size: 12px;
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
  .fail {
    padding: 2px 8px;
    border-radius: 6px;
    color: var(--del);
    background: oklch(from var(--del) l c h / 0.1);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.35);
    font-size: 12px;
    font-weight: 550;
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
    font-size: 12px;
    font-weight: 600;
  }
  .title {
    color: var(--soft);
    font-size: 13px;
  }
  .none {
    padding: 0 4px;
    color: var(--muted);
    font-size: 12px;
  }
</style>
