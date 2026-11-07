<script lang="ts">
  import { app } from '../lib/state.svelte';
  import { t } from '../lib/i18n';
  import { openOutside } from '../lib/links';
  import { checkDuration, checkPhase, ciSummary, failureTitle, hasFailed, phaseIcon, reallyFailed, sortChecks } from '../lib/prview';
  import { tip } from '../lib/tip';
  import type { PrCheck, PrRead } from '../lib/types';
  import CiLog from './CiLog.svelte';
  import Icon from './Icon.svelte';

  // A CI's checks (ESC-305, shared with ESC-63): each with its state and its time, the log of a failed one on a click (in
  // reading only, as a terminal shows it), « Relancer les jobs en échec » on a click when the host can, and, where the
  // caller allows it, « Envoyer à un agent » beside a failed one. The « PR et CI » tab and the window of a CI draw it.
  let {
    checks: given,
    jobLogs,
    canRerun,
    loadLog,
    onRerun,
    onSend,
    titleId,
    title = t('prs.view.ci.title'),
    rerunLabel = t('prs.view.ci.rerun'),
    rerunningLabel = t('prs.view.ci.rerunning'),
    rerunHint = t('prs.view.ci.rerunHint'),
    onHost = t('prs.view.ci.onHost'),
    empty = t('prs.view.ci.none'),
    tall = false,
  }: {
    checks: PrCheck[];
    /** The host keeps its jobs' logs. */
    jobLogs: boolean;
    /** « Relancer les jobs en échec » is offered (the host can, and what it runs on is still open). */
    canRerun: boolean;
    loadLog: (job: string) => Promise<PrRead<string>>;
    onRerun: () => Promise<unknown>;
    /** « Envoyer à un agent » beside a failed check, when given: the check and the button clicked. */
    onSend?: (c: PrCheck, el: HTMLElement) => void;
    titleId: string;
    title?: string;
    rerunLabel?: string;
    rerunningLabel?: string;
    rerunHint?: string;
    onHost?: string;
    empty?: string;
    /** In a window: a taller log. */
    tall?: boolean;
  } = $props();

  const checks = $derived(sortChecks(given));

  // The log of a failed check, by its job.
  let openLog = $state<string | null>(null);
  let logs = $state<Record<string, PrRead<string> | 'loading'>>({});
  async function toggleLog(c: PrCheck) {
    const job = c.job;
    if (!job) return;
    if (openLog === job) {
      openLog = null;
      return;
    }
    openLog = job;
    if (logs[job] && logs[job] !== 'loading' && !(logs[job] as PrRead<string>).failure) return;
    logs[job] = 'loading';
    try {
      logs[job] = await loadLog(job);
    } catch (e) {
      logs[job] = { failure: { kind: 'other', message: String(e) } };
    }
  }

  let rerunning = $state(false);
  async function rerun() {
    rerunning = true;
    try {
      await onRerun();
    } finally {
      rerunning = false;
    }
  }
</script>

