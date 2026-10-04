import { describe, expect, it } from 'vitest';
import { FEATURES } from './features';
import { SCRIPT } from './script';

const norm = (s: string) => s.toLowerCase().replace(/[’']/g, "'");

describe('features', () => {
  it('have unique ids', () => {
    const ids = FEATURES.map((f) => f.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('are each shown by a scene that says it', () => {
    for (const f of FEATURES) {
      const scenes = SCRIPT.filter((s) => s.shows.includes(f.id));
      expect(scenes.length, `${f.id} n'est montrée par aucun plan`).toBeGreaterThan(0);
      const told = scenes.some((s) => s.lines.some((l) => norm(l.text).includes(norm(f.said))));
      expect(told, `« ${f.said} » (${f.id}) n'est dit par aucun des plans qui la montrent`).toBe(true);
    }
  });

  it('are the only things scenes claim to show', () => {
    const ids = new Set(FEATURES.map((f) => f.id));
    for (const s of SCRIPT) for (const id of s.shows) expect(ids.has(id), `${s.id} montre ${id}, inconnue`).toBe(true);
  });
});
