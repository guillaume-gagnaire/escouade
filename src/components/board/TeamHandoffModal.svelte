<script lang="ts">
  // « Passer à un collègue… » (TS-17, `src-tauri/src/teamhandoff.rs`): the ticket's agent goes to a teammate or to the
  // team's machine. The window says what goes before the gesture: the branch pushed and where (nothing is pushed
  // before « Passer »), the agent stopped between two turns (a turn under way interrupted), the uncommitted work as a
  // commit « en cours » or an encrypted patch, the conversation encrypted on the server until it is taken. Refused, it
  // says why and offers nothing.
  import { onMount } from 'svelte';
  import { t } from '../../lib/i18n';
  import { hueOf, initialOf } from '../../lib/sharing';
  import { app } from '../../lib/state.svelte';
  import { teamBoardApi } from '../../lib/teamboard';
  import { errorText } from '../../lib/teams.svelte';
  import type { HandoffPlan } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import Modal from '../modals/Modal.svelte';

  let { ticketId }: { ticketId: string } = $props();

  let plan = $state<HandoffPlan | null>(null);
  let failure = $state('');
  let busy = $state(false);
  let done = $state('');
  let to = $state('');
  let wip = $state<'commit' | 'patch'>('patch');

  onMount(async () => {
    try {
      plan = await teamBoardApi.handoffPlan(ticketId);
      to = plan.targets.find((x) => !x.machine)?.device ?? plan.targets[0]?.device ?? '';
    } catch (e) {
      failure = errorText(e);
    }
  });

  const people = $derived(plan?.targets.filter((x) => !x.machine) ?? []);
  const machines = $derived(plan?.targets.filter((x) => x.machine) ?? []);
  const chosen = $derived(plan?.targets.find((x) => x.device === to) ?? null);

  async function pass() {
    if (!plan || !chosen) return;
    busy = true;
    failure = '';
    try {
      await teamBoardApi.handoff(plan.ticketId, chosen.device, wip);
      done = t('teamBoard.handoff.done', { name: chosen.name });
    } catch (e) {
      failure = errorText(e);
    } finally {
      busy = false;
    }
  }

  const close = () => (app.modal = null);
</script>

