import { fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { setLang } from '../lib/i18n';
import { heat } from '../test/heat';
import HeatFigure from './HeatFigure.svelte';

const AT = heat().at;
const MIN = 60_000;

afterEach(() => setLang('fr'));

describe('HeatFigure on a card', () => {
  it('writes the degrees in the colour of their warmth, and their meaning for a screen reader', () => {
    render(HeatFigure, { agent: { id: 'a1', engine: 'codex', heat: heat() }, now: AT + 3 * MIN + 40_000 });
    const fig = screen.getByRole('img', { name: 'Cache : 27 °C, expire dans 1 min 20' });
    expect(fig).toHaveTextContent('27 °C');
    expect(fig).toHaveAttribute('data-tone', 'cool');
  });

  it('is hot when the cache just served, and mid way half way', () => {
    const { unmount } = render(HeatFigure, { agent: { id: 'a1', engine: 'claude', heat: heat() }, now: AT });
    expect(screen.getByRole('img')).toHaveTextContent('100 °C');
    expect(screen.getByRole('img')).toHaveAttribute('data-tone', 'hot');
    unmount();
    render(HeatFigure, { agent: { id: 'a1', engine: 'claude', heat: heat() }, now: AT + 2 * MIN });
    expect(screen.getByRole('img')).toHaveAttribute('data-tone', 'mid');
  });

  it('says « — °C » when its harness tells nothing of its cache, and 0 °C when it broke', () => {
    const { unmount } = render(HeatFigure, { agent: { id: 'a1', engine: 'cursor', heat: null }, now: AT });
    expect(screen.getByRole('img', { name: /Cursor CLI n’a encore rien dit de son cache/ })).toHaveTextContent('— °C');
    expect(screen.getByRole('img')).toHaveAttribute('data-tone', 'none');
    unmount();
    render(HeatFigure, { agent: { id: 'a1', engine: 'claude', heat: heat({ broken: 'model' }) }, now: AT });
    expect(screen.getByRole('img', { name: /cassé \(modèle changé\)/ })).toHaveTextContent('0 °C');
  });

  it('speaks English too', () => {
    setLang('en');
    render(HeatFigure, { agent: { id: 'a1', engine: 'codex', heat: heat() }, now: AT });
    expect(screen.getByRole('img', { name: 'Cache: 100 °C, expires in 5 min 00' })).toBeInTheDocument();
  });
});

describe('HeatFigure in the header', () => {
  it('shows the detail of the last turn on hover and on focus', async () => {
    render(HeatFigure, { agent: { id: 'a1', engine: 'codex', heat: heat() }, now: AT + 3 * MIN + 40_000, detail: true });
    const fig = screen.getByRole('group', { name: 'Cache : 27 °C, expire dans 1 min 20' });
    expect(fig).toHaveTextContent('Cache 27 °C');
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
    await fireEvent.mouseEnter(fig);
    const pop = screen.getByRole('tooltip');
    expect(fig).toHaveAttribute('aria-describedby', pop.id);
    expect(pop).toHaveTextContent('Cache : 27 °C');
    expect(pop).toHaveTextContent('expire dans 1 min 20 (le cache de Codex vit 5 min sans activité)');
    // Each row: its label, then its value.
    const rows = [...pop.querySelectorAll('.hp-rows > div')].map((r) => [...r.children].map((c) => c.textContent?.trim()));
    expect(rows).toEqual([
      ['Lus depuis le cache (chauds)', '129,0 k · 91 %'],
      ['Mis en cache (froids)', '9,0 k'],
      ['Nouveaux, plein tarif', '4,0 k'],
      ['Économisé sur ce tour', '≈ 0,36 $'],
    ]);
    expect(pop).toHaveTextContent('Si le prochain message part à froid : ≈ 142,0 k tokens relus, ≈ 0,47 $ au lieu de ≈ 0,050 $.');
    await fireEvent.mouseLeave(fig);
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
    await fireEvent.focus(fig);
    expect(screen.getByRole('tooltip')).toBeInTheDocument();
    await fireEvent.keyDown(fig, { key: 'Escape' });
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });
});
