import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { app } from '../../lib/state.svelte';
import { teams, type TeamView } from '../../lib/teams.svelte';
import { fakeBackend, project, resetApp } from '../../test/ipc';
import TeamModal from './TeamModal.svelte';
import { fill } from '../../test/fill';

const SIX = ['alpha', 'bravo', 'charlie', 'delta', 'echo', 'foxtrot'];
const WORDS = Array.from({ length: 24 }, (_, i) => `mot${i + 1}`);

function team(over: Partial<TeamView> = {}): TeamView {
  return {
    id: '00112233445566778899aabbccddeeff',
    name: 'ACME',
    server: 'team.acme.fr',
    joined: true,
    role: 'admin',
    admins: 1,
    members: 1,
    epoch: 0,
    fingerprint: SIX,
    head: ['un', 'deux', 'trois', 'quatre', 'cinq', 'six'],
    escrow: null,
    keysInFile: false,
    recoveryConfirmed: true,
    wordsPending: false,
    recoveryOutdated: false,
    recoveryStale: false,
    alarm: null,
    invitations: [],
    snapshotAlerts: [],
    lastError: null,
    ...over,
  };
}

const INVITE = {
  url: 'escouade://team/join/team.acme.fr/00112233445566778899aabbccddeeff#s=AAAA&t=BBBB',
  expiresAt: 1_900_000_000,
  ready: true,
  qr: '<svg xmlns="http://www.w3.org/2000/svg" data-testid="qr"><rect width="10" height="10"/></svg>',
};

