import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
import { app } from '../lib/state.svelte';
import QuotaMeter from './QuotaMeter.svelte';

/** A time of day on October 10th, in the time zone of the machine (the tooltip writes it there). */
const at = (h: number, m: number) => new Date(2026, 9, 10, h, m).getTime();
const NOW = at(14, 59);
/** 3 h 01 later. */
const SOON = at(18, 0);
/** 4 days and 12 hours later. */
const LATER = NOW + (4 * 86400 + 12 * 3600) * 1000;

/** Any space as a plain one: `Intl` puts a narrow no-break one before « PM ». */
const spaces = (s: string | null) => (s ?? '').replace(/\s/g, ' ');

describe('QuotaMeter in the status bar', () => {
  it('writes the window and its percentage after a bar that is a meter, the reset in its tooltip', () => {
    const { container } = render(QuotaMeter, { minutes: 300, usage: { pct: 42, resetsAt: SOON }, now: NOW });
    expect(spaces(container.textContent)).toMatch(/^\s*5 h\s*42 %/);
    expect(container).not.toHaveTextContent('reset');
    expect(screen.getByText('42 %')).toHaveClass('pct');
    const meter = screen.getByRole('meter', { name: 'Quota sur 5 heures' });
    expect(meter).toHaveAttribute('aria-valuemin', '0');
    expect(meter).toHaveAttribute('aria-valuemax', '100');
    expect(meter).toHaveAttribute('aria-valuenow', '42');
    // The projection too, whatever the bar shows of it (under 1240 pixels, nothing).
    expect(meter).toHaveAttribute('aria-valuetext', '42 % · remise à zéro le 10/10 à 18:00 · projection à la remise à zéro : 106 %');
  });

  it('rounds the value of the meter', () => {
    render(QuotaMeter, { minutes: 10080, usage: { pct: 38.4, resetsAt: LATER }, now: NOW });
    expect(screen.getByRole('meter', { name: 'Quota sur 7 jours' })).toHaveAttribute('aria-valuenow', '38');
    expect(screen.getByText('7 j')).toBeInTheDocument();
    expect(screen.getByText('38 %')).toBeInTheDocument();
  });

  it('shows the percentage and the reset in a tooltip when the pointer is over it, and takes it back when it leaves', async () => {
    render(QuotaMeter, { minutes: 300, usage: { pct: 42, resetsAt: SOON }, now: NOW });
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
    const meter = screen.getByRole('meter');
    await userEvent.hover(meter);
    expect(await screen.findByRole('tooltip')).toHaveTextContent('42 % · remise à zéro le 10/10 à 18:00');
    await userEvent.unhover(meter);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });

  it('shows the tooltip as well over the label and the percentage, a bigger target than the bar', async () => {
    render(QuotaMeter, { minutes: 300, usage: { pct: 42, resetsAt: SOON }, now: NOW });
    await userEvent.hover(screen.getByText('5 h'));
    expect(await screen.findByRole('tooltip')).toHaveTextContent('42 % · remise');
  });

  it('puts the meter in the tab order, shows the tooltip when it has the focus, and Escape takes it back', async () => {
    render(QuotaMeter, { minutes: 300, usage: { pct: 42, resetsAt: SOON }, now: NOW });
    await userEvent.tab();
    expect(screen.getByRole('meter')).toHaveFocus();
    expect(await screen.findByRole('tooltip')).toHaveTextContent('42 % · remise à zéro le 10/10 à 18:00');
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
    // Still on the bar: the focus stays, and the tooltip comes back with the next focus.
    expect(screen.getByRole('meter')).toHaveFocus();
    await userEvent.tab();
    await userEvent.tab({ shift: true });
    expect(await screen.findByRole('tooltip')).toBeInTheDocument();
    await userEvent.tab();
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });

  it('keeps the tooltip while either the pointer or the focus is on it', async () => {
    render(QuotaMeter, { minutes: 300, usage: { pct: 42, resetsAt: SOON }, now: NOW });
    const meter = screen.getByRole('meter');
    await userEvent.tab();
    await userEvent.hover(meter);
    await userEvent.unhover(meter);
    expect(await screen.findByRole('tooltip')).toBeInTheDocument();
  });

  it('turns to the warning color past 80 %, not at 80 %', async () => {
    const { rerender } = render(QuotaMeter, { minutes: 300, usage: { pct: 80, resetsAt: SOON }, now: NOW });
    expect(screen.getByRole('meter')).not.toHaveClass('hot');
    await rerender({ minutes: 300, usage: { pct: 80.5, resetsAt: SOON }, now: NOW });
    expect(screen.getByRole('meter')).toHaveClass('hot');
  });

  it('fills the bar to the percentage, never beyond the bar', async () => {
    const { rerender } = render(QuotaMeter, { minutes: 300, usage: { pct: 42, resetsAt: SOON }, now: NOW });
    expect(screen.getByRole('meter').firstElementChild).toHaveStyle({ width: '42%' });
    await rerender({ minutes: 300, usage: { pct: 130, resetsAt: SOON }, now: NOW });
    expect(screen.getByRole('meter').firstElementChild).toHaveStyle({ width: '100%' });
  });

  it('says the quota is unavailable while the window is unknown, with a dash for its percentage', async () => {
    const { container } = render(QuotaMeter, { minutes: 10080, usage: null, now: NOW });
    expect(container).toHaveTextContent('7 j');
    expect(screen.getByText('—')).toBeInTheDocument();
    const meter = screen.getByRole('meter', { name: 'Quota sur 7 jours' });
    expect(meter).toHaveAttribute('aria-valuenow', '0');
    expect(meter).toHaveAttribute('aria-valuetext', 'Quota indisponible');
    await userEvent.hover(meter);
    expect(await screen.findByRole('tooltip')).toHaveTextContent('Quota indisponible');
  });

  it('gives only the percentage when the reset is unknown', async () => {
    const { container } = render(QuotaMeter, { minutes: 300, usage: { pct: 10, resetsAt: null }, now: NOW });
    expect(container).not.toHaveTextContent('reset');
    expect(screen.getByRole('meter')).toHaveAttribute('aria-valuetext', '10 %');
    await userEvent.hover(screen.getByRole('meter'));
    expect(await screen.findByRole('tooltip')).toHaveTextContent(/^10 %$/);
  });

  it('is not in the tab order when the button around it takes the focus', async () => {
    render(QuotaMeter, { minutes: 300, usage: { pct: 42, resetsAt: SOON }, now: NOW, focusable: false });
    expect(screen.getByRole('meter')).not.toHaveAttribute('tabindex');
    await userEvent.tab();
    expect(screen.getByRole('meter')).not.toHaveFocus();
    // The pointer still gets its tooltip.
    await userEvent.hover(screen.getByRole('meter'));
    expect(await screen.findByRole('tooltip')).toBeInTheDocument();
  });
});

