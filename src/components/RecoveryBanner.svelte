<script lang="ts">
  import { t } from '../lib/i18n';
  import { fDateTime } from '../lib/format';
  import { openPath } from '../lib/opener';
  import { errorText } from '../lib/refusal';
  import { app } from '../lib/state.svelte';
  import { whatOf } from '../lib/backups';
  import type { Recovery, Restored } from '../lib/types';
  import Icon from './Icon.svelte';

  // ESC-469: a file of the data folder that did not read at the start (zeros after a power cut, cut short, no JSON) was
  // set aside and its newest readable copy put back, or none was there. The band says it, file by file, until the user
  // closes it: never a start that loses something in silence. A copy taken back by hand from the screen of the copies
  // (ESC-487) is told here too, after the start that put it in place.
  type Known = 'state' | 'settings' | 'plugins' | 'integrations';
  const KNOWN: Known[] = ['state', 'settings', 'plugins', 'integrations'];
  const known = (file: string): Known => (KNOWN.find((k) => `${k}.json` === file) ?? 'state') as Known;
  const time = (rfc: string | null) => (rfc ? fDateTime(Date.parse(rfc)) : '');

  function lines(r: Recovery): string[] {
    const what = t(`shell.recovery.what.${known(r.file)}`);
    const out: string[] = [];
    if (r.restored && r.restoredAt) {
      out.push(t(`shell.recovery.restored.${r.damage}`, { file: r.file, what, at: time(r.restoredAt) }));
      out.push(
        r.writtenAt && Date.parse(r.writtenAt) > Date.parse(r.restoredAt)
          ? t('shell.recovery.since', { from: time(r.restoredAt), to: time(r.writtenAt) })
          : t('shell.recovery.sinceOpen', { from: time(r.restoredAt) }),
      );
    } else {
      out.push(t(`shell.recovery.lost.${r.damage}`, { file: r.file, what }));
    }
    if (r.skipped > 0) out.push(t('shell.recovery.skipped', { count: r.skipped }));
    if (r.keptAs) out.push(t('shell.recovery.kept', { path: r.keptAs }));
    return out;
  }

  function byHand(r: Restored): string[] {
    const at = time(r.copyAt);
    if (r.error) return [t('shell.recovery.byHandFailed', { at, file: r.file, error: r.error })];
    const out = [t('shell.recovery.byHand', { at, file: r.file, what: whatOf({ file: r.file }).toLowerCase() })];
    if (r.setAside) out.push(t('shell.recovery.byHandKept', { path: r.setAside }));
    return out;
  }

  /** The folder of every file's copies: the parent of the first one's own. */
  const where = $derived(app.recovered[0]?.copies.replace(/[\\/][^\\/]+[\\/]?$/, '') ?? '');
  const open = () => openPath(where).catch((e) => app.toast(errorText(e), 'error'));
</script>

{#if app.recovered.length || app.restored.length}
  <div class="recovery-band" role="alert" data-testid="recovery-band">
    <Icon name="restore" size={14} />
    <div class="msg">
      <strong>{app.recovered.length ? t('shell.recovery.title') : t('shell.recovery.titleByHand')}</strong>
      {#each app.recovered as r, i (i)}
        <p data-testid="recovery-item">{lines(r).join(' ')}</p>
      {/each}
      {#each app.restored as r, i (i)}
        <p data-testid="restored-item">{byHand(r).join(' ')}</p>
      {/each}
      {#if where}<p class="where">{t('shell.recovery.where', { path: where })}</p>{/if}
    </div>
    <div class="actions">
      {#if where}<button class="small" onclick={open}>{t('shell.recovery.open')}</button>{/if}
      <button class="small" data-testid="recovery-close" onclick={() => app.dismissRecovery()}>{t('shell.recovery.close')}</button>
    </div>
  </div>
{/if}

<style>
  .recovery-band {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    margin-bottom: 6px;
    padding: 8px 12px;
    border-radius: 8px;
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--warn-ink);
    background: color-mix(in oklch, var(--warn-ink) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--warn-ink) 35%, transparent);
  }
  .recovery-band > :global(svg) {
    margin-top: 2px;
    flex: none;
  }
  .msg {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .msg p {
    margin: 4px 0 0;
  }
  .where {
    opacity: 0.85;
  }
  .actions {
    display: flex;
    gap: 6px;
    flex: none;
  }
</style>
