// The camera of the « Workspace » (spec docs/superpowers/specs/2026-11-25-vue-workspace-design.md, « La toile »): where
// the canvas is seen from and how close. Pure, so the view only applies it.

/** The point of the screen where the world's origin is drawn, and the scale it is drawn at. */
export type Camera = { x: number; y: number; zoom: number };

export const ZOOM_MIN = 0.3;
export const ZOOM_MAX = 2.5;
/** One click of « + » or « − ». */
export const ZOOM_STEP = 1.25;
/** The step of the dotted background, at a zoom of 1 (px). */
export const DOT = 18;
/** Around the cards: of a fit, and of the first agent at the top left on opening (px). */
export const MARGIN = 32;
/** The height the floating bar and « Quitter » take at the top of the canvas: a fit keeps the cards below it (px). */
export const HUD_H = 56;

/** From this zoom up, every side of a compass is unfolded; below it, down to `ZOOM_MIN`, its sides are in piles. */
export const NEAR_FROM = 0.6;
export type Altitude = 'mid' | 'near';

export const clampZoom = (z: number) => Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, z));

/** Zooms by `factor` around the point (px, py) of the screen, which stays where it is. */
export function zoomAt(cam: Camera, factor: number, px: number, py: number): Camera {
  const zoom = clampZoom(cam.zoom * factor);
  const k = zoom / cam.zoom;
  return { x: px - (px - cam.x) * k, y: py - (py - cam.y) * k, zoom };
}

export const pan = (cam: Camera, dx: number, dy: number): Camera => ({ x: cam.x + dx, y: cam.y + dy, zoom: cam.zoom });

export type Size = { w: number; h: number };
export type Box = { x: number; y: number; w: number; h: number };

/** The camera that shows the whole world in the view, centred, with `margin` around it, and no closer than `max`. */
export function fit(world: Size, view: Size, margin = MARGIN, max = ZOOM_MAX): Camera {
  if (!world.w || !world.h) return { x: view.w / 2, y: view.h / 2, zoom: 1 };
  const zoom = Math.min(max, clampZoom(Math.min((view.w - 2 * margin) / world.w, (view.h - 2 * margin) / world.h)));
  return { x: (view.w - world.w * zoom) / 2, y: (view.h - world.h * zoom) / 2, zoom };
}

export const altitude = (zoom: number): Altitude => (zoom < NEAR_FROM ? 'mid' : 'near');

/** The altitudes and the zooms each is seen at, from the closest to the farthest. */
export const ALTITUDES: { name: Altitude; min: number; max: number }[] = [
  { name: 'near', min: NEAR_FROM, max: ZOOM_MAX },
  { name: 'mid', min: ZOOM_MIN, max: NEAR_FROM - 0.001 },
];

/** Fits the closest altitude whose world holds in the view at its zoom; the farthest otherwise. */
export function fitBest(worlds: Record<Altitude, Size>, view: Size): Camera {
  let cam: Camera = { x: 0, y: 0, zoom: 1 };
  for (const a of ALTITUDES) {
    cam = fit(worlds[a.name], view, MARGIN, a.max);
    if (cam.zoom >= a.min) return cam;
  }
  return cam;
}

const centre = (b: Box) => ({ x: b.x + b.w / 2, y: b.y + b.h / 2 });

/** At the zoom of `cam`, the camera that draws the centre of `to` where the centre of `from` was on screen. */
export function keepAnchor(cam: Camera, from: Box, to: Box): Camera {
  const f = centre(from);
  const t = centre(to);
  return { x: f.x * cam.zoom + cam.x - t.x * cam.zoom, y: f.y * cam.zoom + cam.y - t.y * cam.zoom, zoom: cam.zoom };
}

/** The box whose centre is drawn the nearest the middle of the screen. */
export function nearest<T extends Box>(items: T[], cam: Camera, view: Size): T | undefined {
  const mx = (view.w / 2 - cam.x) / cam.zoom;
  const my = (view.h / 2 - cam.y) / cam.zoom;
  let best: T | undefined;
  let d = Infinity;
  for (const it of items) {
    const c = centre(it);
    const di = (c.x - mx) ** 2 + (c.y - my) ** 2;
    if (di < d) [best, d] = [it, di];
  }
  return best;
}
