import { describe, expect, it } from 'vitest';
import { agent } from '../test/ipc';
import { fInt } from './format';
import { contextUse, fSpentUsd, spent } from './spend';

describe('spent', () => {
  it('adds the running turn to what the finished turns used, as an estimate', () => {
    const s = spent(agent({ tokens: 1000, cost: 0.5, liveTokens: 250, liveCost: 0.125 }));
    expect(s).toEqual({ tokens: 1250, cost: 0.625, estimated: true });
    expect(fSpentUsd(s)).toBe('≈ 0,63 $');
  });

  it('is exact between turns', () => {
    const s = spent(agent({ tokens: 1000, cost: 0.05 }));
    expect(s).toEqual({ tokens: 1000, cost: 0.05, estimated: false });
    expect(fSpentUsd(s)).toBe('0,050 $');
  });
});

describe('contextUse', () => {
  it('tells how full the context is, out of the model’s window, and when it is nearly full', () => {
    expect(contextUse(agent({ contextTokens: 45_200, contextWindow: 200_000 }))).toEqual({
      pct: 23,
      title: `Contexte : 23 % de la fenêtre du modèle (${fInt(45_200)} tokens sur ${fInt(200_000)})`,
      full: false,
    });
    expect(contextUse(agent({ contextTokens: 160_000, contextWindow: 200_000 }))?.full).toBe(true);
  });

  it('knows nothing until a turn told the window', () => {
    expect(contextUse(agent({ contextTokens: 12_300, contextWindow: 0 }))).toBeNull();
  });
});
