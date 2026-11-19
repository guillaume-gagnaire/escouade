<script lang="ts">
  import { onMount } from 'svelte';
  import { grouped, holds, kindOf, whatOf } from '../../lib/backups';
  import { fDateTime, fSize, tildify } from '../../lib/format';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { openPath } from '../../lib/opener';
  import { connection } from '../../lib/connection.svelte';
  import { errorText } from '../../lib/refusal';
  import { app } from '../../lib/state.svelte';
  import type { BackupCopy, BackupDiff, BackupFile, BackupsView, LostHistory } from '../../lib/types';
  import Icon from '../Icon.svelte';
  import Group from './Group.svelte';

  // The screen of the copies (ESC-487), in Avancé under « Données locales »: for each file of the data folder that is not
  // made again, its copies and what each holds (counts, never a value: a copy of the settings or of the integrations may
  // hold tokens), « Comparer » a copy to today (nothing written), « Reprendre » one (confirmed, the file of now set aside,
  // the copy read back first; a file the app holds at the next start, never while an agent works), « Faire une copie
  // maintenant », « Ouvrir le dossier des copies », the closed projects' histories to find again and the files set aside.
  let view = $state<BackupsView | null>(null);
  let error = $state('');
  let busy = $state(false);
  /** The files whose copies are shown. */
  let shown = $state<Set<string>>(new Set());
  /** The comparison open, under its copy. */
  let diff = $state<{ file: string; name: string; at: string; d: BackupDiff } | null>(null);
  /** The closed projects just found again, by id: « Rouvrir » on them. */
  let found = $state<Record<string, string>>({});

  const at = (rfc: string) => fDateTime(Date.parse(rfc));
  const MAX_NAMES = 8;

  async function load() {
    try {
      view = await api.backupsView();
      error = '';
    } catch (e) {
      error = errorText(e);
    }
  }
  onMount(load);

  const groups = $derived(view ? grouped(view.files) : { main: [], closed: [] });

  function toggle(file: string) {
    const next = new Set(shown);
    if (next.has(file)) next.delete(file);
    else next.add(file);
    shown = next;
  }

  async function now() {
    busy = true;
    const v = await app.run(api.backUpNow());
    busy = false;
    if (v) {
      view = v;
      app.toast(t('backups.nowDone'));
    }
  }

  const openFolder = () => view && openPath(view.folder).catch((e) => app.toast(errorText(e), 'error'));

  async function compare(f: BackupFile, c: BackupCopy) {
    if (diff?.file === f.file && diff.name === c.name) {
      diff = null;
      return;
    }
    const d = await app.run(api.compareBackup(f.file, c.name));
    if (d) diff = { file: f.file, name: c.name, at: c.at, d };
  }

  /** The names of a list, the first ones only, « … et 3 autres » after them. */
  function names(list: string[]): string {
    const head = list.slice(0, MAX_NAMES).join(', ');
    return list.length > MAX_NAMES ? `${head} ${t('backups.diff.more', { count: list.length - MAX_NAMES })}` : head;
  }

  const unchanged = (d: BackupDiff) => d.kinds.every((k) => !k.gained.length && !k.lost.length);
  const restorable = (f: BackupFile) => kindOf(f.file) === 'closed' || (view?.atWork ?? 0) === 0;

  /** Back to these settings, on this group, once the confirmation is closed. */
  function back() {
    app.modal = { kind: 'settings', page: 'advanced', at: 'backups', resume: true };
  }

  async function askRestore(f: BackupFile, c: BackupCopy) {
    const d = await app.run(api.compareBackup(f.file, c.name));
    if (!d) return;
    const closed = kindOf(f.file) === 'closed';
    const lines = [
      closed
        ? t('backups.confirm.bodyClosed', { copy: holds(c.summary) })
        : t('backups.confirm.body', { file: f.file, today: f.exists ? holds(f.today) : t('backups.missing'), copy: holds(c.summary) }),
    ];
    const lost = d.kinds.flatMap((k) => k.lost);
    const gained = d.kinds.flatMap((k) => k.gained);
    if (lost.length) lines.push(t('backups.confirm.lost', { list: names(lost) }));
    if (gained.length) lines.push(t('backups.confirm.gained', { list: names(gained) }));
    app.modal = {
      kind: 'confirm',
      title: t('backups.confirm.title', { at: at(c.at) }),
      body: lines.join(' '),
      confirm: t('backups.confirm.confirm'),
      icon: 'restore',
      onCancel: back,
      onConfirm: async () => {
        const out = await app.run(api.restoreBackup(f.file, c.name));
        if (out?.done && out.closedId) {
          const name = c.summary.name ?? '';
          app.toast(t('backups.lost.found', { name }));
        }
        back();
      },
    };
  }

  async function find(l: LostHistory) {
    const out = await app.run(api.restoreBackup(l.file, l.copy));
    if (!out?.done) return;
    app.toast(t('backups.lost.found', { name: l.name }));
    const closed = await api.closedProjects().catch(() => []);
    const c = closed.find((x) => x.id === l.id);
    if (c) found = { ...found, [l.id]: c.path };
    await load();
  }

  function reopen(id: string, path: string) {
    app.modal = { kind: 'newProject', path, historyId: id };
  }

  async function cancel() {
    try {
      await api.cancelRestore();
      app.toast(t('backups.pending.cancelled'));
    } catch (e) {
      app.refused(e);
    }
    await load();
  }

  const restart = () => app.run(api.restartForRestore());
