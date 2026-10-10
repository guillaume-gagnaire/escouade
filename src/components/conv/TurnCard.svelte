<script lang="ts">
  import { accountName, resumeTargets } from '../../lib/accounts';
  import { fDur, fTok, fUsd, fWhen, isAbsPath } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { menu } from '../../lib/menu.svelte';
  import { app } from '../../lib/state.svelte';
  import type { FileEdit } from '../../lib/tools';
  import type { Agent, TurnItem } from '../../lib/types';

  // `edits`: the files this turn edited, recapped once the agent is done. `latest`: the conversation's
  // latest turn (notices may follow it).
  let {
    item,
    agent,
    last,
    latest = last,
    edits = [],
  }: { item: TurnItem; agent: Agent; last: boolean; latest?: boolean; edits?: FileEdit[] } = $props();

  const big = $derived(last && agent.status === 'done' && !item.isError && !item.interrupted);

  const working = $derived(agent.status === 'running' || agent.status === 'waiting');
  const accountById = (id: string) => app.settings.accounts?.find((a) => a.id === id);
  // Stopped by the usage limit (a turn saved before the accounts only has its wait for the reset to tell it): the other
  // accounts it can go on on, with its session. Not once it works again, nor for an earlier turn.
  const targets = $derived(latest && !working && !agent.archived && (item.limited || agent.resumeAt) ? resumeTargets(agent) : []);
  // The first turn after a resume on another account failed there: the sentence replaces the error, which it holds.
  const resumeFailed = $derived(latest && !working && !item.limited && agent.movedFrom ? accountById(agent.account) : undefined);
  const before = $derived(resumeFailed && agent.movedFrom ? accountById(agent.movedFrom) : undefined);

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

{#if item.isError}
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
    {#if (latest && agent.resumeAt) || targets.length}
      <div class="resume">
        {#if latest && agent.resumeAt}
          <span>{t('conv.turn.resumeAt', { when: fWhen(agent.resumeAt, app.now) })}</span>
          <button class="btn small" onclick={() => app.run(api.cancelResume(agent.id))}>{t('conv.turn.cancelResume')}</button>
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
  </div>
{:else if big}
  <div class="card done" data-testid="turn-done">
    <div class="title ok"><span class="check">✓</span>{t('conv.turn.done')}</div>
    <div class="stats mono">
      <span>{fDur(item.durationMs ?? 0)}</span><span>{t('conv.turn.tokens', { tokens: fTok(item.tokens) })}</span><span
        >{fUsd(item.cost)}</span
      ><span>{t('conv.turn.filesEdited', { count: edits.length })}</span>
    </div>
    {#if edits.length}
      <ul class="recap mono" aria-label={t('conv.turn.editedFiles')}>
        {#each edits as e (e.path)}
          <li>
            <button
              class="path link"
              title={t('common.openInEditor')}
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
  </div>
{:else}
  <div class="sep mono">
    <span class="line"></span>
    {item.interrupted ? t('conv.turn.interrupted') : fDur(item.durationMs ?? 0)} · {t('conv.turn.tokens', { tokens: fTok(item.tokens) })} · {fUsd(
      item.cost,
    )}
    <span class="line"></span>
  </div>
{/if}

<style>
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
