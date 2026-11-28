// The band of the choice under an agent's page (AP-4; docs/AGENT-PAGES.md § 6): what the page proposed, said in the
// request's own words and never the page's; « Envoyer » by the box's own command, armed a moment after the proposal
// changed, on the user's click alone; « Annuler »; a go-ahead answered by the band's own buttons, never proposed from the
// page; the time to take the answer back, then the answer. And the page a request comes with, in « À toi ».
import { fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ARM_MS, answerOf, bandOf, chipsOf } from '../lib/choiceband';
import { setLang } from '../lib/i18n';
import { inbox } from '../lib/inbox/inbox.svelte';
import type { PageProposal, WindowAsk } from '../lib/types';
import { windowAsk } from '../test/inbox';
import { agent, fakeBackend, resetApp } from '../test/ipc';
import ChoiceBand from './ChoiceBand.svelte';
import PageThumb from './inbox/PageThumb.svelte';

const A = agent({ id: 'a7', name: 'pagination' });

const verdict = (over: Partial<WindowAsk> = {}) =>
  windowAsk({
    id: 'a-31',
    author: { agent: 'a7', name: 'pagination', chef: false },
    title: 'Fusionner POR-41 ?',
    options: [
      { id: 'a', label: 'Fusionner dans main', description: '' },
      { id: 'b', label: 'Renvoyer avec un mot', description: '' },
      { id: 'c', label: 'Attendre', description: '' },
    ],
    ...over,
  });

const proposal = (options: string[], note: string | null = null, at = 1): PageProposal => ({ ask: 'a-31', options, note, at });

const band = () => document.querySelector<HTMLElement>('[data-testid="choice-band"]');

