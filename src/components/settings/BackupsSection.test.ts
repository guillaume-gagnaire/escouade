import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { app } from '../../lib/state.svelte';
import type { BackupDiff, BackupsView } from '../../lib/types';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import BackupsSection from './BackupsSection.svelte';

// ESC-487: the screen of the copies, in Avancé.

const COPY_NEW = '2026-11-16T09:00:00Z';
const COPY_OLD = '2026-11-15T08:00:00Z';

const view = (over: Partial<BackupsView> = {}): BackupsView => ({
  data: 'C:\\data',
  folder: 'C:\\data\\backups',
  total: 3 * 1024 * 1024,
  max: 512 * 1024 * 1024,
  files: [
    {
      file: 'state.json',
      exists: true,
      today: { readable: true, projects: 1, agents: 2, tickets: 3 },
      folder: 'C:\\data\\backups\\state',
      copies: [
        {
          name: 'state-20261116T090000Z.json.gz',
          at: COPY_NEW,
          size: 2048,
          summary: { readable: true, projects: 1, agents: 2, tickets: 3 },
        },
        {
          name: 'state-20261115T080000Z.json.gz',
          at: COPY_OLD,
          size: 4096,
          summary: { readable: true, projects: 2, agents: 1, tickets: 5 },
        },
        { name: 'state-20261114T080000Z.json.gz', at: '2026-11-14T08:00:00Z', size: 16, summary: { readable: false } },
      ],
    },
    {
      file: 'integrations.json',
      exists: true,
      today: { readable: true, accounts: 2 },
      folder: 'C:\\data\\backups\\integrations',
      copies: [{ name: 'integrations-20261116T090000Z.json.gz', at: COPY_NEW, size: 300, summary: { readable: true, accounts: 2 } }],
    },
    {
      file: 'closed-projects/c1.json',
      exists: true,
      today: { readable: true, agents: 1, tickets: 4, name: 'ancien' },
      folder: 'C:\\data\\backups\\closed-projects\\c1',
      copies: [
        {
          name: 'c1-20261116T090000Z.json.gz',
          at: COPY_NEW,
          size: 900,
          summary: { readable: true, agents: 1, tickets: 4, name: 'ancien' },
        },
      ],
    },
  ],
  pending: null,
  setAside: [],
  lost: [],
  atWork: 0,
  ...over,
});

const DIFF: BackupDiff = {
  copy: { readable: true, projects: 2, agents: 1, tickets: 5 },
  today: { readable: true, projects: 1, agents: 2, tickets: 3 },
  kinds: [
    { kind: 'projects', gained: ['boutique'], lost: [] },
    { kind: 'tickets', gained: ['BTQ-1 Panier', 'BTQ-2 Paiement'], lost: ['ATL-9 Facture'] },
    { kind: 'agents', gained: [], lost: ['Relances'] },
  ],
};

async function stateCopies() {
  const file = await screen.findByText('state.json');
  const row = file.closest('li')!;
  await userEvent.click(within(row).getByRole('button', { name: 'Voir les copies' }));
  return screen.getAllByTestId('backup-copy');
}

