<script lang="ts">
  // The phone paired with no space yet (MO-1): what the app is for, and that the pairing by QR code is coming. MO-4
  // adds the QR code read by the camera and the 8-character code.
  import Icon from '$shared/components/Icon.svelte';
  import { t } from '$shared/lib/i18n';
</script>

<section class="welcome" aria-labelledby="welcome-title">
  <span class="mark"><Icon name="phone" size={30} /></span>
  <h1 id="welcome-title">{t('mobile.welcome.title')}</h1>
  <p>{t('mobile.welcome.text')}</p>
  <p class="how">{t('mobile.welcome.how')}</p>
</section>

<style>
  .welcome {
    min-height: 100%;
    display: grid;
    align-content: center;
    justify-items: start;
    gap: 12px;
    padding: calc(24px + var(--safe-top)) calc(var(--gut) + 8px + var(--safe-right)) calc(24px + var(--safe-bottom))
      calc(var(--gut) + 8px + var(--safe-left));
  }
  .mark {
    width: 60px;
    height: 60px;
    border-radius: var(--r-l);
    display: grid;
    place-items: center;
    color: var(--teal-ink);
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  h1 {
    margin: 8px 0 0;
    font-size: 28px;
    font-weight: 700;
    letter-spacing: -0.025em;
    line-height: 1.1;
  }
  p {
    margin: 0;
    max-width: 36ch;
    font-size: var(--body);
    line-height: 1.5;
    color: var(--soft);
  }
  .how {
    font-size: var(--small);
    color: var(--muted);
  }
</style>