<section class="sec" aria-labelledby={titleId}>
  <div class="sec-h">
    <h3 id={titleId}>{title}</h3>
    {#if checks.length}<span class="sum" class:bad={reallyFailed(checks)}>{ciSummary(checks)}</span>{/if}
    {#if canRerun && hasFailed(checks)}
      <button class="btn rerun" disabled={rerunning} use:tip={rerunHint} onclick={rerun}
        ><Icon name="restart" size={13} />{rerunning ? rerunningLabel : rerunLabel}</button
      >
    {/if}
  </div>
  {#if !checks.length}
    <p class="none">{empty}</p>
  {:else}
    <ul class="list" aria-label={title}>
      {#each checks as c, i (`${c.name}|${c.job ?? i}`)}
        {@const phase = checkPhase(c)}
        {@const dur = checkDuration(c, app.now)}
        {@const withLog = phase === 'failure' && !!c.job && jobLogs}
        <li class="check {phase}" data-check={c.name}>
          <span class="ic {phase}"><Icon name={phaseIcon[phase]} size={13} /></span>
          {#if withLog}
            <button
              class="cn as-btn"
              aria-expanded={openLog === c.job}
              use:tip={{ text: t('prs.view.ci.logShow'), aria: false }}
              onclick={() => toggleLog(c)}>{c.name}</button
            >
          {:else}
            <span class="cn">{c.name}</span>
          {/if}
          {#if phase === 'failure' && !c.job}<span class="third" use:tip={t('board.ci.noLog')}>{t('board.ci.noLog')}</span>{/if}
          <span class="cs">{t(`prs.view.ci.phase.${phase}`)}</span>
          {#if dur}<span class="cd mono">{dur}</span>{/if}
          {#if onSend && phase === 'failure'}
            {@const send = onSend}
            <button
              class="icon-btn send"
              aria-label={`${c.name} · ${t('board.ci.send')}`}
              use:tip={{ text: t('board.ci.sendHint'), aria: false }}
              onclick={(e) => send(c, e.currentTarget as HTMLElement)}><Icon name="agents" size={12} /></button
            >
          {/if}
          {#if c.url}
            {@const url = c.url}
            <button
              class="icon-btn ext"
              aria-label={`${c.name} · ${onHost}`}
              use:tip={{ text: onHost, aria: false }}
              onclick={() => openOutside(url)}><Icon name="ext" size={12} /></button
            >
          {/if}
        </li>
        {#if withLog && openLog === c.job}
          {@const log = logs[c.job ?? '']}
          <li class="log" aria-label={t('prs.view.ci.log', { name: c.name })}>
            {#if !log || log === 'loading'}
              <p class="state" role="status">{t('prs.view.ci.logLoading')}</p>
            {:else if log.failure}
              <div class="state err" role="alert">
                <Icon name="info" size={14} />
                <span class="why"><b>{failureTitle(log.failure)}</b> {log.failure.message}</span>
                <button
                  class="btn"
                  onclick={() => {
                    openLog = null;
                    return toggleLog(c);
                  }}>{t('prs.view.retry')}</button
                >
              </div>
            {:else if log.data?.trim()}
              <span class="log-h">{t('prs.view.ci.logTail')}</span>
              <CiLog text={log.data} {tall} />
            {:else}
              <p class="none">{t('prs.view.ci.logNone')}</p>
            {/if}
          </li>
        {/if}
      {/each}
    </ul>
  {/if}
</section>

<style>
  .sec {
    padding: 10px 8px 0;
  }
  .sec-h {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 28px;
    padding: 0 6px 4px;
  }
  .sec-h h3 {
    margin: 0;
    color: var(--soft);
    font-size: 12px;
    font-weight: 600;
  }
  .sum {
    color: var(--muted);
    font-size: 11.5px;
  }
  .sum.bad {
    color: var(--del);
  }
  .rerun {
    height: 26px;
    margin-left: auto;
    padding: 0 9px;
    font-size: 12px;
  }
  .list {
    display: grid;
    gap: 1px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    height: 30px;
    padding: 0 6px 0 8px;
    border-radius: 7px;
  }
  .check:hover {
    background: var(--hover);
  }
  .ic {
    display: inline-grid;
    place-items: center;
    flex: none;
    color: var(--muted);
  }
  .ic.success {
    color: var(--run);
  }
  .ic.failure {
    color: var(--del);
  }
  .ic.running,
  .ic.queued {
    color: var(--wait);
  }
  .cn {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    color: var(--text);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .as-btn {
    padding: 0;
    border: none;
    background: none;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .as-btn:hover {
    text-decoration: underline;
  }
  .third {
    flex: none;
    max-width: 40%;
    overflow: hidden;
    color: var(--dim);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cs {
    flex: none;
    color: var(--muted);
    font-size: 11.5px;
  }
  .check.failure .cs {
    color: var(--del);
  }
  .cd {
    flex: none;
    color: var(--dim);
    font-family: var(--mono);
    font-size: 11px;
  }
  .ext,
  .send {
    width: 22px;
    height: 22px;
    border-radius: 5px;
  }
  .log {
    display: grid;
    gap: 4px;
    margin: 0 0 6px 29px;
  }
  .log-h {
    color: var(--dim);
    font-size: 11px;
  }
  .none {
    margin: 4px 8px 0;
    color: var(--dim);
    font-size: 11.5px;
  }
  .state {
    margin: 6px 0 0;
    color: var(--muted);
    font-size: 12.5px;
  }
  .state.err {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 8px 10px;
    border-radius: 9px;
    background: color-mix(in oklch, var(--del) 8%, transparent);
    color: var(--del);
  }
  .why {
    flex: 1;
    min-width: 0;
    color: var(--muted);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  .why b {
    color: var(--text);
  }
</style>
