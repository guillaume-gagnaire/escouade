<script lang="ts">
  import { onDestroy, untrack } from 'svelte';
  import { askStash, branchRefusal, tellRefusal } from '../../lib/branch-actions';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { app, type NewBranchDraft } from '../../lib/state.svelte';
  import type { BranchInfo } from '../../lib/types';
  import Modal from '../modals/Modal.svelte';

  // « Nouvelle branche »: a name git accepts (checked as it is typed), where the branch starts, and whether the folder
  // goes to it. `start`: a branch or the commit the graph gave, the current branch by default; `resume`: what was
  // written, back from the question of the stash.
  let { projectId, start, resume }: { projectId: string; start?: string; resume?: NewBranchDraft } = $props();

  /** How long the typing must pause before the name is checked: the check asks git. */
  const CHECK_DELAY = 250;

  let name = $state(untrack(() => resume?.name ?? ''));
  /** Where it starts: a branch's name, or a commit; empty is the folder's HEAD. */
  let from = $state(untrack(() => resume?.start ?? start ?? ''));
  let switchTo = $state(untrack(() => resume?.switchTo ?? true));
  let branches = $state<BranchInfo[] | null>(null);
  /** Why the name can't be taken, as git or the backend says. */
  let problem = $state<string | null>(null);
  /** Why the branch could not be made (not the name: the start, a hook…). */
  let refused = $state<string | null>(null);
  let busy = $state(false);
  /** Replaced by another dialog while it runs (the question of the stash): what comes of it goes to a toast. */
  let gone = false;

  /**
   * The dialog it is. Another one taking its place (the question of the stash, « Quitter Escouade ? ») comes back to it
   * once cancelled: what was written is kept in it.
   */
  const self = untrack(() => app.modal);
  const draft = (): NewBranchDraft => ({ name, start: from, switchTo });
  onDestroy(() => {
    gone = true;
    clearTimeout(timer);
    if (self?.kind === 'newBranch') self.resume = draft();
  });

  // The branches to start from. The current one is where it starts unless the graph said otherwise.
  $effect(() => {
    let alive = true;
    api
      .branchList(projectId)
      .then((all) => {
        if (!alive) return;
        branches = all;
        const current = all.find((b) => b.current);
        if (!untrack(() => from) && current) from = current.name;
      })
      .catch((e) => alive && (refused = String(e)));
    return () => {
      alive = false;
    };
  });

  const COMMIT = /^[0-9a-f]{7,40}$/i;
  const options = $derived.by(() => {
    if (!branches) return [];
    const out: { value: string; label: string }[] = [];
    // A commit the graph gave: not among the branches.
    if (from && !branches.some((b) => b.name === from)) {
      out.push({ value: from, label: t('branches.create.fromCommit', { hash: COMMIT.test(from) ? from.slice(0, 7) : from }) });
    }
    // No branch checked out: the folder's own commit.
    if (!branches.some((b) => b.current) && !from) out.push({ value: '', label: t('branches.create.fromHead') });
    for (const b of branches) out.push({ value: b.name, label: b.name });
    return out;
  });

  let checks = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  /** Checks the name once the typing paused; a check that comes after a later one is dropped. */
  function typed() {
    problem = null;
    refused = null;
    clearTimeout(timer);
    const wanted = name.trim();
    const mine = ++checks;
    if (!wanted) return;
    timer = setTimeout(async () => {
      try {
        await api.branchCheck(projectId, wanted);
        if (mine === checks) problem = null;
      } catch (e) {
        if (mine === checks) problem = String(e);
      }
    }, CHECK_DELAY);
  }

  const ready = $derived(!!name.trim() && !problem && !busy);

  function close() {
    app.modal = null;
  }

  async function create(stash = false) {
    const wanted = name.trim();
    if (!wanted || busy) return;
    busy = true;
    refused = null;
    const [at, here] = [from, switchTo];
    try {
      const stashed = await api.branchCreate(projectId, wanted, at, here, stash);
      if (!gone) close();
      app.toast(t(here ? 'branches.create.createdHere' : 'branches.create.created', { branch: wanted }), 'ok');
      if (stashed) app.toast(t('branches.switch.stashed', { name: stashed }), 'ok');
    } catch (e) {
      if (branchRefusal(e)?.kind === 'dirty') {
        // Cancelling comes back to this window, with what was written in it.
        const back = self?.kind === 'newBranch' ? self : null;
        askStash(
          t('branches.create.dirtyTitle', { branch: wanted }),
          () => create(true),
          () => {
            app.modal = back ?? { kind: 'newBranch', projectId, start, resume: draft() };
          },
        );
      } else if (gone || branchRefusal(e)) {
        tellRefusal(e, wanted);
      } else {
        refused = String(e);
      }
    } finally {
      busy = false;
    }
  }
</script>

<Modal title={t('branches.create.title')} width={460} onclose={close}>
  <div class="form">
    <div class="grp">
      <label class="lab" for="new-branch-name">{t('branches.create.name')}</label>
      <!-- svelte-ignore a11y_autofocus -->
      <input
        id="new-branch-name"
        class="field mono"
        bind:value={name}
        oninput={typed}
        onkeydown={(e) => e.key === 'Enter' && !e.isComposing && (e.preventDefault(), create())}
        autofocus
        spellcheck="false"
        autocomplete="off"
        placeholder={t('branches.create.namePlaceholder')}
        aria-invalid={problem ? 'true' : undefined}
        aria-describedby={problem ? 'new-branch-problem' : undefined}
      />
      <p class="problem" id="new-branch-problem" aria-live="polite">{problem ?? ''}</p>
    </div>
    <div class="grp">
      <label class="lab" for="new-branch-from">{t('branches.create.from')}</label>
      <select id="new-branch-from" class="field mono" bind:value={from} disabled={!branches}>
        {#each options as o (o.value)}<option value={o.value}>{o.label}</option>{/each}
      </select>
    </div>
    {#if refused}<p class="refused" role="alert">{refused}</p>{/if}
  </div>
  {#snippet footer()}
    <label class="opt">
      <input type="checkbox" bind:checked={switchTo} />
      {t('branches.create.switchTo')}
    </label>
    <div style="flex:1"></div>
    <button class="btn ghost" onclick={close}>{t('common.cancel')}</button>
    <button class="btn primary" disabled={!ready} onclick={() => create()}>{t('common.create')}</button>
  {/snippet}
</Modal>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 16px;
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
  .mono {
    font-family: var(--mono);
    font-size: 12.5px;
  }
  .problem {
    margin: 0;
    min-height: 1.2em;
    font-size: 12px;
    color: var(--del);
    overflow-wrap: anywhere;
  }
  .refused {
    margin: 0;
    font-size: 12.5px;
    color: var(--del);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .opt input {
    accent-color: var(--accent);
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    cursor: pointer;
  }
</style>
