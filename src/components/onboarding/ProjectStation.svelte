<script lang="ts">
  import { t } from '../../lib/i18n';
  import type { Primary } from '../../lib/onboarding.svelte';
  import { app } from '../../lib/state.svelte';
  import Icon from '../Icon.svelte';
  import StationHead from './StationHead.svelte';

  // Station 4, the first project (docs/ONBOARDING.md § 4.4), in its short form: a folder of the machine, by the
  // assistant « Nouveau projet » as it is (its other steps keep their defaults). The demonstration, the clone and the
  // projects of a team are ON-2's (ESC-505). A project open already (here or elsewhere) is said, and the station is done.
  let { primary = $bindable() }: { primary: Primary | null } = $props();

  const first = $derived(app.projects[0] ?? null);
  const add = () => (app.modal = { kind: 'newProject' });

  $effect(() => {
    primary = first
      ? { label: t('onboarding.continue'), run: async () => true }
      : {
          label: t('onboarding.project.add'),
          run: async () => {
            add();
            return false;
          },
        };
  });
</script>

<div class="q" data-testid="onboarding-project">
  <StationHead step="project" question={t('onboarding.project.question')}>
    {#snippet why()}{t('onboarding.project.why')}{/snippet}
  </StationHead>
  {#if first}
    <p class="done"><Icon name="check" size={14} />{t('onboarding.project.opened', { name: first.name })}</p>
  {:else}
    <div class="choice">
      <Icon name="folder" size={16} />
      <span class="t"><b>{t('onboarding.project.folder')}</b><span class="d">{t('onboarding.project.folderHint')}</span></span>
    </div>
  {/if}
</div>

<style>
  .q {
    display: grid;
    gap: 22px;
    width: min(640px, 100%);
  }
  .choice {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr);
    gap: 12px;
    padding: 12px 14px;
    border-radius: 10px;
    background: color-mix(in oklch, var(--brand) 7%, transparent);
    box-shadow: inset 0 0 0 1.5px var(--brand);
    color: var(--soft);
  }
  .t {
    display: grid;
    gap: 4px;
  }
  .t b {
    color: var(--text);
    font-weight: 620;
  }
  .d {
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.45;
  }
  .done {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    color: var(--soft);
  }
  .done :global(.icon) {
    color: var(--run);
  }
</style>
