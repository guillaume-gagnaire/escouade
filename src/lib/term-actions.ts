import { t } from './i18n';
import { IS_MAC } from './platform';
import { app } from './state.svelte';
import { disposeTerminal, openTerminal, type TermPlace } from './terminals';

export const SHELL_GLYPH: Record<string, { glyph: string; c: string }> = {
  pwsh: { glyph: 'PS', c: 'var(--info)' },
  powershell: { glyph: 'PS', c: 'var(--info)' },
  bash: { glyph: '$_', c: 'var(--ok)' },
  wsl: { glyph: 'λ', c: 'oklch(0.78 0.13 60)' },
  zsh: { glyph: '%_', c: 'var(--ok)' },
  fish: { glyph: '>_', c: 'oklch(0.78 0.13 60)' },
  sh: { glyph: '$_', c: 'var(--ok)' },
};

/** Shows a terminal just opened in place of whatever the project's main area shows. */
async function show(projectId: string, shell: string | undefined, name: (shell: string) => string, place?: TermPlace) {
  if (!shell) {
    const expected = IS_MAC ? 'zsh, bash' : 'PowerShell 7, Git Bash, WSL';
    app.toast(t('runs.term.noShell', { expected }), 'error');
    return;
  }
  const info = await app.run(openTerminal(projectId, shell, name(shell), place));
  if (!info) return;
  app.terminals.push(info);
  app.selectedTerm[projectId] = info.id;
  app.selectedLaunch[projectId] = null;
  app.closeEditor(projectId);
  app.closeBoard(projectId);
}

export function newTerminal(projectId: string, shell = app.shells[0]?.id) {
  return show(projectId, shell, (s) => {
    const n = app.terminals.filter((t) => t.projectId === projectId && t.shell === s).length + 1;
    return `${s}-${n}`;
  });
}

/**
 * A terminal, with the first shell, in the worktree of an agent (the project's folder for one
 * without) or in a folder of its source: named after it, by `label` (the agent's or the folder's name).
 */
export function terminalIn(projectId: string, place: TermPlace, label: string) {
  return show(projectId, app.shells[0]?.id, (s) => `${s} · ${label}`, place);
}

export function closeTerminal(id: string) {
  const t = app.terminals.find((x) => x.id === id);
  disposeTerminal(id);
  app.terminals = app.terminals.filter((x) => x.id !== id);
  delete app.exitedTerms[id];
  if (t && app.selectedTerm[t.projectId] === id) app.selectedTerm[t.projectId] = null;
}
