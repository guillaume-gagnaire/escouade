<script lang="ts">
  // The pairing of a space (MO-4, docs/MOBILE.md § 3.1, direction B « le fil »): its two steps (on the computer, then
  // scan or type), what the QR code holds once read (« Ce poste · DESKTOP-7QK2 · Escouade 2.3.0 ») and the phone's
  // name on that space, each failure with its words, and the phone without a screen lock that cannot pair (M3). With
  // no space, the app's first screen; else above the shell (« Appairer un espace »).
  import Icon from '$shared/components/Icon.svelte';
  import { t } from '$shared/lib/i18n';
  import { closePairing, nameOf, openPairing, pairing, pairNow, scanQr, typeCode } from '../lib/pairing.svelte';
  import { phone } from '../lib/phone-spaces.svelte';

  let code = $state('');
  let address = $state('');
  let busy = $state(false);

  const ready = $derived(code.replace(/[^0-9a-z]/gi, '').length === 8 && address.trim().length > 0);
  const overShell = $derived(phone.spaces.length > 0);

  async function run(f: () => Promise<unknown>) {
    if (busy) return;
    busy = true;
    try {
      await f();
    } finally {
      busy = false;
    }
  }

  function onCode(e: Event) {
    // Two groups of four, the dash put by the screen; letters in capitals.
    const letters = (e.currentTarget as HTMLInputElement).value
      .replace(/[^0-9a-z]/gi, '')
      .toUpperCase()
      .slice(0, 8);
    code = letters.length > 4 ? `${letters.slice(0, 4)}-${letters.slice(4)}` : letters;
  }

  const found = $derived(pairing.found);
</script>

