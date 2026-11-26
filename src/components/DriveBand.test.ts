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
    expect(last?.textContent).toBe('3Met en évidence le critère 3');
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

  it('speaks English', () => {
    setLang('en');
    render(DriveBand, { agent: 'pagination', trust: true, hand: 'agent' });
    expect(band()?.textContent).toContain('Written by pagination · offline, no access to Escouade or your files');
    expect(screen.getByText('pagination is driving')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Take over' })).toBeTruthy();
  });
});