<Modal title={plan ? t('teamBoard.handoff.title', { key: plan.key }) : t('teamBoard.handoff.menu')} width={560} onclose={close}>
  <div class="handoff" data-testid="team-handoff">
    {#if done}
      <p class="done" role="status" data-testid="team-handoff-done"><Icon name="check" size={15} /><span>{done}</span></p>
    {:else if !plan}
      <p class="muted">{failure || t('teamBoard.handoff.loading')}</p>
    {:else if plan.refusal}
      <p class="refusal" role="alert" data-testid="team-handoff-refusal"><Icon name="info" size={15} /><span>{plan.refusal}</span></p>
    {:else}
      <p class="lead">{t('teamBoard.handoff.lead')}</p>
      <fieldset class="to">
        <legend>{t('teamBoard.handoff.to')}</legend>
        {#if plan.targets.length === 0}
          <p class="muted">{t('teamBoard.handoff.none')}</p>
        {/if}
        {#each [{ list: people, head: t('teamBoard.handoff.people') }, { list: machines, head: t('teamBoard.handoff.machines') }] as group (group.head)}
          {#if group.list.length}
            <div class="group">
              <span class="head">{group.head}</span>
              {#each group.list as x (x.device)}
                <label class="who" class:on={to === x.device} data-testid="team-handoff-target" data-machine={x.machine}>
                  <input type="radio" name="handoff-to" value={x.device} bind:group={to} />
                  {#if x.machine}
                    <span class="mark"><Icon name="server" size={14} /></span>
                  {:else}
                    <i class="disc" style:--h={hueOf(x.device)} aria-hidden="true">{initialOf(x.name || '?')}</i>
                  {/if}
                  <span class="name">{x.name}</span>
                  {#if x.machine}<span class="hint">{t('teamBoard.handoff.machineHint')}</span>
                  {:else if x.mine}<span class="hint">{t('teamBoard.handoff.mineDevice')}</span>{/if}
                </label>
              {/each}
            </div>
          {/if}
        {/each}
      </fieldset>
      <ul class="facts">
        <li data-testid="team-handoff-push">
          <Icon name="branch" size={14} />
          <span><b>{t('teamBoard.handoff.push', { branch: plan.branch, remote: plan.remote })}</b> {t('teamBoard.handoff.pushHint')}</span>
        </li>
        <li data-testid="team-handoff-turn">
          <Icon name={plan.running ? 'stop' : 'pause'} size={14} />
          <span>{plan.running ? t('teamBoard.handoff.stop') : t('teamBoard.handoff.stopped')}</span>
        </li>
        <li>
          <Icon name="lock" size={14} />
          <span>{t('teamBoard.handoff.conversation')}</span>
        </li>
        {#if plan.session}
          <li data-testid="team-handoff-session">
            <Icon name="lock" size={14} />
            <span>{t('teamBoard.handoff.session')}</span>
          </li>
        {/if}
      </ul>
      <fieldset class="wip" data-testid="team-handoff-wip">
        <legend>{plan.dirty.length ? t('teamBoard.handoff.dirty', { count: plan.dirty.length }) : t('teamBoard.handoff.clean')}</legend>
        {#if plan.dirty.length}
          <ul class="files mono">
            {#each plan.dirty.slice(0, 6) as f (f)}<li>{f}</li>{/each}
            {#if plan.dirty.length > 6}<li>…</li>{/if}
          </ul>
          <label class="opt" class:on={wip === 'patch'}>
            <input type="radio" name="handoff-wip" value="patch" bind:group={wip} data-testid="team-handoff-patch" />
            <span><b>{t('teamBoard.handoff.wipPatch')}</b><span class="hint">{t('teamBoard.handoff.wipPatchHint')}</span></span>
          </label>
          <label class="opt" class:on={wip === 'commit'}>
            <input type="radio" name="handoff-wip" value="commit" bind:group={wip} data-testid="team-handoff-commit" />
            <span><b>{t('teamBoard.handoff.wipCommit')}</b><span class="hint">{t('teamBoard.handoff.wipCommitHint')}</span></span>
          </label>
        {/if}
      </fieldset>
      {#if failure}<p class="fail" role="alert" data-testid="team-handoff-failure">{failure}</p>{/if}
    {/if}
  </div>
  {#snippet footer()}
    {#if done || !plan || plan.refusal}
      <button class="btn" onclick={close}>{t('teamBoard.handoff.close')}</button>
    {:else}
      <button class="btn ghost" onclick={close}>{t('common.cancel')}</button>
      <button class="btn primary" disabled={busy || !chosen} onclick={pass} data-testid="team-handoff-submit"
        >{busy ? t('teamBoard.handoff.passing') : t('teamBoard.handoff.pass')}</button
      >
    {/if}
  {/snippet}
</Modal>

<style>
  .handoff {
    display: flex;
    flex-direction: column;
    gap: 12px;
    font-size: 13px;
  }
  .lead {
    margin: 0;
    color: var(--soft);
    line-height: 1.5;
  }
  fieldset {
    margin: 0;
    padding: 0;
    border: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  legend {
    padding: 0;
    margin-bottom: 6px;
    font-weight: 600;
    font-size: 12.5px;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .group + .group {
    margin-top: 6px;
  }
  .head {
    color: var(--muted);
    font-size: 11.5px;
  }
  .who,
  .opt {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 7px 10px;
    border-radius: 8px;
    box-shadow: inset 0 0 0 1px var(--line);
    cursor: pointer;
    min-width: 0;
  }
  .who.on,
  .opt.on {
    box-shadow: inset 0 0 0 1.5px var(--p);
    background: var(--p-soft);
  }
  .who input,
  .opt input {
    margin: 0;
    flex: none;
  }
  .opt {
    align-items: flex-start;
  }
  .opt > span {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .name {
    font-weight: 550;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hint {
    color: var(--muted);
    font-size: 12px;
    min-width: 0;
  }
  .who .hint {
    margin-left: auto;
    text-align: right;
  }
  .disc,
  .mark {
    flex: none;
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
  }
  .disc {
    background: light-dark(oklch(0.9 0.05 var(--h)), oklch(0.42 0.08 var(--h)));
    color: light-dark(oklch(0.36 0.1 var(--h)), oklch(0.97 0.02 var(--h)));
    font: normal 600 10.5px/1 var(--font);
  }
  .mark {
    background: rgba(var(--fg-rgb), 0.06);
    color: var(--soft);
  }
  .facts {
    list-style: none;
    margin: 0;
    padding: 10px 12px;
    border-radius: 8px;
    background: rgba(var(--fg-rgb), 0.035);
    display: flex;
    flex-direction: column;
    gap: 7px;
    color: var(--soft);
    line-height: 1.45;
  }
  .facts li {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr);
    gap: 8px;
    align-items: start;
    overflow-wrap: anywhere;
  }
  .facts :global(.icon) {
    margin-top: 2px;
    color: var(--muted);
  }
  .facts b {
    color: var(--fg);
    font-weight: 550;
  }
  .files {
    list-style: none;
    margin: 0 0 4px;
    padding: 0;
    font-size: 12px;
    color: var(--muted);
    white-space: pre;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .refusal,
  .done {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr);
    gap: 8px;
    margin: 0;
    padding: 10px 12px;
    border-radius: 8px;
    line-height: 1.45;
  }
  .refusal {
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .refusal :global(.icon) {
    color: var(--wait);
    margin-top: 2px;
  }
  .done {
    background: var(--p-soft);
  }
  .muted {
    color: var(--muted);
    margin: 0;
  }
  .fail {
    color: var(--danger, var(--wait));
    margin: 0;
  }
</style>
