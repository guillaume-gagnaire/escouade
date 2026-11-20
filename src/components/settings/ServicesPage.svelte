<script lang="ts">
  import { t } from '../../lib/i18n';
  import { SERVICE_IDS } from '../../lib/integrations';
  import { installedOf } from '../../lib/installed';
  import { settingsForm } from '../../lib/settings.svelte';
  import Chips from './Chips.svelte';
  import ForgesSection from './ForgesSection.svelte';
  import NoPlugin from './NoPlugin.svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import SourceAccount from './SourceAccount.svelte';
  import Switch from './Switch.svelte';
  import SecretsOnServer from '../servers/SecretsOnServer.svelte';

  // « Services connectés », a page of Escouade (design/settings-2-2/PROPOSITION.md §2, page 4): the code hosts, the
  // accounts of the ticket sources (connected at once, apart from the draft, each by `SourceAccount`, which the first
  // launch's « Ce que tu veux brancher » shows too), and the sync and automatic import of every project (the app's
  // settings). What a project links is on its page « Tickets importés ».
  const s = $derived(settingsForm.settings.integrations);
  /** The sources an installed plugin brings (ESC-407): the accounts of one whose plugin is gone stay kept, hidden. */
  const sources = $derived(installedOf('ticketSources', SERVICE_IDS));

  const every = $derived([5, 15, 60].map((n) => ({ value: n, label: t('integrations.autoImport.minutes', { n }) })));

  /** The code hosts' part: GitLab Issues' « Connecter GitLab… » opens its form there (one account for both uses). */
  let forges = $state<ReturnType<typeof ForgesSection>>();
</script>

<!-- Two parts, named apart (ESC-165): the code hosts first (choice 27 of the 2.0 design), they serve every agent's
     pull requests; then the sources of the tickets, their accounts, their sync and their import. -->
<header class="part" data-anchor="hosts">
  <h2>{t('integrations.parts.forges.title')}</h2>
  <p>{t('integrations.parts.forges.note')}</p>
</header>
<ForgesSection bind:this={forges} />

<header class="part" data-anchor="sources">
  <h2>{t('integrations.parts.sources.title')}</h2>
  <p>{t('integrations.parts.sources.note')}</p>
</header>

<Group title={t('integrations.accounts.title')} note={t('integrations.accounts.note')}>
  <!-- On a server, the tokens of the services are kept in a file of its data (ESC-538). -->
  <SecretsOnServer />
  {#if !sources.length}<NoPlugin text={t('settings.noPlugin.sources')} />{/if}
  {#each sources as id (id)}
    <SourceAccount {id} onforge={(kind) => forges?.connectKind(kind)} />
  {/each}
</Group>

<!-- The sync and the import of the sources: while one is installed (ESC-407), their choices kept otherwise. -->
{#if sources.length}
  <Group title={t('integrations.sync.title')}>
    <Row label={t('integrations.sync.updateStatus')} desc={t('integrations.sync.updateStatusDesc')}>
      <Switch label={t('integrations.sync.updateStatus')} bind:on={s.syncStates} />
    </Row>
    <Row label={t('integrations.sync.loopSummary')} desc={t('integrations.sync.loopSummaryDesc')}>
      <Switch label={t('integrations.sync.loopSummary')} bind:on={s.loopComments} />
    </Row>
    <Row label={t('integrations.sync.extractCriteria')} desc={t('integrations.sync.extractCriteriaDesc')}>
      <Switch label={t('integrations.sync.extractCriteria')} bind:on={s.extractCriteria} />
    </Row>
  </Group>

  <Group title={t('integrations.autoImport.title')}>
    <Row label={t('integrations.autoImport.enable')} desc={t('integrations.autoImport.enableDesc')}>
      <Switch label={t('integrations.autoImport.enable')} bind:on={s.autoImport} />
    </Row>
    <Row label={t('integrations.autoImport.label')} hint={t('integrations.autoImport.labelHint')}>
      <input
        class="field mono input"
        aria-label={t('integrations.autoImport.label')}
        spellcheck="false"
        placeholder="claude-ready"
        bind:value={s.importLabel}
      />
    </Row>
    <Row label={t('integrations.autoImport.every')}>
      <Chips label={t('integrations.autoImport.every')} options={every} mono bind:value={s.importEvery} />
    </Row>
  </Group>
{/if}

<style>
  .part {
    padding: 26px 0 2px;
  }
  .part:first-child {
    padding-top: 6px;
  }
  :global(.sg) + .part {
    margin-top: 14px;
    border-top: 1px solid var(--line);
  }
  .part h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 650;
  }
  .part p {
    max-width: 64ch;
    margin: 4px 0 0;
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .part + :global(.sg) {
    border-top: none;
  }
</style>
