import { render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
import type { PullRequestLink } from '../lib/types';
import { fakeBackend, resetApp } from '../test/ipc';
import { getByTip } from '../test/tip';
import PullRequests from './PullRequests.svelte';

const pr = (number: number, over: Partial<PullRequestLink> = {}): PullRequestLink => ({
  url: `https://github.com/acme/demo/pull/${number}`,
  number,
  title: '',
  state: 'open',
  checks: '',
  review: '',
  checkedAt: 1,
  ...over,
});

/** What each row of the list reads, its glyphs left out. */
const rows = () =>
  within(screen.getByRole('list', { name: 'Pull requests' }))
    .getAllByRole('listitem')
    .map((li) =>
      li
        .textContent!.replace(/[⇡●✓✗]/g, '')
        .replace(/\s+/g, ' ')
        .trim(),
    );

describe('PullRequests', () => {
  beforeEach(() => resetApp());

  it('lists each pull request with its number, its state, its checks and its reviews', () => {
    fakeBackend();
    render(PullRequests, {
      pulls: [
        pr(128, { state: 'merged' }),
        pr(131, { checks: 'passing', review: 'approved' }),
        pr(133, { state: 'draft', checks: 'failing' }),
      ],
    });
    expect(rows()).toEqual(['#133 brouillon CI rouge', '#131 ouverte CI verte approuvée', '#128 mergée']);
    const draft = screen.getAllByRole('listitem')[0];
    expect(draft).toHaveAttribute('data-state', 'draft');
    expect(draft.querySelector('.checks')).toHaveAttribute('data-checks', 'failing');
  });

  it('shows nothing for an agent without any', () => {
    fakeBackend();
    render(PullRequests, { pulls: [] });
    expect(screen.queryByRole('list')).toBeNull();
  });

  it('opens the page of a pull request from its number, named for a screen reader', async () => {
    const backend = fakeBackend();
    render(PullRequests, { pulls: [pr(128, { state: 'merged', title: 'Fix the login' })] });
    const number = screen.getByRole('button', { name: 'Ouvrir la pull request #128 dans le navigateur' });
    expect(number).toHaveTextContent('#128');
    await userEvent.click(number);
    expect(backend.called('plugin:opener|open_url')[0].args.url).toBe('https://github.com/acme/demo/pull/128');
  });

  it('leaves every key but the ones that press the number to the window’s shortcuts', async () => {
    fakeBackend();
    const keys: string[] = [];
    const heard = (e: KeyboardEvent) => keys.push(e.key);
    window.addEventListener('keydown', heard);
    try {
      render(PullRequests, { pulls: [pr(128)] });
      screen.getByRole('button', { name: 'Ouvrir la pull request #128 dans le navigateur' }).focus();
      await userEvent.keyboard('k{Escape}{Control>}j{/Control}{Enter} ');
      // Enter and the space bar press it: whatever holds it (an agent's card) is not told.
      expect(keys).toEqual(['k', 'Escape', 'Control', 'j']);
    } finally {
      window.removeEventListener('keydown', heard);
    }
  });

  it('keeps to the number of one GitHub told nothing of yet', () => {
    fakeBackend();
    render(PullRequests, { pulls: [pr(12, { state: '' })] });
    expect(rows()).toEqual(['#12']);
    expect(screen.getByRole('listitem')).toHaveAttribute('data-state', 'unknown');
  });

  it('shows no more than asked on a card, and counts the others', () => {
    fakeBackend();
    render(PullRequests, {
      pulls: [pr(1, { state: 'merged' }), pr(2, { state: 'merged' }), pr(3), pr(4), pr(5, { state: 'closed' })],
      max: 3,
    });
    expect(rows()).toEqual(['#4 ouverte', '#3 ouverte', '#5 fermée', '+ 2 autres']);
  });

  it('names the ones it only counts when the count is pointed at', async () => {
    fakeBackend();
    render(PullRequests, { pulls: [pr(1, { state: 'merged', title: 'Fix' }), pr(2, { state: 'closed' }), pr(3)], max: 1 });
    expect(getByTip(document.body, '#2 — fermée #1 Fix — mergée')).toHaveTextContent('+ 2 autres');
  });

  it('is written in English', () => {
    setLang('en');
    fakeBackend();
    render(PullRequests, {
      pulls: [pr(1, { state: 'merged' }), pr(3, { checks: 'pending', review: 'changes' }), pr(4, { state: 'closed' })],
      max: 2,
    });
    expect(rows()).toEqual(['#3 open checks running changes requested', '#4 closed', '+ 1 more']);
    expect(screen.getByRole('button', { name: 'Open pull request #3 in the browser' })).toBeInTheDocument();
  });
});
