import { describe, expect, it } from 'vitest';
import { cuesOf } from './cues';
import { imagePath, POSTER_SCENE, posterFrame, SITE_SHOTS, subtitlesFile } from './siteShots';
import { sceneOf } from './timeline';

describe('the website’s images', () => {
  it('are named once, and each one is a moment of a scene’s voice', () => {
    const names = SITE_SHOTS.map((s) => s.name);
    expect(new Set(names).size).toBe(names.length);
    for (const s of SITE_SHOTS) {
      const scene = sceneOf(s.scene);
      const frame = s.frame(cuesOf(scene));
      expect(Number.isInteger(frame), s.name).toBe(true);
      expect(frame, s.name).toBeGreaterThanOrEqual(0);
      expect(frame, s.name).toBeLessThan(scene.durationInFrames);
    }
  });

  it('are the website’s eleven images, whatever the language: the moments follow the French voice', () => {
    expect(SITE_SHOTS.map((s) => s.name)).toEqual([
      'agents',
      'chat',
      'notifications',
      'git',
      'editor',
      'board',
      'test',
      'integrations',
      'launch',
      'stats',
      'remote',
    ]);
  });

  it('go to images/ in French and images/en/ in English, with the subtitles beside the video', () => {
    expect(imagePath('fr', 'agents')).toBe('images/agents.jpg');
    expect(imagePath('en', 'agents')).toBe('images/en/agents.jpg');
    expect(imagePath('en', 'poster')).toBe('images/en/poster.jpg');
    expect(subtitlesFile('fr')).toBe('escouade.vtt');
    expect(subtitlesFile('en')).toBe('escouade.en.vtt');
  });

  it('have a poster: the last moments of the Kanban scene, with its title', () => {
    const scene = sceneOf(POSTER_SCENE);
    expect(posterFrame()).toBe(scene.from + scene.durationInFrames - 8);
  });
});
