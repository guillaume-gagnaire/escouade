import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
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
  it('writes the window and when it resets, not the percentage, on a bar that is a meter', () => {
    const { container } = render(QuotaMeter, { kind: 'fiveHour', usage: { pct: 42, resetsAt: SOON }, now: NOW });
    expect(container).toHaveTextContent('5h');
    expect(container).toHaveTextContent('reset 3h01');
    expect(container).not.toHaveTextContent('42');
    const meter = screen.getByRole('meter', { name: 'Quota sur 5 heures' });
    expect(meter).toHaveAttribute('aria-valuemin', '0');
    expect(meter).toHaveAttribute('aria-valuemax', '100');
    expect(meter).toHaveAttribute('aria-valuenow', '42');
    expect(meter).toHaveAttribute('aria-valuetext', '42 % · remise à zéro le 10/10 à 18:00');
  });

  it('rounds the value as the tooltip writes it', () => {
    render(QuotaMeter, { kind: 'sevenDay', usage: { pct: 38.4, resetsAt: LATER }, now: NOW });
    expect(screen.getByRole('meter', { name: 'Quota sur 7 jours' })).toHaveAttribute('aria-valuenow', '38');
    expect(screen.getByText('7j')).toBeInTheDocument();
    expect(screen.getByText('reset 4j 12h')).toBeInTheDocument();
  });

  it('shows the percentage and the reset in a tooltip when the pointer is over it, and takes it back when it leaves', async () => {
    render(QuotaMeter, { kind: 'fiveHour', usage: { pct: 42, resetsAt: SOON }, now: NOW });
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
    const meter = screen.getByRole('meter');
    await userEvent.hover(meter);
    expect(screen.getByRole('tooltip')).toHaveTextContent('42 % · remise à zéro le 10/10 à 18:00');
    await userEvent.unhover(meter);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });

  it('shows the tooltip as well over the label and the reset, a bigger target than the bar', async () => {
    render(QuotaMeter, { kind: 'fiveHour', usage: { pct: 42, resetsAt: SOON }, now: NOW });
    await userEvent.hover(screen.getByText('reset 3h01'));
    expect(screen.getByRole('tooltip')).toHaveTextContent('42 %');
  });

  it('puts the meter in the tab order, shows the tooltip when it has the focus, and Escape takes it back', async () => {
    render(QuotaMeter, { kind: 'fiveHour', usage: { pct: 42, resetsAt: SOON }, now: NOW });
    await userEvent.tab();
    expect(screen.getByRole('meter')).toHaveFocus();
    expect(screen.getByRole('tooltip')).toHaveTextContent('42 % · remise à zéro le 10/10 à 18:00');
    await userEvent.keyboard('{Escape}');
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
    // Still on the bar: the focus stays, and the tooltip comes back with the next focus.
    expect(screen.getByRole('meter')).toHaveFocus();
    await userEvent.tab();
    await userEvent.tab({ shift: true });
    expect(screen.getByRole('tooltip')).toBeInTheDocument();
    await userEvent.tab();
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });

  it('keeps the tooltip while either the pointer or the focus is on it', async () => {
    render(QuotaMeter, { kind: 'fiveHour', usage: { pct: 42, resetsAt: SOON }, now: NOW });
    const meter = screen.getByRole('meter');
    await userEvent.tab();
    await userEvent.hover(meter);
    await userEvent.unhover(meter);
    expect(screen.getByRole('tooltip')).toBeInTheDocument();
  });

  it('turns to the warning color past 80 %, not at 80 %', async () => {
    const { rerender } = render(QuotaMeter, { kind: 'fiveHour', usage: { pct: 80, resetsAt: SOON }, now: NOW });
    expect(screen.getByRole('meter')).not.toHaveClass('warn');
    await rerender({ kind: 'fiveHour', usage: { pct: 80.5, resetsAt: SOON }, now: NOW });
    expect(screen.getByRole('meter')).toHaveClass('warn');
  });

  it('fills the bar to the percentage, never beyond the bar', async () => {
    const { rerender } = render(QuotaMeter, { kind: 'fiveHour', usage: { pct: 42, resetsAt: SOON }, now: NOW });
    expect(screen.getByRole('meter').firstElementChild).toHaveStyle({ width: '42%' });
    await rerender({ kind: 'fiveHour', usage: { pct: 130, resetsAt: SOON }, now: NOW });
    expect(screen.getByRole('meter').firstElementChild).toHaveStyle({ width: '100%' });
  });

  it('says the quota is unavailable while the window is unknown, with a dash for its reset', async () => {
    const { container } = render(QuotaMeter, { kind: 'sevenDay', usage: null, now: NOW });
    expect(container).toHaveTextContent('7j');
    expect(screen.getByText('—')).toBeInTheDocument();
    const meter = screen.getByRole('meter', { name: 'Quota sur 7 jours' });
    expect(meter).toHaveAttribute('aria-valuenow', '0');
    expect(meter).toHaveAttribute('aria-valuetext', 'Quota indisponible');
    await userEvent.hover(meter);
    expect(screen.getByRole('tooltip')).toHaveTextContent('Quota indisponible');
  });

  it('gives only the percentage when the reset is unknown', async () => {
    const { container } = render(QuotaMeter, { kind: 'fiveHour', usage: { pct: 10, resetsAt: null }, now: NOW });
    expect(container).not.toHaveTextContent('reset');
    expect(screen.getByRole('meter')).toHaveAttribute('aria-valuetext', '10 %');
    await userEvent.hover(screen.getByRole('meter'));
    expect(screen.getByRole('tooltip')).toHaveTextContent(/^10 %$/);
  });

  it('is not in the tab order when the button around it takes the focus', async () => {
    render(QuotaMeter, { kind: 'fiveHour', usage: { pct: 42, resetsAt: SOON }, now: NOW, focusable: false });
    expect(screen.getByRole('meter')).not.toHaveAttribute('tabindex');
    await userEvent.tab();
    expect(screen.getByRole('meter')).not.toHaveFocus();
    // The pointer still gets its tooltip.
    await userEvent.hover(screen.getByRole('meter'));
    expect(screen.getByRole('tooltip')).toBeInTheDocument();
  });

  it('follows the clock', async () => {
    const { rerender } = render(QuotaMeter, { kind: 'fiveHour', usage: { pct: 42, resetsAt: SOON }, now: NOW });
    expect(screen.getByText('reset 3h01')).toBeInTheDocument();
    await rerender({ kind: 'fiveHour', usage: { pct: 42, resetsAt: SOON }, now: NOW + 3600_000 });
    expect(screen.getByText('reset 2h01')).toBeInTheDocument();
  });
});