describe('QuotaMeter projection', () => {
  /** A 5 h window with 2 h gone (3 h left). */
  const fiveHour = (pct: number) => ({ minutes: 300, usage: { pct, resetsAt: NOW + 3 * 3600_000 }, now: NOW });

  it('writes none in the status bar while it stays at 80 % or under: the panel has it (choice 21)', () => {
    const { container } = render(QuotaMeter, fiveHour(20));
    expect(container).not.toHaveTextContent('→');
  });

  it('writes every one in the status bar, green ones too, with « Projection des quotas » on « Toujours »', () => {
    app.settings.quotaProjection = 'always';
    try {
      render(QuotaMeter, fiveHour(20));
      expect(screen.getByText('→ 50 %')).toHaveClass('ok');
    } finally {
      app.settings.quotaProjection = 'alert';
    }
  });

  it('says a green one in its tooltip, which the bar leaves out past 80 % only', async () => {
    render(QuotaMeter, fiveHour(20));
    const meter = screen.getByRole('meter');
    expect(meter.getAttribute('aria-valuetext')).toMatch(/ · projection à la remise à zéro : 50 %$/);
    await userEvent.hover(meter);
    expect(await screen.findByRole('tooltip')).toHaveTextContent(/projection à la remise à zéro : 50 %$/);
  });

  it('writes where the window ends at this pace past 80 %, amber, and red past 100 %', async () => {
    const { container, rerender } = render(QuotaMeter, fiveHour(34));
    // Read « projection à la remise à zéro : → 85 % » (the words for screen readers only).
    expect(spaces(container.textContent)).toMatch(/34 %\s*projection à la remise à zéro : → 85 %/);
    expect(container.querySelector('.p .sr')).toHaveTextContent('projection à la remise à zéro :');
    expect(screen.getByText('→ 85 %')).toHaveClass('warn');
    await rerender(fiveHour(40));
    expect(screen.getByText('→ 100 %')).toHaveClass('warn');
    await rerender(fiveHour(48));
    expect(screen.getByText('→ 120 %')).toHaveClass('over');
  });

  it('writes none in the first minutes of the window, nor without a reset', async () => {
    const { container, rerender } = render(QuotaMeter, {
      minutes: 300,
      usage: { pct: 3, resetsAt: NOW + 298 * 60_000 },
      now: NOW,
      detail: true,
    });
    expect(container).not.toHaveTextContent('→');
    await rerender({ minutes: 300, usage: { pct: 40, resetsAt: null }, now: NOW, detail: true });
    expect(container).not.toHaveTextContent('→');
  });

  it('projects the month’s credits to their reset', () => {
    const now = Date.UTC(2026, 9, 11);
    render(QuotaMeter, { minutes: null, usage: { pct: 20, resetsAt: Date.UTC(2026, 10, 1) }, now, detail: true });
    expect(screen.getByText('→ 62 %')).toHaveClass('ok');
  });

  it('writes none when « Projection des quotas » is off, nor says one', () => {
    app.settings.quotaProjection = 'off';
    try {
      const { container } = render(QuotaMeter, { ...fiveHour(48), detail: true });
      expect(container).toHaveTextContent('reset 3h00');
      expect(container).not.toHaveTextContent('→');
      expect(screen.getByRole('meter').getAttribute('aria-valuetext')).not.toMatch(/projection/);
    } finally {
      app.settings.quotaProjection = 'alert';
    }
  });

  it('is in the panel at any level, green up to 80 %, after the percentage', () => {
    const { container } = render(QuotaMeter, { ...fiveHour(20), detail: true });
    expect(spaces(container.textContent)).toMatch(/5 h\s*20 %\s*projection à la remise à zéro : → 50 %\s*reset 3h00/);
    expect(screen.getByText('→ 50 %')).toHaveClass('ok');
  });
});

