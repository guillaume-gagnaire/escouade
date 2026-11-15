/**
 * Colours of the « look » specs, compared in sRGB whatever their writing (ESC-459).
 *
 * The preview (Playwright's Chromium) and the app (WebView2) do not serialise a computed colour the same way: the one gives
 * `oklch(0.84 0.14 82)`, the other `lab(81.57 12.31 61.4)` or `oklch(0.840003 0.140009 82.0039)`, for one same colour. Both are
 * brought here to 8-bit sRGB, and two colours are the same when no channel (alpha included) is further than one unit from the
 * other: a rounding. Everything else in a value (a length, a keyword, the order of a shadow's parts) is compared as written.
 */

/** What one unit of a channel is, in 8 bits. */
export const CHANNEL_TOLERANCE = 1;

/** A colour in 8-bit sRGB (0–255) and its alpha in 8 bits too (0–255). */
export type Srgb = [r: number, g: number, b: number, a: number];

const FUNCTION = /\b(rgba?|hsla?|oklch|oklab|lab|lch|color)\(([^()]*)\)/gi;

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));

/** A number of a colour's parts: `none` is 0, `50%` is `scale` times 0.5, `90deg` is 90. */
function part(token: string, scale: number): number {
  const t = token.trim().toLowerCase();
  if (t === 'none') return 0;
  if (t.endsWith('%')) return (parseFloat(t) / 100) * scale;
  if (t.endsWith('deg')) return parseFloat(t);
  if (t.endsWith('turn')) return parseFloat(t) * 360;
  if (t.endsWith('rad')) return (parseFloat(t) * 180) / Math.PI;
  return parseFloat(t);
}

/** The sRGB channel (0–1) of a linear light one. */
const encode = (c: number) => (Math.abs(c) <= 0.0031308 ? 12.92 * c : Math.sign(c) * (1.055 * Math.abs(c) ** (1 / 2.4) - 0.055));
const decode = (c: number) => (Math.abs(c) <= 0.04045 ? c / 12.92 : Math.sign(c) * ((Math.abs(c) + 0.055) / 1.055) ** 2.4);

const toBytes = (linear: number[], alpha: number): Srgb => [
  ...(linear.map((c) => Math.round(clamp(encode(c), 0, 1) * 255)) as [number, number, number]),
  Math.round(clamp(alpha, 0, 1) * 255),
];

function oklabToLinear(L: number, a: number, b: number): number[] {
  const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  return [
    4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
    -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
    -0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s,
  ];
}

/** CIE Lab (D50, as CSS has it) to linear sRGB (D65, through Bradford). */
function labToLinear(L: number, a: number, b: number): number[] {
  const fy = (L + 16) / 116;
  const fx = fy + a / 500;
  const fz = fy - b / 200;
  const e = 216 / 24389;
  const k = 24389 / 27;
  const f = (t: number) => (t ** 3 > e ? t ** 3 : (116 * t - 16) / k);
  const X = (f(fx) * 0.3457) / 0.3585;
  const Y = L > 8 ? fy ** 3 : L / k;
  const Z = (f(fz) * (1 - 0.3457 - 0.3585)) / 0.3585;
  // D50 to D65.
  const x = 0.955473421488075 * X - 0.02309845494876471 * Y + 0.06325924320057072 * Z;
  const y = -0.0283697093338637 * X + 1.0099953980813041 * Y + 0.021041441191917323 * Z;
  const z = 0.012314014864481998 * X - 0.020507649298898964 * Y + 1.330365926242124 * Z;
  return [
    3.2409699419045226 * x - 1.537383177570094 * y - 0.4986107602930034 * z,
    -0.9692436362808796 * x + 1.8759675015077202 * y + 0.04155505740717559 * z,
    0.05563007969699366 * x - 0.20397695888897652 * y + 1.0569715142428786 * z,
  ];
}

function hslToRgb(h: number, s: number, l: number): number[] {
  const k = (n: number) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  return [0, 8, 4].map((n) => l - a * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1)));
}

