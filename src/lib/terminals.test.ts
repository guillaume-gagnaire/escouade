import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fakeBackend } from '../test/ipc';

// xterm.js draws on a canvas: stood in for, only the spawn is looked at.
vi.mock('@xterm/xterm', () => ({
  Terminal: class {
    cols = 100;
    rows = 30;
    options = {};
    unicode = { activeVersion: '' };
    loadAddon() {}
    open() {}
    attachCustomKeyEventHandler() {}
    onData() {}
    onResize() {}
    dispose() {}
  },
}));
vi.mock('@xterm/addon-fit', () => ({
  FitAddon: class {
    fit() {}
  },
}));
vi.mock('@xterm/addon-search', () => ({ SearchAddon: class {} }));
vi.mock('@xterm/addon-unicode11', () => ({ Unicode11Addon: class {} }));
vi.mock('@xterm/addon-web-links', () => ({ WebLinksAddon: class {} }));
vi.mock('@xterm/addon-webgl', () => ({
  WebglAddon: class {
    onContextLoss() {}
  },
}));

import { openTerminal } from './terminals';

describe('opening a terminal', () => {
  beforeEach(() => {
    // The theme's colors are read back from a 1px canvas, which jsdom does not draw.
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({
      fillRect() {},
      getImageData: () => ({ data: [0, 0, 0] }),
    } as unknown as CanvasRenderingContext2D);
  });

  const info = { id: 't1', projectId: 'p1', name: 'pwsh-1', shell: 'pwsh' };

  it('opens in the project’s folder when no place is given', async () => {
    const backend = fakeBackend({ term_spawn: () => info });
    await openTerminal('p1', 'pwsh', 'pwsh-1');
    expect(backend.called('term_spawn')[0].args).toMatchObject({ projectId: 'p1', shell: 'pwsh', name: 'pwsh-1', cols: 100, rows: 30 });
    expect(backend.called('term_spawn')[0].args).not.toHaveProperty('agentId');
    expect(backend.called('term_spawn')[0].args).not.toHaveProperty('subdir');
  });

  it('asks the backend for the agent’s worktree and the folder of it', async () => {
    const backend = fakeBackend({ term_spawn: () => info });
    await openTerminal('p1', 'pwsh', 'pwsh · lib', { agentId: 'a1', subdir: 'src/lib' });
    expect(backend.called('term_spawn')[0].args).toMatchObject({ projectId: 'p1', agentId: 'a1', subdir: 'src/lib', name: 'pwsh · lib' });
  });

  it('passes on the backend’s refusal of a place', async () => {
    fakeBackend({ term_spawn: () => Promise.reject('chemin hors du dossier : ..') });
    await expect(openTerminal('p1', 'pwsh', 'pwsh · ..', { subdir: '..' })).rejects.toBe('chemin hors du dossier : ..');
  });
});
