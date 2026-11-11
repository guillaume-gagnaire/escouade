<script lang="ts">
  import { t } from '../../lib/i18n';
  import { settingsForm } from '../../lib/settings.svelte';

  // What a part of the settings says when no installed plugin brings what it lists (ESC-407): one line, and the way to
  // Réglages › Plugins, where plugins are installed; never a button that installs from here.
  let { text }: { text: string } = $props();
</script>

<p class="noplugin" data-no-plugin>
  <span>{text}</span>
  <button type="button" class="to-plugins" onclick={() => (settingsForm.page = 'plugins')}>{t('settings.noPlugin.open')}</button>
</p>

<style>
  .noplugin {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 8px;
    margin: 8px 0 0;
    font-size: 12.5px;
    color: var(--muted);
  }
  .to-plugins {
    padding: 0;
    border: none;
    background: none;
    color: var(--p-ink);
    font: inherit;
    cursor: pointer;
  }
  .to-plugins:hover {
    color: var(--text);
  }
</style>