/** The sRGB of one colour function (`name` lower case, `args` what is between its parentheses), or null if not known. */
function convert(name: string, args: string): Srgb | null {
  const [main, alphaPart] = args.split('/');
  const words = main.replace(/,/g, ' ').trim().split(/\s+/);
  let alpha = alphaPart !== undefined ? part(alphaPart, 1) : 1;
  if (name === 'rgb' || name === 'rgba') {
    if (words.length === 4) alpha = part(words[3], 1);
    const [r, g, b] = words.slice(0, 3).map((w) => clamp(Math.round(part(w, 255)), 0, 255));
    return [r, g, b, Math.round(clamp(alpha, 0, 1) * 255)];
  }
  if (name === 'hsl' || name === 'hsla') {
    if (words.length === 4) alpha = part(words[3], 1);
    const [r, g, b] = hslToRgb(part(words[0], 360), part(words[1], 1), part(words[2], 1)).map((c) => clamp(Math.round(c * 255), 0, 255));
    return [r, g, b, Math.round(clamp(alpha, 0, 1) * 255)];
  }
  if (name === 'oklab' || name === 'oklch') {
    const L = part(words[0], 1);
    let a = part(words[1], 0.4);
    let b = part(words[2], 0.4);
    if (name === 'oklch') [a, b] = [a * Math.cos((b * Math.PI) / 180), a * Math.sin((b * Math.PI) / 180)];
    return toBytes(oklabToLinear(L, a, b), alpha);
  }
  if (name === 'lab' || name === 'lch') {
    const L = part(words[0], 100);
    let a = part(words[1], 125);
    let b = part(words[2], 125);
    if (name === 'lch') [a, b] = [a * Math.cos((b * Math.PI) / 180), a * Math.sin((b * Math.PI) / 180)];
    return toBytes(labToLinear(L, a, b), alpha);
  }
  if (name === 'color') {
    const space = words[0].toLowerCase();
    const c = words.slice(1, 4).map((w) => part(w, 1));
    if (space === 'srgb')
      return [...(c.map((v) => Math.round(clamp(v, 0, 1) * 255)) as [number, number, number]), Math.round(clamp(alpha, 0, 1) * 255)];
    if (space === 'srgb-linear') return toBytes(c, alpha);
    if (space === 'display-p3') {
      const [r, g, b] = c.map(decode);
      return toBytes(
        [
          1.2249401762805598 * r - 0.22494017628055996 * g,
          -0.04205695470968816 * r + 1.0420569547096881 * g,
          -0.019637554590334432 * r - 0.07863604555063188 * g + 1.0982736001409663 * b,
        ],
        alpha,
      );
    }
  }
  return null;
}

/** A computed CSS colour (`oklch(…)`, `lab(…)`, `rgb(…)`, `color(srgb …)`…) in 8-bit sRGB; null when it is not one. */
export function toSrgb(css: string): Srgb | null {
  const t = css.trim();
  if (t.toLowerCase() === 'transparent') return [0, 0, 0, 0];
  const m = /^(rgba?|hsla?|oklch|oklab|lab|lch|color)\(([^()]*)\)$/i.exec(t);
  return m ? convert(m[1].toLowerCase(), m[2]) : null;
}

/** A value cut into what is not a colour (its colours replaced by `§`) and its colours, in sRGB. */
function split(value: string): { rest: string; colours: Srgb[] } {
  const colours: Srgb[] = [];
  const rest = value.replace(FUNCTION, (whole) => {
    const c = toSrgb(whole);
    if (!c) return whole;
    colours.push(c);
    return '§';
  });
  return { rest: rest.replace(/\s+/g, ' ').trim(), colours };
}

/** A colour as the failure shows it. */
const show = (c: Srgb) => `rgb(${c[0]} ${c[1]} ${c[2]}${c[3] === 255 ? '' : ` / ${+(c[3] / 255).toFixed(3)}`})`;

/** A value with every colour of it written in sRGB (for a message). */
export function inSrgb(value: string): string {
  const { rest, colours } = split(value);
  let i = 0;
  return rest.replace(/§/g, () => show(colours[i++]));
}

/** Whether two colours differ by no more than `tolerance` units in a channel (alpha included). */
export const sameSrgb = (a: readonly number[], b: readonly number[], tolerance = CHANNEL_TOLERANCE) =>
  a.length === b.length && a.every((v, i) => Math.abs(v - b[i]) <= tolerance);

/** Whether two computed values are the same: the colours in them to a unit of a channel, all the rest to the letter. */
export function sameValue(a: string, b: string, tolerance = CHANNEL_TOLERANCE): boolean {
  if (a === b) return true;
  const x = split(a);
  const y = split(b);
  return x.rest === y.rest && x.colours.length === y.colours.length && x.colours.every((c, i) => sameSrgb(c, y.colours[i], tolerance));
}

/**
 * What differs between two looks (strings, numbers, arrays, objects of them), as paths and both values (colours in sRGB), or
 * an empty list when they are the same.
 */
export function lookDiff(a: unknown, b: unknown, at = '', tolerance = CHANNEL_TOLERANCE): string[] {
  if (typeof a === 'string' && typeof b === 'string') {
    return sameValue(a, b, tolerance) ? [] : [`${at || 'value'}: ${inSrgb(a)}  ≠  ${inSrgb(b)}`];
  }
  if (Array.isArray(a) && Array.isArray(b)) {
    const out = a.length === b.length ? [] : [`${at || 'array'}: length ${a.length}  ≠  ${b.length}`];
    for (let i = 0; i < Math.min(a.length, b.length); i++) out.push(...lookDiff(a[i], b[i], `${at}[${i}]`, tolerance));
    return out;
  }
  if (a && b && typeof a === 'object' && typeof b === 'object') {
    const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
    return [...keys].flatMap((k) =>
      lookDiff((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k], at ? `${at}.${k}` : k, tolerance),
    );
  }
  return Object.is(a, b) ? [] : [`${at || 'value'}: ${JSON.stringify(a)}  ≠  ${JSON.stringify(b)}`];
}
