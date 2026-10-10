import { describe, expect, it } from 'vitest';
import { agent } from '../test/ipc';
import { fInt } from './format';
import { setLang } from './i18n';
import { contextUse, estimateHint, fSpentUsd, spent } from './spend';

describe('spent', () => {
  it('adds the running turn to what the finished turns used, as an estimate', () => {
    const s = spent(agent({ tokens: 1000, cost: 0.5, liveTokens: 250, liveCost: 0.125 }));
    expect(s).toEqual({ tokens: 1250, cost: 0.625, estimated: true });
    expect(fSpentUsd(s)).toBe('≈\u00a00,63\u00a0$');
  });

  it('is exact between turns', () => {
    const s = spent(agent({ tokens: 1000, cost: 0.05 }));
    expect(s).toEqual({ tokens: 1000, cost: 0.05, estimated: false });
    expect(fSpentUsd(s)).toBe('0,050\u00a0$');
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

describe('what is said of the spending', () => {
  it('says why a cost is an estimate, in the language of the interface', () => {
    expect(estimateHint()).toBe('Estimation (tarifs publics) pendant que Claude travaille ; coût exact à la fin du tour');
    setLang('en');
    expect(estimateHint()).toBe('Estimate (public prices) while Claude works; exact cost at the end of the turn');
  });

  it('writes how full the context is in English, with the numbers of the language', () => {
    setLang('en');
    expect(contextUse(agent({ contextTokens: 45_200, contextWindow: 200_000 }))?.title).toBe(
      'Context: 23% of the model’s window (45,200 tokens of 200,000)',
    );
  });

  it('writes an estimated cost in dollars, in English', () => {
    setLang('en');
    expect(fSpentUsd({ cost: 0.625, estimated: true })).toBe('≈ $0.63');
  });
});
