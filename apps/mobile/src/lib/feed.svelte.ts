/**
 * The thread as the screen holds it (MO-5): built from the spaces the chip shows (`threadOf`), each group in the order
 * it had (`steady`: a line that changes stays in its place), each line with its dot until it is seen; the line open,
 * and the one that opens after a line was settled. What the overlays show (a render, a PDF, a command, a file).
 */

import { t } from '$shared/lib/i18n';
import type { AskFile, WindowAsk } from '$shared/lib/types';
import { keepSeen, keptSeen } from './kept';
import { nav, openSheet, unfold } from './nav.svelte';
import { phone, shownSpaces } from './phone-spaces.svelte';
import { GROUPS, inOrder, nextAfter, steady, threadOf, unseen, waitingLines, type Line, type StoppedLine, type Thread } from './thread';

/** The order each group had, by its lines' keys. */
const orders: Record<string, string[]> = {};
const seen = $state<{ marks: Record<string, string> }>({ marks: keptSeen() });

/** A clock for « depuis » and « Aujourd'hui »: a minute is enough. */
export const clock = $state({ now: Date.now() });
if (typeof window !== 'undefined') setInterval(() => (clock.now = Date.now()), 30_000);

/** The thread now, each group in its steady order. */
export function currentThread(): Thread {
  const th = threadOf(shownSpaces(phone.spaces, phone.filter), clock.now, t('inboxViews.planTitle'));
  for (const g of GROUPS) {
    const key = `${phone.filter}:${g}`;
    orders[key] = steady(
      orders[key] ?? [],
      th[g].map((l) => l.key),
    );
    th[g] = inOrder(th[g], orders[key]);
  }
  return th;
}

/** Whether a line keeps its dot. */
export const hasDot = (l: Line) => unseen(l, seen.marks);

/** A line seen in the state it is in: its dot goes. */
export function markSeen(l: Line): void {
  if (seen.marks[l.key] === l.mark) return;
  seen.marks = { ...seen.marks, [l.key]: l.mark };
  keepSeen(seen.marks);
}

/**
 * A line settled (answered, put off, accepted): the next that waits opens in its place (« une demande réglée ouvre la
 * suivante »), from the thread as it was when the gesture was made.
 */
export function openNext(th: Thread, settled: string): void {
  const order = waitingLines(th).map((l) => l.key);
  const next = nextAfter(order, settled, order);
  unfold(next);
}

/** What the overlays show. */
export const overlay = $state<{
  viewer: { space: string; ask: WindowAsk; files: AskFile[]; index: number } | null;
  pdf: { space: string; ask: WindowAsk; file: AskFile } | null;
  command: { line: StoppedLine } | null;
  file: { space: string; ask: WindowAsk; file: AskFile } | null;
}>({ viewer: null, pdf: null, command: null, file: null });

export function showImages(space: string, ask: WindowAsk, files: AskFile[], index: number): void {
  overlay.viewer = { space, ask, files, index };
  openSheet('viewer');
}

export function showPdf(space: string, ask: WindowAsk, file: AskFile): void {
  overlay.pdf = { space, ask, file };
  openSheet('pdf');
}

export function showFile(space: string, ask: WindowAsk, file: AskFile): void {
  overlay.file = { space, ask, file };
  openSheet('file');
}

export function showCommand(line: StoppedLine): void {
  overlay.command = { line };
  openSheet('command');
}

/** Whether the line `key` is the one open. */
export const isOpenLine = (key: string) => nav.open === key;
