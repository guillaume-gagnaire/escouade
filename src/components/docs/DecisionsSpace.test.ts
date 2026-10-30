import { render, screen, waitFor, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { decision } from '../../test/decisions';
import { docsStore } from '../../lib/docs.svelte';
import { app } from '../../lib/state.svelte';
import { fakeBackend, resetApp } from '../../test/ipc';
import type { DocsDecision, DocsTree } from '../../lib/types';
import DecisionsSpace from './DecisionsSpace.svelte';

const TREE: DocsTree = {
  branch: '',
  pages: 1,
  existingSpec: [],
  subjects: [
    {
      name: 'paiement',
      title: 'Paiement',
      pages: [{ path: 'paiement/echeances', title: 'Échéances', compared: '', tickets: [], decisions: [], gaps: 0, draft: false }],
    },
  ],
};

function register(): DocsDecision[] {
  return [
    decision(4, { title: 'Paiement par Stripe', origin: 'discussion', originRef: 'Prestataire de paiement', date: '2026-09-18' }),
    decision(9, {
      title: 'Paiement en 3 fois au plus',
      state: 'superseded',
      supersededBy: 13,
      origin: 'document',
      originRef: 'Cahier des charges v3',
      pages: ['paiement/echeances'],
      date: '2026-10-02',
    }),
    decision(13, {
      title: 'Paiement en 4 fois',
      origin: 'ticket',
      originRef: 'DEM-31',
      supersedes: [9],
      acceptedBy: 'Claire',
      acceptedAt: '2026-10-24T10:00:00+02:00',
      date: '2026-10-24',
      context: 'Les familles demandent plus d’échéances.',
      decision: 'Quatre échéances au plus.',
      consequences: 'Un échéancier à suivre.',
      changes: [{ page: 'paiement/echeances', added: 2, removed: 1 }],
      tickets: ['DEM-38'],
    }),
    // A proposal: no number until it is accepted (ESC-303), cited by its id.
    decision(null, {
      id: 'relances-par-e-mail-seulement',
      title: 'Relances par e-mail seulement',
      state: 'proposed',
      origin: 'ticket',
      originRef: 'DEM-40',
    }),
  ];
}

function backend(list: DocsDecision[]) {
  return fakeBackend({
    docs_spec_tree: () => TREE,
    docs_decisions: () => list,
    docs_drive: () => ({ files: [], folders: [] }),
    docs_recent: () => [],
    docs_pending: () => ({ files: [], count: 0, codeCommits: 0, conflict: null }),
    docs_decision_accept: (a) => {
      // Numbered when accepted: the next of the register.
      const d = list.find((x) => x.id === a.reference || x.number === a.reference)!;
      d.number = Math.max(...list.map((x) => x.number ?? 0)) + 1;
      d.state = 'accepted';
      d.acceptedBy = 'Léa';
      return d;
    },
    docs_decision_propose: (a) => {
      const d = decision(null, { ...a.decision, id: `proposee-${list.length}`, state: 'proposed' });
      list.push(d);
      return d;
    },
    docs_decision_replace: (a) => {
      const d = decision(null, { ...a.decision, id: `proposee-${list.length}`, state: 'proposed', supersedes: [a.number] });
      list.push(d);
      return d;
    },
  });
}

const rowOf = (n: number | string) => document.querySelector(`tr[data-decision="${n}"]`) as HTMLElement;
const RELANCES = 'relances-par-e-mail-seulement';

describe('the register of the decisions', () => {
  beforeEach(() => {
    resetApp();
    docsStore.decisions = {};
    docsStore.tree = {};
    app.now = new Date(2026, 9, 28, 15).getTime();
  });

  it('lists them numbered and dated, with their state, origin and what they changed, the latest first', async () => {
    backend(register());
    await docsStore.load('p1');
    render(DecisionsSpace, { project: app.projects[0] });
    expect(document.querySelector('[data-counts]')).toHaveTextContent('4 décisions · 2 acceptées · 1 proposée · 1 remplacée');
    const table = screen.getByRole('table', { name: 'Registre des décisions' });
    const numbers = [...table.querySelectorAll('tbody tr')].map((r) => r.getAttribute('data-decision'));
    expect(numbers).toEqual([RELANCES, '13', '9', '4']);
    expect(rowOf(13)).toHaveTextContent('013');
    expect(rowOf(13)).toHaveTextContent('Ticket DEM-31');
    expect(rowOf(13)).toHaveTextContent('Acceptée');
    expect(rowOf(13)).toHaveTextContent('Paiement');
    expect(rowOf(13)).toHaveTextContent('24/10');
    expect(rowOf(9)).toHaveTextContent('Remplacée');
    expect(rowOf(9)).toHaveTextContent('Paiement · par 013');
    expect(rowOf(9)).toHaveTextContent('Cahier des charges v3');
    expect(rowOf(4)).toHaveTextContent('Discussion « Prestataire de paiement »');
    // A proposal has no number yet.
    expect(rowOf(RELANCES).querySelector('.c-num')).toHaveTextContent('—');
    // Only a proposal is accepted.
    expect(within(rowOf(RELANCES)).getByRole('button', { name: `Accepter la décision ${RELANCES}` })).toBeInTheDocument();
    expect(within(rowOf(13)).queryByRole('button', { name: /Accepter/ })).toBeNull();
  });

  it('filters them by state', async () => {
    backend(register());
    await docsStore.load('p1');
    render(DecisionsSpace, { project: app.projects[0] });
    await userEvent.click(screen.getByRole('button', { name: 'Filtrer les décisions par état' }));
    await userEvent.click(screen.getByRole('menuitemradio', { name: /Remplacées/ }));
    expect([...document.querySelectorAll('tbody tr')].map((r) => r.getAttribute('data-decision'))).toEqual(['9']);
    expect(screen.getByRole('button', { name: 'Filtrer les décisions par état' })).toHaveTextContent('Remplacées');
  });

  it('shows the detail of the one chosen: its parts, what it changed, its links and its lineage', async () => {
    backend(register());
    await docsStore.load('p1');
    render(DecisionsSpace, { project: app.projects[0] });
    await userEvent.click(within(rowOf(13)).getByRole('button', { name: 'Paiement en 4 fois' }));
    const detail = screen.getByRole('complementary', { name: 'Paiement en 4 fois' });
    expect(detail).toHaveTextContent('ADR-013 · acceptée le 24/10 par Claire');
    expect(detail).toHaveTextContent('Les familles demandent plus d’échéances.');
    expect(detail).toHaveTextContent('Quatre échéances au plus.');
    expect(detail).toHaveTextContent('Un échéancier à suivre.');
    expect(within(detail).getByRole('button', { name: 'Ouvrir la page Échéances' })).toHaveTextContent('Échéances +2 −1');
    expect(detail.querySelector('[data-links]')).toHaveTextContent('DEM-38');
    expect(detail.querySelector('[data-links]')).toHaveTextContent('Ticket DEM-31');
    // What it replaces, and the old one says by which.
    await userEvent.click(within(detail).getByRole('button', { name: 'Voir la décision 009' }));
    const old = screen.getByRole('complementary', { name: 'Paiement en 3 fois au plus' });
    expect(old).toHaveTextContent('ADR-009 · remplacée par 013');
    expect(within(old).getByRole('button', { name: 'Voir la décision 013' })).toHaveTextContent('Remplacée par 013');
    // An accepted decision is not edited: only « La remplacer »; a superseded one, neither.
    expect(within(old).queryByRole('button', { name: 'La remplacer' })).toBeNull();
    // A page it changed opens in the spec.
    await userEvent.click(within(old).getByRole('button', { name: 'Ouvrir la page Échéances' }));
    expect(app.docsSpace.p1).toBe('spec');
    expect(app.docsPage.p1).toBe('paiement/echeances');
  });

  it('accepts a proposal in one click, and says one of a ticket is accepted with it', async () => {
    const b = backend(register());
    await docsStore.load('p1');
    render(DecisionsSpace, { project: app.projects[0] });
    // The latest, chosen by default: a proposal of ticket DEM-40.
    const detail = screen.getByRole('complementary', { name: 'Relances par e-mail seulement' });
    expect(detail).toHaveTextContent('Acceptée d’office quand le ticket DEM-40 sera validé.');
    expect(detail).toHaveTextContent(`${RELANCES} · proposée aujourd’hui, numérotée à son acceptation`);
    await userEvent.click(within(rowOf(RELANCES)).getByRole('button', { name: `Accepter la décision ${RELANCES}` }));
    await waitFor(() => expect(b.called('docs_decision_accept')).toHaveLength(1));
    // By its id, never a number.
    expect(b.called('docs_decision_accept')[0].args).toEqual({ id: 'p1', reference: RELANCES });
    // It takes the next number: 14, still the one shown.
    await waitFor(() => expect(rowOf(14)).toHaveTextContent('Acceptée'));
    expect(rowOf(14)).toHaveTextContent('014');
    expect(within(rowOf(14)).queryByRole('button', { name: /Accepter/ })).toBeNull();
    expect(screen.getByRole('complementary', { name: 'Relances par e-mail seulement' })).toHaveTextContent('ADR-014 · acceptée');
  });

  it('proposes a decision by hand, and replaces an accepted one with a new one', async () => {
    const b = backend(register());
    await docsStore.load('p1');
    render(DecisionsSpace, { project: app.projects[0] });
    await userEvent.click(screen.getByRole('button', { name: 'Proposer une décision' }));
    const form = screen.getByRole('dialog', { name: 'Proposer une décision' });
    await userEvent.type(within(form).getByRole('textbox', { name: 'Titre' }), 'Tarif famille');
    await userEvent.type(within(form).getByRole('textbox', { name: 'Décision' }), '−20 % dès le troisième membre.');
    await userEvent.click(within(form).getByRole('button', { name: 'Proposer' }));
    await waitFor(() => expect(b.called('docs_decision_propose')).toHaveLength(1));
    expect(b.called('docs_decision_propose')[0].args).toMatchObject({
      id: 'p1',
      reference: null,
      decision: { title: 'Tarif famille', decision: '−20 % dès le troisième membre.' },
    });
    await waitFor(() => expect(screen.getByRole('complementary', { name: 'Tarif famille' })).toBeInTheDocument());

    // « La remplacer » on an accepted one: a new proposal that replaces it.
    await userEvent.click(within(rowOf(4)).getByRole('button', { name: 'Paiement par Stripe' }));
    await userEvent.click(screen.getByRole('button', { name: 'La remplacer' }));
    const again = screen.getByRole('dialog', { name: 'Remplacer la décision 004' });
    expect(within(again).getByRole('combobox', { name: 'Remplace' })).toHaveValue('4');
    await userEvent.type(within(again).getByRole('textbox', { name: 'Titre' }), 'Paiement par Mollie');
    await userEvent.click(within(again).getByRole('button', { name: 'Proposer' }));
    await waitFor(() => expect(b.called('docs_decision_replace')).toHaveLength(1));
    expect(b.called('docs_decision_replace')[0].args).toMatchObject({ id: 'p1', number: 4, decision: { title: 'Paiement par Mollie' } });
  });
});
