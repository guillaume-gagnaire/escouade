// The phone's appearance (MO-1): the system's theme followed, the system's size of a target.
import { describe, expect, it, vi } from 'vitest';
import { applyAppearance, followSystemTheme, systemTheme } from './appearance';
import { platformOf } from './native';

function media(light: boolean) {
  const listeners: (() => void)[] = [];
  const query = {
    matches: light,
    addEventListener: (_: string, f: () => void) => listeners.push(f),
    removeEventListener: vi.fn(),
  };
  return { win: { matchMedia: () => query as unknown as MediaQueryList }, query, fire: () => listeners.forEach((f) => f()) };
}

describe('the phone’s appearance', () => {
  it('takes the system’s theme, and follows it as it changes', () => {
    const m = media(false);
    expect(systemTheme(m.win)).toBe('dark');
    const seen: string[] = [];
    followSystemTheme((t) => seen.push(t), m.win);
    m.query.matches = true;
    m.fire();
    expect(seen).toEqual(['light']);
  });

  it('puts the theme and the system on the root, for the app’s tokens and the phone’s', () => {
    const root = document.createElement('html');
    applyAppearance('light', 'android', root);
    expect(root.dataset).toMatchObject({ theme: 'light', platform: 'android' });
    expect(root.style.colorScheme).toBe('light');
  });

  it('knows the system from Capacitor, else from a browser’s user agent, iOS by default', () => {
    expect(platformOf('android', '')).toBe('android');
    expect(platformOf('ios', 'Android')).toBe('ios');
    expect(platformOf('web', 'Mozilla/5.0 (Linux; Android 14; Pixel 8)')).toBe('android');
    expect(platformOf('web', 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)')).toBe('ios');
    expect(platformOf('web', 'Mozilla/5.0 (Windows NT 10.0)')).toBe('ios');
  });
});
