import { cleanup, render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
import { app } from '../lib/state.svelte';
import type { Recovery } from '../lib/types';
import { fakeBackend, resetApp } from '../test/ipc';
import RecoveryBanner from './RecoveryBanner.svelte';

const DATA = 'C:\\Users\\ada\\.escouade';

function recovery(over: Partial<Recovery> = {}): Recovery {
  return {
    file: 'state.json',
    damage: 'zeros',
    foundAt: '2026-11-15T10:40:00Z',
    writtenAt: '2026-11-15T09:36:00Z',
    keptAs: `${DATA}\\state.unreadable-20261115T104000Z.json`,
    restored: `${DATA}\\backups\\state\\state-20261115T092000Z.json.gz`,
    restoredAt: '2026-11-15T09:20:00Z',
    skipped: 0,
    copies: `${DATA}\\backups\\state`,
    ...over,
  };
}

describe('RecoveryBanner (ESC-469)', () => {
  beforeEach(() => {
    resetApp();
    setLang('fr');
  });
  afterEach(cleanup);

  it('is absent when nothing was recovered', () => {
    fakeBackend();
    render(RecoveryBanner);
    expect(screen.queryByTestId('recovery-band')).toBeNull();
  });

  it('says what happened, the copy taken back and its time, what may be lost, where the file and the copies are', () => {
    fakeBackend();
    app.recovered = [recovery({ skipped: 1 })];
    render(RecoveryBanner);
    const band = screen.getByTestId('recovery-band');
    expect(band).toHaveTextContent('Des données d’Escouade étaient illisibles au démarrage');
    const item = screen.getByTestId('recovery-item');
    expect(item).toHaveTextContent(
      'Le fichier state.json (tes projets, tickets et agents) ne contenait que des zéros, comme après une coupure du poste : Escouade a repris sa copie du',
    );
    expect(item).toHaveTextContent(/Ce qui a changé entre le .+ et le .+ est perdu\./);
    expect(item).toHaveTextContent('1 copie plus récente, illisible elle aussi, a été sautée.');
    expect(item).toHaveTextContent(`Le fichier illisible est gardé sous ${DATA}\\state.unreadable-20261115T104000Z.json.`);
    expect(band).toHaveTextContent(`Les copies sont dans ${DATA}\\backups :`);
  });

  it('says the app started without a file no copy of which reads, in English too', () => {
    fakeBackend();
    setLang('en');
    app.recovered = [recovery({ file: 'settings.json', damage: 'cut', restored: null, restoredAt: null })];
    render(RecoveryBanner);
    expect(screen.getByTestId('recovery-item')).toHaveTextContent(
      'The file settings.json (your settings) was cut short and no readable copy exists: Escouade started without it.',
    );
    expect(screen.getByTestId('recovery-item')).not.toHaveTextContent('is lost');
  });

  it('says why the core refused to open the copies’ folder (audit SEC-12)', async () => {
    const why =
      '« D:\\ailleurs » est hors des dossiers des projets, de leurs worktrees et du dossier de données d’Escouade : Escouade ne le fait pas ouvrir par le système.';
    fakeBackend({
      open_path: () => {
        throw why;
      },
    });
    app.recovered = [recovery()];
    render(RecoveryBanner);
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir le dossier des copies' }));
    await waitFor(() => expect(app.toasts.map((t) => [t.text, t.kind])).toContainEqual([why, 'error']));
  });

  it('stays until closed, then forgets it in the data folder too, and opens the copies’ folder', async () => {
    const backend = fakeBackend();
    app.recovered = [recovery(), recovery({ file: 'plugins.json', damage: 'invalid' })];
    render(RecoveryBanner);
    expect(screen.getAllByTestId('recovery-item')).toHaveLength(2);
    await userEvent.click(screen.getByRole('button', { name: 'Ouvrir le dossier des copies' }));
    expect(backend.called('open_path')).toEqual([{ cmd: 'open_path', args: { path: `${DATA}\\backups` } }]);
    expect(screen.getByTestId('recovery-band')).toBeInTheDocument();
    await userEvent.click(screen.getByTestId('recovery-close'));
    await waitFor(() => expect(screen.queryByTestId('recovery-band')).toBeNull());
    expect(backend.called('dismiss_recovery')).toHaveLength(1);
    expect(app.recovered).toEqual([]);
  });
});
