<script lang="ts">
  import { t } from '../../lib/i18n';
  import { linkSource, SERVICE_IDS, SERVICES } from '../../lib/integrations';
  import { api } from '../../lib/ipc';
  import { app } from '../../lib/state.svelte';
  import type { Container, Service } from '../../lib/types';
  import type { WizardForm } from '../../lib/wizard-form.svelte';
  import Row from '../settings/Row.svelte';

  // « Intégrations », optional: a source to link for each service whose account is connected (its states by default;
  // the rest is set in Réglages › Services connectés and on the project's page Tickets importés).
  let { form }: { form: WizardForm } = $props();

  const connected = $derived(SERVICE_IDS.filter((id) => app.accounts.find((a) => a.service === id)?.connected));
  let containers = $state<Partial<Record<Service, Container[] | null>>>({});

  $effect(() => {
    for (const id of connected) {
      if (id in containers) continue;
      containers[id] = null;
      api
        .integrationContainers(id, form.project?.id ?? null)
        .then((list) => (containers[id] = list))
        .catch(() => (containers[id] = []));
    }
  });

  const linked = (id: Service) => form.integrations.links.find((l) => l.service === id)?.container ?? '';

  async function choose(id: Service, value: string) {
    const c = containers[id]?.find((x) => x.id === value) ?? null;
    const states = c
      ? await api
          .integrationStates(id, c.id)
          .then((v) => v.defaults)
          .catch(() => ({}))
      : {};
    form.integrations = linkSource(form.integrations, id, c, states);
  }
</script>

{#if connected.length}
  <div class="box">
    {#each connected as id (id)}
      <Row label={t('wizard.integrations.source', { service: SERVICES[id].name })}>
        <select
          class="field select"
          aria-label={t('wizard.integrations.source', { service: SERVICES[id].name })}
          disabled={!containers[id]}
          value={linked(id)}
          onchange={(e) => choose(id, e.currentTarget.value)}
        >
          <option value="">{containers[id] ? t('wizard.integrations.noSource') : t('wizard.integrations.loading')}</option>
          {#each containers[id] ?? [] as c (c.id)}<option value={c.id}>{c.name}</option>{/each}
        </select>
      </Row>
    {/each}
  </div>
{:else}
  <p class="none">{t('wizard.integrations.none')}</p>
{/if}

<style>
  /* The step's settings, a row each between rules (no box: the window is the box). */
  .box {
    border-top: 1px solid var(--line);
    border-bottom: 1px solid var(--line);
  }
  .box > :global(.row) {
    padding-inline: 0;
  }
  .select {
    height: 30px;
    max-width: 260px;
    font-size: 12.5px;
  }
  .none {
    margin: 0;
    color: var(--muted);
    font-size: 12.5px;
    line-height: 1.5;
  }
</style>
