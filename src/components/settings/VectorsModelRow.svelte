<script lang="ts">
  import Icon from '../Icon.svelte';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { fBytes } from '../../lib/format';
  import { app } from '../../lib/state.svelte';
  import { tip } from '../../lib/tip';
  import type { VectorsModelInfo } from '../../lib/types';
  import Row from './Row.svelte';

  // « Recherche par le sens » (ESC-254): the vectors' model, one for every project of this machine, downloaded as a voice
  // is (Réglages › Voix): its progress, « Annuler » (what came is kept, « Reprendre » goes on from there), « Effacer ».
  let info = $state<VectorsModelInfo | null>(null);
  let busy = $state(false);

  async function load() {
    info = await api.vectorsModel().catch(() => null);
  }
  $effect(() => void load());

  const downloading = $derived(busy || !!info?.downloading);
  const progress = $derived(app.vectorsProgress);
  const pct = $derived(progress ? Math.round((progress.done / Math.max(1, progress.total)) * 100) : 0);
  /** What a download cut earlier left, in percent: offered as « Reprendre ». */
  const started = $derived(info && !info.downloaded && info.have > 0 ? Math.floor((info.have / Math.max(1, info.size)) * 100) : 0);

  async function download() {
    busy = true;
    app.vectorsProgress = { done: info?.have ?? 0, total: info?.size ?? 1 };
    const got = await app.run(api.vectorsModelDownload());
    busy = false;
    app.vectorsProgress = null;
    if (got) info = got;
    else await load();
  }

  async function remove() {
    const got = await app.run(api.vectorsModelDelete());
    if (got) info = got;
  }
</script>

{#if info}
  {@const size = fBytes(info.size)}
  <Row label={t('vectors.label')} desc={t('vectors.desc', { model: info.id, license: info.license, size })}>
    <div class="vm" data-vectors-model>
      {#if downloading}
        <span class="dl"
          ><span
            class="gauge"
            role="progressbar"
            aria-label={t('vectors.progress')}
            aria-valuemin="0"
            aria-valuemax="100"
            aria-valuenow={pct}><span class="fill" style:width="{pct}%"></span></span
          >{pct} %</span
        >
        <button class="btn" onclick={() => api.vectorsModelCancel()}>{t('vectors.cancel')}</button>
      {:else if info.downloaded}
        <span class="ok"><Icon name="check" size={14} />{t('vectors.downloaded')}</span>
        <button class="btn" use:tip={t('vectors.deleteTitle', { size })} onclick={remove}>{t('vectors.delete')}</button>
      {:else}
        <button class="btn" onclick={download}>{started ? t('vectors.resume', { pct: started }) : t('vectors.download', { size })}</button>
      {/if}
    </div>
  </Row>
{/if}

<style>
  .vm {
    display: inline-flex;
    align-items: center;
    gap: 10px;
  }
  .dl {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    min-width: 150px;
    color: var(--muted);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .dl .gauge {
    flex: 1;
  }
  .ok {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--add);
    font-size: 12px;
  }
</style>
