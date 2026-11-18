import { render, screen, waitFor } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { filesToPour, ticketByKey, type PourItem, type PourPreview } from '../../lib/teamboard';
import { fakeBackend, resetApp, ticket } from '../../test/ipc';
import TeamPourModal from './TeamPourModal.svelte';

// « Verser à l'équipe » (TS-7): the preview first (the tickets of the files, ids kept; those of this machine to choose,
// all at once; what leaves and what stays), nothing sent before « Verser »; then what was done, and the files frozen
// that may be removed by hand.

const item = (over: Partial<PourItem>): PourItem => ({
  id: 'x',
  key: 'DEM-1',
  title: 'Un ticket',
  column: 'todo',
  external: false,
  poured: false,
  problem: null,
  ...over,
});
const PREVIEW: PourPreview = {
  files: [
    item({ id: 'k7m2pq9x', key: 'DEM-12', title: 'Le bail' }),
    item({ id: 'r4tz8wn3', key: 'ACME-7', title: 'Importé de Jira', external: true }),
    item({ id: 'old', key: 'DEM-3', title: 'Déjà versé', poured: true }),
    item({ id: 'bad', key: 'bad', title: 'bad.yml', problem: 'son fichier est en conflit après un pull' }),
  ],
  locals: [item({ id: 't1', key: 'DEM-L1', title: 'Brouillon' }), item({ id: 't2', key: 'DEM-L2', title: 'Note' })],
  folder: '.escouade/tickets',
  docs: {
    state: 'pending',
    pages: ['spec/general/accueil.md'],
    decisions: [],
    documents: ['documents/cahier.md'],
    memory: 'pending',
    memories: ['build-avant-tests'],
  },
};
const NO_DOCS = { state: 'none', pages: [], decisions: [], documents: [] } as const;

