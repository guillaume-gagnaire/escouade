import { render, screen } from '@testing-library/svelte';
import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from '../../lib/i18n';
import { art, ribbon } from '../../test/workspace';
import LandingZone from './LandingZone.svelte';

describe('the landing zone', () => {
  beforeEach(() => setLang('fr'));

  it('is drawn even empty', () => {
    render(LandingZone, { agentId: 'a1', items: [], onsize: () => {} });
    expect(screen.getByText('Rien n’a encore atterri')).toBeTruthy();
    expect(screen.getByText('Zone d’atterrissage · 0 posé, 0 en approche')).toBeTruthy();
  });

  it('lands what is finished and keeps the rest above', () => {
    const { container } = render(LandingZone, { agentId: 'a1', items: [art(true), ribbon('ci', 'running')], onsize: () => {} });
    expect(screen.getByText('Zone d’atterrissage · 1 posé, 1 en approche')).toBeTruthy();
    expect(container.querySelectorAll('.coming .ribbon.stripes')).toHaveLength(1);
    expect(container.querySelectorAll('.landed article')).toHaveLength(1);
  });

  it('says its counts in the plural past one', () => {
    render(LandingZone, { agentId: 'a1', items: [art(true), { ...art(true), id: 'b' }], onsize: () => {} });
    expect(screen.getByText('Zone d’atterrissage · 2 posés, 0 en approche')).toBeTruthy();
  });

  it('leaves room under its title for a row on approach, drawn above its slot', () => {
    const { container, rerender } = render(LandingZone, { agentId: 'a1', items: [art(true)], onsize: () => {} });
    expect(container.querySelector('h3.above')).toBeNull();
    rerender({ agentId: 'a1', items: [ribbon('ci', 'running')], onsize: () => {} });
    return Promise.resolve().then(() => expect(container.querySelector('h3.above')).toBeTruthy());
  });

  it("names the button of a page by the page's title for a screen reader", () => {
    render(LandingZone, { agentId: 'a1', items: [art(true)], onsize: () => {} });
    expect(screen.getByRole('button', { name: 'Ouvrir Rapport' }).textContent).toBe('Ouvrir ↗');
  });
});
