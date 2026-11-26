import { describe, expect, it } from 'vitest';
import { altitude, clampZoom, fit, fitBest, keepAnchor, nearest, pan, zoomAt } from './workspace-camera';

describe('the camera of the workspace', () => {
  it('clamps the zoom', () => {
    expect(clampZoom(0.1)).toBe(0.3);
    expect(clampZoom(9)).toBe(2.5);
  });

  it('keeps the point under the pointer fixed', () => {
    const c = zoomAt({ x: 0, y: 0, zoom: 1 }, 2, 100, 50);
    expect(c).toEqual({ x: -100, y: -50, zoom: 2 });
    expect((100 - c.x) / c.zoom).toBe(100);
  });

  it('does not move when the zoom is already at its bound', () => {
    expect(zoomAt({ x: 10, y: 20, zoom: 2.5 }, 2, 0, 0)).toEqual({ x: 10, y: 20, zoom: 2.5 });
  });

  it('pans', () => {
    expect(pan({ x: 1, y: 2, zoom: 1.5 }, 10, -5)).toEqual({ x: 11, y: -3, zoom: 1.5 });
  });

  it('fits a world in the view, centred', () => {
    const c = fit({ w: 1000, h: 500 }, { w: 1064, h: 800 });
    expect(c.zoom).toBe(1);
    expect(c.x).toBe(32);
    expect(c.y).toBe(150);
  });

  it('fits within the zoom bounds, even in a tiny view', () => {
    expect(fit({ w: 5000, h: 5000 }, { w: 200, h: 100 }).zoom).toBe(0.3);
    expect(fit({ w: 10, h: 10 }, { w: 2000, h: 2000 }).zoom).toBe(2.5);
  });

  it('centres an empty world', () => {
    expect(fit({ w: 0, h: 0 }, { w: 800, h: 600 })).toEqual({ x: 400, y: 300, zoom: 1 });
  });

  it('tells the altitude', () => {
    expect(altitude(0.3)).toBe('mid');
    expect(altitude(0.45)).toBe('mid');
    expect(altitude(0.59)).toBe('mid');
    expect(altitude(0.6)).toBe('near');
  });
  it('fits the piles when the unfolded compasses do not hold', () => {
    const view = { w: 1200, h: 800 };
    const c = fitBest({ mid: { w: 2000, h: 1200 }, near: { w: 4000, h: 2600 } }, view);
    expect(altitude(c.zoom)).toBe('mid');
    expect(c.zoom).toBeLessThan(0.6);
  });
  it('caps the zoom of a fit', () => {
    expect(fit({ w: 10, h: 10 }, { w: 2000, h: 2000 }, 32, 0.44).zoom).toBe(0.44);
  });
  it('fits the compasses when they hold at 60 % or more, else the piles, down to 30 %', () => {
    const view = { w: 1000, h: 800 };
    expect(fitBest({ mid: { w: 600, h: 400 }, near: { w: 900, h: 600 } }, view).zoom).toBeGreaterThanOrEqual(0.6);
    const c = fitBest({ mid: { w: 6000, h: 3000 }, near: { w: 6000, h: 3000 } }, view);
    expect(altitude(c.zoom)).toBe('mid');
    expect(c.zoom).toBe(0.3);
  });
  it('keeps an agent where it was on screen', () => {
    const cam = { x: 100, y: 50, zoom: 1 };
    const c = keepAnchor(cam, { x: 0, y: 0, w: 300, h: 300 }, { x: 1000, y: 400, w: 800, h: 600 });
    // centre de `from` à l'écran : (250, 200) ; celui de `to`, (1400, 700), y tombe
    expect([1400 * c.zoom + c.x, 700 * c.zoom + c.y]).toEqual([250, 200]);
  });
  it('finds the box nearest the middle of the screen', () => {
    const items = [
      { id: 'a', x: 0, y: 0, w: 100, h: 100 },
      { id: 'b', x: 400, y: 0, w: 100, h: 100 },
    ];
    expect(nearest(items, { x: 0, y: 0, zoom: 1 }, { w: 900, h: 100 })?.id).toBe('b');
  });
});
