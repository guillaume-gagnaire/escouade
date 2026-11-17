import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { app } from '../../lib/state.svelte';
import type { TeamDocsView } from '../../lib/team-docs';
import type { Project } from '../../lib/types';
import { fakeBackend, resetApp } from '../../test/ipc';
import { teamDocsView } from '../../test/team-docs';
import TeamDocsPlace from '../settings/TeamDocsPlace.svelte';
import TeamDocsBands from './TeamDocsBands.svelte';

// The docs of a team's project on the team server (TS-9): the bands above the Docs (a page written on both sides, a
// decision's field that lost, the repository's files after the import, what waits), and the choice per project.

/** The element `sel` once it shows. */
const found = (sel: string) =>
  waitFor(() => {
    const e = document.querySelector(sel);
    if (!e) throw new Error(`no ${sel}`);
    return e as HTMLElement;
  });

const PAGE = '---\ntitle: "Échéances"\n---\n\n';

function ofTeam(over: Partial<Project> = {}): Project {
  const p = { ...app.projects[0], team: 'acme', teamProject: '00112233445566778899aabbccddeeff', ...over };
  app.projects[0] = p;
  return p;
}

describe('the bands above the docs of a team’s project', () => {
  beforeEach(() => resetApp());

  it('shows a page written on both sides, its two versions, and settles it as chosen', async () => {
    let view: TeamDocsView = teamDocsView({
      conflicts: [
        {
          path: 'paiement/echeances',
          title: 'Échéances',
          mine: `${PAGE}Quatre échéances (moi).`,
          theirs: `${PAGE}Quatre échéances (Bob).`,
          base: `${PAGE}Trois échéances.`,
          agentId: null,
        },
      ],
    });
    const b = fakeBackend({
      team_docs_status: () => view,
      team_docs_settle: (a) => {
        view = teamDocsView();
        return { ...view, settled: a.choice };
      },
    });
    render(TeamDocsBands, { project: ofTeam() });
    const band = await screen.findByRole('alert');
    expect(band).toHaveTextContent('La page « Échéances » a été écrite ici et par un collègue en même temps');
    // Its two versions, side by side.
    await userEvent.click(within(band).getByRole('button', { name: 'Comparer' }));
    const sides = document.querySelector('[data-sides="paiement/echeances"]') as HTMLElement;
    expect(sides).toHaveTextContent('Quatre échéances (moi).');
    expect(sides).toHaveTextContent('Quatre échéances (Bob).');
    await userEvent.click(within(band).getByRole('button', { name: 'Prendre la sienne' }));
    expect(b.called('team_docs_settle')[0].args).toEqual({ id: 'p1', path: 'paiement/echeances', choice: 'theirs' });
    await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
  });

  it('gives a conflict to the writer agent, and says it works on it', async () => {
    const conflict = { path: 'a/b', title: 'B', mine: 'm', theirs: 't', base: '', agentId: null as string | null };
    const b = fakeBackend({
      team_docs_status: () => teamDocsView({ conflicts: [conflict] }),
      team_docs_settle: () => teamDocsView({ conflicts: [{ ...conflict, agentId: 'a1' }] }),
    });
    render(TeamDocsBands, { project: ofTeam() });
    await userEvent.click(await screen.findByRole('button', { name: 'Donner à l’agent rédacteur' }));
    expect(b.called('team_docs_settle')[0].args.choice).toBe('agent');
    expect(await screen.findByText('L’agent rédacteur fusionne les deux versions.')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Garder la mienne' })).toBeNull();
  });

  it('tells the one who lost a decision’s field, until « Compris »', async () => {
    const b = fakeBackend({
      team_docs_status: () =>
        teamDocsView({
          decisionConflicts: [
            { id: 'facturer', title: 'Facturer au mois', field: 'context', mine: 'Le mien.', theirs: 'Le sien.', stamp: 'ab' },
          ],
        }),
      team_docs_dismiss: () => teamDocsView(),
    });
    render(TeamDocsBands, { project: ofTeam() });
    const band = await found('[data-lost="facturer"]');
    expect(band).toHaveTextContent('Décision « Facturer au mois » : un collègue a changé « context » en même temps');
    expect(band).toHaveTextContent('La tienne : « Le mien. »');
    await userEvent.click(within(band).getByRole('button', { name: 'Compris' }));
    expect(b.called('team_docs_dismiss')[0].args).toEqual({ id: 'p1', stamp: 'ab' });
  });

  it('names the repository’s files left after the import, removes them at the user’s word, and says what waits', async () => {
    const files = ['.escouade/spec/a/b.md', '.escouade/spec/a/c.md', '.escouade/decisions/0004-x.md', '.escouade/documents/cahier.pdf'];
    const b = fakeBackend({
      team_docs_status: () =>
        teamDocsView({ repoFiles: files, pending: 2, online: false, waitingNumbers: ['choix'], tooLarge: ['film.mp4'] }),
      team_docs_remove_repo_files: () => teamDocsView({ online: false, pending: 2 }),
    });
    render(TeamDocsBands, { project: ofTeam() });
    const band = await found('[data-repo-files="4"]');
    expect(band).toHaveTextContent(
      '4 fichiers restent dans .escouade/ du dépôt : .escouade/spec/a/b.md, .escouade/spec/a/c.md, .escouade/decisions/0004-x.md et 1 de plus',
    );
    expect(band).toHaveTextContent('Escouade ne les retire pas sans toi');
    expect(document.querySelector('[data-sync="offline"]')).toHaveTextContent('Hors ligne · 2 changements en attente');
    expect(document.querySelector('[data-waiting="1"]')).toHaveTextContent('1 décision acceptée attend son numéro');
    expect(document.querySelector('[data-too-large="1"]')).toHaveTextContent('dépasse 20 Mo');
    await userEvent.click(within(band).getByRole('button', { name: 'Retirer du dépôt' }));
    expect(b.called('team_docs_remove_repo_files')).toHaveLength(1);
    await waitFor(() => expect(document.querySelector('[data-repo-files]')).toBeNull());
  });

  it('shows nothing for a project of no team, nor for docs kept in the repository', async () => {
    const b = fakeBackend({ team_docs_status: () => teamDocsView({ onServer: false }) });
    render(TeamDocsBands, { project: app.projects[0] });
    expect(document.querySelector('[data-team-docs]')).toBeNull();
    expect(b.called('team_docs_status')).toHaveLength(0);
  });
});

describe('the choice of a team project’s docs', () => {
  beforeEach(() => resetApp());

  it('says where they are, and moves them to the repository saying what it does', async () => {
    const b = fakeBackend({
      team_docs_status: () => teamDocsView(),
      team_docs_choose: (a) => teamDocsView({ onServer: a.server }),
    });
    render(TeamDocsPlace, { project: ofTeam() });
    const server = await screen.findByRole('button', { name: 'Sur le serveur d’équipe' });
    expect(server).toHaveAttribute('aria-pressed', 'true');
    expect(screen.getByText(/Revenir au dépôt y écrit les docs du serveur/)).toBeInTheDocument();
    expect(document.querySelector('[data-journal]')).toHaveTextContent('00112233445566778899aabbccddeeff');
    await userEvent.click(screen.getByRole('button', { name: 'Dans le dépôt' }));
    expect(b.called('team_docs_choose')[0].args).toEqual({ id: 'p1', server: false, journal: null });
    await waitFor(() => expect(screen.getByRole('button', { name: 'Dans le dépôt' })).toHaveAttribute('aria-pressed', 'true'));
    expect(screen.getByText(/Passer sur le serveur importe la spec/)).toBeInTheDocument();
  });

  it('follows a teammate’s journal for a project that has none yet', async () => {
    const b = fakeBackend({
      team_docs_status: () => teamDocsView({ onServer: false, journal: null }),
      team_docs_choose: () => teamDocsView(),
    });
    render(TeamDocsPlace, { project: ofTeam({ teamProject: null }) });
    const go = await screen.findByRole('button', { name: 'Suivre' });
    expect(go).toBeDisabled();
    await userEvent.type(screen.getByRole('textbox'), 'ffeeddccbbaa99887766554433221100');
    expect(go).toBeEnabled();
    await userEvent.click(go);
    expect(b.called('team_docs_choose')[0].args).toEqual({ id: 'p1', server: true, journal: 'ffeeddccbbaa99887766554433221100' });
  });

  it('says a project of no team, or a private one, keeps its docs', () => {
    fakeBackend();
    const { unmount } = render(TeamDocsPlace, { project: app.projects[0] });
    expect(screen.getByText('Ce projet n’est d’aucune équipe : ses docs sont dans son dépôt.')).toBeInTheDocument();
    unmount();
    render(TeamDocsPlace, { project: ofTeam({ private: true }) });
    expect(screen.getByText('Projet privé : ses docs ne partent jamais sur le serveur d’équipe.')).toBeInTheDocument();
  });
});
