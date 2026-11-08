<script lang="ts">
  import Icon from '../Icon.svelte';
  import { t } from '../../lib/i18n';
  import { api } from '../../lib/ipc';
  import { revealHidden } from '../../lib/recipe';
  import { settingsForm } from '../../lib/settings.svelte';
  import { pendingGroups, sharedFile } from '../../lib/shared-file';
  import { app } from '../../lib/state.svelte';
  import type { Project } from '../../lib/types';
  import Group from './Group.svelte';
  import Row from './Row.svelte';

  // The project's .escouade.yml (ESC-146): what it set, what of it was left out, its commands waiting for the user's
  // approval, read here in full; and « Exporter vers .escouade.yml ».
  let { project }: { project: Project } = $props();

  const file = $derived(sharedFile(project));
  const groups = $derived(pendingGroups(project));
  let busy = $state(false);

  /** The project as the backend has it now: in the window, and in the draft unless the user changed it. */
  function took(p: Project | null | undefined) {
    if (!p) return false;
    app.replaceProject(p);
    settingsForm.refresh(p);
    return true;
  }

  async function trust() {
    const hash = file?.pending?.hash;
    if (!hash || busy) return;
    busy = true;
    if (took(await app.run(api.sharedTrust(project.id, hash)))) app.toast(t('settings.shared.trusted'));
    busy = false;
  }

  async function exportFile() {
    if (busy) return;
    busy = true;
    if (took(await app.run(api.sharedWrite(project.id)))) app.toast(t('settings.shared.exported'));
    busy = false;
  }
</script>

<Group title={t('settings.shared.title')} anchor="shared">
  <!-- The name of the file under the title (PROPOSITION.md §2, « Les noms »). -->
  {#snippet state()}<span class="file mono">{t('settings.shared.file')}</span>{/snippet}
  {#snippet action()}
    <button class="btn" aria-describedby="shared-export-desc" disabled={busy} onclick={exportFile}
      ><Icon name="file" size={14} />{t('settings.shared.export')}</button
    >
  {/snippet}
  <div class="state">
    {#if !file}
      <p>{t('settings.shared.none')}</p>
    {:else if file.error}
      <p class="error">
        {t('settings.shared.error', {
          text: file.error.line ? t('settings.shared.warningAt', { line: file.error.line, text: file.error.text }) : file.error.text,
        })}
      </p>
    {:else}
      <p>{t('settings.shared.read', { count: file.keys.length, version: file.version })}</p>
    {/if}
    {#if file?.warnings.length}
      <p class="sub">{t('settings.shared.warningsTitle')}</p>
      <ul class="warnings">
        {#each file.warnings as w, i (i)}
          <li>{w.line ? t('settings.shared.warningAt', { line: w.line, text: w.text }) : w.text}</li>
        {/each}
      </ul>
    {/if}
  </div>
  {#if file?.pending}
    <div class="pending" role="region" aria-label={t('settings.shared.pendingTitle')}>
      <b class="pt">{t('settings.shared.pendingTitle')}</b>
      <p class="notice">{t('settings.shared.pendingDesc')}</p>
      {#each groups as g (g.kind)}
        <section class="part">
          <h5>{t(`settings.shared.keys.${g.kind}`)}</h5>
          {#if g.commands.length}
            <ol class="steps">
              {#each g.commands as c, i (i)}
                <li>
                  {#if c.name}<span class="name">{revealHidden(c.name)}</span>{/if}
                  <pre class="command mono">{revealHidden(c.command)}</pre>
                  {#if c.cwd || c.shell}
                    <dl class="facts">
                      {#if c.shell}<dt>{t('settings.fields.shell')}</dt>
                        <dd>{revealHidden(c.shell)}</dd>{/if}
                      {#if c.cwd}<dt>{t('common.folder')}</dt>
                        <dd class="mono">{revealHidden(c.cwd)}</dd>{/if}
                    </dl>
                  {/if}
                </li>
              {/each}
            </ol>
          {:else}
            <p class="none">{t('settings.project.noCommands')}</p>
          {/if}
        </section>
      {/each}
      <div class="actions">
        <button class="btn primary" disabled={busy} onclick={trust}><Icon name="check" size={14} />{t('settings.shared.trust')}</button>
      </div>
    </div>
  {/if}
  <p class="export" id="shared-export-desc">{t('settings.shared.exportDesc')}</p>
</Group>

<style>
  .file {
    color: var(--muted);
    font-size: 12px;
  }
  .state {
    max-width: 64ch;
    margin-top: 4px;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }
  .state p {
    margin: 0;
  }
  .state .error {
    color: var(--del);
  }
  .sub {
    margin-top: 8px !important;
    color: var(--text);
    font-weight: 600;
  }
  .warnings {
    margin: 4px 0 0;
    padding-left: 18px;
    color: var(--warn-ink);
    overflow-wrap: anywhere;
  }
  .export {
    max-width: 64ch;
    margin: 6px 0 12px;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }
  /* The commands that wait for the user's trust, read whole (`.pending` of the 2.0 design). */
  .pending {
    display: grid;
    gap: 6px;
    margin: 12px 0 10px;
    padding: 11px 12px;
    border-radius: 10px;
    background: var(--wait-bg);
    box-shadow: inset 0 0 0 1px var(--wait-line);
  }
  .pt {
    font-weight: 600;
  }
  h5 {
    margin: 0;
    color: var(--muted);
    font-size: 11.5px;
    font-weight: 600;
  }
  .notice {
    margin: 0;
    color: var(--soft);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .part {
    display: grid;
    gap: 6px;
  }
  .steps {
    display: grid;
    gap: 6px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .steps li {
    display: grid;
    gap: 4px;
  }
  /* Read as the shell reads it: right-to-left letters must not reorder a « ; » or a « | ». */
  .name,
  .command,
  .facts dd {
    unicode-bidi: bidi-override;
    direction: ltr;
  }
  .name {
    font-weight: 600;
  }
  /* The whole command, as written: never cut, however long. */
  .command {
    margin: 0;
    padding: 6px 9px;
    border-radius: 7px;
    background: var(--well);
    color: var(--code-fg);
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 2px 10px;
    margin: 0;
    font-size: 12px;
  }
  .facts dt {
    color: var(--muted);
  }
  .facts dd {
    margin: 0;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .none {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 4px;
  }
</style>