beforeEach(() => {
  setLang('fr');
  resetApp({ agents: [A] });
  inbox.reset();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('what the band shows', () => {
  it('a proposal in the request’s words; an option it does not have, or two for one, is never shown', () => {
    const ask = verdict();
    expect(bandOf(ask, null)).toEqual({ kind: 'idle' });
    expect(bandOf(ask, proposal(['a']))).toEqual({
      kind: 'proposed',
      chips: [{ id: 'a', letter: 'A', label: 'Fusionner dans main' }],
      options: ['a'],
      note: null,
    });
    expect(bandOf(ask, proposal(['z']))).toEqual({ kind: 'idle' });
    expect(bandOf(ask, proposal(['a', 'b']))).toEqual({ kind: 'idle' });
    expect(bandOf(verdict({ multi: true }), proposal(['a', 'b'])).kind).toBe('proposed');
    // Another request's proposal: not this one's.
    expect(bandOf(ask, { ...proposal(['a']), ask: 'a-2' })).toEqual({ kind: 'idle' });
    expect(chipsOf(ask, [])).toBeNull();
  });

  it('a go-ahead is the band’s own yes and no, whatever the page proposes', () => {
    const gate = verdict({ kind: 'confirm', gate: true, options: [], yesLabel: 'Publier', noLabel: '' });
    expect(bandOf(gate, proposal(['yes']))).toEqual({ kind: 'gate', yes: 'Publier', no: 'Non' });
    const confirm = verdict({ kind: 'confirm', options: [], yesLabel: 'Garder' });
    expect(bandOf(confirm, proposal(['yes']))).toMatchObject({ kind: 'proposed', chips: [{ label: 'Garder' }] });
    expect(answerOf(confirm, ['no'], 'pas encore')).toEqual({ yes: false, note: 'pas encore', via: 'click' });
    expect(answerOf(verdict(), ['b'], null)).toEqual({ options: ['b'], via: 'click' });
  });

  it('answered: the time to take it back, then the answer; over, no band', () => {
    const at = Date.now();
    const answer = {
      options: ['a'],
      yes: null,
      text: null,
      done: null,
      steps: [],
      note: null,
      by: null,
      byName: null,
      via: 'click' as const,
      at,
    };
    const sending = verdict({ state: 'answered', answer: { ...answer, sendAt: at + 10_000, sentAt: null } });
    expect(bandOf(sending, null, at)).toMatchObject({ kind: 'sending', chips: [{ id: 'a' }] });
    expect(bandOf(sending, null, at + 11_000).kind).toBe('answered');
    expect(bandOf(verdict({ state: 'read', answer: { ...answer, sendAt: null, sentAt: at } }), null).kind).toBe('answered');
    for (const state of ['withdrawn', 'expired', 'retired'] as const) expect(bandOf(verdict({ state }), null)).toEqual({ kind: 'none' });
    expect(bandOf(verdict({ kind: 'answer' }), null)).toEqual({ kind: 'none' });
    expect(bandOf(null, proposal(['a']))).toEqual({ kind: 'none' });
  });
});

describe('the band under the page', () => {
  it('says what the page proposed in the request’s words, and sends it by the box’s command once armed', async () => {
    vi.useFakeTimers();
    const sent = verdict({ state: 'answered' });
    const be = fakeBackend({ inbox_answer: () => sent });
    inbox.asks = [verdict()];
    render(ChoiceBand, { page: 'p-12', askId: 'a-31', proposal: proposal(['a'], 'garde la courbe'), agent: 'pagination' });
    await tick();
    expect(band()?.dataset.state).toBe('proposed');
    expect(band()?.textContent).toContain('Choisi dans la page :');
    expect(band()?.textContent).toContain('A · Fusionner dans main');
    expect(band()?.textContent).toContain('« garde la courbe »');
    const send = screen.getByTestId('choice-send') as HTMLButtonElement;
    expect(send.textContent).toContain('Envoyer à pagination');
    // Not armed yet: a click does nothing.
    expect(send.disabled).toBe(true);
    await fireEvent.click(send);
    expect(be.called('inbox_answer')).toHaveLength(0);
    vi.advanceTimersByTime(ARM_MS + 10);
    await tick();
    expect(send.disabled).toBe(false);
    await fireEvent.click(send);
    await vi.waitFor(() => expect(be.called('inbox_answer')).toHaveLength(1));
    expect(be.called('inbox_answer')[0].args).toMatchObject({
      id: 'a-31',
      answer: { options: ['a'], note: 'garde la courbe', via: 'click' },
    });
  });

  it('« Annuler » lets the proposal go; no proposal, it waits for one, nothing to send', async () => {
    const be = fakeBackend({ agentpage_drop_proposal: () => true });
    inbox.asks = [verdict()];
    const r = render(ChoiceBand, { page: 'p-12', askId: 'a-31', proposal: proposal(['b']), agent: 'pagination' });
    await tick();
    await fireEvent.click(screen.getByTestId('choice-cancel'));
    expect(be.called('agentpage_drop_proposal')[0].args).toEqual({ page: 'p-12' });
    await r.rerender({ page: 'p-12', askId: 'a-31', proposal: null, agent: 'pagination' });
    await tick();
    expect(band()?.dataset.state).toBe('idle');
    expect(band()?.textContent).toContain('rien ne part sans ton geste ici');
    expect(screen.queryByTestId('choice-send')).toBeNull();
  });

  it('a go-ahead is answered by the band’s own buttons, never by what the page proposed', async () => {
    const be = fakeBackend({ inbox_answer: () => verdict({ state: 'answered' }) });
    inbox.asks = [verdict({ kind: 'confirm', gate: true, options: [], yesLabel: 'Publier' })];
    render(ChoiceBand, { page: 'p-12', askId: 'a-31', proposal: proposal(['yes']), agent: 'pagination' });
    await tick();
    expect(band()?.dataset.state).toBe('gate');
    expect(band()?.textContent).toContain('jamais depuis la page');
    expect(screen.queryByTestId('choice-send')).toBeNull();
    expect(be.called('inbox_answer')).toHaveLength(0);
    await fireEvent.click(screen.getByTestId('choice-yes'));
    await vi.waitFor(() => expect(be.called('inbox_answer')).toHaveLength(1));
    expect(be.called('inbox_answer')[0].args.answer).toEqual({ yes: true, via: 'click' });
  });

  it('in English, and answered: « Take my answer back »', async () => {
    setLang('en');
    const at = Date.now();
    const be = fakeBackend({ inbox_reopen: () => verdict() });
    inbox.asks = [
      verdict({
        state: 'answered',
        answer: {
          options: ['c'],
          yes: null,
          text: null,
          done: null,
          steps: [],
          note: null,
          by: null,
          byName: null,
          via: 'click',
          at,
          sendAt: at + 10_000,
          sentAt: null,
        },
      }),
    ];
    render(ChoiceBand, { page: 'p-12', askId: 'a-31', proposal: null, agent: 'pagination' });
    await tick();
    expect(band()?.dataset.state).toBe('sending');
    expect(band()?.textContent).toContain('C · Attendre');
    await fireEvent.click(screen.getByText('Take my answer back'));
    await vi.waitFor(() => expect(be.called('inbox_reopen')).toHaveLength(1));
  });
});

describe('the page in « À toi »', () => {
  it('its thumbnail, its title, its template and version, and « Ouvrir la page »', async () => {
    const be = fakeBackend({ agentpage_info: () => null });
    render(PageThumb, { page: { page: 'p-12', title: 'Revue avant fusion · POR-41', template: 'review', version: 2 } });
    const tile = screen.getByTestId('ask-page');
    expect(tile.textContent).toContain('La page qui l’accompagne');
    expect(tile.textContent).toContain('Revue avant fusion · POR-41');
    expect(tile.textContent).toContain('Revue · v2');
    expect(tile.querySelector('.mini.review')).not.toBeNull();
    await fireEvent.click(screen.getByTestId('ask-open-page'));
    await vi.waitFor(() => expect(be.called('agentpage_info')).toHaveLength(1));
  });
});
