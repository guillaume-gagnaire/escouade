import { createContext, useContext, type FC, type ReactNode } from 'react';
import { AbsoluteFill, Html5Audio, interpolate, Sequence, staticFile, useCurrentFrame, useVideoConfig } from 'remotion';
import { pop } from '../anim';
import { useScene } from '../cues';
import { C, UI } from '../theme';

/** The backdrop of every scene: dark, with a warm glow. */
export const Stage: FC<{ children?: ReactNode }> = ({ children }) => (
  <AbsoluteFill
    style={{
      background: `radial-gradient(1300px 760px at 50% 62%, color-mix(in oklch, ${C.spark} 13%, ${C.bg}), ${C.bg} 72%)`,
      fontFamily: UI,
      color: C.text,
      overflow: 'hidden',
    }}
  >
    {children}
  </AbsoluteFill>
);

/** Off for the website's images: they show the interface alone. */
export const CaptionsContext = createContext(true);

/** Frames between two words of a caption, and its words' spring: quick, with a little bounce. */
const STAGGER = 2;
const SNAP = 11;

/** Text, word by word, each springing up from a blur; an accent sweeps under it once it is all there. `out` (0 to 1) fades it away. */
export const Caption: FC<{ text: string; delay?: number; top?: number; size?: number; out?: number }> = ({
  text,
  delay = 3,
  top = 62,
  size = 50,
  out = 0,
}) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  if (!useContext(CaptionsContext)) return null;
  const words = text.split(' ');
  const underline = interpolate(frame, [delay + words.length * STAGGER + 4, delay + words.length * STAGGER + 14], [0, 1], {
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  });
  return (
    <div
      style={{
        position: 'absolute',
        top,
        left: 0,
        right: 0,
        textAlign: 'center',
        fontSize: size,
        fontWeight: 700,
        letterSpacing: -0.5,
        opacity: 1 - out,
        zIndex: 5,
      }}
    >
      {words.map((w, i) => {
        const p = pop(frame, fps, delay + i * STAGGER, SNAP);
        return (
          <span
            key={i}
            style={{
              display: 'inline-block',
              margin: '0 0.13em',
              opacity: Math.min(1, p * 1.5),
              transform: `translateY(${(1 - p) * 36}px) scale(${0.82 + 0.18 * p})`,
              filter: p < 0.98 ? `blur(${(1 - Math.min(1, p)) * 6}px)` : undefined,
            }}
          >
            {w}
          </span>
        );
      })}
      <div
        style={{
          margin: '10px auto 0',
          width: 120 * underline,
          height: 4,
          borderRadius: 2,
          background: C.spark,
          boxShadow: `0 0 18px ${C.spark}`,
          opacity: underline,
        }}
      />
    </div>
  );
};

/** The scene's title, from the script. */
export const Title: FC<{ out?: number }> = ({ out }) => <Caption text={useScene().title} out={out} />;

/** Window size and place on the stage. */
export const WIN = { w: 1500, h: 844, left: 210, top: 190 } as const;

/** Where the camera looks: a point of the window (its centre by default), and how close. */
export interface Cam {
  x: number;
  y: number;
  zoom: number;
}
export const WIDE: Cam = { x: WIN.w / 2, y: WIN.h / 2, zoom: 1 };

/** From `a` to `b` over [from, from + dur], eased. */
export function camMove(frame: number, from: number, dur: number, a: Cam, b: Cam): Cam {
  const t = interpolate(frame, [from, from + dur], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' });
  const e = t < 0.5 ? 4 * t ** 3 : 1 - (-2 * t + 2) ** 3 / 2;
  return { x: a.x + (b.x - a.x) * e, y: a.y + (b.y - a.y) * e, zoom: a.zoom + (b.zoom - a.zoom) * e };
}

/** Chains camera moves: each [start frame, duration, target] from where the previous one left it. */
export function camPath(frame: number, moves: [number, number, Cam][]): Cam {
  let cam = WIDE;
  for (const [from, dur, to] of moves) {
    if (frame < from) break;
    cam = camMove(frame, from, dur, cam, to);
  }
  return cam;
}

/** The app's window on the stage (1500×844, under the caption); `cam` moves in on a point of it. */
export const AppWindow: FC<{ enter?: number; x?: number; scale?: number; cam?: Cam; children: ReactNode }> = ({
  enter = 1,
  x = 0,
  scale = 1,
  cam = WIDE,
  children,
}) => (
  <div
    style={{
      position: 'absolute',
      left: WIN.left + x,
      top: WIN.top,
      width: WIN.w,
      height: WIN.h,
      opacity: Math.min(1, enter),
      transform: `scale(${scale * (0.9 + 0.1 * Math.min(1, enter))})`,
      transformOrigin: '50% 50%',
      borderRadius: 14,
      overflow: cam.zoom > 1 ? 'hidden' : 'visible',
      boxShadow: '0 40px 120px rgba(0, 0, 0, 0.6)',
    }}
  >
    <div
      style={{
        position: 'absolute',
        inset: 0,
        transformOrigin: '0 0',
        transform: `translate(${WIN.w / 2 - cam.zoom * cam.x}px, ${WIN.h / 2 - cam.zoom * cam.y}px) scale(${cam.zoom})`,
      }}
    >
      {children}
    </div>
  </div>
);

/** Dims the window but for a rectangle of it, ringed; `on` (0 to 1) fades it. */
export const Spotlight: FC<{ x: number; y: number; w: number; h: number; on: number; radius?: number }> = ({
  x,
  y,
  w,
  h,
  on,
  radius = 10,
}) =>
  on <= 0 ? null : (
    <div
      style={{
        position: 'absolute',
        left: x,
        top: y,
        width: w,
        height: h,
        borderRadius: radius,
        boxShadow: `0 0 0 3000px rgba(10, 9, 8, ${0.55 * on}), 0 0 0 2px rgba(217, 119, 87, ${on}), 0 0 30px rgba(217, 119, 87, ${0.4 * on})`,
        pointerEvents: 'none',
        zIndex: 20,
      }}
    />
  );

/** A sound effect at frame `at` of the scene. */
export const Sfx: FC<{ at: number; name: 'click' | 'chime'; volume?: number }> = ({ at, name, volume = name === 'click' ? 0.35 : 0.5 }) =>
  useContext(CaptionsContext) ? (
    <Sequence from={Math.round(at)} durationInFrames={name === 'click' ? 6 : 26} layout="none">
      <Html5Audio src={staticFile(`sfx/${name}.wav`)} volume={volume} />
    </Sequence>
  ) : null;
