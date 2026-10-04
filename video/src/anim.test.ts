import { describe, expect, it } from 'vitest';
import { count, fr, pop, ramp, SCENE_IN, SCENE_OUT, sceneMotion, typed } from './anim';

describe('animation helpers', () => {
  it('ramps from 0 to 1 over a window, clamped', () => {
    expect(ramp(0, 10, 20)).toBe(0);
    expect(ramp(20, 10, 20)).toBe(0.5);
    expect(ramp(99, 10, 20)).toBe(1);
  });

  it('springs from 0 once started', () => {
    expect(pop(5, 30, 10)).toBe(0);
    expect(pop(200, 30, 10)).toBeCloseTo(1, 2);
  });

  it('types text at a given speed', () => {
    expect(typed('abcdef', 0, 30, 10)).toBe('');
    expect(typed('abcdef', 30, 30, 0, 3)).toBe('abc');
    expect(typed('abcdef', 999, 30, 0)).toBe('abcdef');
  });

  it('counts between two values and writes French numbers', () => {
    expect(count(15, 10, 10, 0, 2)).toBe(1);
    expect(fr(1.5)).toBe('1,50');
    expect(fr(12.345, 1)).toBe('12,3');
  });
});

describe('a scene between its neighbours', () => {
  const D = 300;

  it('slides in from the right, blurred, and settles within its entry', () => {
    const first = sceneMotion(0, D);
    expect(first.x).toBeGreaterThan(100);
    expect(first.blur).toBeGreaterThan(8);
    expect(first.opacity).toBeLessThan(0.2);
    const settled = sceneMotion(SCENE_IN, D);
    expect(settled.x).toBeCloseTo(0, 5);
    expect(settled.blur).toBe(0);
    expect(settled.opacity).toBe(1);
  });

  it('leaves to the left the same way, in its last frames only', () => {
    expect(sceneMotion(D - SCENE_OUT - 1, D)).toMatchObject({ blur: 0, opacity: 1 });
    const last = sceneMotion(D - 1, D);
    expect(last.x).toBeLessThan(-100);
    expect(last.blur).toBeGreaterThan(8);
    expect(last.opacity).toBeLessThan(0.3);
  });

  it('pushes in slowly all along: never still, never more than 3 %', () => {
    expect(sceneMotion(SCENE_IN, D).scale).toBeLessThan(sceneMotion(D / 2, D).scale);
    expect(sceneMotion(D / 2, D).scale).toBeLessThan(sceneMotion(D - SCENE_OUT - 1, D).scale);
    expect(sceneMotion(D - 1, D).scale).toBeLessThanOrEqual(1.03);
  });

  it('enters without a slide for the first scene, and leaves without one for the last', () => {
    expect(sceneMotion(0, D, { first: true })).toMatchObject({ x: 0, blur: 0 });
    expect(sceneMotion(D - 1, D, { last: true })).toMatchObject({ x: 0, blur: 0, opacity: 1 });
  });
});
