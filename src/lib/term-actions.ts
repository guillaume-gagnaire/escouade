import { IS_MAC } from './platform';
import { app } from './state.svelte';
import { disposeTerminal, openTerminal } from './terminals';

export const SHELL_GLYPH: Record<string, { glyph: string; c: string }> = {
  pwsh: { glyph: 'PS', c: 'var(--info)' },
  powershell: { glyph: 'PS', c: 'var(--info)' },
  bash: { glyph: '$_', c: 'var(--ok)' },
  wsl: { glyph: 'λ', c: 'oklch(0.78 0.13 60)' },
  zsh: { glyph: '%_', c: 'var(--ok)' },
  fish: { glyph: '>_', c: 'oklch(0.78 0.13 60)' },
  sh: { glyph: '$_', c: 'var(--ok)' },
};

export async function newTerminal(projectId: string, shell = app.shells[0]?.id) {
  if (!shell) {
    const expected = IS_MAC ? 'zsh, bash' : 'PowerShell 7, Git Bash, WSL';
    app.toast(`Aucun shell détecté (${expected}). Vérifie les réglages.`, 'error');
    return;
  }
  const n = app.terminals.filter((t) => t.projectId === projectId && t.shell === shell).length + 1;
  const info = await app.run(openTerminal(projectId, shell, `${shell}-${n}`));
  if (!info) return;
  app.terminals.push(info);
  app.selectedTerm[projectId] = info.id;
  app.selectedLaunch[projectId] = null;
  app.closeEditor(projectId);
  app.closeBoard(projectId);
}

export function closeTerminal(id: string) {
  const t = app.terminals.find((x) => x.id === id);
  disposeTerminal(id);
  app.terminals = app.terminals.filter((x) => x.id !== id);
  delete app.exitedTerms[id];
  if (t && app.selectedTerm[t.projectId] === id) app.selectedTerm[t.projectId] = null;
}
