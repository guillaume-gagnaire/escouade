import { describe, expect, it } from 'vitest';
import MUSIC from './music.json';
import { compositionPlan, planKey } from './musicPlan';
import { FPS, TIMELINE, TOTAL_FRAMES } from './timeline';

const totalMs = Math.round((TOTAL_FRAMES / FPS) * 1000);

describe('compositionPlan', () => {
  const plan = compositionPlan(TIMELINE);

  it('lasts exactly as long as the video, in sections ElevenLabs accepts', () => {
    expect(plan.sections.reduce((n, s) => n + s.duration_ms, 0)).toBe(totalMs);
    for (const s of plan.sections) {
      expect(s.duration_ms, s.section_name).toBeGreaterThanOrEqual(3000);
      expect(s.duration_ms, s.section_name).toBeLessThanOrEqual(120000);
      expect(s.lines).toEqual([]);
    }
  });

  it('follows the script: the beat from the start, a peak on the board, a big ending on the logo', () => {
    // A part cut in two keeps its name: « Tableau 1/2 », « Tableau 2/2 ».
    expect([...new Set(plan.sections.map((s) => s.section_name.replace(/ \d+\/\d+$/, '')))]).toEqual([
      'Intro',
      'Drive',
      'Drive 2',
      'Tableau',
      'Drive 3',
      'Final',
    ]);
    expect(plan.negative_global_styles).toEqual(expect.arrayContaining(['vocals', 'slow tempo']));
  });

  it('cuts a part longer than ElevenLabs accepts into equal sections of the same style', () => {
    const long = TIMELINE.map((s) => (s.id === 'loop' ? { ...s, durationInFrames: s.durationInFrames + 150 * FPS } : s));
    const sections = compositionPlan(long).sections;
    const board = sections.filter((s) => s.section_name.startsWith('Tableau'));
    expect(board.length).toBeGreaterThan(1);
    expect(new Set(board.map((s) => JSON.stringify(s.positive_local_styles))).size).toBe(1);
    for (const s of sections) expect(s.duration_ms, s.section_name).toBeLessThanOrEqual(120000);
    expect(sections.reduce((n, s) => n + s.duration_ms, 0)).toBe(
      Math.round((long.reduce((n, s) => n + s.durationInFrames, 0) / FPS) * 1000),
    );
  });

  it('changes key when the timeline changes', () => {
    const shorter = TIMELINE.map((s, i) => (i === 3 ? { ...s, durationInFrames: s.durationInFrames - 30 } : s));
    expect(planKey(compositionPlan(shorter))).not.toBe(planKey(plan));
  });
});

describe('the music', () => {
  it('was composed for the current timeline, and lasts as long', () => {
    expect(MUSIC.key, 'lance npm run music').toBe(planKey(compositionPlan(TIMELINE)));
    expect(MUSIC.durationMs).toBeGreaterThan(totalMs - 1000);
  });
});
