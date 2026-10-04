import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';

// The scenes, drawn outside Remotion: its frame comes from here, its fonts are not loaded.
let frame = 0;
vi.mock('remotion', async (original) => ({
  ...(await original<typeof import('remotion')>()),
  useCurrentFrame: () => frame,
  useVideoConfig: () => ({ fps: 30, width: 1920, height: 1080, durationInFrames: 99999, id: 'Presentation' }),
}));
vi.mock('@remotion/google-fonts/HankenGrotesk', () => ({ loadFont: () => ({ fontFamily: 'Hanken Grotesk' }) }));
vi.mock('@remotion/google-fonts/JetBrainsMono', () => ({ loadFont: () => ({ fontFamily: 'JetBrains Mono' }) }));

const { SCENES } = await import('./index');
const { SceneContext } = await import('../cues');
const { CaptionsContext } = await import('../ui/Stage');
const { TIMELINE } = await import('../timeline');

describe('every scene', () => {
  for (const s of TIMELINE) {
    it(`${s.id} draws from its first frame to its last, on the words its voice says`, () => {
      const Scene = SCENES[s.id];
      // Each line's start, middle and end, and the scene's ends: every cue the scene looks for is computed on each.
      const frames = [0, s.durationInFrames - 1, ...s.lines.flatMap((l) => [l.from - s.from, l.from - s.from + (l.durationInFrames >> 1)])];
      for (const f of frames) {
        frame = f;
        const html = renderToStaticMarkup(
          <CaptionsContext.Provider value={false}>
            <SceneContext.Provider value={s.id}>
              <Scene />
            </SceneContext.Provider>
          </CaptionsContext.Provider>,
        );
        expect(html.length, `${s.id} à l'image ${f}`).toBeGreaterThan(100);
      }
    });
  }
});
