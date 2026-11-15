<script lang="ts">
  // The team made: its six words, the advice of a second administrator, and the id the server's host needs for a
  // recovery token, the day it is needed.
  import { t } from '../../lib/i18n';
  import type { TeamView } from '../../lib/teams.svelte';
  import SixWords from './SixWords.svelte';

  let { team, oninvite, ondone }: { team: TeamView; oninvite: () => void; ondone: () => void } = $props();
</script>

<p class="lead">{t('team.created.lead', { name: team.name, server: team.server })}</p>
<SixWords words={team.fingerprint} label={t('team.card.fingerprint')} hint={t('team.created.fingerprint')} />
{#if team.escrow}
  <p class="escrow" role="note">{t('team.card.escrow', { holder: team.escrow })}</p>
{/if}
<p class="advice">{t('team.created.secondAdmin')}</p>
<div class="id">
  <span class="lbl">{t('team.card.id')}</span>
  <code class="mono">{team.id}</code>
  <span class="hint">{t('team.created.recoveryToken', { id: team.id })}</span>
</div>
<div class="foot">
  <button class="btn ghost" onclick={ondone}>{t('team.created.done')}</button>
  <button class="btn primary" onclick={oninvite}>{t('team.created.invite')}</button>
</div>

<style>
  .lead,
  .advice,
  .escrow {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
  }
  .lead {
    color: var(--soft);
  }
  .advice {
    color: var(--warn-ink);
  }
  .escrow {
    padding: 9px 12px;
    border-radius: 8px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--text);
  }
  .id {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .lbl {
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 560;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }
  code {
    color: var(--text);
    font-size: 12.5px;
    word-break: break-all;
  }
  .hint {
    color: var(--dim);
    font-size: 12px;
    line-height: 1.45;
  }
  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
