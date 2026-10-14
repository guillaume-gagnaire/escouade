import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import { pullFacts, pullTip, shownPulls } from './pulls';
import type { PullRequestLink } from './types';

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

describe('pull requests of an agent', () => {
  beforeEach(() => setLang('fr'));

  it('shows those still to merge first, the latest first among each', () => {
    const list = [pr(3, { state: 'merged' }), pr(12), pr(9, { state: 'closed' }), pr(14, { state: 'draft' }), pr(7, { state: '' })];
    expect(shownPulls(list).map((p) => p.number)).toEqual([14, 12, 7, 9, 3]);
    // The agent's own list is left as it is.
    expect(list.map((p) => p.number)).toEqual([3, 12, 9, 14, 7]);
  });

  it('says what is known of one: its state, its checks, its reviews', () => {
    expect(pullFacts(pr(12))).toEqual(['ouverte']);
    expect(pullFacts(pr(12, { state: 'draft', checks: 'pending', review: 'required' }))).toEqual([
      'brouillon',
      'CI en cours',
      'relecture attendue',
    ]);
    expect(pullFacts(pr(12, { checks: 'failing', review: 'changes' }))).toEqual(['ouverte', 'CI rouge', 'changements demandés']);
    expect(pullFacts(pr(12, { checks: 'passing', review: 'approved' }))).toEqual(['ouverte', 'CI verte', 'approuvée']);
    expect(pullFacts(pr(12, { state: 'merged' }))).toEqual(['mergée']);
    expect(pullFacts(pr(12, { state: 'closed' }))).toEqual(['fermée']);
    expect(pullFacts(pr(12, { state: '' }))).toEqual([]);
  });

  it('names one with its title, and says why nothing is known of it', () => {
    expect(pullTip(pr(12, { title: 'Fix the login', checks: 'passing' }))).toBe('#12 Fix the login — ouverte · CI verte');
    expect(pullTip(pr(12, { state: '' }))).toBe(
      '#12 — état pas encore lu sur GitHub (il faut gh, la CLI de GitHub, connecté : gh auth login)',
    );
  });

  it('says it in English', () => {
    setLang('en');
    expect(pullFacts(pr(12, { state: 'draft', checks: 'pending', review: 'required' }))).toEqual([
      'draft',
      'checks running',
      'review required',
    ]);
    expect(pullFacts(pr(12, { checks: 'failing', review: 'changes' }))).toEqual(['open', 'checks failing', 'changes requested']);
    expect(pullFacts(pr(12, { state: 'merged' }))).toEqual(['merged']);
    expect(pullTip(pr(12, { state: 'closed', title: 'Fix' }))).toBe('#12 Fix — closed');
    expect(pullTip(pr(12, { state: '' }))).toBe(
      '#12 — state not read on GitHub yet (it takes gh, the GitHub CLI, signed in: gh auth login)',
    );
  });
});
