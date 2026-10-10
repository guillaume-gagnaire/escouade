import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp } from '../anim';
import { useCues } from '../cues';
import { EMPTY_STATUS, TABS } from '../data';
import { useFmt } from '../lang';
import { Shell } from '../ui/Shell';
import { AppWindow, Stage, Title } from '../ui/Stage';
import { TermWindow } from '../ui/TermWindow';

/** How many different projects the terminals say (their names are in the component, for the language). */
const PROJECTS = 5;
/** Pseudo-random but fixed positions. */
const rnd = (i: number, k: number) => {
  const x = Math.sin(i * 12.9898 + k * 78.233) * 43758.5453;
  return x - Math.floor(x);
};
const WINDOWS = Array.from({ length: 30 }, (_, i) => ({
  x: 40 + rnd(i, 1) * 1420,
  y: 170 + rnd(i, 2) * 620,
  rot: (rnd(i, 3) - 0.5) * 8,
  project: i % PROJECTS,
}));

/** Terminals pile up while the voice counts them, then the app swallows them. */
export const Chaos: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const { tr } = useFmt();
  const projects = ['demo-api', 'studio-web', 'mobile-app', 'infra', tr('site-vitrine', 'marketing-site')];
  const pile = c.word('mess', 'ouverts') - 10;
  const suck = ramp(frame, c.word('one', 'réunit') - 6, 30);
  const app = pop(frame, fps, c.word('one', 'réunit') + 14, 18);
  return (
    <Stage>
      {WINDOWS.map((w, i) => {
        const p = Math.min(1, pop(frame, fps, (i * pile) / WINDOWS.length, 12));
        return (
          <TermWindow
            key={i}
            project={projects[w.project]}
            style={{
              position: 'absolute',
              left: w.x + (730 - w.x) * suck,
              top: w.y + (400 - w.y) * suck,
              opacity: p * (1 - suck),
              transform: `rotate(${w.rot * (1 - suck)}deg) scale(${(0.7 + 0.3 * p) * (1 - 0.8 * suck)})`,
            }}
          />
        );
      })}
      <Title />
      <AppWindow enter={app}>
        <Shell tabs={TABS.map((t) => ({ ...t, enter: 0 }))} status={EMPTY_STATUS} sidebar={null} />
      </AppWindow>
    </Stage>
  );
};
