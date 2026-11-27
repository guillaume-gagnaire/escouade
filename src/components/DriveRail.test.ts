// The rail of an agent's page (ESC-680, AB-8; docs/AGENT-PAGES.md § 12, direction A « Les repères »): a numbered pin
// per gesture where the agent had put the page, never two over each other, the last one filled, the part of the page
// on screen on its track; a pin clicked takes the page back there. In French and in English.
import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { drives } from '../lib/drives.svelte';
import { setLang } from '../lib/i18n';
import type { DrivePin } from '../lib/types';
import { fakeBackend } from '../test/ipc';
import DriveRail, { PIN, pinTops, viewOnTrack } from './DriveRail.svelte';

const pins: DrivePin[] = [
  { n: 1, what: 'Défile jusqu’à « Critères »', y: 0, height: 2400, view: 600 },
  { n: 2, what: 'Met en évidence « Critère 3 »', y: 900, height: 2400, view: 600 },
  { n: 3, what: 'Défile jusqu’à « Ton verdict »', y: 1800, height: 2400, view: 600 },
];

beforeEach(() => {
  setLang('fr');
  drives.rows = {};
});

describe('where the pins sit', () => {
  it('at the middle of the view the agent left, as a share of the page', () => {
    const tops = pinTops(pins, 600);
    // (0 + 300) / 2400 × 600 − 9 ≈ 66; (900 + 300) / 2400 × 600 − 9 = 291; (1800 + 300) / 2400 × 600 − 9 ≈ 516.
    expect([tops.get(1), tops.get(2), tops.get(3)]).toEqual([66, 291, 516]);
  });

  it('never two over each other, the later below; never past the track’s end', () => {
    const same = [1, 2, 3].map((n) => ({ n, what: '', y: 2400, height: 2400, view: 0 }));
    const tops = pinTops(same, 100);
    const all = [tops.get(1)!, tops.get(2)!, tops.get(3)!];
    expect(all.every((t) => t <= 100 - PIN)).toBe(true);
    const spread = pinTops(
      [1, 2, 3].map((n) => ({ n, what: '', y: 0, height: 2400, view: 0 })),
      600,
    );
    expect([spread.get(1), spread.get(2), spread.get(3)]).toEqual([0, PIN + 3, 2 * (PIN + 3)]);
    // A page that said nothing of its height: at the top.
    expect(pinTops([{ n: 1, y: 0, height: 0, view: 0 }], 300).get(1)).toBe(0);
  });

  it('the part of the page on screen', () => {
    expect(viewOnTrack({ y: 600, height: 2400, view: 600 }, 400)).toEqual({ top: 100, height: 100 });
    // Never smaller than a pin, never past the end.
    expect(viewOnTrack({ y: 9000, height: 10000, view: 10 }, 400)).toEqual({ top: 360, height: PIN });
    expect(viewOnTrack(null, 400)).toBeNull();
  });
});

describe('the rail', () => {
  it('has a numbered pin per gesture, the last filled, and takes the page back where a pin says', async () => {
    const b = fakeBackend({ browser_drive_place: () => ({ y: 900, height: 2400, view: 600 }), browser_drive_revisit: () => null });
    drives.rows['page:p-12'] = {
      browser: 'page:p-12',
      agent: 'a7',
      hand: 'agent',
      stopped: false,
      step: 3,
      what: '',
      waiting: false,
      passwordAsks: [],
      pins,
    };
    render(DriveRail, { pageKey: 'page:p-12', agent: 'pagination', pins });
    expect(screen.getByRole('navigation', { name: 'Repères de pagination' })).toBeTruthy();
    const buttons = [...document.querySelectorAll<HTMLButtonElement>('[data-pin]')];
    expect(buttons.map((p) => p.textContent)).toEqual(['1', '2', '3']);
    expect(buttons.map((p) => p.classList.contains('cur'))).toEqual([false, false, true]);
    expect(buttons[1].getAttribute('aria-label')).toBe('2 · Met en évidence « Critère 3 » — y revenir');
    await fireEvent.click(buttons[1]);
    expect(b.called('browser_drive_revisit').map((c) => [c.args.key, c.args.n])).toEqual([['page:p-12', 2]]);
    // Where the page is, read from the core while it shows.
    expect(b.called('browser_drive_place').length).toBeGreaterThan(0);
  });

  it('reads nothing while the page does not show', () => {
    const b = fakeBackend({ browser_drive_place: () => null });
    render(DriveRail, { pageKey: 'page:p-12', agent: 'pagination', pins, live: false });
    expect(b.called('browser_drive_place')).toEqual([]);
  });

  it('speaks English', () => {
    setLang('en');
    fakeBackend();
    render(DriveRail, { pageKey: 'page:p-12', agent: 'pagination', pins, live: false });
    expect(screen.getByRole('navigation', { name: 'pagination’s marks' })).toBeTruthy();
    expect(document.querySelector('[data-pin="1"]')?.getAttribute('aria-label')).toBe('1 · Défile jusqu’à « Critères » — go back there');
  });
});
