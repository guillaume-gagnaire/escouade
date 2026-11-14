import './src/rune-shim';
import { registerRootComponent } from 'expo';
import * as Notifications from 'expo-notifications';
import * as TaskManager from 'expo-task-manager';

import App from './App';
import { open } from './src/rust';
import { loadPairing, mark } from './src/server';

// Un push « données » reçu app fermée : expo-notifications lance ce JavaScript sans interface (headless), qui ouvre
// le contenu scellé par le Rust. Défini au niveau du module, avant tout rendu, comme Expo le demande.
export const PUSH_TASK = 'escouade-push';
TaskManager.defineTask<Notifications.NotificationTaskPayload>(PUSH_TASK, async ({ data, error }) => {
  if (error) return;
  const p = await loadPairing();
  const payload = data as unknown as { data?: Record<string, string> } & Record<string, unknown>;
  const sealed = payload?.data?.sealed ?? (payload as Record<string, string>).sealed;
  let r = 'plain';
  if (sealed) {
    try {
      open(p.key, sealed);
      r = 'decrypted';
    } catch {
      r = 'error';
    }
  }
  mark('push-bg', r);
});
Notifications.registerTaskAsync(PUSH_TASK).catch(() => {});

registerRootComponent(App);