</script>

{#snippet copyRow(f: BackupFile, c: BackupCopy)}
  <li class="copy" data-testid="backup-copy">
    <div class="txt">
      <span class="when">{at(c.at)}</span>
      <span class="detail" class:bad={!c.summary.readable}>{holds(c.summary)} · {fSize(c.size)}</span>
    </div>
    <button class="btn" aria-pressed={diff?.file === f.file && diff.name === c.name} onclick={() => compare(f, c)}
      >{t('backups.compare')}</button
    >
    <button class="btn" disabled={!c.summary.readable || !restorable(f)} onclick={() => askRestore(f, c)}>{t('backups.restore')}</button>
  </li>
  {#if diff && diff.file === f.file && diff.name === c.name}
    <li class="diff" data-testid="backup-diff">
      <strong>{t('backups.diff.title', { at: at(diff.at) })}</strong>
      <span class="detail">{t('backups.diff.note')}</span>
      {#if !diff.d.copy.readable}
        <p>{t('backups.diff.unreadable')}</p>
      {:else if unchanged(diff.d)}
        <p>{t('backups.diff.same')}</p>
      {:else}
        <dl>
          {#each diff.d.kinds.filter((k) => k.gained.length || k.lost.length) as k (k.kind)}
            <dt>{t(`backups.diff.kinds.${k.kind}`)}</dt>
            <dd>
              {#if k.gained.length}<span class="gain">{t('backups.confirm.gained', { list: names(k.gained) })}</span>{/if}
              {#if k.lost.length}<span class="loss">{t('backups.confirm.lost', { list: names(k.lost) })}</span>{/if}
            </dd>
          {/each}
        </dl>
      {/if}
      <button class="btn small" onclick={() => (diff = null)}>{t('backups.diff.close')}</button>
    </li>
  {/if}
{/snippet}

{#snippet fileRow(f: BackupFile)}
  <li class="file" data-testid="backup-file" data-file={f.file}>
    <div class="txt">
      <span class="name">{whatOf(f)}</span>
      <span class="path mono">{f.file}</span>
      <span class="detail"
        >{f.exists ? t('backups.today', { what: holds(f.today) }) : t('backups.missing')} · {t('backups.copies', {
          count: f.copies.length,
        })}</span
      >
    </div>
    {#if f.copies.length}
      <button class="btn" aria-expanded={shown.has(f.file)} onclick={() => toggle(f.file)}
        >{shown.has(f.file) ? t('backups.hideCopies') : t('backups.showCopies')}</button
      >
    {:else}
      <span class="detail">{t('backups.none')}</span>
    {/if}
  </li>
  {#if shown.has(f.file)}
    <li class="copies">
      <ul class="list inner">
        {#each f.copies as c (c.name)}
          {@render copyRow(f, c)}
        {/each}
      </ul>
    </li>
  {/if}
{/snippet}

<Group title={t('backups.group')} note={t('backups.note')} anchor="backups" data-testid="backups">
  {#if !view}
    <p class="detail">{error || t('backups.loading')}</p>
  {:else}
    <div class="head">
      <div class="txt">
        <span class="name">{t('backups.folder')}</span>
        <span class="path mono" data-testid="backups-folder">{tildify(view.folder)}</span>
        <span class="detail" data-testid="backups-total">{t('backups.total', { size: fSize(view.total), max: fSize(view.max) })}</span>
      </div>
      <div class="actions">
        <button class="btn" disabled={busy} onclick={now}><Icon name="restore" size={14} />{t('backups.now')}</button>
        {#if !connection.server}<button class="btn" onclick={openFolder}><Icon name="folder" size={14} />{t('backups.open')}</button>{/if}
      </div>
    </div>

    {#if view.pending}
      <div class="band" role="status" data-testid="backups-pending">
        <strong>{t('backups.pending.title', { at: at(view.pending.copyAt), file: view.pending.file })}</strong>
        <p>{t('backups.pending.body')}</p>
        {#if view.atWork}<p>{t('backups.atWork', { count: view.atWork })}</p>{/if}
        <div class="actions">
          <button class="btn primary" disabled={view.atWork > 0} onclick={restart}>{t('backups.pending.restart')}</button>
          <button class="btn" onclick={cancel}>{t('backups.pending.cancel')}</button>
        </div>
      </div>
    {:else if view.atWork}
      <p class="detail" data-testid="backups-at-work">{t('backups.atWork', { count: view.atWork })}</p>
    {/if}

    {#if view.lost.length}
      <h4>{t('backups.lost.title')}</h4>
      <p class="detail">{t('backups.lost.note')}</p>
      <ul class="list">
        {#each view.lost as l (l.id)}
          <li class="file" data-testid="backup-lost">
            <div class="txt">
              <span class="name">{l.name || t('backups.what.closedUnnamed')}</span>
              <span class="detail"
                >{t('backups.lost.detail', {
                  at: at(l.at),
                  what: holds({ readable: true, agents: l.agents, tickets: l.tickets }),
                })}</span
              >
            </div>
            <button class="btn" onclick={() => find(l)}>{t('backups.lost.find')}</button>
          </li>
        {/each}
      </ul>
    {/if}
    {#each Object.entries(found) as [id, path] (id)}
      <p class="found">
        <button class="btn" data-testid="backup-reopen" onclick={() => reopen(id, path)}>{t('backups.lost.reopen')}</button>
        <span class="path mono">{path}</span>
      </p>
    {/each}

    <ul class="list">
      {#each groups.main as f (f.file)}
        {@render fileRow(f)}
      {/each}
    </ul>
    {#if groups.closed.length}
      <details class="closed">
        <summary>{t('backups.closedGroup', { count: groups.closed.length })}</summary>
        <ul class="list">
          {#each groups.closed as f (f.file)}
            {@render fileRow(f)}
          {/each}
        </ul>
      </details>
    {/if}

    {#if view.setAside.length}
      <h4>{t('backups.setAside.title')}</h4>
      <p class="detail">{t('backups.setAside.note')}</p>
      <ul class="list" data-testid="backups-set-aside">
        {#each view.setAside as s (s.path)}
          <li class="file">
            <div class="txt">
              <span class="name"
                >{s.why === 'unreadable'
                  ? t('backups.setAside.unreadable', { file: s.file })
                  : t('backups.setAside.beforeRestore', { file: s.file })}</span
              >
              <span class="path mono">{s.path}</span>
              <span class="detail">{fSize(s.size)}{s.at ? ` · ${at(s.at)}` : ''}</span>
            </div>
          </li>
        {/each}
      </ul>
    {/if}

    <p class="detail why"><strong>{t('backups.why.title')}</strong> · {t('backups.why.body')}</p>
  {/if}
</Group>

<style>
  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 10px 16px;
    padding: 10px 0;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  h4 {
    margin: 14px 0 2px;
    font-size: 12.5px;
    font-weight: 600;
  }
  .list {
    margin: 4px 0 0;
    padding: 0;
    list-style: none;
  }
  .list.inner {
    margin: 0 0 6px 14px;
    border-left: 1px solid rgba(var(--fg-rgb), 0.08);
    padding-left: 12px;
  }
  .file,
  .copy {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 10px;
    padding: 9px 0;
    border-bottom: 1px solid rgba(var(--fg-rgb), 0.05);
  }
  .file {
    grid-template-columns: minmax(0, 1fr) auto;
  }
  .copies {
    list-style: none;
  }
  .txt {
    display: grid;
    min-width: 0;
  }
  .name,
  .when {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .when {
    font-weight: 500;
  }
  .path,
  .detail {
    color: var(--muted);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
  .detail.bad {
    color: var(--warn-ink);
  }
  .band {
    margin: 6px 0 4px;
    padding: 10px 12px;
    border-radius: 8px;
    font-size: 12.5px;
    color: var(--warn-ink);
    background: color-mix(in oklch, var(--warn-ink) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--warn-ink) 35%, transparent);
  }
  .band p {
    margin: 4px 0 8px;
  }
  .diff {
    display: grid;
    gap: 4px;
    margin: 2px 0 8px;
    padding: 10px 12px;
    border-radius: 8px;
    font-size: 12.5px;
    background: rgba(var(--fg-rgb), 0.04);
    list-style: none;
  }
  .diff p {
    margin: 4px 0;
  }
  .diff dl {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr);
    gap: 4px 12px;
    margin: 6px 0;
  }
  .diff dt {
    font-weight: 600;
  }
  .diff dd {
    display: grid;
    gap: 2px;
    margin: 0;
    overflow-wrap: anywhere;
  }
  .gain {
    color: var(--fg);
  }
  .loss {
    color: var(--warn-ink);
  }
  .diff .btn {
    justify-self: start;
  }
  .closed {
    margin-top: 8px;
  }
  .closed summary {
    cursor: pointer;
    padding: 6px 0;
    font-size: 12.5px;
    color: var(--soft);
  }
  .found {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 6px 0;
  }
  .why {
    margin-top: 12px;
  }
</style>
