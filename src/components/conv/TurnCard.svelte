<script lang="ts">
  import { accountName, resumeTargets } from '../../lib/accounts';
  import { engineName, showsCost } from '../../lib/engines';
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
  import Icon from '../Icon.svelte';

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
  const cost = $derived(estimated ? `≈ ${fUsd(item.cost ?? 0)} ${t('stats.cost.estimated')}` : fUsd(item.cost ?? 0));
  // A harness that tells neither tokens nor cost (Cursor CLI): neither is shown.
  const counted = $derived(showsCost({ engine: item.engine ?? agent.engine }));

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
  <!-- Stopped by a sign-in Claude Code could not refresh (ESC-162): an amber card, as the quota's. -->
  <div class="card limit" data-testid="turn-sign-in">
    <div class="lt"><Icon name="user" size={14} />{t('conv.turn.signInTitle')}</div>
    <p class="say">{t('conv.turn.signIn', { account: signInName })}</p>
    {#if item.error}<pre class="err">{item.error}</pre>{/if}
    <div class="row-btns">
      <button class="btn primary" onclick={() => (app.modal = { kind: 'account', accountId: signIn.account })}
        >{t('conv.turn.signInAction')}</button
      >
    </div>
  </div>
{:else if item.isError && !item.signIn}
  <!-- Stopped by the quota (an amber card, with where it goes on), or failed (a red one). -->
  <div class="card error" class:limit={stopped}>
    <div class="lt">
      <Icon name={stopped ? 'clock' : 'close'} size={14} />{item.refused ? t('conv.turn.refused') : t('conv.turn.failed')}
    </div>
    <!-- Refused by a safeguard (ESC-416): left out of the agent's history, or how to leave it out in its own interface. -->
    {#if item.refused}
      <p class="say" data-testid="turn-refused">{item.cut ? t('conv.turn.refusedCut') : t('conv.turn.refusedView')}</p>
    {/if}
    {#if resumeFailed}
      <!-- The sentence closes with its own period: an error that ends with one does not make two. -->
      <pre class="err">{t('accounts.resume.failed', {
          account: accountName(resumeFailed),
          error: item.error?.trimEnd().replace(/\.+$/, '') || '—',
        })}</pre>
      {#if before?.active}
        <div class="row-btns">
          <button class="btn" onclick={() => app.run(api.backToPreviousAccount(agent.id))}
            >{t('accounts.resume.back', { account: accountName(before) })}</button
          >
        </div>
      {/if}
    {:else if item.error}<pre class="err">{item.error}</pre>{/if}
    {#if (latest && agent.resumeAt) || targets.length || engines.length}
      <div class="row-btns">
        {#if latest && agent.resumeAt}
          <span class="when">{t('conv.turn.resumeAt', { when: fWhen(agent.resumeAt, app.now) })}</span>
          <button class="btn" onclick={() => app.run(api.cancelResume(agent.id))}>{t('conv.turn.cancelResume')}</button>
        {/if}
        {#if engines.length}
          <span class="pair">
            <button class="btn primary" onclick={() => continueOn(engines[0])}
              >{t('handover.continueOn', { harness: engineName(engines[0]) })}</button
            >
            {#if engines.length > 1}
              <button class="btn primary icon" aria-haspopup="menu" aria-label={t('handover.another')} onclick={moreEngines}
                ><Icon name="down" size={14} /></button
              >
            {/if}
          </span>
        {/if}
        {#if targets.length}
          <span class="pair">
            <button class="btn" class:primary={!engines.length} onclick={() => resumeOn(targets[0].id)}
              >{t('accounts.resume.on', { account: accountName(targets[0]) })}</button
            >
            {#if targets.length > 1}
              <button
                class="btn icon"
                class:primary={!engines.length}
                aria-haspopup="menu"
                aria-label={t('accounts.resume.another')}
                onclick={more}><Icon name="down" size={14} /></button
              >
            {/if}
          </span>
        {/if}
      </div>
    {/if}
    {@render backRow()}
  </div>
{:else if big}
  <div class="card done" data-testid="turn-done">
    <div class="lt ok"><Icon name="check-c" size={14} />{t('conv.turn.done')}</div>
    <div class="stats">
      <span>{fDur(item.durationMs ?? 0)}</span>{#if counted}<span>{t('conv.turn.tokens', { tokens: fTok(item.tokens ?? 0) })}</span><span
          use:tip={estimated ? t('stats.cost.estimatedHint') : undefined}>{cost}</span
        >{/if}<span>{t('conv.turn.filesEdited', { count: edits.length })}</span>{#if mark}<span>{mark}</span>{/if}
    </div>
    {#if edits.length}
      <ul class="recap" aria-label={t('conv.turn.editedFiles')}>
        {#each edits as e (e.path)}
          <li>
            <Icon name="edit" size={14} /><button
              class="path link mono"
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
  <div class="sep">
    <span class="sep-text"
      >{item.interrupted ? t('conv.turn.interrupted') : fDur(item.durationMs ?? 0)}{#if counted}{' · '}{t('conv.turn.tokens', {
          tokens: fTok(item.tokens ?? 0),
        })} · {cost}{/if}{mark ? ` · ${mark}` : ''}</span
    >
  </div>
  {#if back}<div class="back-alone">{@render backRow()}</div>{/if}
{/if}

{#snippet backRow()}
  {#if back}
    <div class="row-btns">
      <button class="btn" use:tip={t('handover.backHint', { harness: engineName(back) })} onclick={() => continueOn(back!)}
        ><Icon name="swap" size={14} />{t('handover.back', { harness: engineName(back) })}</button
      >
    </div>
  {/if}
{/snippet}

<style>
  /* A turn's end: a quiet line, or a card for what asks for a look (done, stopped, failed). */
  .card {
    display: grid;
    gap: 10px;
    margin: 4px 0;
    padding: 13px 14px;
    border-radius: 11px;
    box-shadow: inset 0 0 0 1px var(--line);
    animation: ccFadeIn 0.2s ease-out;
  }
  .card.error {
    background: oklch(from var(--del) l c h / 0.06);
    box-shadow: inset 0 0 0 1px oklch(from var(--del) l c h / 0.35);
  }
  /* The quota spent, the sign-in lost: amber, as what waits for the user. */
  .card.limit {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .lt {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
  }
  .lt :global(.icon) {
    color: var(--del);
  }
  .limit .lt :global(.icon) {
    color: var(--wait);
  }
  .lt.ok :global(.icon) {
    color: var(--add);
  }
  .say {
    margin: 0;
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
    color: var(--muted);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .recap {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 2px;
    font-size: 12px;
  }
  .recap li {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--soft);
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    font: inherit;
    font-family: var(--mono);
    text-align: left;
    cursor: pointer;
  }
  .link:hover {
    color: var(--p-ink);
    text-decoration: underline;
  }
  .add {
    color: var(--add);
  }
  .del {
    color: var(--del);
  }
  .row-btns {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
  }
  .when {
    margin-right: 6px;
    color: var(--soft);
  }
  .pair {
    display: inline-flex;
    gap: 2px;
  }
  .btn.icon {
    width: 30px;
    padding: 0;
    justify-content: center;
  }
  .err {
    margin: 0;
    font-family: var(--mono);
    font-size: 12px;
    white-space: pre-wrap;
    color: var(--soft);
    max-height: 200px;
    overflow: auto;
  }
  .sep {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--muted);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .sep::before,
  .sep::after {
    content: '';
    flex: 1;
    height: 1px;
    background: var(--line);
  }
  .back-alone {
    margin-top: 8px;
  }
  @media (prefers-reduced-motion: reduce) {
    .card {
      animation: none;
    }
  }
</style>
