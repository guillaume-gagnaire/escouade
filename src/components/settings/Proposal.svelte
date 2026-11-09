<script module lang="ts">
  import { tick } from 'svelte';

  /**
   * Asks Claude with `button`, then hands its proposal the focus, to be read from the keyboard: only when the focus is
   * still where the button left it (the reading takes minutes, the user may have gone on elsewhere).
   */
  export async function ask(read: () => Promise<void>, button: HTMLButtonElement | undefined, proposalId: string) {
    await read();
    await tick();
    const at = document.activeElement;
    if (!at || at === document.body || at === button) document.getElementById(proposalId)?.focus();
  }

  /** Takes or ignores a proposal, the focus back on the button that asked for it. */
  export async function settle(run: () => void, button: HTMLButtonElement | undefined) {
    run();
    await tick();
    button?.focus();
  }
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';
  import { t } from '../../lib/i18n';

  // What Claude proposed for a project (its worktrees' setup, its launch commands), read here in full before it replaces
  // anything of the draft: its notice, the list, « Ignorer » and « Remplacer ».
  let {
    id,
    label,
    notice,
    take,
    ignore,
    children,
  }: { id: string; label: string; notice: string; take: () => void; ignore: () => void; children: Snippet } = $props();
</script>

<div {id} class="proposal" role="region" aria-label={label} tabindex="-1">
  <p class="notice">{notice}</p>
  {@render children()}
  <div class="actions">
    <button class="btn ghost" onclick={ignore}>{t('settings.project.ignore')}</button>
    <button class="btn primary" onclick={take}>{t('settings.project.replace')}</button>
  </div>
</div>

<style>
  .proposal {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin: 4px 0 12px;
    padding: 12px;
    border-radius: 10px;
    background: rgba(var(--fg-rgb), 0.03);
    box-shadow: inset 0 0 0 1px var(--line-2);
  }
  .notice {
    margin: 0;
    padding: 8px 12px;
    border-radius: 7px;
    background: var(--wait-bg);
    color: var(--text);
    font-size: 12.5px;
    line-height: 1.5;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .proposal :global(.part) {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .proposal :global(h4) {
    margin: 0;
  }
  .proposal :global(.none) {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .proposal :global(.steps) {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .proposal :global(.steps li) {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  /* What is read here is what the shell reads, in that order: right-to-left letters must not reorder a « ; » or a « | ». */
  .proposal :global(.name),
  .proposal :global(.command),
  .proposal :global(.facts dd) {
    unicode-bidi: bidi-override;
    direction: ltr;
  }
  .proposal :global(.name) {
    font-weight: 600;
  }
  /* The whole command, as written: never cut, never scrolled out of sight, however long. */
  .proposal :global(.command) {
    margin: 0;
    padding: 6px 10px;
    border-radius: 7px;
    background: var(--well);
    box-shadow: inset 0 0 0 1px var(--line);
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .proposal :global(.facts) {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 2px 10px;
    margin: 0;
    font-size: 12px;
  }
  .proposal :global(.facts dt) {
    color: var(--muted);
  }
  .proposal :global(.facts dd) {
    margin: 0;
    color: var(--muted);
    overflow-wrap: anywhere;
  }
</style>
