<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { deletable, deleteBranch, startAgentOn, switchBranch, worktreeReason } from '../../lib/branch-actions';
  import { rankFiles } from '../../lib/editor/quick-open';
  import { trapFocus } from '../../lib/focus';
  import { gitSync, syncInfo, type SyncOp } from '../../lib/git-sync.svelte';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { menu, type MenuItem } from '../../lib/menu.svelte';
  import { app } from '../../lib/state.svelte';
  import type { BranchInfo } from '../../lib/types';

  // The picker the status bar's branch button opens, above it: the branches of the project's repository, searched, and
  // under them what is done to branches (create one, clean up, sync). `anchor`: the button, to open over it (under it
  // when there is more room there). In `pick` mode it only chooses a branch for an agent or a ticket, `onpick` being
  // given it: the branch of the project's folder and those a worktree has can't be chosen, and nothing else is offered.
  let {
    projectId,
    anchor,
    onclose,
    mode = 'switch',
    onpick,
    over = false,
  }: {
    projectId: string;
    anchor?: HTMLElement;
    onclose: () => void;
    mode?: 'switch' | 'pick';
    onpick?: (b: BranchInfo) => void;
    /** Opened from a dialog: over it, not under its overlay. */
    over?: boolean;
  } = $props();
  const picking = $derived(mode === 'pick');

  /** Branches shown per group: a repository can have thousands of remote ones, which the search finds. */
  const SHOWN = 300;
  /** The width of the picker. */
  const WIDTH = 400;

  let branches = $state<BranchInfo[] | null>(null);
  /** Why the branches could not be read. */
  let failure = $state<string | null>(null);
  let text = $state('');
  /** The branch the cursor is on, by name: it stays on it when the list is read again. */
  let active = $state<string | null>(null);
  let switching = $state(false);
  let list = $state<HTMLDivElement>();
  let place = $state<{ left: number; bottom?: number; top?: number; maxHeight: number }>({ left: 16, bottom: 38, maxHeight: 560 });

  const sync = $derived(syncInfo(app.git[projectId]));

  onMount(() => {
    if (!anchor) return;
    const r = anchor.getBoundingClientRect();
    const left = Math.max(8, Math.min(r.left, window.innerWidth - WIDTH - 8));
    // Over the button, as the status bar's is at the foot of the window; under it when it is in the upper half.
    place =
      r.top >= window.innerHeight - r.bottom
        ? { left, bottom: Math.max(8, window.innerHeight - r.top + 6), maxHeight: Math.max(240, Math.min(560, r.top - 22)) }
        : { left, top: r.bottom + 6, maxHeight: Math.max(240, Math.min(560, window.innerHeight - r.bottom - 22)) };
  });

  /** Reads the branches; a read that comes after a later one is dropped. */
  let reads = 0;
  async function load() {
    const mine = ++reads;
    try {
      const all = await api.branchList(projectId);
      if (mine !== reads) return;
      branches = all;
      failure = null;
    } catch (e) {
      if (mine === reads) failure = String(e);
    }
  }

  // Read when it opens, and again when the checkout moves under it (a switch, a pull, an agent's commit).
  $effect(() => {
    const g = app.git[projectId];
    void [g?.branch, g?.upstream, g?.ahead, g?.behind];
    void load();
  });

  const locals = $derived((branches ?? []).filter((b) => !b.remote));
  const remotes = $derived((branches ?? []).filter((b) => b.remote));
  const localByName = $derived(new Map(locals.map((b) => [b.name, b])));

  /** The local branch a worktree other than the project's folder has, when picking `b` would need it. */
  function held(b: BranchInfo): BranchInfo | null {
    const own = b.remote ? (b.trackedBy ? localByName.get(b.trackedBy) : undefined) : b;
    return own && !own.current && own.worktree ? own : null;
  }

  /** The branch the project's folder is on, or the remote branch of the local one that is. */
  const isFolder = (b: BranchInfo) => b.current || (b.remote && !!b.trackedBy && !!localByName.get(b.trackedBy)?.current);

  /** The board's target branch, which the tickets are merged into: the local branch, or the remote one that tracks it. */
  const target = $derived(app.projects.find((p) => p.id === projectId)?.board.target.trim() ?? '');
  const isTarget = (b: BranchInfo) =>
    !!target && (b.remote ? (b.trackedBy ?? b.name.slice(b.name.indexOf('/') + 1)) === target : b.name === target);

  /** Why `b` can't be picked: an agent's worktree (or another one) has its branch, or, to pick one for an agent, the folder does, or the board's target. */
  function reasonOf(b: BranchInfo): string | null {
    // The target first: when the folder is on it, that is the reason.
    if (picking && isTarget(b)) return t('branches.agent.isTargetBranch');
    if (picking && isFolder(b)) return t('branches.agent.isFolderBranch');
    const h = held(b);
    return h && worktreeReason(h);
  }

  /** The short word after a branch's name. */
  function tagOf(b: BranchInfo): string | null {
    const h = held(b);
    if (h) return h.agent ? t('branches.picker.usedBy', { agent: app.agents[h.agent]?.name ?? '?' }) : t('branches.picker.otherWorktree');
    if (b.remote) return b.trackedBy ? t('branches.picker.trackedBy', { branch: b.trackedBy }) : null;
    return b.upstreamGone ? t('branches.picker.gone') : null;
  }

  /** What the branches matching the search are, best first, `hidden` being those beyond what is shown. */
  function matching(group: BranchInfo[]) {
    const byName = new Map(group.map((b) => [b.name, b]));
    const found = rankFiles(
      group.map((b) => b.name),
      text,
      [],
      group.length || 1,
    );
    return { shown: found.slice(0, SHOWN).map((n) => byName.get(n)!), hidden: Math.max(0, found.length - SHOWN) };
  }

  const found = $derived({ local: matching(locals), remote: matching(remotes) });
  const hidden = $derived(found.local.hidden + found.remote.hidden);

  interface Row {
    b: BranchInfo;
    index: number;
    why: string | null;
    tag: string | null;
  }
  const groups = $derived.by(() => {
    let index = 0;
    const rowsOf = (shown: BranchInfo[]): Row[] => shown.map((b) => ({ b, index: index++, why: reasonOf(b), tag: tagOf(b) }));
    const local = rowsOf(found.local.shown);
    const remote = rowsOf(found.remote.shown);
    return [
      { id: 'local', label: t('branches.picker.local'), rows: local },
      { id: 'remote', label: t('branches.picker.remote'), rows: remote },
    ];
  });
  const rows = $derived(groups.flatMap((g) => g.rows));
  const cursor = $derived(
    Math.max(
      0,
      rows.findIndex((r) => r.b.name === active),
    ),
  );
  const optionId = (index: number) => `branch-option-${index}`;

  /** What the row under the cursor says about itself: why it can't be picked, or what it tracks. */
  const description = $derived.by(() => {
    const r = rows[cursor];
    if (!r) return '';
    if (r.why) return r.why;
    const b = r.b;
    if (b.remote) return b.trackedBy ? t('branches.picker.trackedByWhy', { branch: b.trackedBy }) : '';
    if (b.upstreamGone) return t('branches.picker.goneWhy', { upstream: b.upstream ?? '' });
    return b.upstream ? t('branches.picker.tracks', { upstream: b.upstream, ahead: b.ahead, behind: b.behind }) : '';
  });

  // The row under the cursor stays in view as the arrows move through a long list.
  $effect(() => {
    void rows;
    list?.querySelector(`#${optionId(cursor)}`)?.scrollIntoView?.({ block: 'nearest' });
  });

  function move(by: number) {
    const n = rows.length;
    if (n) active = rows[(cursor + by + n) % n].b.name;
  }

  /** Closes the picker and waits for it to be gone. */
  async function release() {
    onclose();
    await tick();
  }

  async function pick(row: Row | undefined) {
    if (!row || row.why || switching) return;
    const b = row.b;
    // For an agent or a ticket: the branch is given back once the picker is gone.
    if (picking) {
      await release();
      onpick?.(b);
      return;
    }
    // Already on it (a remote branch: on the local one that tracks it).
    if (isFolder(b)) {
      onclose();
      return;
    }
    switching = true;
    // The picker is out of the way before a question opens: the focus then goes back to the button it came from.
    const result = await switchBranch(projectId, b.name, false, release);
    switching = false;
    if (result === 'refused') await load();
    else onclose();
  }

  /**
   * What can be done to a branch other than switching to it: start an agent on it, and delete it. Nothing for the one
   * the folder is on, nor for a worktree's (an agent there, or its branch to delete).
   */
  function itemsOf(b: BranchInfo): MenuItem[] {
    if (picking) return [];
    const items: MenuItem[] = [];
    if (!isFolder(b) && !held(b)) {
      items.push({
        label: t('branches.agent.launchHere'),
        onClick: async () => {
          await release();
          void startAgentOn(projectId, b.name);
        },
      });
    }
    if (deletable(b)) {
      if (items.length) items.push({ label: '', separator: true });
      items.push({
        label: b.remote ? t('branches.picker.deleteRemote') : t('branches.picker.deleteLocal'),
        danger: true,
        onClick: async () => {
          await release();
          deleteBranch(projectId, b);
        },
      });
    }
    return items;
  }

  function onContextMenu(e: MouseEvent, b: BranchInfo) {
    const items = itemsOf(b);
    if (items.length) menu.show(e, items);
    else e.preventDefault();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      move(e.key === 'ArrowDown' ? 1 : -1);
    } else if (e.key === 'Enter' && !e.isComposing) {
      e.preventDefault();
      void pick(rows[cursor]);
    } else if (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey)) {
      e.preventDefault();
      const row = rows[cursor];
      const el = row && list?.querySelector<HTMLElement>(`#${optionId(row.index)}`);
      const items = row ? itemsOf(row.b) : [];
      if (el && items.length) menu.showAt(el, items);
    }
  }

  /** Escape closes the picker, unless a menu of one of its rows is open: that one goes first. */
  function onWindowKeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape' || menu.open) return;
    e.preventDefault();
    e.stopPropagation();
    onclose();
  }

  async function newBranch() {
    await release();
    app.modal = { kind: 'newBranch', projectId };
  }

  async function mergedBranches() {
    await release();
    app.modal = { kind: 'mergedBranches', projectId };
  }

  function runSync(op: SyncOp) {
    onclose();
    void gitSync.run(op, projectId);
  }
