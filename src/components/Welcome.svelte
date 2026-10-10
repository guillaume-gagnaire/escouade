<script lang="ts">
  import { t } from '../lib/i18n';
  import Rich from '../lib/i18n/Rich.svelte';
  import { app } from '../lib/state.svelte';
</script>

<div class="welcome">
  <div class="card">
    <span class="mark">C</span>
    <h1>Escouade</h1>
    <p>{t('shell.welcome.intro')}</p>
    {#if !app.claudeFound}
      <p class="warn">
        <Rich k="shell.welcome.claudeMissing"
          >{#snippet command()}<span class="mono">irm https://claude.ai/install.ps1 | iex</span>{/snippet}</Rich
        >
      </p>
    {/if}
    <button
      class="btn primary big"
      onclick={() => {
        app.modal = { kind: 'newProject' };
      }}>+ {t('shell.welcome.addProject')}</button
    >
  </div>
</div>

<style>
  .welcome {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .card {
    max-width: 460px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    text-align: center;
  }
  .mark {
    width: 44px;
    height: 44px;
    border-radius: 12px;
    background: var(--accent);
    color: var(--accent-ink);
    font-family: var(--mono);
    font-size: 22px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  h1 {
    margin: 6px 0 0;
    font-size: 22px;
  }
  p {
    margin: 0;
    color: var(--muted);
    line-height: 1.55;
  }
  .warn {
    color: var(--wait);
    font-size: 12.5px;
  }
  .big {
    margin-top: 10px;
    height: 36px;
    padding: 0 18px;
    font-size: 13px;
  }
</style>
