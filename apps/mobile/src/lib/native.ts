/**
 * What the app takes from the phone's system through Capacitor (MO-1): which system it runs on, its back button
 * (Android), going to the background and back, and the colour of its bars' icons. In a browser (the e2e, `npm run
 * dev`), the same page runs without them: the history stands for the back button (`nav.svelte.ts`).
 */

import { App } from '@capacitor/app';
import { Capacitor, SystemBars, SystemBarsStyle } from '@capacitor/core';

export type Platform = 'android' | 'ios';

/** The system: Capacitor's, else the browser's (its user agent: the e2e give a phone's). iOS's sizes by default. */
export function platformOf(
  native: string = Capacitor.getPlatform(),
  ua: string = typeof navigator === 'undefined' ? '' : navigator.userAgent,
): Platform {
  if (native === 'android' || native === 'ios') return native;
  return /Android/i.test(ua) ? 'android' : 'ios';
}

export const isNative = (): boolean => Capacitor.isNativePlatform();

/** Android's back button: `step` says whether the app stays; at its first screen it goes to the background. */
export function onBackButton(step: () => boolean): void {
  if (!isNative()) return;
  void App.addListener('backButton', () => {
    if (!step()) void App.minimizeApp();
  });
}

/** The app went to the background, or came back. */
export function onForeground(changed: (active: boolean) => void): void {
  if (!isNative()) return;
  void App.addListener('appStateChange', ({ isActive }) => changed(isActive));
}

/** The icons of the system's bars, readable on the app's theme (light icons on the dark one). */
export function barsFor(theme: 'light' | 'dark'): void {
  if (!isNative()) return;
  void SystemBars.setStyle({ style: theme === 'dark' ? SystemBarsStyle.Dark : SystemBarsStyle.Light }).catch(() => {});
}
