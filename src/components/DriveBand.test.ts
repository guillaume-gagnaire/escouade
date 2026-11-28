// The driving band common to every row of « Navigateurs » (AP-3; docs/AGENT-PAGES.md § 10): the trust mention of an
// agent's page, and what AB-8 will wire for the pages and the tabs alike — who drives, the last gesture with its number,
// « Reprendre la main » or « La rendre », « Continuer » — in French and in English.
import { fireEvent, render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { setLang } from '../lib/i18n';
import DriveBand from './DriveBand.svelte';

beforeEach(() => setLang('fr'));

const band = () => document.querySelector<HTMLElement>('[data-testid="drive-band"]');

describe('the driving band', () => {
  it('says on an agent’s page who wrote it and what it cannot reach, nothing driving it yet', () => {
    render(DriveBand, { agent: 'pagination', trust: true });
    expect(screen.getByRole('group', { name: 'Pilotage et confiance' })).toBeTruthy();
    expect(band()?.textContent).toContain('Écrite par pagination · hors réseau, sans accès à Escouade ni à tes fichiers');
    expect(screen.queryByRole('button')).toBeNull();
  });

  it('is not there on a tab nobody drives', () => {
    render(DriveBand, { agent: 'pagination' });
    expect(band()).toBeNull();
  });

  it('while the agent drives: its name, its last gesture and its number, and « Reprendre la main »', async () => {
    const ontake = vi.fn();
    render(DriveBand, { agent: 'pagination', trust: true, hand: 'agent', step: 3, last: 'Met en évidence le critère 3', ontake });
    expect(screen.getByText('pagination pilote')).toBeTruthy();
    const last = document.querySelector('[data-testid="drive-last"]');
    // Its number to the eye, « Geste 3 : » to a screen reader.
    expect(last?.textContent).toBe('3Geste 3 : Met en évidence le critère 3');
    await fireEvent.click(screen.getByRole('button', { name: 'Reprendre la main' }));
    expect(ontake).toHaveBeenCalledOnce();
    // The trust mention stays beside it.
    expect(band()?.textContent).toContain('Écrite par pagination');
  });

  it('once the user took the hand: « Tu as la main · La rendre », no caption; « Continuer » while the agent waits', async () => {
    const ongive = vi.fn();
    const oncontinue = vi.fn();
    render(DriveBand, { agent: 'pagination', hand: 'user', last: 'Fait défiler', waiting: true, ongive, oncontinue });
    expect(screen.getByText('Tu as la main')).toBeTruthy();
    expect(document.querySelector('[data-testid="drive-last"]')).toBeNull();
    await fireEvent.click(screen.getByRole('button', { name: 'La rendre' }));
    await fireEvent.click(screen.getByRole('button', { name: 'Continuer' }));
    expect(ongive).toHaveBeenCalledOnce();
    expect(oncontinue).toHaveBeenCalledOnce();
  });

  it('once the user stopped it: says so until the row closes, no button gives it back, no « Continuer »', () => {
    render(DriveBand, { agent: 'pagination', hand: 'user', stopped: true, waiting: true, last: 'Fait défiler' });
    expect(screen.getByText('Tu ne laisses plus piloter')).toBeTruthy();
    expect(screen.getByText('Jusqu’à la fermeture de ce navigateur')).toBeTruthy();
    expect(screen.queryByRole('button')).toBeNull();
    expect(document.querySelector('[data-testid="drive-hand"]')?.getAttribute('data-hand')).toBe('stopped');
  });

  it('is always there on a driven tab, even without a gesture caption: the user always sees that an agent drives', () => {
    render(DriveBand, { agent: 'esc-412', hand: 'agent', step: null, last: '' });
    expect(band()).toBeTruthy();
    expect(screen.getByText('esc-412 pilote')).toBeTruthy();
    expect(document.querySelector('[data-testid="drive-last"]')).toBeNull();
    expect(document.querySelector('[data-testid="drive-hand"]')?.getAttribute('data-hand')).toBe('agent');
  });

  it('speaks English', () => {
    setLang('en');
    render(DriveBand, { agent: 'pagination', trust: true, hand: 'agent' });
    expect(band()?.textContent).toContain('Written by pagination · offline, no access to Escouade or your files');
    expect(screen.getByText('pagination is driving')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Take over' })).toBeTruthy();
  });

  it('speaks English once the user has the hand, and once they stopped it', () => {
    setLang('en');
    const { unmount } = render(DriveBand, { agent: 'pagination', hand: 'user', waiting: true });
    expect(screen.getByText('You have control')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Hand it back' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Continue' })).toBeTruthy();
    unmount();
    render(DriveBand, { agent: 'pagination', hand: 'user', stopped: true });
    expect(screen.getByText('You no longer let it drive')).toBeTruthy();
  });
});
