// Ce que Tauri 2 donne à la maquette : le Rust par `invoke` (sans pont à générer), les plugins officiels pour le QR code
// et la biométrie, le plugin communautaire de Choochmeque pour le push (FCM, APNs).
import { invoke } from '@tauri-apps/api/core';
import { scan, Format, requestPermissions as cameraPermission } from '@tauri-apps/plugin-barcode-scanner';
import { authenticate, checkStatus } from '@tauri-apps/plugin-biometric';
import { registerForPushNotifications, requestPermission, onNotificationClicked } from '@choochmeque/tauri-plugin-notifications-api';
import type { Platform } from '../../svelte-ui/src/platform';

export const platform: Platform = {
  name: 'tauri',
  rust: {
    version: () => invoke<string>('core_version'),
    open: (key, sealed) => invoke<string>('open', { key, sealed }),
    seal: (key, plain) => invoke<string>('seal', { key, plain }),
    benchInRust: (n, size) => invoke<number>('bench_in_rust', { n, size }),
  },
  async biometric(reason) {
    try {
      const status = await checkStatus();
      if (!status.isAvailable) return { ok: false, detail: `indisponible : ${status.error ?? status.errorCode ?? '?'}` };
      await authenticate(reason, { allowDeviceCredential: true });
      return { ok: true, detail: 'empreinte' };
    } catch (e) {
      return { ok: false, detail: String(e) };
    }
  },
  async scanQr() {
    try {
      await cameraPermission();
      const r = await scan({ windowed: false, formats: [Format.QRCode] });
      return r.content;
    } catch (e) {
      console.warn('scan', e);
      return null;
    }
  },
  push: {
    async register() {
      try {
        await requestPermission();
        return { token: await registerForPushNotifications(), detail: 'ok' };
      } catch (e) {
        return { token: null, detail: String(e) };
      }
    },
    onOpened(cb) {
      onNotificationClicked((d) => {
        const data = (d as { data?: Record<string, string> }).data ?? {};
        cb({ sealed: data.sealed, raw: d });
      }).catch((e) => console.warn('push', e));
    },
  },
};
