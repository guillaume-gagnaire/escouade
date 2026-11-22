<script lang="ts">
  // A band for each space shown that is cut (MO-1, docs/MOBILE.md § 5): its name, the next try and « Réessayer »; a
  // space that refuses this phone says why (its token revoked, another version). The other spaces go on.
  import Icon from '$shared/components/Icon.svelte';
  import { t } from '$shared/lib/i18n';
  import { phone, retrySpace, shownSpaces, stateOf } from '../lib/phone-spaces.svelte';

  const cut = $derived(shownSpaces(phone.spaces, phone.filter).filter((s) => ['cut', 'refused'].includes(stateOf(s))));
  let now = $state(Date.now());

  $effect(() => {
    if (!cut.length) return;
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });

  function textOf(s: (typeof phone.spaces)[number]): string {
    if (stateOf(s) === 'refused') return t('mobile.band.refused', { name: s.name, reason: s.conn.reason });
    const at = s.conn.retryAt;
    const seconds = at ? Math.ceil((at - now) / 1000) : 0;
    return seconds > 0 ? t('mobile.band.cut', { name: s.name, seconds }) : t('mobile.band.cutNow', { name: s.name });
  }
</script>

{#each cut as s (s.id)}
  <div class="band" role="status" data-space={s.id}>
    <Icon name="unlink" size={16} />
    <span class="tx">{textOf(s)}</span>
    {#if stateOf(s) === 'cut'}
      <button type="button" onclick={() => retrySpace(s.id)}>{t('mobile.band.retry')}</button>
    {/if}
  </div>
{/each}

<style>
  .band {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 4px var(--gut) 6px;
    padding: 6px 6px 6px 12px;
    border-radius: var(--r-m);
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
    color: var(--wait);
    font-size: var(--small);
    font-weight: 600;
  }
  .tx {
    flex: 1;
    min-width: 0;
    color: var(--text);
  }
  button {
    flex: none;
    min-height: var(--target);
    padding: 0 12px;
    border: 0;
    border-radius: 10px;
    background: transparent;
    color: var(--wait);
    font-weight: 650;
    cursor: pointer;
  }
</style>