<section class="pair" aria-labelledby="pair-title">
  {#if overShell}
    <button type="button" class="back" onclick={closePairing}><Icon name="left" size={18} />{t('mobile.pair.back')}</button>
  {/if}

  {#if pairing.step === 'nolock'}
    <span class="mark warn"><Icon name="lock" size={28} /></span>
    <h1 id="pair-title">{t('mobile.pair.noLock')}</h1>
    <p class="lead">{t('mobile.pair.noLockText')}</p>
    <div class="foot">
      <button type="button" class="go" onclick={openPairing}>{t('mobile.pair.retry')}</button>
    </div>
  {:else if pairing.step === 'found' || pairing.step === 'pairing'}
    <span class="mark"><Icon name={found?.here ? 'monitor' : 'remote'} size={28} /></span>
    <h1 id="pair-title">{t('mobile.pair.title')}</h1>
    <p class="found" data-testid="pair-found">
      {#if found?.kind === 'tunnel'}
        {t('mobile.pair.found', { name: nameOf(found), machine: found.name ?? '', version: found.version ?? '' })}
      {:else}
        {t('mobile.pair.foundDirect', { address: found?.endpoint ?? '' })}
      {/if}
    </p>
    <label class="fld">
      <span>{t('mobile.pair.nameLabel')}</span>
      <input type="text" maxlength="64" autocomplete="off" bind:value={pairing.name} disabled={pairing.step === 'pairing'} />
    </label>
    <div class="foot">
      <button type="button" class="go" disabled={pairing.step === 'pairing'} onclick={() => run(pairNow)}>
        {pairing.step === 'pairing' ? t('mobile.pair.pairing') : t('mobile.pair.go')}
      </button>
      <button type="button" class="alt" disabled={pairing.step === 'pairing'} onclick={openPairing}>{t('mobile.pair.cancel')}</button>
    </div>
  {:else if pairing.step === 'failed'}
    <span class="mark warn"><Icon name="warn" size={28} /></span>
    <h1 id="pair-title">{t('mobile.pair.failed')}</h1>
    <p class="lead" role="alert">{pairing.error}</p>
    <div class="foot">
      {#if pairing.again === 'scan'}
        <button type="button" class="go" onclick={() => run(scanQr)}>{t('mobile.pair.scanAgain')}</button>
      {:else if pairing.again === 'retry' && found}
        <button type="button" class="go" onclick={() => run(pairNow)}>{t('mobile.pair.retry')}</button>
      {/if}
      <button type="button" class="alt" onclick={openPairing}>{t('mobile.pair.back')}</button>
    </div>
  {:else}
    <h1 id="pair-title">{t('mobile.welcome.title')}</h1>
    <p class="lead">{t('mobile.welcome.text')}</p>
    <ol class="steps">
      <li>
        <span class="n">1</span>
        <span><b>{t('mobile.welcome.step1')}</b>{t('mobile.welcome.step1Text')}</span>
      </li>
      <li>
        <span class="n on">2</span>
        <span><b>{t('mobile.welcome.step2')}</b>{t('mobile.welcome.step2Text')}</span>
      </li>
    </ol>
    <button type="button" class="scan" disabled={busy} onclick={() => run(scanQr)}>
      <svg viewBox="0 0 24 24" width="28" height="28" aria-hidden="true">
        <path
          d="M3 8V4a1 1 0 0 1 1-1h4M16 3h4a1 1 0 0 1 1 1v4M21 16v4a1 1 0 0 1-1 1h-4M8 21H4a1 1 0 0 1-1-1v-4M7 7h4v4H7zM13 13h4v4h-4zM13 7h4M7 13v4h4"
          fill="none"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linejoin="round"
        />
      </svg>
      {t('mobile.welcome.scan')}
    </button>
    <form
      class="manual"
      onsubmit={(e) => {
        e.preventDefault();
        if (ready) run(() => typeCode(code, address));
      }}
    >
      <label class="fld">
        <span>{t('mobile.welcome.orCode')}</span>
        <input
          class="code"
          type="text"
          inputmode="text"
          autocapitalize="characters"
          autocomplete="one-time-code"
          spellcheck="false"
          placeholder="ABCD-EFGH"
          aria-label={t('mobile.welcome.code')}
          value={code}
          oninput={onCode}
        />
      </label>
      <label class="fld">
        <span>{t('mobile.welcome.address')}</span>
        <input type="text" inputmode="url" autocapitalize="off" spellcheck="false" placeholder="100.64.1.2:7878" bind:value={address} />
        <small>{t('mobile.welcome.addressHint')}</small>
      </label>
      <button type="submit" class="go" disabled={!ready || busy}>{t('mobile.welcome.pair')}</button>
    </form>
  {/if}
</section>

<style>
  .pair {
    min-height: 100%;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: calc(20px + var(--safe-top)) calc(var(--gut) + var(--safe-right)) calc(20px + var(--safe-bottom))
      calc(var(--gut) + var(--safe-left));
    background: var(--panel);
  }
  .back {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-height: var(--target);
    margin-left: -8px;
    padding: 0 8px;
    border: 0;
    border-radius: var(--r-m);
    background: transparent;
    color: var(--p-ink);
    font-size: var(--body);
    font-weight: 600;
    cursor: pointer;
  }
  .mark {
    width: 56px;
    height: 56px;
    border-radius: var(--r-l);
    display: grid;
    place-items: center;
    color: var(--teal-ink);
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .mark.warn {
    color: var(--warn-ink);
  }
  h1 {
    margin: 4px 0 0;
    font-size: 28px;
    font-weight: 700;
    letter-spacing: -0.025em;
    line-height: 1.1;
  }
  .lead,
  .found {
    margin: 0;
    font-size: var(--body);
    line-height: 1.5;
    color: var(--soft);
  }
  .found {
    font-weight: 600;
    color: var(--text);
  }
  .steps {
    list-style: none;
    margin: 4px 0 0;
    padding: 0;
    display: grid;
    gap: 14px;
  }
  .steps li {
    display: flex;
    gap: 14px;
    align-items: flex-start;
    color: var(--muted);
    line-height: 1.45;
  }
  .steps b {
    display: block;
    color: var(--text);
    font-weight: 650;
  }
  .n {
    flex: none;
    width: 32px;
    height: 32px;
    border-radius: 9px;
    display: grid;
    place-items: center;
    font-weight: 650;
    box-shadow: inset 0 0 0 1px var(--line-3);
    color: var(--soft);
  }
  .n.on {
    background: var(--primary);
    color: var(--on-primary);
    box-shadow: none;
  }
  .scan {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    min-height: 88px;
    border: 0;
    border-radius: var(--r-l);
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--teal-ink);
    font-size: 17px;
    font-weight: 650;
    cursor: pointer;
  }
  .scan:active {
    background: var(--raise-2);
  }
  .manual {
    display: grid;
    gap: 12px;
  }
  .fld {
    display: grid;
    gap: 6px;
    font-size: var(--small);
    font-weight: 600;
    color: var(--muted);
  }
  .fld small {
    font-weight: 400;
    line-height: 1.4;
  }
  input {
    min-height: var(--target);
    padding: 0 12px;
    border: 0;
    border-radius: var(--r-m);
    background: var(--raise);
    box-shadow: inset 0 0 0 1px var(--line-2);
    color: var(--text);
    font: inherit;
    font-size: var(--body);
    font-weight: 500;
  }
  input:focus-visible {
    outline: 2px solid var(--p-ink);
    outline-offset: 1px;
  }
  .code {
    font-family: var(--mono);
    font-size: 20px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
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
  .go:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .alt {
    background: transparent;
    color: var(--p-ink);
  }
</style>