describe('QuotaMeter in the panel', () => {
  it('writes the percentage over the bar and the reset under it, a meter without a tooltip', async () => {
    const { container } = render(QuotaMeter, { minutes: 300, usage: { pct: 42, resetsAt: SOON }, now: NOW, detail: true });
    expect(container).toHaveTextContent('5 h');
    expect(container).toHaveTextContent('42 %');
    expect(container).toHaveTextContent('reset 3h01');
    const meter = screen.getByRole('meter', { name: 'Quota sur 5 heures' });
    expect(meter).toHaveAttribute('aria-valuenow', '42');
    // The projection too, whatever the bar shows of it (under 1240 pixels, nothing).
    expect(meter).toHaveAttribute('aria-valuetext', '42 % · remise à zéro le 10/10 à 18:00 · projection à la remise à zéro : 106 %');
    expect(meter).not.toHaveAttribute('tabindex');
    await userEvent.hover(meter);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });

  it('follows the clock', async () => {
    const { rerender } = render(QuotaMeter, { minutes: 300, usage: { pct: 42, resetsAt: SOON }, now: NOW, detail: true });
    expect(screen.getByText('reset 3h01')).toBeInTheDocument();
    await rerender({ minutes: 300, usage: { pct: 42, resetsAt: SOON }, now: NOW + 3600_000, detail: true });
    expect(screen.getByText('reset 2h01')).toBeInTheDocument();
  });

  it('writes a dash for the percentage of a window that is not known', () => {
    render(QuotaMeter, { minutes: 10080, usage: null, now: NOW, detail: true });
    expect(screen.getAllByText('—')).toHaveLength(1);
    expect(screen.getByRole('meter')).toHaveAttribute('aria-valuetext', 'Quota indisponible');
  });

  it('turns to the warning color past 80 % here too', () => {
    render(QuotaMeter, { minutes: 10080, usage: { pct: 95, resetsAt: LATER }, now: NOW, detail: true });
    expect(screen.getByRole('meter')).toHaveClass('hot');
  });
});

describe('QuotaMeter in English', () => {
  it('writes the 7-day window as « W », its name, its percentage and its tooltip in English', async () => {
    setLang('en');
    const { container } = render(QuotaMeter, { minutes: 10080, usage: { pct: 42, resetsAt: LATER }, now: NOW });
    expect(container).toHaveTextContent('W');
    expect(container).toHaveTextContent('42%');
    const meter = screen.getByRole('meter', { name: '7-day quota' });
    const end = new Date(LATER);
    const time = spaces(end.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' }));
    const date = end.toLocaleDateString('en-US', { day: '2-digit', month: '2-digit' });
    expect(spaces(meter.getAttribute('aria-valuetext'))).toBe(`42% · resets on ${date} at ${time} · projected at the reset: 118%`);
    await userEvent.hover(meter);
    expect(spaces(screen.getByRole('tooltip').textContent)).toBe(`42% · resets on ${date} at ${time} · projected at the reset: 118%`);
  });

  it('writes the 5-hour window as « 5h », its reset in the panel, and the tooltip’s example the way the design has it', () => {
    setLang('en');
    render(QuotaMeter, { minutes: 300, usage: { pct: 42, resetsAt: SOON }, now: NOW, detail: true });
    expect(screen.getByRole('meter', { name: '5-hour quota' })).toBeInTheDocument();
    expect(screen.getByText('5h')).toBeInTheDocument();
    expect(screen.getByText('reset 3h01')).toBeInTheDocument();
    expect(spaces(screen.getByRole('meter').getAttribute('aria-valuetext'))).toBe(
      '42% · resets on 10/10 at 6:00 PM · projected at the reset: 106%',
    );
  });

  it('follows a change of language without a new render', async () => {
    render(QuotaMeter, { minutes: 10080, usage: { pct: 42, resetsAt: LATER }, now: NOW });
    expect(screen.getByText('7 j')).toBeInTheDocument();
    setLang('en');
    expect(await screen.findByText('W')).toBeInTheDocument();
    expect(screen.getByRole('meter', { name: '7-day quota' })).toBeInTheDocument();
  });
});
