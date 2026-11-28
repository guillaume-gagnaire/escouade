<script lang="ts">
  // The microphone of a field (M9): the system's dictation, on the device; what it heard is added to the field, to read
  // again before it goes. Touched again while it listens (iOS), it stops; Android's sheet stops by itself.
  import Icon from '$shared/components/Icon.svelte';
  import { t } from '$shared/lib/i18n';
  import { dictate, stopDictation } from '../lib/voice';

  let { onHeard, disabled = false }: { onHeard: (text: string) => void; disabled?: boolean } = $props();
  let listening = $state(false);
  let failed = $state(false);

  async function go() {
    if (listening) {
      await stopDictation();
      return;
    }
    listening = true;
    failed = false;
    const heard = await dictate();
    listening = false;
    if (heard) onHeard(heard);
    else failed = true;
  }
</script>

<button
  type="button"
  class="mic"
  class:on={listening}
  {disabled}
  aria-label={listening ? t('mobile.talk.listening') : t('mobile.talk.dictate')}
  aria-pressed={listening}
  onclick={go}
>
  <Icon name="mic" size={20} />
</button>
{#if failed}<span class="sr" role="status">{t('mobile.talk.dictateFailed')}</span>{/if}

<style>
  .mic {
    flex: none;
    width: var(--target);
    height: var(--target);
    border: 0;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: transparent;
    color: var(--soft);
  }
  .mic.on {
    color: var(--panel);
    background: var(--teal-ink);
  }
  .mic:disabled {
    opacity: 0.4;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
