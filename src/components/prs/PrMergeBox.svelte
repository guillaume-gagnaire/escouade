<script lang="ts">
  import { untrack } from 'svelte';
  import { t } from '../../lib/i18n';
  import {
    mergeable,
    methodsOf,
    onlyWaiting,
    type MergeAsk,
    type MergeStrategy,
    type PrControlRead,
    type Row,
    type RowAction,
  } from '../../lib/prcontrol';
  import type { PullRequest } from '../../lib/types';
  import Icon from '../Icon.svelte';

  // The merge box « prête à merger ? » of a pull request's view (ESC-488, PR-8; direction A of design/pr-2-3, at the foot
  // of the right column): what is missing, each line with the gesture that unblocks it, the merge method (those the
  // repository allows, the Kanban's first), the branch and the worktree deleted after, the host's auto-merge, and the
  // merge itself after a confirmation that says the method, the target and what goes. Nothing leaves without a click.
  let {
    pr,
    rows,
    read,
    host,
    word,
    busy = null,
    failure = null,
    done = null,
    note = null,
    holder = null,
    ticketKey = null,
    onAction,
    onMerge,
    onAutoMerge,
    onCancelAutoMerge,
  }: {
    pr: PullRequest;
    rows: Row[];
    read: PrControlRead | null;
    host: string;
    word: string;
    /** The gesture on its way (`merge`, `autoMerge`, a row's key). */
    busy?: string | null;
    failure?: string | null;
    /** What the last merge did, in sentences. */
    done?: string[] | null;
    /** A word after a gesture (the ticket's agent updates, auto-merge off). */
    note?: string | null;
    /** The agent whose worktree would go with the merge. */
    holder?: string | null;
    /** The ticket that moves to « Terminé ». */
    ticketKey?: string | null;
    onAction: (a: RowAction, key: Row['key']) => void;
    onMerge: (ask: MergeAsk) => void;
    onAutoMerge: (strategy: MergeStrategy) => void;
    onCancelAutoMerge: () => void;
  } = $props();

  const methods = $derived(methodsOf(read));
  let method = $state<MergeStrategy | null>(null);
  const chosen = $derived(method && methods.includes(method) ? method : (methods[0] ?? null));
  let menu = $state(false);
  let confirming = $state(false);
  const w = $derived(read?.writing);
  const c = $derived(read?.control);
  const open = $derived(pr.state === 'open');
  const ready = $derived(open && mergeable(rows));
  const autoOn = $derived(pr.autoMerge || !!c?.autoMerge);
  const canDeleteBranch = $derived(!!w?.deleteBranch && !pr.fork);
  let deleteBranch = $state(true);
  let deleteWorktree = $state(true);
  // GitHub deletes it itself: nothing to ask.
  $effect(() => {
    if (c?.deletesBranch) untrack(() => (deleteBranch = false));
  });

  function ask() {
    if (!chosen) return;
    onMerge({ strategy: chosen, deleteBranch: canDeleteBranch && deleteBranch, deleteWorktree: !!holder && deleteWorktree });
    confirming = false;
  }

  const methodName = (m: MergeStrategy) => t(`prs.page.control.method.${m}`);
  const methodSub = (m: MergeStrategy) => t(`prs.page.control.methodSub.${m}`, { base: pr.base });
</script>

