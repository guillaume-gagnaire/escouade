import { registerPlugin } from '@capacitor/core';

import type { EscouadeMobilePlugin } from './definitions';
import { EscouadeMobileWeb } from './web';

/**
 * The mobile app's native side and its Rust, in Capacitor (`definitions.ts`). In a browser (the e2e, `npm run dev`),
 * `web.ts` stands for it: a space at its address, no tunnel, the screen lock the test's.
 */
export const EscouadeMobile = registerPlugin<EscouadeMobilePlugin>('EscouadeMobile', {
  web: () => new EscouadeMobileWeb(),
});

export * from './definitions';
