import { registerPlugin } from '@capacitor/core';

import type { EscouadeMobilePlugin } from './definitions';

/**
 * The mobile app's Rust, in Capacitor (`definitions.ts`). Native only: in a browser, every call rejects with
 * Capacitor's « not implemented on web » (the desktop app reaches the core without a tunnel).
 */
export const EscouadeMobile = registerPlugin<EscouadeMobilePlugin>('EscouadeMobile');

export * from './definitions';