describe('« Verser à l’équipe » (TS-7)', () => {
  beforeEach(() => {
    setLang('fr');
    resetApp();
  });

  it('pours only what can be poured, and finds a ticket by a key it had', () => {
    expect(filesToPour(PREVIEW).map((f) => f.id)).toEqual(['k7m2pq9x', 'r4tz8wn3']);
    const t = ticket({ id: 'a', key: 'DEM-7', formerKeys: ['DEM-12@alice', 'DEM-12'] });
    expect(ticketByKey([t], 'p1', 'dem-12@alice')?.id).toBe('a');
    expect(ticketByKey([t], 'p1', 'DEM-7')?.id).toBe('a');
    expect(ticketByKey([t], 'other', 'DEM-12')).toBeUndefined();
    // Its own key goes first: another ticket that has it now wins over a former one.
    const now = ticket({ id: 'b', key: 'DEM-12' });
    expect(ticketByKey([t, now], 'p1', 'DEM-12')?.id).toBe('b');
  });

  it('shows the preview, sends nothing before the gesture, then pours what is checked', async () => {
    const b = fakeBackend({
      project_team_pour_preview: () => PREVIEW,
      project_team_pour: () => ({
        imported: 2,
        already: 1,
        locals: 2,
        files: ['k7m2pq9x.yml', 'r4tz8wn3.yml'],
        folder: '.escouade/tickets',
        docs: ['.escouade/spec/general/accueil.md', '.escouade/documents/cahier.md'],
      }),
    });
    render(TeamPourModal, { projectId: 'p1' });
    await screen.findByText('Le bail');
    expect(screen.getByText('Déjà sur le Kanban de l’équipe')).toBeInTheDocument();
    expect(screen.getByText('garde sa clé')).toBeInTheDocument();
    expect(screen.getByText('son fichier est en conflit après un pull')).toBeInTheDocument();
    expect(screen.getByText(/Ce qui part : la clé, le titre/)).toBeInTheDocument();
    expect(screen.getByText(/Ce qui reste sur ce poste : les images/)).toBeInTheDocument();
    // The docs of `.escouade/` that would go, named, and the shared memory with them (TS-10).
    expect(screen.getByTestId('team-pour-docs')).toHaveTextContent('1 page, 0 décision, 1 document');
    expect(screen.getByTestId('team-pour-docs')).toHaveTextContent('spec/general/accueil.md');
    expect(screen.getByTestId('team-pour-memory')).toHaveTextContent('1 souvenir · La mémoire partagée (.escouade/memory) part aussi');
    expect(screen.getByTestId('team-pour-docs')).toHaveTextContent('memory/build-avant-tests.md');
    // The two files that can go, and the docs, by default; none of this machine's tickets.
    expect(screen.getByTestId('team-pour-submit')).toHaveTextContent('Verser 2 tickets et 3 fichiers de docs');
    await userEvent.click(screen.getByTestId('team-pour-docs-take'));
    expect(screen.getByTestId('team-pour-submit')).toHaveTextContent(/^Verser 2 tickets$/);
    await userEvent.click(screen.getByTestId('team-pour-docs-take'));
    expect(b.calls.some((c) => c.cmd === 'project_team_pour')).toBe(false);
    // All of this machine's at once.
    await userEvent.click(screen.getByRole('button', { name: 'Tout cocher' }));
    expect(screen.getByTestId('team-pour-submit')).toHaveTextContent('Verser 4 tickets et 3 fichiers de docs');
    await userEvent.click(screen.getByRole('button', { name: 'Tout décocher' }));
    await userEvent.click(screen.getAllByTestId('team-pour-local')[1]);
    expect(screen.getByTestId('team-pour-submit')).toHaveTextContent('Verser 3 tickets et 3 fichiers de docs');
    await userEvent.click(screen.getByTestId('team-pour-submit'));
    await waitFor(() =>
      expect(b.calls.find((c) => c.cmd === 'project_team_pour')?.args).toEqual({ project: 'p1', files: true, tickets: ['t2'], docs: true }),
    );
    expect(await screen.findByTestId('team-pour-done')).toHaveTextContent(
      'Versé : 2 depuis les fichiers, 2 de ce poste. 1 déjà sur le Kanban de l’équipe.',
    );
    // The files frozen, named; removing them is the user's.
    expect(screen.getByTestId('team-pour-frozen')).toHaveTextContent('.escouade/tickets/k7m2pq9x.yml');
    expect(screen.getByText('git rm -r .escouade/tickets')).toBeInTheDocument();
    expect(screen.getByTestId('team-pour-docs-done')).toHaveTextContent('Docs versées : 2 fichiers');
  });

  it('says why the docs do not go, and pours the docs alone', async () => {
    for (const [state, said] of [
      ['server', 'déjà sur le serveur d’équipe'],
      ['repo', 'L’équipe garde les docs de ce projet dans le dépôt'],
      ['private', 'Projet privé'],
      ['none', 'Aucune spec, décision ni document dans le dépôt.'],
    ] as const) {
      fakeBackend({ project_team_pour_preview: () => ({ ...PREVIEW, files: [], locals: [], docs: { ...NO_DOCS, state } }) });
      const { unmount } = render(TeamPourModal, { projectId: 'p1' });
      expect(await screen.findByTestId('team-pour-docs')).toHaveTextContent(said);
      expect(screen.queryByTestId('team-pour-docs-take')).toBeNull();
      expect(screen.getByTestId('team-pour-submit')).toBeDisabled();
      unmount();
    }
    const b = fakeBackend({
      project_team_pour_preview: () => ({ ...PREVIEW, files: [], locals: [] }),
      project_team_pour: () => ({ imported: 0, already: 0, locals: 0, files: [], folder: '.escouade/tickets', docs: [] }),
    });
    render(TeamPourModal, { projectId: 'p1' });
    await waitFor(() => expect(screen.getByTestId('team-pour-submit')).toHaveTextContent('Verser 3 fichiers de docs'));
    await userEvent.click(screen.getByTestId('team-pour-submit'));
    await waitFor(() =>
      expect(b.calls.find((c) => c.cmd === 'project_team_pour')?.args).toEqual({ project: 'p1', files: false, tickets: [], docs: true }),
    );
  });

  it('pours the shared memory alone when the docs are on the team server already (TS-10)', async () => {
    const b = fakeBackend({
      project_team_pour_preview: () => ({
        ...PREVIEW,
        files: [],
        locals: [],
        docs: { ...NO_DOCS, state: 'server', memory: 'pending', memories: ['build-avant-tests', 'ports'] },
      }),
      project_team_pour: () => ({ imported: 0, already: 0, locals: 0, files: [], folder: '.escouade/tickets', docs: [] }),
    });
    render(TeamPourModal, { projectId: 'p1' });
    expect(await screen.findByTestId('team-pour-memory')).toHaveTextContent('2 souvenirs');
    expect(screen.getByTestId('team-pour-docs')).toHaveTextContent('Les docs sont déjà sur le serveur d’équipe');
    expect(screen.getByTestId('team-pour-docs')).toHaveTextContent('memory/ports.md');
    expect(screen.getByTestId('team-pour-submit')).toHaveTextContent('Verser 2 fichiers de docs');
    await userEvent.click(screen.getByTestId('team-pour-submit'));
    await waitFor(() =>
      expect(b.calls.find((c) => c.cmd === 'project_team_pour')?.args).toEqual({ project: 'p1', files: false, tickets: [], docs: true }),
    );
  });

  it('pours nothing without a file nor a ticket chosen', async () => {
    fakeBackend({ project_team_pour_preview: () => ({ files: [], locals: [], folder: '.escouade/tickets', docs: NO_DOCS }) });
    render(TeamPourModal, { projectId: 'p1' });
    expect(await screen.findByText('Aucun fichier de ticket dans ce dépôt.')).toBeInTheDocument();
    expect(screen.getByTestId('team-pour-submit')).toBeDisabled();
    expect(screen.getByTestId('team-pour-submit')).toHaveTextContent('Rien à verser');
  });
});
