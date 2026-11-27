<script lang="ts">
  // A space that revoked this phone (MO-4, docs/MOBILE.md § 3.10): « Ce téléphone n'est plus appairé à Travail », when
  // and from which device (the trace of the revoked, `remote-revoked.json`), « Appairer de nouveau » and « Retirer
  // Travail de ce téléphone ». Its keys and token are forgotten either way; the other spaces stay as they are.
  import Icon from '$shared/components/Icon.svelte';
  import { t } from '$shared/lib/i18n';
  import { openPairing, removeSpace } from '../lib/pairing.svelte';
  import type { PhoneSpace } from '../lib/phone-spaces.svelte';

  let { space }: { space: PhoneSpace } = $props();
  let busy = $state(false);

  const revoked = $derived(space.conn.refusal?.revoked ?? null);
  const date = $derived(revoked ? new Date(revoked.at).toLocaleString(undefined, { dateStyle: 'long', timeStyle: 'short' }) : '');

  async function forget(again: boolean) {
    if (busy) return;
    busy = true;
    try {
      // Revoked: the core knows it no more, nothing to ask it.
      await removeSpace(space.id, false);
      if (again) openPairing();
    } finally {
      busy = false;
    }
  }
</script>

<div class="revoked" role="alertdialog" aria-modal="true" aria-labelledby="revoked-title" aria-describedby="revoked-text">
  <span class="mark"><Icon name="unlink" size={28} /></span>
  <h1 id="revoked-title">{t('mobile.revoked.title', { name: space.name })}</h1>
  <p id="revoked-text">
    {#if revoked}
      {revoked.by ? t('mobile.revoked.when', { date, by: revoked.by }) : t('mobile.revoked.whenNoBy', { date })}
    {:else}
      {t('mobile.revoked.unknown', { name: space.name })}
    {/if}
  </p>
  <p class="others">{t('mobile.revoked.others')}</p>
  <div class="foot">
    <button type="button" class="go" disabled={busy} onclick={() => forget(true)}>{t('mobile.revoked.again')}</button>
    <button type="button" class="alt" disabled={busy} onclick={() => forget(false)}>
      {t('mobile.revoked.remove', { name: space.name })}
    </button>
  </div>
</div>

<style>
  .revoked {
    position: fixed;
    inset: 0;
    z-index: 20;
    max-width: 600px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: calc(40px + var(--safe-top)) calc(var(--gut) + var(--safe-right)) calc(20px + var(--safe-bottom))
      calc(var(--gut) + var(--safe-left));
    background: var(--panel);
  }
  .mark {
    width: 56px;
    height: 56px;
    border-radius: var(--r-l);
    display: grid;
    place-items: center;
    color: var(--warn-ink);
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  h1 {
    margin: 4px 0 0;
    font-size: 26px;
    font-weight: 700;
    letter-spacing: -0.025em;
    line-height: 1.15;
  }
  p {
    margin: 0;
    font-size: var(--body);
    line-height: 1.5;
    color: var(--soft);
  }
  .others {
    color: var(--muted);
    font-size: var(--small);
  }
  .foot {
    margin-top: auto;
    display: grid;
    gap: var(--gap-target);
  }
  .go,
  .alt {
    min-height: calc(var(--target) + 6px);
    border: 0;
    border-radius: var(--r-l);
    font-size: 16px;
    font-weight: 650;
    cursor: pointer;
  }
  .go {
    background: var(--primary);
    color: var(--on-primary);
  }
  .alt {
    background: transparent;
    color: var(--p-ink);
  }
</style>
