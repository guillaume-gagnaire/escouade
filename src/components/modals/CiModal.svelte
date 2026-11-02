<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { sendTargets } from '../../lib/branch-ci';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { menu } from '../../lib/menu.svelte';
  import { POLL_MS, failureTitle, stillRunning } from '../../lib/prview';
  import { app } from '../../lib/state.svelte';
  import type { CiDetail, CiOf, PrCheck, PrFailure } from '../../lib/types';
  import CiChecks from '../CiChecks.svelte';
  import Icon from '../Icon.svelte';
  import Modal from './Modal.svelte';

  // The window of a CI (ESC-63): the jobs of a project's target, of a branch a ticket pushed or of a ticket's pull
  // request, as its host tells them now. A failed one's log opens in reading only, as a terminal shows it; « Envoyer à un
  // agent » gives it to an agent of the project; « Relancer les jobs en échec » runs them again on the host, on this click
  // alone, where the host can. Read when it opens, on « Rafraîchir », and every 20 s while a job runs.
  let { of, projectId }: { of: CiOf; projectId: string } = $props();

  let detail = $state<CiDetail | null>(null);
  let failure = $state<PrFailure | null>(null);
  let seq = 0;
  let gone = false;
  onDestroy(() => (gone = true));

  async function load(quiet = false) {
    const my = ++seq;
    if (!quiet) [detail, failure] = [null, null];
    try {
      const read = await api.ciDetail(of);
      if (my !== seq || gone) return;
      if (read.data || !quiet) detail = read.data ?? null;
      failure = read.failure ?? null;
    } catch (e) {
      if (my === seq && !gone) failure = { kind: 'other', message: String(e) };
    }
  }
  onMount(() => void load());
  $effect(() => {
    if (!detail || !stillRunning(detail.checks)) return;
    const timer = setInterval(() => !document.hidden && void load(true), POLL_MS);
    return () => clearInterval(timer);
  });

  const title = $derived(
    detail?.number != null
      ? t('board.ci.titlePr', { word: detail.word, number: detail.number })
      : t('board.ci.title', { branch: detail?.branch ?? '…' }),
  );
  const host = $derived(detail?.hostName ?? '');

  /** « Rafraîchir »: the jobs, and the pills of the project (its target, its pushed branches), read now. */
  async function refresh() {
    void load();
    await app.run(api.ciRefresh(projectId));
  }

  async function rerun() {
    const n = await app.run(api.ciRerun(of));
    if (n === undefined) return;
    app.toast(n ? t('board.ci.rerunDone', { count: n }) : t('board.ci.rerunNone', { host }), n ? 'ok' : 'info');
    void load(true);
  }

  /** « Envoyer à un agent »: the agents of the project, the most recent first. */
  function send(c: PrCheck, el: HTMLElement) {
    const agents = sendTargets(Object.values(app.agents), projectId);
    if (!agents.length) {
      app.toast(t('board.ci.sendNone'), 'info');
      return;
    }
    menu.showAt(
      el,
      agents.map((a) => ({
        label: a.name,
        onClick: async () => {
          const done = await app.run(api.ciLogToAgent(of, c.job, c.name, c.url, a.id));
          if (done !== undefined) app.toast(t('board.ci.sent', { name: c.name, agent: a.name }), 'ok');
        },
      })),
    );
  }
</script>

<Modal {title} width={760} onclose={() => (app.modal = null)}>
  {#snippet lead()}<span class="lead"><Icon name="check" /></span>{/snippet}
  {#snippet badge()}
    <button class="btn refresh" onclick={refresh}><Icon name="restart" size={13} />{t('board.ci.refresh')}</button>
  {/snippet}
  <div class="ci-win" data-ci={of.kind}>
    {#if failure}
      <div class="state err" role="alert">
        <Icon name="info" size={14} />
        <span class="why"><b>{failureTitle(failure)}</b> {failure.message}</span>
      </div>
    {/if}
    {#if !detail && !failure}
      <p class="state" role="status">{t('board.ci.loading')}</p>
    {:else if detail}
      {@const d = detail}
      <CiChecks
        checks={d.checks}
        jobLogs={d.can.jobLogs}
        canRerun={d.can.rerun}
        loadLog={(job) => api.ciJobLog(of, job)}
        onRerun={rerun}
        onSend={send}
        titleId="ci-win-jobs"
        title={t('prs.view.ci.title')}
        rerunLabel={t('board.ci.rerun')}
        rerunningLabel={t('board.ci.rerunning')}
        rerunHint={t('board.ci.rerunHint', { host })}
        onHost={t('board.ci.onHost', { host })}
        empty={t('board.ci.empty', { branch: d.branch })}
        tall
      />
      {#if !d.can.rerun && d.checks.some((c) => c.state === 'failure')}
        <p class="note">{t('board.ci.noRerun', { host })}</p>
      {/if}
    {/if}
  </div>
</Modal>

<style>
  .lead {
    display: inline-grid;
    place-items: center;
    color: var(--p-ink);
  }
  .refresh {
    height: 26px;
    padding: 0 9px;
    font-size: 12px;
  }
  .ci-win {
    min-height: 120px;
    padding-bottom: 8px;
  }
  .state {
    margin: 12px 14px 0;
    color: var(--muted);
    font-size: 12.5px;
  }
  .state.err {
    display: flex;
    gap: 9px;
    padding: 10px 12px;
    border-radius: 9px;
    background: color-mix(in oklch, var(--del) 8%, transparent);
    color: var(--del);
  }
  .why {
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .why b {
    color: var(--text);
  }
  .note {
    margin: 10px 14px 0;
    color: var(--dim);
    font-size: 11.5px;
  }
</style>
