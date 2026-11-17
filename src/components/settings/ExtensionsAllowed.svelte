<script lang="ts">
  // « Extensions permises de l'éditeur » (ESC-481): VS Code's `extensions.allowed` of this device, as JSON. The core holds
  // it (an extension refused is not even downloaded) and crosses it with each project's `.escouade.yml`, the team's and
  // the managed device's list. Empty: every extension allowed. Greyed with who imposes it when a policy does (its row).
  import { t } from '../../lib/i18n';
  import { settingsForm } from '../../lib/settings.svelte';

  const EXAMPLE = '{"*": false, "dracula-theme": true}';
  const s = $derived(settingsForm.settings);
  const shown = (v: unknown) => (v && typeof v === 'object' && Object.keys(v).length ? JSON.stringify(v, null, 2) : '');
  let text = $state(shown(settingsForm.settings.extensionsAllowed));
  let error = $state('');

  /** The text read: an object, or empty (null); anything else is said and not kept. */
  function take() {
    const raw = text.trim();
    if (!raw) {
      error = '';
      s.extensionsAllowed = null;
      return;
    }
    try {
      const v = JSON.parse(raw);
      if (!v || typeof v !== 'object' || Array.isArray(v)) throw new Error('{ … }');
      error = '';
      s.extensionsAllowed = v;
    } catch (e) {
      error = t('settings.extensions.invalid', { error: e instanceof Error ? e.message : String(e) });
    }
  }
</script>

<div class="allowed">
  <textarea
    class="field mono"
    data-testid="extensions-allowed"
    aria-label={t('settings.extensions.title')}
    spellcheck="false"
    rows="4"
    placeholder={t('settings.extensions.placeholder', { example: EXAMPLE })}
    bind:value={text}
    onblur={take}></textarea>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</div>

<style>
  .allowed {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
  }
  textarea {
    width: 100%;
    min-height: 72px;
    resize: vertical;
    font-family: var(--mono);
    font-size: 12px;
  }
  .error {
    margin: 0;
    color: var(--del);
    font-size: 12px;
  }
</style>
