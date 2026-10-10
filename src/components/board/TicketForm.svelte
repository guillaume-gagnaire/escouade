<script lang="ts">
  import { onDestroy, tick, untrack } from 'svelte';
  import { cycleRefusal, ticketBranch } from '../../lib/board';
  import { t } from '../../lib/i18n';
  import { menu } from '../../lib/menu.svelte';
  import { keyLabel, primaryKey } from '../../lib/platform';
  import { ariaEnter } from '../../lib/shortcuts';
  import { app, type TicketFormDraft } from '../../lib/state.svelte';
  import type { Ticket, TicketDraft } from '../../lib/types';
  import BranchPicker from '../branches/BranchPicker.svelte';
  import Modal from '../modals/Modal.svelte';

  // The form of a ticket, new or edited, in a large window to write long texts in. `onsubmit` saves the ticket and tells
  // whether it did: the window closes then, else it stays with what was typed. `resume`: what had been typed, back from a
  // dialog that took the window's place (the confirmation of its closing).
  let {
    ticket,
    projectId,
    onsubmit,
    resume,
  }: {
    ticket?: Ticket;
    projectId: string;
    onsubmit: (d: TicketDraft) => boolean | void | Promise<boolean | void>;
    resume?: TicketFormDraft;
  } = $props();

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

  let title = $state(untrack(() => resume?.title ?? start.title));
  let description = $state(untrack(() => resume?.description ?? start.description));
  let criteria = $state(untrack(() => resume?.criteria ?? start.criteria));
  let maxLoops = $state(untrack(() => resume?.maxLoops ?? start.maxLoops));
  /** The tickets it comes after, in the order they were checked; those done since stay, unlisted (they hold nothing back). */
  let after = $state(untrack(() => [...(resume?.after ?? start.after)]));
  /** The existing branch its agent takes up; empty: the ticket's own, made when it starts. */
  let branch = $state(untrack(() => resume?.branch ?? start.branch));
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

  /**
   * The dialog it is. Another one taking its place (the confirmation of its closing, « Quitter Escouade ? ») comes back to
   * it once cancelled: what was typed is kept in it.
   */
  const self = untrack(() => app.modal);
  onDestroy(() => {
    if (self?.kind === 'ticket') self.resume = { title, description, criteria, maxLoops, after: [...after], branch };
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

  /**
   * Leaves the window (Escape, its cross, a click outside, « Annuler »); what was typed is only given up once the user
   * agrees, « Annuler » there coming back to the form. Not while the ticket is saved: a refusal would have nowhere to show.
   */
  function leave() {
    if (busy) return;
    if (!changed) {
      app.modal = null;
      return;
    }
    app.modal = {
      kind: 'confirm',
      title: t('board.form.discardTitle'),
      body: t('board.form.discardBody'),
      confirm: t('board.form.discard'),
      danger: true,
      onConfirm: () => {},
      onCancel: () => (app.modal = self),
    };
  }

  async function submit() {
    if (!ready) return;
    busy = true;
    // Replaced while it saves (« Quitter Escouade ? »), the window does not come back to save twice; refused, it does.
    app.markAnswered(self);
    let saved: boolean | void = false;
    try {
      saved = await onsubmit({
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
    } finally {
      busy = false;
    }
    if (!saved) app.markAnswered(self, false);
    // Not a dialog that took its place meanwhile.
    else if (app.modal === self) app.modal = null;
  }

  /** Ctrl+Enter (Cmd+Enter on macOS) adds the ticket from any field; Enter alone only from the title, the textareas take it as a new line. */
  function chord(e: KeyboardEvent) {
    if (e.key !== 'Enter' || e.isComposing || !primaryKey(e) || e.shiftKey || e.altKey) return;
    e.preventDefault();
    void submit();
  }

  /** The textarea grows with what is typed in it (up to its `max-height`), and keeps a height the user dragged it to. */
  function grow(node: HTMLTextAreaElement, _text: string) {
    const fit = () => {
      if (node.scrollHeight > node.clientHeight) node.style.height = `${node.scrollHeight + node.offsetHeight - node.clientHeight}px`;
    };
    fit();
    return { update: fit };
  }
</script>

<Modal title={ticket ? t('board.form.editTitle', { key: ticket.key }) : t('board.form.newTitle')} width={760} tall onclose={leave}>
  <div class="form" role="presentation" onkeydown={chord}>
    <!-- svelte-ignore a11y_autofocus -->
    <input
      class="title"
      placeholder={t('board.form.title')}
      aria-label={t('board.form.title')}
      bind:value={title}
      autofocus
      onkeydown={(e) => e.key === 'Enter' && !primaryKey(e) && submit()}
    />
    <div class="grp">
      <label class="lab" for="{uid}-description">{t('board.form.description')}</label>
      <textarea
        id="{uid}-description"
        class="description"
        rows="12"
        placeholder={t('board.form.descriptionPlaceholder')}
        bind:value={description}
        use:grow={description}
      ></textarea>
    </div>
    <div class="grp">
      <label class="lab" for="{uid}-criteria">{t('board.form.criteria')}</label>
      <textarea id="{uid}-criteria" rows="6" placeholder={t('board.form.criteriaPlaceholder')} bind:value={criteria}></textarea>
    </div>
    <div class="meta" class:single={!candidates.length}>
      <div class="side">
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
                    ><input type="checkbox" checked={after.includes(x.id)} onchange={(e) => toggle(x, e.currentTarget)} /><span
                      class="key mono">{x.key}</span
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
    </div>
  </div>
  {#snippet footer()}
    <span class="hint">{t(ticket ? 'board.form.saveHint' : 'board.form.addHint', { key: keyLabel('Ctrl+Enter') })}</span>
    <button class="btn ghost" onclick={leave}>{t('common.cancel')}</button>
    <button class="btn primary" disabled={!ready} aria-keyshortcuts={ariaEnter()} onclick={submit}
      >{ticket ? t('common.save') : t('common.add')}</button
    >
  {/snippet}
</Modal>

{#if choosing}
  <BranchPicker over {projectId} anchor={branchButton} mode="pick" onclose={() => (choosing = false)} onpick={(b) => (branch = b.name)} />
{/if}

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-width: 0;
  }
  .title,
  textarea {
    padding: 0 12px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line2);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    outline: none;
  }
  .title:focus,
  textarea:focus {
    border-color: var(--accent);
  }
  .title {
    height: 40px;
    font-size: 14.5px;
    font-weight: 600;
  }
  .grp {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .lab {
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
  }
  textarea {
    resize: vertical;
    padding: 10px 12px;
    font-size: 13px;
    line-height: 1.55;
  }
  /* Grows with its text up to this, then scrolls in itself. */
  .description {
    max-height: 60vh;
  }
  /* Beside each other: the loops and the branch, the dependencies. */
  .meta {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1.2fr);
    gap: 16px 24px;
    align-items: start;
    padding-top: 14px;
    border-top: 1px solid var(--line);
  }
  .meta.single {
    grid-template-columns: minmax(0, 1fr);
  }
  .side {
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0;
  }
  .loops {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .k {
    font-size: 12px;
    font-weight: 600;
    color: var(--muted);
    margin-right: 4px;
  }
  .loops button {
    height: 26px;
    min-width: 30px;
    padding: 0 8px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-family: var(--mono);
    font-size: 11.5px;
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
    height: 26px;
    padding: 0 8px;
    border: 1px solid var(--line2);
    border-radius: var(--r-sm);
    background: transparent;
    color: var(--muted);
    font-size: 11.5px;
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
    min-width: 0;
  }
  .search {
    height: 30px;
    padding: 0 10px;
    border-radius: var(--r-sm);
    border: 1px solid var(--line2);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-size: 12.5px;
    outline: none;
  }
  .search:focus {
    border-color: var(--accent);
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
    font-size: 12.5px;
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
  /* The key that adds the ticket, at the foot's left: the buttons stay on the right. */
  .hint {
    flex: 1;
    align-self: center;
    font-size: 11.5px;
    color: var(--dim);
  }
</style>
