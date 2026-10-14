import { t } from './i18n';
import type { PullRequestLink } from './types';

/** Still to merge, or not told yet: what is shown first. */
const pending = (p: PullRequestLink) => p.state !== 'merged' && p.state !== 'closed';

/** An agent's pull requests as shown: those still to merge first, then the others, the latest first among each. */
export function shownPulls(pulls: PullRequestLink[]): PullRequestLink[] {
  return [...pulls].sort((a, b) => Number(pending(b)) - Number(pending(a)) || b.number - a.number);
}

/** What is known of a pull request, in words: its state, then its checks and its reviews while it is to merge. */
export function pullFacts(p: PullRequestLink): string[] {
  const facts: string[] = [];
  if (p.state) facts.push(t(`git.pulls.state.${p.state}`));
  if (p.checks) facts.push(t(`git.pulls.checks.${p.checks}`));
  if (p.review) facts.push(t(`git.pulls.review.${p.review}`));
  return facts;
}

/** What the number of a pull request says when pointed at: its title, and what is known of it (or why nothing is). */
export function pullTip(p: PullRequestLink): string {
  const facts = pullFacts(p);
  const name = p.title ? `#${p.number} ${p.title}` : `#${p.number}`;
  return `${name} — ${facts.length ? facts.join(' · ') : t('git.pulls.unknown')}`;
}
