<script lang="ts">
  import { tick, untrack } from 'svelte';
  import { cycleRefusal, ticketBranch } from '../../lib/board';
  import { t } from '../../lib/i18n';
  import { menu } from '../../lib/menu.svelte';
  import { app } from '../../lib/state.svelte';
  import type { Ticket, TicketDraft } from '../../lib/types';
  import BranchPicker from '../branches/BranchPicker.svelte';

  let {
    ticket,
    projectId,
    onsubmit,
    oncancel,
  }: { ticket?: Ticket; projectId: string; onsubmit: (d: TicketDraft) => void | Promise<void>; oncancel: () => void } = $props();

  // What the form opened with, to tell whether leaving it loses anything.
  const start = untrack(() => ({
    title: ticket?.title ?? '',
    description: ticket?.description ?? '',
    criteria: ticket?.criteria.map((c) => c.text).join('\n') ?? '',
    maxLoops: ticket?.maxLoops ?? 5,
    after: ticket?.after ?? [],
    branch: ticket?.branch ?? '',
  }));
  const uid = $props.id();

  let title = $state(start.title);
  let description = $state(start.description);
  let criteria = $state(start.criteria);
  let maxLoops = $state(start.maxLoops);
  /** The tickets it comes after, in the order they were checked; those done since stay, unlisted (they hold nothing back). */
  let after = $state([...start.after]);
  /** The existing branch its agent takes up; empty: the ticket's own, made when it starts. */
  let branch = $state(start.branch);
  let branchButton = $state<HTMLButtonElement>();
  let choosing = $state(false);
  let query = $state('');
  /** Why the last ticket checked was refused (it waits for this one already). */
  let refusal = $state<string | null>(null);
  let busy = $state(false);
  const ready = $derived(title.trim().length > 0 && !busy);
  const changed = $derived(
    title !== start.title ||
      description !== start.description ||
      criteria !== start.criteria ||
      maxLoops !== start.maxLoops ||
      branch !== start.branch ||
      after.length !== start.after.length ||
      after.some((id) => !start.after.includes(id)),
  );
  /** What it may come after: the other tickets of the project not done yet, oldest first. */
  const candidates = $derived(
    Object.values(app.tickets)
      .filter((x) => x.projectId === projectId && x.column !== 'done' && x.id !== ticket?.id)
      .sort((a, b) => a.createdAt - b.createdAt),
  );
  /** Those whose key (or, imported, the key there) holds the search; those checked stay in sight. */
  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const holds = (key: string | undefined) => !!key && key.toLowerCase().includes(q);
    return candidates.filter((x) => after.includes(x.id) || holds(x.key) || holds(x.external?.key));
  });

  /** What the ticket does when there is no branch to take up: its own, named after its key once it has one. */
  const own = $derived(ticket ? t('branches.ticket.own', { branch: ticketBranch(ticket.key) }) : t('branches.ticket.ownNew'));

  function branchMenu() {
    if (!branchButton) return;
    menu.showAt(branchButton, [
      { label: own, onClick: () => (branch = '') },
      {
        label: t('branches.ticket.existing'),
        // The branch is the user's: validating the ticket never deletes it.
        title: t('branches.ticket.existingKept'),
        // Once the menu has given the focus back to its button: the picker gives it back there in turn.
        onClick: async () => {
          await tick();
          choosing = true;
        },
      },
    ]);
  }

  /** A ticket checked that already waits for this one is refused: neither would ever start. */
  function toggle(x: Ticket, box: HTMLInputElement) {
    refusal = null;
    if (!box.checked) {
      after = after.filter((id) => id !== x.id);
      return;
    }
    const refused = ticket ? cycleRefusal(ticket, x, app.tickets) : null;
    if (refused) {
      refusal = refused;
      box.checked = false;
      return;
    }
    after = [...after, x.id];
  }

  /** Leaves the form; what was typed is only given up once the user agrees. */
  function leave() {
    if (!changed) {
      oncancel();
      return;
    }
    app.modal = {
      kind: 'confirm',
      title: t('board.form.discardTitle'),
      body: t('board.form.discardBody'),
      confirm: t('board.form.discard'),
      danger: true,
      onConfirm: oncancel,
    };
  }

  function escape(e: KeyboardEvent) {
    if (e.key !== 'Escape') return;
    // The confirmation closes on Escape through a window listener, which is already there when this very event reaches the window: it would close at once.
    if (changed) e.stopPropagation();
    leave();
  }

  async function submit() {
    if (!ready) return;
    busy = true;
    await onsubmit({
      title: title.trim(),
      description: description.trim(),
      criteria: criteria
        .split('\n')
        .map((l) => l.trim())
        .filter(Boolean),
      maxLoops,
      after: [...after],
      // Said only when it matters: a branch taken up, or the one of an edited ticket given up (empty).
      ...(branch || start.branch ? { branch } : {}),
    });
    busy = false;
  }
</script>