describe('the screens of the team server (TS-3)', () => {
  beforeEach(() => {
    resetApp();
    teams.list = [];
    teams.loaded = false;
    app.modal = { kind: 'team' };
  });

  it('lists the teams: role, six words, escrow always shown, a second administrator advised', async () => {
    fakeBackend({ team_list: () => [team({ escrow: 'le RSSI d’ACME' })] });
    render(TeamModal);
    const card = await screen.findByRole('region', { name: 'ACME' });
    expect(within(card).getByText('Administrateur')).toBeInTheDocument();
    expect(within(card).getByText('team.acme.fr')).toBeInTheDocument();
    for (const w of SIX) expect(within(card).getByText(w)).toBeInTheDocument();
    expect(within(card).getByRole('note')).toHaveTextContent(
      'Cette équipe a une clé de séquestre : l’organisation peut lire son contenu. Détenteur : le RSSI d’ACME.',
    );
    expect(within(card).getByText(/nomme-en un second/)).toBeInTheDocument();
    // The ways in, for this device.
    for (const name of [/Créer une équipe/, /Rejoindre par un lien/, /Ajouter cet appareil par un code/, /Récupérer une équipe/])
      expect(screen.getByRole('button', { name })).toBeInTheDocument();
  });

  it('says the fork, what is stopped, the head to compare, and lets the user try again or remove the team', async () => {
    const backend = fakeBackend({
      team_list: () => [team({ alarm: { kind: 'fork', epoch: 4, at: 1 } })],
      team_sync: () => team({ alarm: { kind: 'fork', epoch: 4, at: 2 } }),
      team_forget: () => null,
    });
    render(TeamModal);
    const alarm = await screen.findByRole('alert');
    expect(alarm).toHaveTextContent('Le serveur montre deux versions de l’équipe : contacte un administrateur.');
    expect(alarm).toHaveTextContent('époque 4');
    expect(alarm).toHaveTextContent(/n’y envoie plus rien : ni invitation/);
    for (const w of ['un', 'deux', 'six']) expect(within(alarm).getByText(w)).toBeInTheDocument();
    // Nothing that sends to the team is offered.
    expect(screen.queryByRole('button', { name: 'Inviter…' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'Ajouter un appareil…' })).toBeNull();
    await userEvent.click(within(alarm).getByRole('button', { name: 'Réessayer' }));
    expect(backend.called('team_sync')[0].args).toEqual({ team: team().id });
    await userEvent.click(within(alarm).getByRole('button', { name: 'Retirer l’équipe de cet appareil' }));
    const confirm = screen.getByRole('alertdialog');
    await userEvent.click(within(confirm).getByRole('button', { name: 'Retirer de cet appareil' }));
    expect(backend.called('team_forget')[0].args).toEqual({ team: team().id });
  });

  it('says a snapshot put aside, who posted it, and lets an administrator remove it (SR2-01)', async () => {
    const alert = {
      project: 'aa'.repeat(16),
      upto: 42,
      postedBy: '0123456789abcdef0123456789abcdef',
      postedByRole: 'member' as const,
      postedByMe: false,
      postedAt: 1_800_000_000,
      foundAt: 1_800_000_100,
      foundHere: false,
    };
    app.projects = [project({ id: 'p1', name: 'Atlas', teamProject: 'aa'.repeat(16) })];
    const backend = fakeBackend({
      team_list: () => [team({ snapshotAlerts: [alert] })],
      team_remove_snapshot: () => team(),
    });
    render(TeamModal);
    const band = await screen.findByRole('alert', { name: 'Instantanés écartés' });
    expect(band).toHaveTextContent('Atlas · jusqu’à l’entrée 42');
    expect(band).toHaveTextContent(/par l’appareil 01234567 \(Membre\)/);
    expect(band).toHaveTextContent(/Signalé par un appareil de l’équipe/);
    expect(within(band).queryByRole('button', { name: 'Masquer' })).toBeNull();
    await userEvent.click(within(band).getByRole('button', { name: 'Retirer l’instantané' }));
    expect(backend.called('team_remove_snapshot')[0].args).toEqual({ team: team().id, project: alert.project, upto: 42 });
    expect(screen.queryByRole('alert', { name: 'Instantanés écartés' })).toBeNull();
  });

  it('lets a member hide a snapshot this device put aside (SR2-01)', async () => {
    const alert = {
      project: 'bb'.repeat(16),
      upto: 7,
      postedBy: 'ffeeddccbbaa99887766554433221100',
      postedByRole: null,
      postedByMe: false,
      postedAt: 1_800_000_000,
      foundAt: 1_800_000_100,
      foundHere: true,
    };
    const backend = fakeBackend({
      team_list: () => [team({ role: 'member', snapshotAlerts: [alert] })],
      team_dismiss_snapshot: () => team({ role: 'member' }),
    });
    render(TeamModal);
    const band = await screen.findByRole('alert', { name: 'Instantanés écartés' });
    expect(band).toHaveTextContent('bbbbbbbb · jusqu’à l’entrée 7');
    expect(band).toHaveTextContent(/Écarté par cet appareil/);
    expect(band).toHaveTextContent(/administrateurs de l’équipe sont prévenus/);
    expect(within(band).queryByRole('button', { name: 'Retirer l’instantané' })).toBeNull();
    await userEvent.click(within(band).getByRole('button', { name: 'Masquer' }));
    expect(backend.called('team_dismiss_snapshot')[0].args).toEqual({ team: team().id, project: alert.project, upto: 7 });
  });

  it('creates a team, shows its 24 words once, and asks three of them back', async () => {
    const created = team({ recoveryConfirmed: false, wordsPending: true });
    const backend = fakeBackend({
      team_list: () => [],
      team_create: () => ({ team: created, words: WORDS }),
      team_confirm_words: (a: { typed: [number, string][] }) => a.typed.every(([i, w]) => w === WORDS[i]),
    });
    render(TeamModal, { props: { step: 'create' } });
    const create = screen.getByRole('button', { name: 'Créer l’équipe' });
    expect(create).toBeDisabled();
    await fill(screen.getByLabelText(/Adresse du serveur/), 'team.acme.fr');
    await fill(screen.getByLabelText(/Jeton de création/), 'le-jeton');
    await fill(screen.getByLabelText(/Nom de l’équipe/), 'ACME');
    await userEvent.click(create);
    expect(backend.called('team_create')[0].args).toEqual({
      server: 'team.acme.fr',
      token: 'le-jeton',
      name: 'ACME',
      escrowPublic: null,
      escrowHolder: null,
    });
    // The words, numbered, and the team's six words for the same sheet (SR2-09: the recovery asks for both).
    const grid = await screen.findByRole('list', { name: 'La clé de secours' });
    expect(within(grid).getAllByRole('listitem')).toHaveLength(24);
    expect(within(grid).getByText('mot24')).toBeInTheDocument();
    const sheet = screen.getByRole('group', { name: 'Les six mots de l’équipe' });
    for (const w of SIX) expect(within(sheet).getByText(w)).toBeInTheDocument();
    expect(within(sheet).getByText(/la récupération les demande aussi/)).toBeInTheDocument();
    const next = screen.getByRole('button', { name: 'Continuer' });
    expect(next).toBeDisabled();
    await userEvent.click(screen.getByLabelText('J’ai noté les 24 mots et les six mots de l’équipe'));
    await userEvent.click(next);
    // Three words asked, by their numbers: a wrong one is refused.
    const inputs = screen.getAllByLabelText(/Mot n° \d+/);
    expect(inputs).toHaveLength(3);
    const asked = inputs.map((i) => Number(i.closest('label')!.textContent!.match(/\d+/)![0]) - 1);
    await fill(inputs[0], 'faux');
    await fill(inputs[1], WORDS[asked[1]]);
    await fill(inputs[2], WORDS[asked[2]]);
    await userEvent.click(screen.getByRole('button', { name: 'Confirmer' }));
    expect(await screen.findByText('Ce ne sont pas les bons mots : regarde ta copie.')).toBeInTheDocument();
    // Put in place of the wrong one.
    await fill(inputs[0], WORDS[asked[0]]);
    backend.calls.length = 0;
    await userEvent.click(screen.getByRole('button', { name: 'Confirmer' }));
    expect(backend.called('team_confirm_words')[0].args).toEqual({
      team: created.id,
      typed: asked.map((p) => [p, WORDS[p]]),
    });
    // The summary: the six words, the second administrator, the id for a recovery token.
    expect(await screen.findByRole('dialog', { name: 'L’équipe est prête' })).toBeInTheDocument();
    for (const w of SIX) expect(screen.getByText(w)).toBeInTheDocument();
    expect(screen.getByText(/invite un second administrateur/)).toBeInTheDocument();
    expect(screen.getByText(/recovery-token --team 00112233445566778899aabbccddeeff/)).toBeInTheDocument();
  });

  it('adds an escrow key made here, its secret shown once for its holder', async () => {
    const backend = fakeBackend({
      team_list: () => [],
      team_escrow_new: (a: { holder: string }) => ({
        public: 'escouade-escrow-team:2:PUB.ANC',
        secret: 'escouade-escrow-secret:1:SEC',
        holder: a.holder,
      }),
      team_create: () => ({ team: team({ escrow: 'le RSSI' }), words: WORDS }),
    });
    render(TeamModal, { props: { step: 'create' } });
    await userEvent.type(screen.getByLabelText(/Adresse du serveur/), 'team.acme.fr');
    await userEvent.type(screen.getByLabelText(/Jeton de création/), 'le-jeton');
    await userEvent.click(screen.getByLabelText('Ajouter une clé de séquestre de l’organisation'));
    expect(screen.getByText(/l’organisation peut toujours le lire. Chaque membre le voit/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Créer l’équipe' })).toBeDisabled();
    await userEvent.type(screen.getByLabelText('Qui la détient'), 'le RSSI');
    await userEvent.click(screen.getByRole('button', { name: 'Créer la paire ici' }));
    expect(await screen.findByText('escouade-escrow-secret:1:SEC')).toBeInTheDocument();
    expect(screen.getByLabelText('Sa moitié pour l’équipe')).toHaveValue('escouade-escrow-team:2:PUB.ANC');
    await userEvent.click(screen.getByRole('button', { name: 'Créer l’équipe' }));
    expect(backend.called('team_create')[0].args).toMatchObject({
      escrowPublic: 'escouade-escrow-team:2:PUB.ANC',
      escrowHolder: 'le RSSI',
    });
  });

  it('invites: a role, a life, one use; the link to send and its QR code', async () => {
    const backend = fakeBackend({ team_list: () => [team()], team_invite: () => INVITE });
    render(TeamModal, { props: { step: 'invite', teamId: team().id } });
    expect(screen.getByText(/Un lien sert une seule fois/)).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: '72 h' })).toHaveAttribute('aria-checked', 'true');
    await userEvent.click(screen.getByRole('radio', { name: 'Administrateur' }));
    await userEvent.click(screen.getByRole('radio', { name: '7 jours' }));
    await userEvent.click(screen.getByRole('button', { name: 'Créer le lien' }));
    expect(backend.called('team_invite')[0].args).toEqual({ team: team().id, role: 'admin', hours: 168 });
    expect(await screen.findByLabelText('Le lien à envoyer')).toHaveValue(INVITE.url);
    expect(screen.getByRole('img', { name: 'Ou fais scanner ce QR code' }).querySelector('svg')).not.toBeNull();
    expect(screen.getByText(/Prêt : l’invité peut venir/)).toBeInTheDocument();
  });

  it('joins by a link: its server and six words to check, then waits for an administrator online', async () => {
    const waiting = team({ joined: false, role: 'none' });
    const backend = fakeBackend({
      team_list: () => [waiting],
      team_join_preview: () => ({ server: 'team.acme.fr', fingerprint: SIX, ownDevice: false }),
      team_join: () => waiting,
    });
    render(TeamModal, { props: { step: 'join', link: INVITE.url } });
    expect(screen.getByLabelText('Lien d’invitation')).toHaveValue(INVITE.url);
    expect(await screen.findByText(/Vérifie ces six mots avec la personne qui t’invite/)).toBeInTheDocument();
    for (const w of SIX) expect(screen.getByText(w)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Rejoindre' }));
    expect(backend.called('team_join')[0].args).toEqual({ link: INVITE.url, name: '' });
    expect(await screen.findByRole('status')).toHaveTextContent('Un administrateur en ligne te fait entrer');
    // The administrator let it in: the backend says the team changed.
    teams.put(team({ role: 'member', escrow: 'le RSSI' }));
    expect(await screen.findByText('Tu es dans l’équipe « ACME ».')).toBeInTheDocument();
    expect(screen.getByRole('note')).toHaveTextContent('l’organisation peut lire son contenu');
  });

  it('refuses a link of the old form (https://…/join/…) with the word to ask for a new one, nothing sent', async () => {
    const OLD = 'Ce lien d’invitation a l’ancienne forme (https://…/join/…) et ne sert plus : demande un nouveau lien à qui t’invite.';
    const backend = fakeBackend({
      team_list: () => [],
      team_join_preview: () => {
        throw OLD;
      },
    });
    render(TeamModal, { props: { step: 'join', link: 'https://team.acme.fr/join/00112233445566778899aabbccddeeff#s=AAAA&t=BBBB' } });
    expect(await screen.findByText(/a l’ancienne forme/)).toHaveTextContent('demande un nouveau lien');
    expect(screen.queryByText(/Vérifie ces six mots/)).toBeNull();
    expect(screen.getByRole('button', { name: 'Rejoindre' })).toBeDisabled();
    expect(backend.called('team_join')).toEqual([]);
  });

  it('opened by a link, shows its server and six words and joins only on « Rejoindre »', async () => {
    const backend = fakeBackend({
      team_list: () => [],
      team_join_preview: () => ({ server: 'team.acme.fr', fingerprint: SIX, ownDevice: false }),
      team_join: () => team({ joined: false, role: 'none' }),
    });
    render(TeamModal, { props: { step: 'join', link: INVITE.url } });
    expect(await screen.findByText('team.acme.fr')).toBeInTheDocument();
    for (const w of SIX) expect(screen.getByText(w)).toBeInTheDocument();
    expect(backend.called('team_join')).toEqual([]);
    await userEvent.click(screen.getByRole('button', { name: 'Retour' }));
    expect(backend.called('team_join')).toEqual([]);
  });

  it('says an invitation already used, as the backend says it', async () => {
    fakeBackend({
      team_list: () => [],
      team_join_preview: () => ({ server: 'team.acme.fr', fingerprint: SIX, ownDevice: false }),
      team_join: () => {
        throw 'Cette invitation a déjà servi : demande un nouveau lien à son administrateur, qui est prévenu.';
      },
    });
    render(TeamModal, { props: { step: 'join', link: INVITE.url } });
    await screen.findByText(/Vérifie ces six mots/);
    await userEvent.click(screen.getByRole('button', { name: 'Rejoindre' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Cette invitation a déjà servi');
  });

  it('adds a device of one’s own by a QR code, or by a short code that has one try', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    let state = 'waiting';
    const backend = fakeBackend({
      team_list: () => [team()],
      team_own_device: () => INVITE,
      team_pair_start: () => ({ code: 'K7Q4-M2XZ', server: 'team.acme.fr', expiresAt: 1_900_000_000, state: 'waiting' }),
      team_pair_step: () => ({ code: 'K7Q4-M2XZ', server: 'team.acme.fr', expiresAt: 1_900_000_000, state }),
      team_pair_cancel: () => null,
    });
    render(TeamModal, { props: { step: 'device', teamId: team().id } });
    expect(await screen.findByLabelText('Le lien à envoyer')).toHaveValue(INVITE.url);
    expect(backend.called('team_own_device')[0].args).toEqual({ team: team().id });
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    await user.click(screen.getByRole('tab', { name: 'Code court' }));
    await user.click(screen.getByRole('button', { name: 'Afficher un code' }));
    expect(await screen.findByTestId('pair-code')).toHaveTextContent('K7Q4-M2XZ');
    expect(screen.getByText('Serveur : team.acme.fr')).toBeInTheDocument();
    expect(screen.getByText('Un code n’a qu’un essai, et 10 minutes.')).toBeInTheDocument();
    await vi.advanceTimersByTimeAsync(1100);
    expect(screen.getByRole('status')).toHaveTextContent('En attente de l’autre appareil');
    state = 'wrongCode';
    await vi.advanceTimersByTimeAsync(1100);
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent('Un mauvais code a été tapé : ce code ne sert plus.'));
    const steps = backend.called('team_pair_step').length;
    await vi.advanceTimersByTimeAsync(3000);
    expect(backend.called('team_pair_step')).toHaveLength(steps);
    expect(screen.getByRole('button', { name: 'Nouveau code' })).toBeInTheDocument();
    vi.useRealTimers();
  });

  it('adds this device by a code: typed loosely, sent whole', async () => {
    const backend = fakeBackend({ team_list: () => [], team_pair_join: () => team({ joined: false, role: 'none' }) });
    render(TeamModal, { props: { step: 'pairJoin' } });
    const submit = screen.getByRole('button', { name: 'Ajouter cet appareil' });
    await userEvent.type(screen.getByLabelText('Adresse du serveur'), 'team.acme.fr');
    await userEvent.type(screen.getByLabelText('Code de 8 caractères'), 'k7q4m2x');
    expect(submit).toBeDisabled();
    await userEvent.type(screen.getByLabelText('Code de 8 caractères'), 'z');
    expect(screen.getByLabelText('Code de 8 caractères')).toHaveValue('K7Q4-M2XZ');
    await userEvent.click(submit);
    expect(backend.called('team_pair_join')[0].args).toEqual({ server: 'team.acme.fr', code: 'K7Q4-M2XZ', name: '' });
  });

  it('recovers a team by its 24 words, its six words and the operator’s token, then shows the new key', async () => {
    const backend = fakeBackend({
      team_list: () => [],
      team_recover: () => ({ team: team({ members: 3, admins: 2 }), words: WORDS }),
    });
    render(TeamModal, { props: { step: 'recover' } });
    const submit = screen.getByRole('button', { name: 'Récupérer l’équipe' });
    await userEvent.type(screen.getByLabelText('Adresse du serveur'), 'team.acme.fr');
    await userEvent.type(screen.getByLabelText(/Jeton de récupération/), 'jeton-de-l-hebergeur');
    // The words are pasted, as from a password manager.
    const field = screen.getByLabelText(/Les 24 mots/);
    await userEvent.click(field);
    await userEvent.paste(WORDS.slice(0, 23).join(' '));
    expect(submit).toBeDisabled();
    await userEvent.paste(' MOT24');
    // The team's six words, from the sheet: the recovery waits for all six.
    expect(submit).toBeDisabled();
    const six = screen.getByLabelText(/Les six mots de l’équipe/);
    await fill(six, 'Alpha bravo charlie delta echo');
    expect(submit).toBeDisabled();
    await fill(six, 'Alpha  bravo charlie delta echo FOXTROT');
    await userEvent.click(submit);
    expect(backend.called('team_recover')[0].args).toEqual({
      server: 'team.acme.fr',
      token: 'jeton-de-l-hebergeur',
      words: [...WORDS.slice(0, 23), 'mot24'].join(' '),
      teamWords: SIX.join(' '),
      name: '',
    });
    expect(await screen.findByRole('list', { name: 'La clé de secours' })).toBeInTheDocument();
  });

  it('tells a rescue sheet without the team’s six words as too old, and sends nothing', async () => {
    const backend = fakeBackend({ team_list: () => [] });
    render(TeamModal, { props: { step: 'recover' } });
    await fill(screen.getByLabelText('Adresse du serveur'), 'team.acme.fr');
    await fill(screen.getByLabelText(/Jeton de récupération/), 'jeton');
    await fill(screen.getByLabelText(/Les 24 mots/), WORDS.join(' '));
    await userEvent.click(screen.getByLabelText('Ma feuille n’a pas ces six mots'));
    expect(screen.getByRole('note')).toHaveTextContent(/fais-y une nouvelle clé de secours/);
    expect(screen.getByLabelText(/Les six mots de l’équipe/)).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Récupérer l’équipe' })).toBeDisabled();
    expect(backend.called('team_recover')).toHaveLength(0);
  });

  it('asks an administrator for a new rescue sheet when the recovery key predates the six words', async () => {
    fakeBackend({ team_list: () => [team({ recoveryOutdated: true })] });
    render(TeamModal);
    expect(await screen.findByText(/date d’avant les six mots de l’équipe/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Nouvelle clé de secours' })).toBeInTheDocument();
  });

  it('advises a new rescue sheet once an administrator under the current one has left their role', async () => {
    fakeBackend({
      team_list: () => [team({ recoveryStale: true }), team({ id: 'ff', name: 'Autre', role: 'member', recoveryStale: true })],
    });
    render(TeamModal);
    const own = await screen.findByRole('region', { name: 'ACME' });
    expect(within(own).getByText(/Un administrateur a quitté son rôle/)).toBeInTheDocument();
    // A member cannot make one: not told.
    expect(within(screen.getByRole('region', { name: 'Autre' })).queryByText(/Un administrateur a quitté son rôle/)).toBeNull();
  });

  it('tells an invitation reused after its invitee', async () => {
    fakeBackend({
      team_list: () => [
        team({
          invitations: [
            { id: 'a1', kind: 'member', role: 'member', expiresAt: 1_900_000_000, used: true, ready: true, reusedAt: 1_800_000_000 },
          ],
        }),
      ],
    });
    render(TeamModal);
    expect(await screen.findByText('Nouveau membre · Membre')).toBeInTheDocument();
    expect(screen.getByText('Utilisée')).toBeInTheDocument();
    expect(screen.getByText(/Quelqu’un a réutilisé ce lien après son invité/)).toBeInTheDocument();
  });
});
