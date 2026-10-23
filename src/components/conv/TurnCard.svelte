<script lang="ts">
  import { accountName, resumeTargets } from '../../lib/accounts';
  import { engineName } from '../../lib/engines';
  import { backTarget, forwardTargets } from '../../lib/handover';
  import { fDur, fTok, fUsd, fWhen, isAbsPath } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { menu } from '../../lib/menu.svelte';
  import { estimatedBy } from '../../lib/spend';
  import { app } from '../../lib/state.svelte';
  import type { FileEdit } from '../../lib/tools';
  import type { Agent, EngineId, TurnItem } from '../../lib/types';
  import { tip } from '../../lib/tip';

  // `edits`: the files this turn edited, recapped once the agent is done. `latest`: the conversation's
  // latest turn (notices may follow it). `marked`: the conversation went from one harness to another, each turn says its own.
  let {
    item,
    agent,
    last,
    latest = last,
    edits = [],
    marked = false,
  }: { item: TurnItem; agent: Agent; last: boolean; latest?: boolean; edits?: FileEdit[]; marked?: boolean } = $props();

  const big = $derived(last && agent.status === 'done' && !item.isError && !item.interrupted);

  const working = $derived(agent.status === 'running' || agent.status === 'waiting');
  const accountById = (id: string) => app.settings.accounts?.find((a) => a.id === id);
  // Stopped by the usage limit (a turn saved before the accounts only has its wait for the reset to tell it): the other
  // accounts it can go on on, with its session. Not once it works again, nor for an earlier turn.
  const targets = $derived(latest && !working && !agent.archived && (item.limited || agent.resumeAt) ? resumeTargets(agent) : []);
  // The first turn after a resume on another account failed there: the sentence replaces the error, which it holds.
  const resumeFailed = $derived(latest && !working && !item.limited && agent.movedFrom ? accountById(agent.account) : undefined);
  const before = $derived(resumeFailed && agent.movedFrom ? accountById(agent.movedFrom) : undefined);

  // Every account of its harness serving the project spent (none to resume on): the project's fallback harnesses (ESC-88).
  const stopped = $derived(latest && !working && !agent.archived && (item.limited || !!agent.resumeAt));
  const engines = $derived(stopped && !targets.length ? forwardTargets(agent) : []);
  // The harness it left, once its quota came back: offered, never taken by itself.
  const back = $derived(latest && !working && !agent.archived ? backTarget(agent) : null);
  const mark = $derived(marked ? t('handover.turnOn', { harness: engineName(item.engine ?? 'claude') }) : '');
  // A harness that tells no cost (Codex): Escouade's estimate, said so.
  const estimated = $derived(estimatedBy(item.engine ?? agent.engine));
  const cost = $derived(estimated ? `≈ ${fUsd(item.cost)} ${t('stats.cost.estimated')}` : fUsd(item.cost));

  // Stopped by a sign-in Claude Code could not refresh (ESC-162): the account it waits for, while it does. Its process started
  // again instead, the turn is a line like any other: it goes on.
  const signIn = $derived(item.signIn && latest && !working && !agent.archived && agent.signIn ? agent.signIn : null);
  const signInAccount = $derived(signIn ? accountById(signIn.account) : undefined);
  const signInName = $derived(signInAccount ? accountName(signInAccount) : (signIn?.account ?? ''));

  function continueOn(engine: EngineId) {
    app.run(api.continueOnEngine(agent.id, engine));
  }

  /** The other harnesses, in a menu under their button. */
  function moreEngines(e: MouseEvent) {
    menu.showAt(
      e.currentTarget as HTMLElement,
      engines.slice(1).map((x) => ({ label: t('handover.continueOn', { harness: engineName(x) }), onClick: () => continueOn(x) })),
    );
  }

  function resumeOn(account: string) {
    app.run(api.resumeOnAccount(agent.id, account));
  }

  /** The other accounts, in a menu under their button. */
  function more(e: MouseEvent) {
    menu.showAt(
      e.currentTarget as HTMLElement,
      targets.slice(1).map((a) => ({ label: t('accounts.resume.on', { account: accountName(a) }), onClick: () => resumeOn(a.id) })),
    );
  }
</script>

