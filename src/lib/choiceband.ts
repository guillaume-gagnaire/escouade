// The band of the choice under an agent's page (AP-4, docs/AGENT-PAGES.md § 6): what it shows of the page's request.
// A click in the page only proposes (`_/choose`, checked by the core); the band, native and out of the page's webview,
// says the proposal **in the request's own words** (never the page's) and sends it on the user's gesture alone, by the
// box's own command. A go-ahead (`gate`) is never proposed from the page: the band answers it with its own two buttons.
import { t } from './i18n';
import type { AskAnswerIn, PageProposal, WindowAsk } from './types';

/** An option as the band names it: its letter and the request's label. */
export interface Chip {
  id: string;
  letter: string;
  label: string;
}

export type BandView =
  /** No request, or one that is over (withdrawn, expired, retired): no band. */
  | { kind: 'none' }
  /** A go-ahead that waits: the band's own « Non » and « Oui ». */
  | { kind: 'gate'; yes: string; no: string }
  /** It waits, nothing proposed: « Choisis dans la page ». */
  | { kind: 'idle' }
  /** The page proposed: the options in the request's words, the note it joined, « Annuler » and « Envoyer ». */
  | { kind: 'proposed'; chips: Chip[]; options: string[]; note: string | null }
  /** Answered, within the time to take it back. */
  | { kind: 'sending'; chips: Chip[] }
  /** Answered and sent (or read). */
  | { kind: 'answered'; chips: Chip[] };

/** A confirm's yes and no, as the request names them. */
export function yesNo(ask: Pick<WindowAsk, 'yesLabel' | 'noLabel'>): { yes: string; no: string } {
  return { yes: ask.yesLabel.trim() || t('devpage.choice.yes'), no: ask.noLabel.trim() || t('devpage.choice.no') };
}

/** The chips of `options`, in the request's words; null when one of them is not the request's. */
export function chipsOf(ask: WindowAsk, options: string[]): Chip[] | null {
  const chips: Chip[] = [];
  for (const id of options) {
    if (ask.kind === 'confirm') {
      if (id !== 'yes' && id !== 'no') return null;
      const { yes, no } = yesNo(ask);
      chips.push({ id, letter: '', label: id === 'yes' ? yes : no });
      continue;
    }
    const o = ask.options.find((x) => x.id === id);
    if (!o) return null;
    chips.push({ id, letter: id.toUpperCase(), label: o.label });
  }
  return chips.length ? chips : null;
}

/** The options an answer chose (a confirm's as `yes` or `no`). */
function answered(ask: WindowAsk): string[] {
  const a = ask.answer;
  if (!a) return [];
  if (a.yes === true) return ['yes'];
  if (a.yes === false) return ['no'];
  return a.options ?? [];
}

/** What the band shows of request `ask` and the page's `proposal`, at `now`. */
export function bandOf(ask: WindowAsk | null | undefined, proposal: PageProposal | null | undefined, now = Date.now()): BandView {
  if (!ask || (ask.kind !== 'choice' && ask.kind !== 'confirm')) return { kind: 'none' };
  if (ask.state === 'open' || ask.state === 'snoozed') {
    if (ask.gate) return { kind: 'gate', ...yesNo(ask) };
    if (proposal && proposal.ask === ask.id) {
      const chips = chipsOf(ask, proposal.options);
      // Two for a choice of one: never shown (the core refuses it too).
      if (chips && (ask.multi || chips.length === 1)) return { kind: 'proposed', chips, options: proposal.options, note: proposal.note };
    }
    return { kind: 'idle' };
  }
  if (ask.state === 'answered' || ask.state === 'read') {
    const chips = chipsOf(ask, answered(ask)) ?? [];
    const a = ask.answer;
    if (ask.state === 'answered' && a?.sendAt && !a.sentAt && a.sendAt > now) return { kind: 'sending', chips };
    return { kind: 'answered', chips };
  }
  return { kind: 'none' };
}

/** The answer « Envoyer » gives for `options`: the box's own (`inbox_answer`). */
export function answerOf(ask: Pick<WindowAsk, 'kind'>, options: string[], note: string | null): AskAnswerIn {
  const base: AskAnswerIn = ask.kind === 'confirm' ? { yes: options[0] === 'yes' } : { options };
  return { ...base, ...(note ? { note } : {}), via: 'click' };
}

/** How long « Envoyer » waits after the proposal changed, so that a page cannot swap it under the user's click. */
export const ARM_MS = 700;
