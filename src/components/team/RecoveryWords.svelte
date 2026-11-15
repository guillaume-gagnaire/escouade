<script lang="ts">
  // The recovery key (C7-B): its 24 words shown once, then three of them typed back to prove they were written down.
  import { untrack } from 'svelte';
  import { t } from '../../lib/i18n';
  import { pickPositions } from '../../lib/team-words';
  import { errorText, teams, teamsApi } from '../../lib/teams.svelte';

  let { teamId, words, ondone }: { teamId: string; words: string[]; ondone: () => void } = $props();

  let confirming = $state(false);
  let noted = $state(false);
  const positions = untrack(() => pickPositions(words.length || 24, 3));
  let typed = $state<string[]>(['', '', '']);
  let failure = $state('');
  let busy = $state(false);

  async function confirm() {
    busy = true;
    failure = '';
    try {
      const ok = await teamsApi.confirmWords(
        teamId,
        positions.map((p, i): [number, string] => [p, typed[i].trim()]),
      );
      if (!ok) {
        failure = t('team.words.wrong');
        return;
      }
      await teams.load();
      ondone();
    } catch (e) {
      failure = errorText(e);
    } finally {
      busy = false;
    }
  }
</script>

{#if !confirming}
  <p class="lead">{t('team.words.lead')}</p>
  <ol class="grid mono" aria-label={t('team.title.words')}>
    {#each words as w, i (i)}<li><span class="n">{i + 1}</span><span class="w">{w}</span></li>{/each}
  </ol>
  <p class="hint">{t('team.words.where')}</p>
  <label class="check"><input type="checkbox" bind:checked={noted} /><span>{t('team.words.noted')}</span></label>
  <div class="foot">
    <button class="btn primary" disabled={!noted} onclick={() => (confirming = true)}>{t('team.words.next')}</button>
  </div>
{:else}
  <p class="lead">{t('team.words.confirmLead')}</p>
  <form class="typed" onsubmit={(e) => (e.preventDefault(), void confirm())}>
    {#each positions as p, i (p)}
      <label class="row">
        <span class="lbl">{t('team.words.word', { n: p + 1 })}</span>
        <!-- svelte-ignore a11y_autofocus -->
        <input class="field mono" bind:value={typed[i]} autocomplete="off" spellcheck="false" autofocus={i === 0} />
      </label>
    {/each}
    {#if failure}<p class="error" role="alert">{failure}</p>{/if}
    <div class="foot">
      <button type="button" class="btn ghost" onclick={() => (confirming = false)}>{t('team.words.back')}</button>
      <button type="submit" class="btn primary" disabled={busy || typed.some((w) => !w.trim())}>{t('team.words.confirm')}</button>
    </div>
  </form>
{/if}

<style>
  .lead,
  .hint,
  .error {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
  }
  .lead {
    color: var(--soft);
  }
  .hint {
    color: var(--dim);
  }
  .error {
    color: var(--del);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 6px;
    margin: 0;
    padding: 12px;
    border-radius: 10px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line-2);
    list-style: none;
  }
  @media (max-width: 560px) {
    .grid {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }
  .grid li {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 5px 8px;
    border-radius: 6px;
    background: rgba(var(--fg-rgb), 0.04);
    font-size: 13px;
  }
  .n {
    min-width: 1.6em;
    color: var(--dim);
    font-size: 11px;
    text-align: right;
  }
  .w {
    color: var(--text);
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--soft);
    font-size: 13px;
  }
  .typed {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .lbl {
    color: var(--soft);
    font-size: 12.5px;
    font-weight: 560;
  }
  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