{#if signIn}
  <div class="card signin" data-testid="turn-sign-in">
    <div class="title warn">
      <span class="dot" style="width:8px;height:8px;background:var(--wait)"></span>{t('conv.turn.signInTitle')}
    </div>
    <p class="say">{t('conv.turn.signIn', { account: signInName })}</p>
    {#if item.error}<pre class="err">{item.error}</pre>{/if}
    <div class="resume">
      <button class="btn small" onclick={() => (app.modal = { kind: 'account', accountId: signIn.account })}
        >{t('conv.turn.signInAction')}</button
      >
    </div>
  </div>
{:else if item.isError && !item.signIn}
  <div class="card error">
    <div class="title"><span class="dot" style="width:8px;height:8px;background:var(--del)"></span>{t('conv.turn.failed')}</div>
    {#if resumeFailed}
      <!-- The sentence closes with its own period: an error that ends with one does not make two. -->
      <pre class="err">{t('accounts.resume.failed', {
          account: accountName(resumeFailed),
          error: item.error?.trimEnd().replace(/\.+$/, '') || '—',
        })}</pre>
      {#if before?.active}
        <div class="resume">
          <button class="btn small" onclick={() => app.run(api.backToPreviousAccount(agent.id))}
            >{t('accounts.resume.back', { account: accountName(before) })}</button
          >
        </div>
      {/if}
    {:else if item.error}<pre class="err">{item.error}</pre>{/if}
    {#if (latest && agent.resumeAt) || targets.length || engines.length}
      <div class="resume">
        {#if latest && agent.resumeAt}
          <span>{t('conv.turn.resumeAt', { when: fWhen(agent.resumeAt, app.now) })}</span>
          <button class="btn small" onclick={() => app.run(api.cancelResume(agent.id))}>{t('conv.turn.cancelResume')}</button>
        {/if}
        {#if engines.length}
          <span class="pair">
            <button class="btn small" onclick={() => continueOn(engines[0])}
              >{t('handover.continueOn', { harness: engineName(engines[0]) })}</button
            >
            {#if engines.length > 1}
              <button class="btn small" aria-haspopup="menu" aria-label={t('handover.another')} onclick={moreEngines}>▾</button>
            {/if}
          </span>
        {/if}
        {#if targets.length}
          <span class="pair">
            <button class="btn small" onclick={() => resumeOn(targets[0].id)}
              >{t('accounts.resume.on', { account: accountName(targets[0]) })}</button
            >
            {#if targets.length > 1}
              <button class="btn small" aria-haspopup="menu" aria-label={t('accounts.resume.another')} onclick={more}>▾</button>
            {/if}
          </span>
        {/if}
      </div>
    {/if}
    {@render backRow()}
  </div>
{:else if big}
  <div class="card done" data-testid="turn-done">
    <div class="title ok"><span class="check">✓</span>{t('conv.turn.done')}</div>
    <div class="stats mono">
      <span>{fDur(item.durationMs ?? 0)}</span><span>{t('conv.turn.tokens', { tokens: fTok(item.tokens) })}</span><span
        use:tip={estimated ? t('stats.cost.estimatedHint') : undefined}>{cost}</span
      ><span>{t('conv.turn.filesEdited', { count: edits.length })}</span>{#if mark}<span>{mark}</span>{/if}
    </div>
    {#if edits.length}
      <ul class="recap mono" aria-label={t('conv.turn.editedFiles')}>
        {#each edits as e (e.path)}
          <li>
            <button
              class="path link"
              use:tip={t('common.openInEditor')}
              onclick={() =>
                app.openEditor({
                  projectId: agent.projectId,
                  source: agent.worktree ? agent.id : 'project',
                  // A file outside the agent's folder is recapped by its absolute path.
                  abs: isAbsPath(e.path) ? e.path : `${agent.cwd}/${e.path}`,
                })}>{e.path}</button
            ><span class="add">+{e.add}</span><span class="del">−{e.del}</span>
          </li>
        {/each}
      </ul>
    {/if}
    {@render backRow()}
  </div>
{:else}
  <div class="sep mono">
    <span class="line"></span>
    {item.interrupted ? t('conv.turn.interrupted') : fDur(item.durationMs ?? 0)} · {t('conv.turn.tokens', { tokens: fTok(item.tokens) })} · {cost}{mark
      ? ` · ${mark}`
      : ''}
    <span class="line"></span>
  </div>
  {#if back}<div class="back-alone">{@render backRow()}</div>{/if}
{/if}

{#snippet backRow()}
  {#if back}
    <div class="resume">
      <button class="btn small" use:tip={t('handover.backHint', { harness: engineName(back) })} onclick={() => continueOn(back!)}
        >{t('handover.back', { harness: engineName(back) })}</button
      >
    </div>
  {/if}
{/snippet}

<style>
  .back-alone {
    margin: 8px 0 0 34px;
  }
  .card {
    margin-left: 34px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 16px 18px;
    border-radius: var(--r);
    border: 1px solid var(--line2);
    background: var(--panel);
    animation: ccFadeIn 0.2s ease-out;
  }
  .card.error {
    border-color: color-mix(in oklch, var(--del) 55%, transparent);
  }
  .card.signin {
    border-color: color-mix(in oklch, var(--wait) 55%, transparent);
  }
  .title.warn {
    color: var(--wait);
  }
  .say {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text);
  }
  .title {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.02em;
    color: var(--del);
  }
  .title.ok {
    color: var(--ok);
  }
  .check {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--ok);
    color: var(--bg);
    font-size: 9px;
    font-weight: 800;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: 22px;
    font-size: 12px;
    color: var(--muted);
  }
  .recap {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
  }
  .recap li {
    display: flex;
    gap: 10px;
    min-width: 0;
  }
  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    font: inherit;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .link:hover {
    color: var(--accent);
    text-decoration: underline;
  }
  .add {
    color: var(--add);
  }
  .del {
    color: var(--del);
  }
  .resume {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    font-size: 12.5px;
    color: var(--text);
  }
  .pair {
    display: inline-flex;
    gap: 2px;
  }
  .err {
    margin: 0;
    font-family: var(--mono);
    font-size: 12px;
    white-space: pre-wrap;
    color: var(--muted);
    max-height: 200px;
    overflow: auto;
  }
  .sep {
    margin-left: 34px;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 10.5px;
    color: var(--dim);
    white-space: nowrap;
  }
  .line {
    flex: 1;
    height: 1px;
    background: var(--line);
  }
</style>