<section class="mbox" aria-label={t('prs.page.control.box.title')} data-ready={ready}>
  <h3>
    {#if pr.state === 'merged'}<Icon name="merge" size={13} />{t('prs.page.control.box.merged')}
    {:else if pr.state === 'closed'}{t('prs.page.control.box.closed')}
    {:else if ready}<span class="ic ok"><Icon name="check" size={12} /></span>{t('prs.page.control.box.ready')}
    {:else}{t('prs.page.control.box.title')}{/if}
  </h3>

  {#if failure}<p class="fail" role="alert"><Icon name="info" size={13} />{failure}</p>{/if}
  {#if done}
    <div class="done" role="status">
      {#each done as line (line)}<p>{line}</p>{/each}
    </div>
  {/if}
  {#if note}<p class="note" role="status">{note}</p>{/if}

  {#if open}
    <ul class="rows">
      {#each rows as r (r.key)}
        <li class="mr" data-row={r.key} data-tone={r.tone}>
          <span class="ic {r.tone}" aria-hidden="true"
            >{#if r.tone === 'wait'}<span class="spin"></span>{:else}<Icon
                name={r.tone === 'ok' ? 'check' : r.tone === 'ko' ? 'close' : 'info'}
                size={12}
              />{/if}</span
          >
          <span class="tx">
            <b>{r.title}</b>
            {#if r.sub}<small>{r.sub}</small>{/if}
            {#if r.elsewhere}<small class="else">{r.elsewhere}</small>{/if}
          </span>
          {#if r.action && r.label}
            <button class="btn small" class:ko={r.tone === 'ko'} disabled={busy !== null} onclick={() => onAction(r.action!, r.key)}
              >{#if busy === r.key}<span class="spinner" aria-hidden="true"></span>{/if}{r.label}</button
            >
          {/if}
        </li>
      {/each}
    </ul>

    {#if autoOn}
      <div class="auto" role="status">
        <Icon name="clock" size={13} /><span
          >{t('prs.page.control.box.autoMergeOn', {
            method: methodName(c?.autoMerge ?? chosen ?? 'squash'),
            host,
          })}</span
        >
        {#if w?.cancelAutoMerge}
          <button class="btn small" disabled={busy !== null} onclick={onCancelAutoMerge}>{t('prs.page.control.box.autoMergeOff')}</button>
        {/if}
      </div>
    {/if}

    {#if !methods.length}
      <p class="muted">{t('prs.page.control.box.noMerge', { host, word })}</p>
    {:else if confirming && chosen}
      <div class="confirm" role="alertdialog" aria-label={t('prs.page.control.confirm.title', { word, number: pr.number })}>
        <b>{t('prs.page.control.confirm.title', { word, number: pr.number })}</b>
        <ul>
          <li>{t('prs.page.control.confirm.method', { method: methodName(chosen), base: pr.base, sub: methodSub(chosen) })}</li>
          {#if canDeleteBranch && deleteBranch}<li>{t('prs.page.control.confirm.branch', { branch: pr.head, host })}</li>{/if}
          {#if holder && deleteWorktree}<li>{t('prs.page.control.confirm.worktree', { agent: holder })}</li>{/if}
          {#if !(canDeleteBranch && deleteBranch) && !(holder && deleteWorktree)}<li>{t('prs.page.control.confirm.nothingElse')}</li>{/if}
          {#if ticketKey}<li>{t('prs.page.control.confirm.ticket', { key: ticketKey })}</li>{/if}
        </ul>
        <div class="acts">
          <button class="btn" onclick={() => (confirming = false)}>{t('prs.page.control.confirm.cancel')}</button>
          <!-- svelte-ignore a11y_autofocus -->
          <button class="btn primary" autofocus onclick={ask}><Icon name="merge" size={13} />{t('prs.page.control.confirm.go')}</button>
        </div>
      </div>
    {:else if chosen}
      <div class="mf">
        <span class="split">
          <button class="btn go" class:primary={ready} disabled={!ready || busy !== null} onclick={() => (confirming = true)}
            >{#if busy === 'merge'}<span class="spinner" aria-hidden="true"></span>{:else}<Icon name="merge" size={13} />{/if}{methodName(
              chosen,
            )}</button
          ><button
            class="btn icon"
            class:primary={ready}
            aria-label={t('prs.page.control.box.chooseMethod')}
            aria-expanded={menu}
            aria-haspopup="menu"
            disabled={busy !== null}
            onclick={() => (menu = !menu)}><Icon name="down" size={12} /></button
          >
        </span>
        {#if menu}
          <div class="menu" role="menu" aria-label={t('prs.page.control.box.method')}>
            {#each methods as m, i (m)}
              <button
                role="menuitemradio"
                aria-checked={m === chosen}
                class="it"
                onclick={() => {
                  method = m;
                  menu = false;
                }}
              >
                <span class="radio" class:on={m === chosen}></span>
                <span
                  ><b>{methodName(m)}</b>{#if i === 0 && read?.preferred === m}<em> · {t('prs.page.control.box.preferred')}</em>{/if}<small
                    >{methodSub(m)}</small
                  ></span
                >
              </button>
            {/each}
          </div>
        {/if}
        {#if canDeleteBranch && !c?.deletesBranch}
          <label class="opt"
            ><input type="checkbox" bind:checked={deleteBranch} />{t('prs.page.control.box.deleteBranch', { branch: pr.head })}</label
          >
        {:else if c?.deletesBranch}
          <p class="muted">{t('prs.page.control.box.deleteBranchAuto', { host })}</p>
        {:else if pr.fork}
          <p class="muted">{t('prs.page.control.box.deleteBranchFork')}</p>
        {/if}
        {#if holder}
          <label class="opt"
            ><input type="checkbox" bind:checked={deleteWorktree} />{t('prs.page.control.box.deleteWorktree', { agent: holder })}</label
          >
        {/if}
        {#if ticketKey}<p class="muted">{t('prs.page.control.box.ticketDone', { key: ticketKey })}</p>{/if}
        {#if !ready && !autoOn && onlyWaiting(rows)}
          {#if read?.options.autoMerge}
            <button class="btn small auto-on" disabled={busy !== null} onclick={() => chosen && onAutoMerge(chosen)}
              >{#if busy === 'autoMerge'}<span class="spinner" aria-hidden="true"></span>{:else}<Icon name="clock" size={12} />{/if}{t(
                'prs.page.control.box.autoMerge',
              )}</button
            >
          {:else}
            <p class="muted">{t('prs.page.control.box.noAutoMerge', { host })}</p>
          {/if}
        {/if}
      </div>
    {/if}
    <p class="clicks">{t('prs.page.control.box.clicks', { host })}</p>
  {/if}
</section>

<style>
  .mbox {
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--well);
    padding: 10px;
    display: grid;
    gap: 8px;
    font-size: 12.5px;
  }
  .mbox[data-ready='true'] {
    box-shadow: inset 0 0 0 1px oklch(from var(--run) l c h / 0.4);
  }
  h3 {
    margin: 0;
    font-size: 12.5px;
    font-weight: 600;
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }
  .mr {
    display: grid;
    grid-template-columns: 22px minmax(0, 1fr);
    column-gap: 8px;
    row-gap: 5px;
    align-items: center;
    padding: 5px 2px;
    border-top: 1px solid var(--line);
  }
  .mr:first-child {
    border-top: 0;
  }
  .ic {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--muted);
    background: oklch(from var(--muted) l c h / 0.12);
  }
  .ic.ok {
    color: var(--run);
    background: oklch(from var(--run) l c h / 0.14);
  }
  .ic.ko {
    color: var(--del);
    background: oklch(from var(--del) l c h / 0.14);
  }
  .ic.wait {
    color: var(--wait);
    background: oklch(from var(--wait) l c h / 0.14);
  }
  .spin {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 2px solid currentColor;
    border-right-color: transparent;
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .tx {
    display: grid;
    min-width: 0;
  }
  .tx b {
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .tx small {
    color: var(--muted);
    font-size: 11.5px;
    overflow-wrap: anywhere;
  }
  .tx small.else {
    color: var(--dim);
  }
  .mr > .btn {
    grid-column: 2;
    justify-self: start;
  }
  .btn.small {
    height: 24px;
    font-size: 12px;
    padding: 0 8px;
  }
  .btn.ko {
    color: var(--del);
  }
  .mf {
    display: grid;
    gap: 6px;
    position: relative;
  }
  .split {
    display: flex;
  }
  .split .go {
    flex: 1;
    justify-content: center;
    border-top-right-radius: 0;
    border-bottom-right-radius: 0;
  }
  .split .icon {
    border-top-left-radius: 0;
    border-bottom-left-radius: 0;
    margin-left: -1px;
    width: 30px;
    justify-content: center;
  }
  .menu {
    position: absolute;
    top: 34px;
    left: 0;
    right: 0;
    z-index: 5;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 8px;
    box-shadow: 0 8px 28px oklch(0 0 0 / 0.22);
    padding: 4px;
    display: grid;
  }
  .it {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    text-align: left;
    padding: 6px 8px;
    border-radius: 6px;
    background: none;
    border: 0;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .it:hover,
  .it:focus-visible {
    background: var(--hover);
  }
  .it span:last-child {
    display: grid;
  }
  .it small {
    color: var(--muted);
    font-size: 11.5px;
  }
  .it em {
    font-style: normal;
    color: var(--muted);
    font-weight: 400;
  }
  .radio {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 1.5px solid var(--muted);
    margin-top: 3px;
    flex: none;
  }
  .radio.on {
    border-color: var(--primary);
    box-shadow: inset 0 0 0 2.5px var(--panel);
    background: var(--primary);
  }
  .opt {
    display: flex;
    gap: 7px;
    align-items: center;
    color: var(--soft);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  .auto {
    display: flex;
    gap: 7px;
    align-items: center;
    flex-wrap: wrap;
    color: var(--soft);
    font-size: 12px;
  }
  .auto span {
    flex: 1 1 140px;
  }
  .auto-on {
    justify-self: start;
  }
  .confirm {
    border: 1px solid oklch(from var(--primary) l c h / 0.45);
    border-radius: 8px;
    padding: 9px 10px;
    display: grid;
    gap: 6px;
    background: var(--panel);
  }
  .confirm ul {
    margin: 0;
    padding-left: 16px;
    display: grid;
    gap: 3px;
    color: var(--soft);
    overflow-wrap: anywhere;
  }
  .acts {
    display: flex;
    gap: 6px;
    justify-content: flex-end;
  }
  .fail {
    margin: 0;
    display: flex;
    gap: 6px;
    align-items: flex-start;
    color: var(--del);
    background: oklch(from var(--del) l c h / 0.07);
    border-radius: 6px;
    padding: 6px 8px;
    overflow-wrap: anywhere;
  }
  .done,
  .note {
    margin: 0;
    color: var(--soft);
    background: oklch(from var(--run) l c h / 0.08);
    border-radius: 6px;
    padding: 6px 8px;
    overflow-wrap: anywhere;
  }
  .done p {
    margin: 0;
  }
  .muted {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .clicks {
    margin: 0;
    color: var(--dim);
    font-size: 11px;
  }
</style>
