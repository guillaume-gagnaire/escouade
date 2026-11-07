<script lang="ts">
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { onlineVoice } from '../../lib/online-voice.svelte';
  import { app } from '../../lib/state.svelte';
  import Icon from '../Icon.svelte';
  import Row from './Row.svelte';

  // The key of a service on line, kept in the keychain at once (never shown back): one by service, the dictation's
  // and the conversation's voice's (ESC-132). Saved, the service's models and voices are read with it.
  let { service }: { service: { id: string; label: string } } = $props();
  let draft = $state('');
  let replacing = $state(false);
  // Mistral's may be a Mistral Vibe account's (ESC-327): the voice goes by it, it is not this row's to erase.
  const borrowed = $derived(service.id === 'mistral' ? onlineVoice.mistral.borrowed : null);
  const has = $derived(onlineVoice.keys.includes(service.id) && !borrowed);
  const desc = $derived(service.id === 'mistral' ? t('voice.settings.keyMistralDesc') : t('voice.settings.keyDesc'));

  // Another service: its own key.
  $effect(() => {
    void service.id;
    draft = '';
    replacing = false;
  });

  async function save() {
    const { id, label } = service;
    try {
      await api.voiceOnlineKeySet(id, draft);
      draft = '';
      replacing = false;
      app.toast(t('voice.settings.keySavedToast', { service: label }));
    } catch (e) {
      app.toast(t('voice.settings.keyFailed', { error: String(e) }), 'error');
    }
    await onlineVoice.loadKeys();
    if (onlineVoice.keys.includes(id)) void onlineVoice.load(id, true);
  }

  async function remove() {
    try {
      await api.voiceOnlineKeyDelete(service.id);
    } catch (e) {
      app.toast(String(e), 'error');
    }
    await onlineVoice.loadKeys();
  }
</script>

<Row
  label={t('voice.settings.key', { service: service.label })}
  desc={has || borrowed ? desc : `${t('voice.settings.keyMissing')} ${desc}`}
>
  <div class="keys" data-online-key={service.id}>
    {#if borrowed && !replacing}
      <span class="ok"
        ><Icon name="check" size={14} />{t('voice.settings.keyBorrowed', { account: borrowed.account, hint: borrowed.hint })}</span
      >
      <button class="btn" onclick={() => (replacing = true)}>{t('voice.settings.keyGiveOther')}</button>
    {:else if has && !replacing}
      <span class="ok"><Icon name="check" size={14} />{t('voice.settings.keySaved')}</span>
      <button class="btn" onclick={() => (replacing = true)}>{t('voice.settings.keyReplace')}</button>
      <button class="btn" onclick={remove}>{t('voice.settings.keyDelete')}</button>
    {:else}
      <input
        class="field input mono"
        type="password"
        aria-label={t('voice.settings.key', { service: service.label })}
        placeholder={t('voice.settings.keyPlaceholder')}
        autocomplete="off"
        spellcheck="false"
        bind:value={draft}
        onkeydown={(e) => e.key === 'Enter' && draft.trim() && (e.preventDefault(), void save())}
      />
      <button class="btn" disabled={!draft.trim()} onclick={save}>{t('voice.settings.keySave')}</button>
    {/if}
  </div>
</Row>

<style>
  .keys {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }
  .input {
    width: 340px;
    max-width: 100%;
    flex: none;
  }
  .ok {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--add);
    font-size: 12px;
  }
</style>