</script>

<svelte:window onkeydowncapture={onWindowKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="backdrop" class:over onclick={onclose}></div>
<div
  class="picker"
  class:over
  use:trapFocus
  role="dialog"
  tabindex="-1"
  aria-modal="true"
  aria-label={picking ? t('branches.agent.pickTitle') : t('branches.picker.title')}
  style:left="{place.left}px"
  style:bottom={place.bottom === undefined ? undefined : `${place.bottom}px`}
  style:top={place.top === undefined ? undefined : `${place.top}px`}
  style:width="{WIDTH}px"
  style:max-height="{place.maxHeight}px"
>
  <div class="search">
    <!-- svelte-ignore a11y_autofocus -->
    <input
      bind:value={text}
      autofocus
      spellcheck="false"
      autocomplete="off"
      placeholder={t('branches.picker.searchPlaceholder')}
      role="combobox"
      aria-label={t('branches.picker.searchLabel')}
      aria-expanded={rows.length > 0}
      aria-controls="branch-list"
      aria-autocomplete="list"
      aria-activedescendant={rows.length ? optionId(cursor) : undefined}
      oninput={() => (active = null)}
      onkeydown={onKeydown}
    />
  </div>
  <!-- Keeps the focus in the search field when a row is clicked: the keyboard goes on working. -->
  <div
    class="list"
    id="branch-list"
    role="listbox"
    tabindex="-1"
    aria-label={picking ? t('branches.agent.pickTitle') : t('branches.picker.title')}
    bind:this={list}
    onmousedown={(e) => e.preventDefault()}
  >
    {#each groups as g (g.id)}
      {#if g.rows.length}
        <div role="group" aria-labelledby="branch-group-{g.id}">
          <div class="grp" id="branch-group-{g.id}">{g.label}</div>
          {#each g.rows as r (r.b.name)}
            {@const b = r.b}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div
              class="row"
              class:on={r.index === cursor}
              class:blocked={!!r.why}
              id={optionId(r.index)}
              role="option"
              tabindex="-1"
              aria-selected={r.index === cursor}
              aria-current={b.current ? 'true' : undefined}
              aria-disabled={r.why ? 'true' : undefined}
              aria-describedby={r.index === cursor ? 'branch-description' : undefined}
              title={r.why ?? undefined}
              onclick={() => pick(r)}
              onmousemove={() => (active = b.name)}
              oncontextmenu={(e) => onContextMenu(e, b)}
            >
              <span class="chk" aria-hidden="true">{b.current ? '✓' : ''}</span>
              <span class="name">{b.name}</span>
              {#if b.current}<span class="sr">{t('branches.picker.current')}</span>{/if}
              {#if r.tag}<span class="tag">{r.tag}</span>{/if}
              {#if b.ahead || b.behind}
                <span class="ab">
                  {#if b.ahead}<span aria-hidden="true">↑{b.ahead}</span><span class="sr"
                      >{t('branches.picker.ahead', { count: b.ahead })}</span
                    >{/if}
                  {#if b.behind}<span aria-hidden="true">↓{b.behind}</span><span class="sr"
                      >{t('branches.picker.behind', { count: b.behind })}</span
                    >{/if}
                </span>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    {/each}
    {#if failure}
      <p class="none error" role="alert">{failure}</p>
    {:else if !branches}
      <p class="none">{t('common.loading')}</p>
    {:else if !rows.length}
      <p class="none">{text.trim() ? t('branches.picker.noMatch') : t('branches.picker.empty')}</p>
    {/if}
    {#if hidden}<p class="none">{t('branches.picker.more', { count: hidden })}</p>{/if}
  </div>
  <div class="note">
    {#if switching}
      <p role="status">{t('branches.picker.switching')}</p>
    {:else}
      <p id="branch-description">{description}</p>
    {/if}
  </div>
  {#if !picking}
    <div class="foot">
      <div class="acts">
        <button class="act" onclick={newBranch}>{t('branches.picker.newBranch')}</button>
        <button class="act" onclick={mergedBranches}>{t('branches.picker.mergedBranches')}</button>
      </div>
      {#if sync}
        <div class="acts">
          <button class="act" disabled={!sync.tracked || sync.behind === 0} onclick={() => runSync('pull')}
            >{t('branches.sync.pull')} <span class="hint">↓{sync.behind}</span></button
          >
          {#if sync.tracked}
            <button class="act" disabled={sync.ahead === 0} onclick={() => runSync('push')}
              >{t('branches.sync.push')} <span class="hint">↑{sync.ahead}</span></button
            >
          {:else}
            <button class="act" onclick={() => runSync('push')}>{t('branches.sync.publish')}</button>
          {/if}
          <button class="act" onclick={() => runSync('fetch')}
            >{t('branches.sync.fetch')} <span class="hint">{t('branches.sync.now')}</span></button
          >
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
  }
  /* Over the dialogs (z-index 50), under the menus (90). */
  .backdrop.over {
    z-index: 60;
  }
  .picker.over {
    z-index: 61;
  }
  .picker {
    position: fixed;
    z-index: 41;
    max-width: calc(100vw - 16px);
    display: flex;
    flex-direction: column;
    background: var(--elev);
    border: 1px solid var(--line2);
    border-radius: var(--r);
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.45);
    font-size: 12.5px;
    color: var(--text);
    animation: ccFadeIn 0.1s ease-out;
  }
  @media (prefers-reduced-motion: reduce) {
    .picker {
      animation: none;
    }
  }
  .search {
    flex: none;
    padding: 8px;
    border-bottom: 1px solid var(--line);
  }
  .search input {
    width: 100%;
    height: 30px;
    padding: 0 10px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    outline: none;
  }
  .search input:focus {
    border-color: var(--accent);
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 4px 6px;
  }
  .grp {
    padding: 8px 8px 4px;
    font-size: 10.5px;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--dim);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    padding: 0 8px;
    border-radius: var(--r-sm);
    cursor: pointer;
    white-space: nowrap;
  }
  .row.on {
    background: var(--elev2);
  }
  .row.blocked {
    cursor: default;
    color: var(--muted);
  }
  .chk {
    flex: none;
    width: 12px;
    color: var(--ok);
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: var(--mono);
    font-size: 12px;
  }
  .tag {
    flex: none;
    padding: 0 6px;
    border: 1px solid var(--line2);
    border-radius: 99px;
    font-size: 10.5px;
    color: var(--dim);
  }
  .ab {
    flex: none;
    margin-left: auto;
    display: flex;
    gap: 6px;
    font-family: var(--mono);
    font-size: 11px;
    color: var(--muted);
  }
  .none {
    margin: 0;
    padding: 10px 8px;
    color: var(--dim);
  }
  .none.error {
    color: var(--del);
  }
  .note {
    flex: none;
    min-height: 30px;
    padding: 6px 14px;
    border-top: 1px solid var(--line);
    font-size: 11.5px;
    color: var(--muted);
  }
  .note p {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .foot {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 6px;
    border-top: 1px solid var(--line);
  }
  .acts {
    display: flex;
    gap: 4px;
  }
  /* What is done to the branches, then the sync with the remote. */
  .acts + .acts {
    margin-top: 2px;
    padding-top: 4px;
    border-top: 1px solid var(--line);
  }
  .act {
    height: 26px;
    padding: 0 8px;
    border: none;
    border-radius: var(--r-sm);
    background: transparent;
    font: inherit;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
  }
  .act:hover:not(:disabled) {
    background: var(--elev2);
  }
  .act:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .hint {
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--dim);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
</style>