describe('QuotaMeter in the panel', () => {
  it('writes the percentage and the reset next to the bar, which is a meter without a tooltip', async () => {
    const { container } = render(QuotaMeter, { kind: 'fiveHour', usage: { pct: 42, resetsAt: SOON }, now: NOW, detail: true });
    expect(container).toHaveTextContent('5h');
    expect(container).toHaveTextContent('42 %');
    expect(container).toHaveTextContent('reset 3h01');
    const meter = screen.getByRole('meter', { name: 'Quota sur 5 heures' });
    expect(meter).toHaveAttribute('aria-valuenow', '42');
    expect(meter).toHaveAttribute('aria-valuetext', '42 % · remise à zéro le 10/10 à 18:00');
    expect(meter).not.toHaveAttribute('tabindex');
    await userEvent.hover(meter);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });

  it('writes a dash for the percentage of a window that is not known', () => {
    render(QuotaMeter, { kind: 'sevenDay', usage: null, now: NOW, detail: true });
    expect(screen.getAllByText('—')).toHaveLength(1);
    expect(screen.getByRole('meter')).toHaveAttribute('aria-valuetext', 'Quota indisponible');
  });

  it('turns to the warning color past 80 % here too', () => {
    render(QuotaMeter, { kind: 'sevenDay', usage: { pct: 95, resetsAt: LATER }, now: NOW, detail: true });
    expect(screen.getByRole('meter')).toHaveClass('warn');
  });
});

describe('QuotaMeter in English', () => {
  it('writes the 7-day window as « W », its name, its reset and its tooltip in English', async () => {
    setLang('en');
    const { container } = render(QuotaMeter, { kind: 'sevenDay', usage: { pct: 42, resetsAt: LATER }, now: NOW });
    expect(container).toHaveTextContent('W');
    expect(container).toHaveTextContent('reset 4d 12h');
    expect(container).not.toHaveTextContent('42');
    const meter = screen.getByRole('meter', { name: '7-day quota' });
    const end = new Date(LATER);
    const time = spaces(end.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' }));
    const date = end.toLocaleDateString('en-US', { day: '2-digit', month: '2-digit' });
    expect(spaces(meter.getAttribute('aria-valuetext'))).toBe(`42% · resets on ${date} at ${time}`);
    await userEvent.hover(meter);
    expect(spaces(screen.getByRole('tooltip').textContent)).toBe(`42% · resets on ${date} at ${time}`);
  });

  it('writes the 5-hour window as « 5h » and the tooltip’s example the way the design has it', () => {
    setLang('en');
    render(QuotaMeter, { kind: 'fiveHour', usage: { pct: 42, resetsAt: SOON }, now: NOW });
    expect(screen.getByRole('meter', { name: '5-hour quota' })).toBeInTheDocument();
    expect(screen.getByText('5h')).toBeInTheDocument();
    expect(spaces(screen.getByRole('meter').getAttribute('aria-valuetext'))).toBe('42% · resets on 10/10 at 6:00 PM');
  });

  it('follows a change of language without a new render', async () => {
    render(QuotaMeter, { kind: 'sevenDay', usage: { pct: 42, resetsAt: LATER }, now: NOW });
    expect(screen.getByText('7j')).toBeInTheDocument();
    setLang('en');
    expect(await screen.findByText('W')).toBeInTheDocument();
    expect(screen.getByRole('meter', { name: '7-day quota' })).toBeInTheDocument();
  });
});
