<script lang="ts">
  // A direct commit in a project of several repositories (ESC-169): one message per repository changed, each proposed
  // apart by Haiku in its own style, and « Commiter les N dépôts », the only thing that commits. A repository whose
  // commit fails says so; the others are committed all the same.
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { groupByRepo, inRepo, repoLabel, type RepoGroup } from '../../lib/repos';
  import { app } from '../../lib/state.svelte';
  import Icon from '../Icon.svelte';
  import Modal from './Modal.svelte';

  let { projectId, agentId }: { projectId: string; agentId: string | null } = $props();

  const project = $derived(app.projects.find((p) => p.id === projectId) ?? null);
  const owner = $derived((agentId ? app.agents[agentId]?.name : project?.name) ?? '');

  interface Draft {
    message: string;
    proposed: string;
    proposing: boolean;
    noProposal: string | null;
    refused: string | null;
    done: string | null;
    typed: boolean;
  }
  let groups = $state<RepoGroup[] | null>(null);
  let leftOut = $state<string[]>([]);
  let failure = $state<string | null>(null);
  let drafts = $state<Record<string, Draft>>({});
  let committing = $state(false);

  onMount(async () => {
    try {
      const scope = await api.commitPreview(projectId, agentId);
      groups = groupByRepo(scope.files, project?.repos ?? []);
      leftOut = scope.leftOut;
    } catch (e) {
      failure = String(e);
      return;
    }
    for (const g of groups) {
      drafts[g.repo] = { message: '', proposed: '', proposing: false, noProposal: null, refused: null, done: null, typed: false };
      void propose(g);
    }
  });

  async function propose(g: RepoGroup) {
    const d = drafts[g.repo];
    d.proposing = true;
    d.noProposal = null;
    d.typed = false;
    try {
      const proposal = await api.commitPropose(
        projectId,
        agentId,
        g.files.map((f) => f.path),
      );
      if (!d.typed) d.message = d.proposed = proposal;
    } catch (e) {
      d.noProposal = String(e).trim().replace(/\.$/, '');
    } finally {
      d.proposing = false;
    }
  }

  const pending = $derived((groups ?? []).filter((g) => !drafts[g.repo]?.done));
  const ready = $derived(pending.length > 0 && pending.every((g) => drafts[g.repo]?.message.trim()));

  async function commit() {
    if (!ready || committing) return;
    committing = true;
    for (const g of pending) {
      const d = drafts[g.repo];
      d.refused = null;
      try {
        d.done = await api.commitDirect(
          projectId,
          agentId,
          g.files.map((f) => f.path),
          d.message,
        );
      } catch (e) {
        d.refused = String(e);
      }
    }
    committing = false;
    const hashes = (groups ?? []).map((g) => drafts[g.repo]?.done).filter(Boolean);
    if (pending.length === 0) {
      app.modal = null;
      app.toast(t('git.commit.created', { hash: hashes.join(', ') }), 'ok');
    }
  }

  const close = () => {
    if (!committing) app.modal = null;
  };
</script>

<Modal title={t('repos.commit.title', { count: groups?.length ?? 0 })} width={640} onclose={close}>
  <p class="scope">{owner} · {t('repos.commit.scope')}</p>
  {#if failure}
    <p class="error" role="alert">{failure}</p>
  {:else if groups && !groups.length}
    <p class="note">{agentId ? t('git.commit.noneForAgent') : t('git.commit.noneForProject')}</p>
  {:else if groups}
    {#each groups as g (g.repo)}
      {@const d = drafts[g.repo]}
      {#if d}
        <section class="rmsg" aria-label={repoLabel(g.repo)}>
          <div class="rhead">
            <span class="rn">{repoLabel(g.repo)}</span><span>{t('repos.commit.files', { count: g.files.length })}</span><span class="add"
              >+{g.add}</span
            ><span class="del">−{g.del}</span>
            {#if d.proposing}<span class="state">{t('repos.commit.proposing')}</span>{/if}
            {#if d.done}<span class="state ok"><Icon name="check" size={12} />{d.done}</span>{/if}
          </div>
          <ul class="paths">
            {#each g.files.slice(0, 6) as f (f.path)}<li>
                <span class="letter {f.status.toLowerCase()}">{f.status}</span>{inRepo(f.path, g.repo)}
              </li>{/each}
            {#if g.files.length > 6}<li class="more">+{g.files.length - 6}</li>{/if}
          </ul>
          <textarea
            class="field mono message"
            rows="3"
            aria-label={t('git.commit.message') + ' · ' + repoLabel(g.repo)}
            disabled={!!d.done}
            bind:value={d.message}
            oninput={() => (d.typed = true)}></textarea>
          {#if d.noProposal}<p class="note">{t('git.commit.noProposal', { reason: d.noProposal })}</p>{/if}
          {#if d.refused}<p class="error" role="alert">{d.refused}</p>{/if}
        </section>
      {/if}
    {/each}
    {#if leftOut.length}<p class="note">{t('git.commit.leftOut', { count: leftOut.length, files: leftOut.join(', ') })}</p>{/if}
    <p class="note">{t('repos.commit.failedNote')}</p>
  {/if}
  {#snippet footer()}
    <button class="btn" disabled={committing} onclick={close}>{t('common.cancel')}</button>
    <button class="btn primary" disabled={!ready || committing} onclick={commit}
      ><Icon name="check" size={14} />{committing
        ? t('git.commit.committing')
        : t('repos.commit.confirm', { count: pending.length })}</button
    >
  {/snippet}
</Modal>

<style>
  .scope {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
  }
  .rmsg {
    display: grid;
    gap: 6px;
    padding: 10px 12px 12px;
    border-radius: 10px;
    box-shadow: inset 0 0 0 1px var(--line);
  }
  .rhead {
    display: flex;
    align-items: baseline;
    gap: 8px;
    color: var(--muted);
    font-size: 12px;
  }
  .rn {
    color: var(--text);
    font: 600 12px var(--mono);
  }
  .state {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-left: auto;
  }
  .state.ok {
    color: var(--add);
    font-family: var(--mono);
  }
  .paths {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 2px 12px;
    font: 11.5px var(--mono);
    color: var(--code-fg);
  }
  .paths .more {
    color: var(--muted);
  }
  .letter {
    margin-right: 5px;
    font-weight: 600;
    color: var(--git-m);
  }
  .letter.a {
    color: var(--add);
  }
  .letter.d {
    color: var(--del);
  }
  .add {
    color: var(--add);
  }
  .del {
    color: var(--del);
  }
  .message {
    height: auto;
    min-height: 64px;
    padding: 8px 10px;
    resize: vertical;
    font-size: 12px;
    line-height: 1.55;
  }
  .note {
    margin: 0;
    font-size: 12px;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .error {
    margin: 0;
    font-size: 12.5px;
    color: var(--del);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
