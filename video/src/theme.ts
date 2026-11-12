import { loadFont as loadUi } from '@remotion/google-fonts/Inter';
import { loadFont as loadMono } from '@remotion/google-fonts/JetBrainsMono';

// The app's fonts; loadFont holds each frame until they are loaded.
export const UI = loadUi('normal', { weights: ['400', '500', '600', '700', '800'], subsets: ['latin', 'latin-ext'] }).fontFamily;
export const MONO = loadMono('normal', { weights: ['400', '500', '600'], subsets: ['latin', 'latin-ext'] }).fontFamily;

/** The colour of the project in front, which tints the frame and the panels as in the app (`--p`, set by Shell and Stage). */
export const P = 'var(--p)';
/** The project colour the scenes start on: demo-api's. */
export const P0 = 'oklch(0.72 0.12 48)';

/**
 * The app's colours (src/app.css, the 2.x design, dark theme): an almost black frame tinted by the project, a raised
 * panel for the work, hairlines in white at low alpha. The old names are kept: `bg` is the panel the work sits on,
 * `panel` the frame around it, `spark` the accent (the project's colour).
 */
export const C = {
  bg: `color-mix(in oklch, ${P} 4.5%, #0e0f11)`,
  panel: `color-mix(in oklch, ${P} 8%, #08090a)`,
  elev: `color-mix(in oklch, ${P} 6%, #15161a)`,
  elev2: `color-mix(in oklch, ${P} 6%, #1b1c20)`,
  user: `color-mix(in oklch, ${P} 6%, #1b1c20)`,
  term: `color-mix(in oklch, ${P} 2%, #0a0b0d)`,
  bar: `color-mix(in oklch, ${P} 8%, #08090a)`,
  line: 'rgba(255, 255, 255, 0.075)',
  line2: 'rgba(255, 255, 255, 0.12)',
  line3: 'rgba(255, 255, 255, 0.2)',
  hover: 'rgba(255, 255, 255, 0.045)',
  text: '#f7f8f8',
  soft: '#d0d3d8',
  muted: '#989da6',
  dim: '#7a7f88',
  /** The accent: the project's colour, and its ink for text on the dark. */
  spark: P,
  pInk: `color-mix(in oklch, ${P} 70%, white)`,
  pSoft: `color-mix(in oklch, ${P} 14%, transparent)`,
  /** Claude's own orange: its spark in the conversation. */
  claude: '#D97757',
  /** Escouade's own colour: its logo and name. */
  brand: '#35C1BD',
  ok: 'oklch(0.79 0.15 155)',
  wait: 'oklch(0.84 0.14 82)',
  del: 'oklch(0.74 0.14 22)',
  info: 'oklch(0.8 0.1 235)',
  /** On a filled tile, on the wait colour, on the primary button. */
  onTile: '#101114',
  onWait: '#1a1405',
  primary: '#f2f3f4',
  onPrimary: '#08090a',
  /** The temperature of a prompt cache: hot, half way, cooling. */
  heatHot: 'oklch(0.72 0.17 38)',
  heatMid: 'oklch(0.82 0.13 75)',
  heatCool: 'oklch(0.78 0.11 235)',
} as const;

/** A project colour, from its hue. */
export const hue = (h: number) => `oklch(0.72 0.12 ${h})`;
/** `color` at `pct` % opacity. */
export const soft = (color: string, pct: number) => `color-mix(in oklch, ${color} ${pct}%, transparent)`;
