/**
 * The system's lock before a gesture that makes an agent act (M3, ESC-570, `docs/MOBILE.md` § 3.9): the one function
 * the screens call — MO-5 (an authorization, an answer that acts), MO-7 (« Valider », « Lancer »), MO-11 (« Créer et
 * lancer »). Refusing, answering a question or choosing between options does not ask it.
 *
 * - **Android**: BiometricPrompt, the fingerprint or the face, « Utiliser le code » as a fallback; **iOS**: Face ID or
 *   Touch ID, the phone's code as a fallback. The reason says what goes (« Autoriser la commande de tests-e2e ? ·
 *   Escouade · Travail »).
 * - **A failure or a cancel changes nothing**: `false`, the request stays open, the gesture is made again.
 * - **In a browser** (the e2e), the plugin's web side stands for it: `window.__ESCOUADE_LOCK__` says what the system
 *   answers.
 */

import { EscouadeMobile, type EscouadeMobilePlugin, type LockState } from '@escouade/capacitor-mobile';

type Lock = Pick<EscouadeMobilePlugin, 'unlock' | 'lockState'>;

/** The plugin the lock goes through: the app's, or a test's. */
let plugin: Lock = EscouadeMobile;

/** A test's plugin in place of the app's; the app's again with none. */
export function useLock(fake: Lock | null): void {
  plugin = fake ?? EscouadeMobile;
}

/**
 * The system's lock asked, saying `reason` (what goes, and where): `true` once it passed, `false` when it was cancelled,
 * failed, or the phone has no lock any more. Never throws: the gesture is simply not made.
 */
export async function confirmByLock(reason: string): Promise<boolean> {
  try {
    await plugin.unlock({ reason });
    return true;
  } catch {
    return false;
  }
}

/** Whether the phone has a screen lock, and its biometry: no pairing without one (M3). */
export async function lockState(): Promise<LockState> {
  try {
    return await plugin.lockState();
  } catch {
    return { screenLock: false, biometry: 'none' };
  }
}