<!-- Escape leaves the form from any of its fields; Enter only submits from the title (the textareas take it as a new line). -->
<div class="form" role="presentation" onkeydown={escape}>
  <!-- svelte-ignore a11y_autofocus -->
  <input
    class="title"
    placeholder={t('board.form.title')}
    aria-label={t('board.form.title')}
    bind:value={title}
    autofocus
    onkeydown={(e) => e.key === 'Enter' && submit()}
  />
  <textarea rows="2" placeholder={t('board.form.descriptionPlaceholder')} aria-label={t('board.form.description')} bind:value={description}
  ></textarea>
  <textarea rows="4" placeholder={t('board.form.criteriaPlaceholder')} aria-label={t('board.form.criteria')} bind:value={criteria}
  ></textarea>
  <div class="loops" role="group" aria-label={t('board.form.maxLoops')}>
    <span class="k">{t('board.form.maxLoops')}</span>
    {#each [3, 5, 8] as n (n)}
      <button class:on={maxLoops === n} aria-pressed={maxLoops === n} onclick={() => (maxLoops = n)}>{n}</button>
    {/each}
  </div>
  <div class="branch" role="group" aria-label={t('branches.ticket.branch')}>
    <span class="k" id="{uid}-branch">{t('branches.ticket.branch')}</span>
    <button
      class="pick mono"
      bind:this={branchButton}
      aria-haspopup="menu"
      aria-labelledby="{uid}-branch {uid}-branch-value"
      title={branch || own}
      onclick={branchMenu}
    >
      <span id="{uid}-branch-value">{branch || own}</span>
      <span aria-hidden="true">▾</span>
    </button>
  </div>
  {#if candidates.length}
    <div class="after" role="group" aria-label={t('board.form.after')}>
      <span class="k">{t('board.form.after')}</span>
      <input
        class="search"
        role="searchbox"
        placeholder={t('board.form.searchKey')}
        aria-label={t('board.form.searchKey')}
        spellcheck="false"
        autocomplete="off"
        bind:value={query}
      />
      {#if shown.length}
        <ul class="deps">
          {#each shown as x (x.id)}
            <li>
              <label
                ><input type="checkbox" checked={after.includes(x.id)} onchange={(e) => toggle(x, e.currentTarget)} /><span class="key mono"
                  >{x.key}</span
                > <span class="name" title={x.title}>{x.title}</span></label
              >
            </li>
          {/each}
        </ul>
      {:else}
        <span class="none">{t('board.form.noMatch')}</span>
      {/if}
      {#if refusal}<span class="refusal" role="alert">{refusal}</span>{/if}
    </div>
  {/if}
  <div class="actions">
    <button class="btn ghost" onclick={leave}>{t('common.cancel')}</button>
    <button class="btn primary" disabled={!ready} onclick={submit}>{ticket ? t('common.save') : t('common.add')}</button>
  </div>
</div>

{#if choosing}
  <BranchPicker {projectId} anchor={branchButton} mode="pick" onclose={() => (choosing = false)} onpick={(b) => (branch = b.name)} />
{/if}

<style>
  .form {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    border-radius: var(--r-sm);
    background: var(--elev);
    border: 1px solid var(--accent);
  }
  .title,
  textarea {
    padding: 0 10px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line2);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    outline: none;
  }
  .title {
    height: 32px;
    font-size: 13px;
    font-weight: 600;
  }
  textarea {
    resize: none;
    padding: 8px 10px;
    font-size: 12px;
    line-height: 1.5;
  }
  .loops {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .k {
    font-size: 11px;
    color: var(--dim);
    margin-right: 4px;
  }
  .loops button {
    height: 24px;
    min-width: 28px;
    padding: 0 8px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-family: var(--mono);
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
  }
  .loops button.on {
    background: var(--elev2);
    color: var(--text);
  }
  .branch {
    display: flex;
    align-items: center;
    gap: 4px;
    min-width: 0;
  }
  .pick {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    height: 24px;
    padding: 0 8px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: 11px;
    cursor: pointer;
  }
  .pick:hover {
    color: var(--text);
    border-color: var(--accent);
  }
  .pick span:first-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .after {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .search {
    height: 28px;
    padding: 0 10px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line2);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 12px;
  }
  /* A long column of tickets scrolls rather than stretch the form. */
  .deps {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 136px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .deps label {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    padding: 3px 4px;
    border-radius: var(--r-sm);
    font-size: 12px;
    cursor: pointer;
  }
  .deps label:hover {
    background: var(--elev2);
  }
  .deps input {
    margin: 0;
    flex: none;
    accent-color: var(--accent);
  }
  .deps .key {
    flex: none;
    font-size: 11px;
    color: var(--muted);
  }
  /* A long title is cut, in full in its tooltip. */
  .deps .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .none {
    font-size: 11.5px;
    color: var(--dim);
  }
  .refusal {
    font-size: 11.5px;
    color: var(--del);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }
  .actions .btn {
    height: 28px;
    font-size: 12px;
  }
</style>
