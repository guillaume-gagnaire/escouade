import { renderToStaticMarkup } from 'react-dom/server';
import { expect, it, vi } from 'vitest';

// The French video's interface, as text: the pictures of the French video must not change when the English is worked on.
// Run `npx vitest run -u src/scenes/frenchText.test.tsx` after a deliberate change to the French, and read the diff.
let frame = 0;
vi.mock('remotion', async (original) => ({
  ...(await original<typeof import('remotion')>()),
  useCurrentFrame: () => frame,
  useVideoConfig: () => ({ fps: 30, width: 1920, height: 1080, durationInFrames: 99999, id: 'Presentation' }),
  Sequence: ({ children }: { children?: unknown }) => children ?? null,
  Html5Audio: () => null,
}));
vi.mock('@remotion/google-fonts/HankenGrotesk', () => ({ loadFont: () => ({ fontFamily: 'Hanken Grotesk' }) }));
vi.mock('@remotion/google-fonts/JetBrainsMono', () => ({ loadFont: () => ({ fontFamily: 'JetBrains Mono' }) }));

const { SCENES } = await import('./index');
const { SceneContext } = await import('../cues');
const { CaptionsContext } = await import('../ui/Stage');
const { TIMELINE } = await import('../timeline');

const ENTITIES: Record<string, string> = { '&amp;': '&', '&lt;': '<', '&gt;': '>', '&quot;': '"', '&#x27;': "'", '&#39;': "'" };
const texts = (html: string) =>
  html
    .split(/<[^>]*>/)
    .map((t) => t.replace(/&(amp|lt|gt|quot|#x27|#39);/g, (m) => ENTITIES[m]).trim())
    .filter(Boolean);

it('shows the French interface the way it always did, scene by scene (the texts it says, in the order they appear)', async () => {
  const out: string[] = [];
  for (const s of TIMELINE) {
    const Scene = SCENES[s.id];
    // The end of each line, when what it talks about is on screen, and the scene's last frame.
    const frames = [...s.lines.map((l) => l.from - s.from + l.durationInFrames - 2), s.durationInFrames - 1];
    const seen = new Set<string>();
    out.push(`## ${s.id}`);
    for (const f of frames) {
      frame = f;
      const html = renderToStaticMarkup(
        <CaptionsContext.Provider value={true}>
          <SceneContext.Provider value={s.id}>
            <Scene />
          </SceneContext.Provider>
        </CaptionsContext.Provider>,
      );
      for (const t of texts(html)) if (!seen.has(t)) (seen.add(t), out.push(t));
    }
    out.push('');
  }
  await expect(out.join('\n')).toMatchFileSnapshot('./__snapshots__/french-text.txt');
});
