<script lang="ts">
  // « Plus tard » (MO-5, `docs/INBOX.md` § 8.4): the times the desktop offers, from now: in an hour, this evening,
  // tomorrow at 9:00, Monday at 9:00.
  import { t } from '$shared/lib/i18n';
  import { laterTimes } from '$shared/lib/inbox/rules';

  let { onPick, onCancel, disabled = false }: { onPick: (at: number) => void; onCancel: () => void; disabled?: boolean } = $props();
  const times = laterTimes();
</script>

<div class="later" role="group" aria-label={t('inbox.later.menu')}>
  <span class="lb">{t('inbox.later.menu')}</span>
  {#each times as x (x.key)}
    <button type="button" {disabled} onclick={() => onPick(x.at)}>{t(`inbox.later.${x.key}`)}</button>
  {/each}
  <button type="button" class="cancel" onclick={onCancel}>{t('inbox.later.cancel')}</button>
</div>

<style>
  .later {
    display: flex;
    flex-wrap: wrap;
    gap: var(--gap-target);
    align-items: center;
  }
  .lb {
    flex-basis: 100%;
    font-size: var(--small);
    color: var(--muted);
  }
  button {
    min-height: var(--target);
    padding: 0 14px;
    border: 0;
    border-radius: 999px;
    background: rgba(var(--fg-rgb), 0.06);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font: inherit;
    font-size: 14px;
    font-weight: 600;
  }
  .cancel {
    background: transparent;
    box-shadow: none;
    color: var(--muted);
  }
</style>
