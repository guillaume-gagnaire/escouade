import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';

// The scenes in each language, drawn outside Remotion (see scenes.test.tsx); its sounds and sequences draw nothing.
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
const { LangContext } = await import('../lang');
const { Shot } = await import('../Shot');
const { CaptionsContext } = await import('../ui/Stage');
const { TIMELINE } = await import('../timeline');

type Lang = 'fr' | 'en';

const draw = (id: (typeof TIMELINE)[number]['id'], lang: Lang, at: number, captions: boolean) => {
  frame = at;
  const Scene = SCENES[id];
  return renderToStaticMarkup(
    <LangContext.Provider value={lang}>
      <CaptionsContext.Provider value={captions}>
        <SceneContext.Provider value={id}>
          <Scene />
        </SceneContext.Provider>
      </CaptionsContext.Provider>
    </LangContext.Provider>,
  );
};

const ENTITIES: Record<string, string> = { '&amp;': '&', '&lt;': '<', '&gt;': '>', '&quot;': '"', '&#x27;': "'", '&#39;': "'" };
/** The text nodes of some markup, as a reader sees them. */
const texts = (html: string) =>
  html
    .split(/<[^>]*>/)
    .map((t) => t.replace(/&(amp|lt|gt|quot|#x27|#39);/g, (m) => ENTITIES[m]).trim())
    .filter(Boolean);

/** Frames to look at: every few frames of a scene, and its last one. */
const framesOf = (s: (typeof TIMELINE)[number]) => {
  const frames = new Set<number>([s.durationInFrames - 1]);
  for (let f = 0; f < s.durationInFrames; f += 9) frames.add(f);
  for (const l of s.lines) {
    const at = l.from - s.from;
    [at, at + 12, at + (l.durationInFrames >> 1), at + l.durationInFrames - 2].forEach((f) => frames.add(f));
  }
  return [...frames].filter((f) => f >= 0 && f < s.durationInFrames);
};

/** Text that is the same in both languages: names, shell and code, units. */
const NEUTRAL = new Set([
  'Agents',
  'Agent',
  'Kanban',
  'Escouade',
  'Claude',
  'Claude Code',
  'Jira',
  'Trello',
  'GitHub Issues',
  'GitHub',
  'Stats',
  'Stop',
  'Commit…',
  'Pull',
  'Push',
  'Fetch',
  'Merge',
  'Squash',
  'Rebase',
  'Session',
  'Max',
  'Auto',
  'Plan',
  'Bypass',
  'Windows',
  'macOS',
  'Read',
  'Edit',
  'Bash',
  'Grep',
  'Write',
  'Story',
  'Bug',
  'Question',
  'Description',
  'Terminal',
  'Front',
  'API',
  'Worker',
  'Git Bash',
  'PowerShell',
  'TypeScript',
  'Opus',
  'Sonnet',
  'Haiku',
  'Fable',
  'Message',
  'Notifications',
  'Merge commit',
  'Mac',
  'Chrome',
  'Firefox',
  'Cache',
  'Effort',
  'Mode',
  'Tokens',
  'Prompts',
  'CPU',
  'Mem',
  'Ctrl',
  'Tab',
  'PDF',
  'TXT',
  'Page',
  'Query',
  'Router',
  'README.md',
  'Webhooks Stripe',
  'Tests…',
  'Merge…',
  'Pull…',
  // The start of a typed text can be the same in both languages: « Limit… » / « Limiter… ».
  'Limit',
]);

/** French words that no English text of the interface says. */
const FRENCH =
  /(^|[^\p{L}\p{N}_])(le|la|les|des|une|un|du|et|est|sont|pour|avec|sur|dans|que|qui|pas|ton|ce|cette|aux|ou|où|mais)(?![\p{L}\p{N}_])/iu;
const ACCENT = /[àâäçéèêëîïôöùûüœ«»]/i;

describe('every scene, in English', () => {
  const french = new Set<string>();
  for (const s of TIMELINE) for (const f of framesOf(s)) for (const t of texts(draw(s.id, 'fr', f, true))) french.add(t);

  for (const s of TIMELINE) {
    it(`${s.id} draws from its first frame to its last, with and without its caption`, () => {
      for (const f of framesOf(s))
        for (const captions of [true, false]) expect(draw(s.id, 'en', f, captions).length, `${s.id} à l'image ${f}`).toBeGreaterThan(100);
    });

    it(`${s.id} shows English only: no French word, accent or untranslated text`, () => {
      const left = new Set<string>();
      for (const f of framesOf(s))
        for (const t of texts(draw(s.id, 'en', f, true))) {
          if (ACCENT.test(t) || FRENCH.test(t)) left.add(t);
          // The same words in both languages that are neither a name nor code: a text nobody translated.
          else if (french.has(t) && /^[A-Z][A-Za-z’' ,.…!?:-]{2,}$/.test(t) && !NEUTRAL.has(t) && !NEUTRAL.has(t.replace(/[,.:!?…]+$/, '')))
            left.add(t);
        }
      expect([...left], `${s.id} : à traduire`).toEqual([]);
    });
  }

  it('titles each scene in English, from the script', () => {
    for (const s of TIMELINE) {
      const en = texts(draw(s.id, 'en', Math.min(s.durationInFrames - 1, 120), true)).join(' ');
      expect(en, s.id).toContain(s.titleEn.split(' ')[0]);
    }
  });
});

describe('the French scenes', () => {
  it('stay French by default, and when asked for', () => {
    const s = TIMELINE.find((x) => x.id === 'agents')!;
    const at = s.lines[1].from - s.from + 20;
    frame = at;
    const Scene = SCENES.agents;
    const bare = renderToStaticMarkup(
      <CaptionsContext.Provider value={false}>
        <SceneContext.Provider value="agents">
          <Scene />
        </SceneContext.Provider>
      </CaptionsContext.Provider>,
    );
    expect(bare).toContain('Nouvel agent');
    expect(draw('agents', 'fr', at, false)).toBe(bare);
    expect(draw('agents', 'en', at, false)).toContain('New agent');
    expect(draw('agents', 'en', at, false)).not.toContain('Nouvel agent');
  });
});

describe('Shot, the website’s image', () => {
  it('draws a scene in the language it is given, French when it is given none', () => {
    const at = TIMELINE.find((x) => x.id === 'agents')!.lines[1].from - TIMELINE.find((x) => x.id === 'agents')!.from + 20;
    frame = at;
    expect(renderToStaticMarkup(<Shot scene="agents" />)).toContain('Nouvel agent');
    expect(renderToStaticMarkup(<Shot scene="agents" lang="fr" />)).toContain('Nouvel agent');
    const en = renderToStaticMarkup(<Shot scene="agents" lang="en" />);
    expect(en).toContain('New agent');
    expect(en).not.toContain('Nouvel agent');
  });
});