describe('BackupsSection', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp({ projects: [project()] });
  });

  it('lists each file with what it holds today, its copies and their contents, the total and the folder', async () => {
    fakeBackend({ backups_view: () => view() });
    render(BackupsSection);
    expect(await screen.findByText('Projets, tickets et agents')).toBeInTheDocument();
    expect(screen.getByText('Aujourd’hui : 1 projet, 2 agents, 3 tickets · 3 copies')).toBeInTheDocument();
    expect(screen.getByText('Comptes de tickets et d’hébergeurs')).toBeInTheDocument();
    expect(screen.getByText('Aujourd’hui : 2 comptes · 1 copie')).toBeInTheDocument();
    expect(screen.getByTestId('backups-total')).toHaveTextContent('3 Mo de copies, 512 Mo au plus');
    expect(screen.getByTestId('backups-folder')).toHaveTextContent('C:\\data\\backups');
    // The closed projects' histories, folded under their own line.
    expect(screen.getByText('Historique de 1 projet fermé')).toBeInTheDocument();
    expect(screen.getByText('Projet fermé « ancien »')).toBeInTheDocument();
    const copies = await stateCopies();
    expect(copies).toHaveLength(3);
    expect(copies[1]).toHaveTextContent('2 projets, 1 agent, 5 tickets · 4 Ko');
    // A copy that does not read: said, and it cannot be taken back.
    expect(copies[2]).toHaveTextContent('Illisible');
    expect(within(copies[2]).getByRole('button', { name: 'Reprendre' })).toBeDisabled();
  });

  it('compares a copy to today by name, writing nothing', async () => {
    const backend = fakeBackend({ backups_view: () => view(), compare_backup: () => DIFF });
    render(BackupsSection);
    const copies = await stateCopies();
    await userEvent.click(within(copies[1]).getByRole('button', { name: 'Comparer' }));
    const diff = await screen.findByTestId('backup-diff');
    expect(backend.called('compare_backup')[0].args).toEqual({ file: 'state.json', name: 'state-20261115T080000Z.json.gz' });
    expect(diff).toHaveTextContent('Rien n’est écrit');
    expect(diff).toHaveTextContent('Reviendraient : boutique.');
    expect(diff).toHaveTextContent('Reviendraient : BTQ-1 Panier, BTQ-2 Paiement.');
    expect(diff).toHaveTextContent('Seraient perdus : ATL-9 Facture.');
    expect(diff).toHaveTextContent('Seraient perdus : Relances.');
    expect(backend.called('restore_backup')).toHaveLength(0);
    expect(backend.called('back_up_now')).toHaveLength(0);
  });

  it('takes a copy back only once confirmed, saying what it replaces and what would be lost', async () => {
    const backend = fakeBackend({
      backups_view: () => view(),
      compare_backup: () => DIFF,
      restore_backup: () => ({ done: false, restart: true, setAside: null, closedId: null }),
    });
    render(BackupsSection);
    const copies = await stateCopies();
    await userEvent.click(within(copies[1]).getByRole('button', { name: 'Reprendre' }));
    await waitFor(() => expect(app.modal?.kind).toBe('confirm'));
    const modal = app.modal as any;
    expect(modal.title).toMatch(/^Reprendre la copie du /);
    expect(modal.body).toContain(
      'Le fichier state.json d’aujourd’hui (1 projet, 2 agents, 3 tickets) sera remplacé par cette copie (2 projets, 1 agent, 5 tickets)',
    );
    expect(modal.body).toContain('mis de côté sous un nom daté');
    expect(modal.body).toContain('Seraient perdus : ATL-9 Facture, Relances.');
    expect(backend.called('restore_backup')).toHaveLength(0);
    await modal.onConfirm(false);
    expect(backend.called('restore_backup')[0].args).toEqual({ file: 'state.json', name: 'state-20261115T080000Z.json.gz' });
    // Back to Avancé, on the copies.
    expect(app.modal).toMatchObject({ kind: 'settings', page: 'advanced', at: 'backups' });
  });

  it('holds the copies of a file the app holds back while an agent works, a history’s never', async () => {
    fakeBackend({ backups_view: () => view({ atWork: 2 }) });
    render(BackupsSection);
    expect(await screen.findByTestId('backups-at-work')).toHaveTextContent(
      '2 agents travaillent : la reprise attend la fin de leur travail, un redémarrage couperait leur tour.',
    );
    const copies = await stateCopies();
    expect(within(copies[0]).getByRole('button', { name: 'Reprendre' })).toBeDisabled();
    await userEvent.click(screen.getByText('Historique de 1 projet fermé'));
    const history = screen.getByText('closed-projects/c1.json').closest('li')!;
    await userEvent.click(within(history).getByRole('button', { name: 'Voir les copies' }));
    const all = screen.getAllByTestId('backup-copy');
    expect(within(all[all.length - 1]).getByRole('button', { name: 'Reprendre' })).toBeEnabled();
  });

  it('says the copy chosen for the next start, restarts or calls it off', async () => {
    const pending = { file: 'state.json', copy: 'state-20261115T080000Z.json.gz', copyAt: COPY_OLD, askedAt: COPY_NEW };
    const backend = fakeBackend({
      backups_view: () => view({ pending }),
      restart_for_restore: () => undefined,
      cancel_restore: () => undefined,
    });
    render(BackupsSection);
    const band = await screen.findByTestId('backups-pending');
    expect(band).toHaveTextContent(/La copie du .+ de state\.json sera reprise au prochain démarrage/);
    await userEvent.click(within(band).getByRole('button', { name: 'Redémarrer maintenant' }));
    expect(backend.called('restart_for_restore')).toHaveLength(1);
    await userEvent.click(within(band).getByRole('button', { name: 'Annuler la reprise' }));
    await waitFor(() => expect(backend.called('cancel_restore')).toHaveLength(1));
    expect(app.toasts.map((x) => x.text)).toContain('Reprise annulée : le fichier d’aujourd’hui reste tel quel.');
  });

  it('finds a lost closed project again and offers to reopen it', async () => {
    const lost = {
      id: 'c9',
      file: 'closed-projects/c9.json',
      name: 'perdu',
      agents: 2,
      tickets: 7,
      copy: 'c9-20261115T080000Z.json.gz',
      at: COPY_OLD,
    };
    let found = false;
    const backend = fakeBackend({
      backups_view: () => view({ lost: found ? [] : [lost] }),
      restore_backup: () => {
        found = true;
        return { done: true, restart: false, setAside: null, closedId: 'c9' };
      },
      closed_projects: () => [
        {
          id: 'c9',
          name: 'perdu',
          path: 'C:\\code\\perdu',
          color: '',
          worktreePerAgent: false,
          agents: 2,
          tickets: 7,
          lastActivity: 0,
          closedAt: 0,
          size: 0,
          unreadable: false,
        },
      ],
    });
    render(BackupsSection);
    const row = await screen.findByTestId('backup-lost');
    expect(row).toHaveTextContent('perdu');
    expect(row).toHaveTextContent('2 agents, 7 tickets');
    await userEvent.click(within(row).getByRole('button', { name: 'Retrouver' }));
    expect(backend.called('restore_backup')[0].args).toEqual({ file: 'closed-projects/c9.json', name: 'c9-20261115T080000Z.json.gz' });
    await userEvent.click(await screen.findByTestId('backup-reopen'));
    expect(app.modal).toEqual({ kind: 'newProject', path: 'C:\\code\\perdu', historyId: 'c9' });
  });

  it('makes a copy now and lists the files set aside, with their place on the disk', async () => {
    const backend = fakeBackend({
      backups_view: () =>
        view({
          setAside: [
            {
              path: 'C:\\data\\state.before-restore-20261116T100000Z.json',
              file: 'state.json',
              why: 'before-restore',
              size: 1024,
              at: COPY_NEW,
            },
          ],
        }),
      back_up_now: () => view(),
    });
    render(BackupsSection);
    expect(await screen.findByTestId('backups-set-aside')).toHaveTextContent('state.json, avant une reprise');
    expect(screen.getByText('C:\\data\\state.before-restore-20261116T100000Z.json')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Faire une copie maintenant' }));
    await waitFor(() => expect(backend.called('back_up_now')).toHaveLength(1));
    expect(app.toasts.map((x) => x.text)).toContain('Copies faites des fichiers qui ont changé.');
  });

  it('never shows a value of a field: only counts and names come from the core', async () => {
    fakeBackend({ backups_view: () => view() });
    render(BackupsSection);
    await screen.findByText('Comptes de tickets et d’hébergeurs');
    expect(document.body.textContent).not.toMatch(/token|ghp_|glpat/i);
  });
});
