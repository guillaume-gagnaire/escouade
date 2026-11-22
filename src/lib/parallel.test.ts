import { beforeEach, describe, expect, it } from 'vitest';
import { setLang } from './i18n';
import { ceilingLine, parallelMax, parallelWarning, withinBound } from './parallel';
import type { ParallelCapacity } from './types';

// « En parallèle » (ESC-620): the bound the core gives, the ceiling and who imposes it, the warning beyond 6.
const cap = (over: Partial<ParallelCapacity> = {}): ParallelCapacity => ({
  server: false,
  limit: 16,
  max: 16,
  ceiling: null,
  warnAbove: 6,
  machine: { cores: 16, memoryMb: 32 * 1024 },
  fits: 4,
  agentMb: 6 * 1024,
  ...over,
});

describe('the bound of the tickets in parallel', () => {
  beforeEach(() => setLang('fr'));

  it('goes to 16 before the core answered, and to what it says after', () => {
    expect(parallelMax(null)).toBe(16);
    expect(parallelMax(cap({ server: true, limit: 64, max: 64 }))).toBe(64);
    expect(withinBound(20, null)).toBe(16);
    expect(withinBound(20, cap({ server: true, limit: 64, max: 64 }))).toBe(20);
    expect(withinBound(0, null)).toBe(1);
    expect(withinBound(Number.NaN, null)).toBe(1);
    expect(withinBound(9, cap({ max: 8 }))).toBe(8);
  });

  it('says nothing up to 6, then what the machine holds and the quota, never a refusal', () => {
    expect(parallelWarning(6, cap())).toBeNull();
    expect(parallelWarning(12, null)).toBeNull();
    expect(parallelWarning(7, cap())).toBe(
      'Au-delà de 6 : ce poste (16 cœurs, 32 Go) tient environ 4 agents qui compilent et lancent des tests (≈ 6 Go chacun, mesuré sur le dépôt d’Escouade). Le quota du compte part aussi plus vite.',
    );
    // A server's space: the server's machine; its memory unknown, its cores alone; one agent in the singular.
    expect(parallelWarning(40, cap({ server: true, machine: { cores: 4, memoryMb: 0 }, fits: 1 }))).toBe(
      'Au-delà de 6 : le serveur (4 cœurs) tient environ 1 agent qui compile et lance des tests (≈ 6 Go chacun, mesuré sur le dépôt d’Escouade). Le quota du compte part aussi plus vite.',
    );
    setLang('en');
    expect(parallelWarning(10, cap({ server: true, machine: { cores: 64, memoryMb: 256 * 1024 }, fits: 16 }))).toBe(
      'Beyond 6: the server (64 cores, 256 GB) holds about 16 agents that compile and run tests (≈ 6 GB each, measured on Escouade’s repository). The account’s quota also goes faster.',
    );
  });

  it('says the ceiling imposed and who imposes it', () => {
    expect(ceilingLine(cap())).toBeNull();
    expect(ceilingLine(null)).toBeNull();
    expect(ceilingLine(cap({ max: 8, ceiling: { value: 8, by: { kind: 'device', source: 'registry' } } }))).toBe(
      'Plafond de 8, imposé par la stratégie de ce poste',
    );
    expect(ceilingLine(cap({ max: 4, ceiling: { value: 4, by: { kind: 'team', team: 'Acme' } } }))).toBe(
      'Plafond de 4, imposé par l’équipe Acme',
    );
  });
});
