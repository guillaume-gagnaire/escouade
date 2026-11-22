<script lang="ts">
  // An invitation's link, to copy, and its QR code (drawn by the backend, dark on light whatever the theme).
  import { t } from '../../lib/i18n';
  import type { InviteView } from '../../lib/teams.svelte';

  let { invite, label, qrLabel }: { invite: InviteView; label: string; qrLabel: string } = $props();

  let copied = $state(false);

  async function copy() {
    try {
      await navigator.clipboard.writeText(invite.url);
      copied = true;
    } catch {
      copied = false;
    }
  }
</script>

<div class="link">
  <span class="lbl">{label}</span>
  <div class="line">
    <input class="field mono" readonly value={invite.url} aria-label={label} onfocus={(e) => e.currentTarget.select()} />
    <button class="btn" onclick={copy}>{copied ? t('team.create.copied') : t('team.create.copy')}</button>
  </div>
  <span class="paste">{t('team.invite.paste')}</span>
</div>
{#if invite.qr}
  <figure class="qr">
    <!-- The backend's SVG of the link: no script, no reference outside. -->
    <div class="code" role="img" aria-label={qrLabel}>{@html invite.qr}</div>
    <figcaption>{qrLabel}</figcaption>
  </figure>
{/if}

<style>
  .link {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .lbl {
    color: var(--soft);
    font-size: 12.5px;
    font-weight: 560;
  }
  .line {
    display: flex;
    gap: 8px;
  }
  .paste {
    color: var(--dim);
    font-size: 12px;
  }
  .line input {
    flex: 1;
    min-width: 0;
    font-size: 12px;
  }
  .qr {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    margin: 0;
  }
  .code {
    width: 220px;
    height: 220px;
    padding: 8px;
    border-radius: 10px;
    background: #fff;
  }
  .code :global(svg) {
    display: block;
    width: 100%;
    height: 100%;
  }
  figcaption {
    color: var(--dim);
    font-size: 12px;
  }
</style>
