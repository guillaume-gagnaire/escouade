<script lang="ts">
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { codeIndexOn } from '../../lib/codeindex';
  import { fAgo, fInt, fList, fSize } from '../../lib/format';
  import { app } from '../../lib/state.svelte';
  import type { CodeIndexInfo, Project } from '../../lib/types';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Switch from './Switch.svelte';
  import VectorsModelRow from './VectorsModelRow.svelte';

  // « Index du code » of a project's settings (ESC-254, docs/FACTORY.md « Proposé d'office, en option »): what it does,
  // that nothing leaves the machine, what a private project's fence leaves out, its switch (saved at once: it is not of
  // the draft, `update_project` leaves it alone), and the model of the search by meaning. Its state follows « Index du
  // code sur tous les projets » until the project makes its own choice (ESC-293): the section says which, and goes back
  // to the app's setting.
  let { project }: { project: Project } = $props();

  let info = $state<CodeIndexInfo | null>(null);
  /** The project is being read (turned on) or its index erased (turned off). */
  let busy = $state(false);

  async function load(id: string) {
    const got = await api.getCodeIndex(id).catch(() => null);
    if (id === project.id && !busy) info = got;
  }
  // Read again when the backend says its index changed (read for the app's setting, erased, waiting its turn).
  $effect(() => {
    void app.codeIndexTicks[project.id];
    void app.settings?.codeIndexAll;
    void load(project.id);
  });

  const live = $derived(app.projects.find((p) => p.id === project.id) ?? project);
  const chosen = $derived(info?.chosen ?? live.codeIndexChoice != null);
  const enabled = $derived(info?.enabled ?? codeIndexOn(live, app.settings));

  async function set(choice: boolean | null) {
    if (busy) return;
    const id = project.id;
    busy = true;
    // Shown at once: the reading of a big project takes a while.
    const on = choice ?? !!app.settings?.codeIndexAll;
    if (info) info = { ...info, enabled: on, chosen: choice != null };
    const got = await app.run(api.setCodeIndex(id, choice));
    busy = false;
    if (id !== project.id) return;
    if (!got) return void load(id);
    info = got;
    if (choice == null) app.toast(t('codeindex.settings.followed'));
    else app.toast(on ? t('codeindex.settings.turnedOn', { files: fInt(got.status.files) }) : t('codeindex.settings.turnedOff'));
  }

  /** « Voir le réglage »: the app's, further down the same tab. */
  function seeAll(e: MouseEvent) {
    const target = (e.currentTarget as HTMLElement).closest('.s-body')?.querySelector('[data-anchor="code-index-all"]');
    target?.scrollIntoView({ block: 'start', behavior: 'smooth' });
  }

  const status = $derived.by(() => {
    if ((busy && enabled) || info?.reading) return t('codeindex.settings.reading');
    if (info?.waiting) return t('codeindex.settings.waiting');
    const s = info?.status;
    if (!enabled || !s?.indexedAt) return '';
    return t('codeindex.settings.status', {
      files: fInt(s.files),
      symbols: fInt(s.symbols),
      size: fSize(s.bytes),
      when: fAgo(s.indexedAt / 1000, app.now),
    });
  });
  const excluded = $derived(info?.excluded ?? []);
</script>

<Group title={t('codeindex.settings.title')} anchor="code-index" note={`${t('codeindex.what')} ${t('codeindex.local')}`}>
  <Row label={t('codeindex.settings.on')} desc={t('codeindex.settings.onDesc')}>
    {#snippet extra()}
      <p class="more origin" data-code-index-origin={chosen ? 'chosen' : 'follows'}>
        {#if chosen}
          <span>{t('codeindex.settings.chosen')}</span>
          <button class="btn ghost small" disabled={busy} onclick={() => void set(null)}>{t('codeindex.settings.follow')}</button>
        {:else}
          <span>{app.settings?.codeIndexAll ? t('codeindex.settings.follows.on') : t('codeindex.settings.follows.off')}</span>
          <button class="btn ghost small" onclick={seeAll}>{t('codeindex.settings.seeAll')}</button>
        {/if}
      </p>
      {#if status}<span class="status" data-code-index-status aria-live="polite">{status}</span>{/if}
    {/snippet}
    <Switch label={t('codeindex.settings.on')} disabled={busy} bind:on={() => enabled, (on) => void set(on)} />
  </Row>
  {#if live.private || excluded.length}
    <div class="fenced" data-code-index-fence>
      {#if live.private}<p>{t('codeindex.privateNote')}</p>{/if}
      {#if excluded.length}<p>{t('codeindex.excluded', { count: excluded.length, list: fList(excluded) })}</p>{/if}
    </div>
  {/if}
  <VectorsModelRow />
</Group>

<style>
  .origin {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 4px 10px;
  }
  .status {
    color: var(--muted);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .fenced {
    max-width: 64ch;
    margin: 2px 0 10px;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }
  .fenced p {
    margin: 0 0 4px;
  }
</style>
