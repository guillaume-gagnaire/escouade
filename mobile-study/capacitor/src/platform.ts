// Ce que Capacitor donne à la maquette : le Rust par un plugin natif à nous (Kotlin et Swift sur les liaisons UniFFI,
// `android/app/src/main/java/.../RustCorePlugin.kt`, `ios/App/App/RustCorePlugin.swift`), le push et le QR code par les
// plugins officiels, la biométrie par le plugin communautaire de Capgo (Capacitor n'en a pas d'officiel).
import { registerPlugin } from '@capacitor/core';
import { PushNotifications } from '@capacitor/push-notifications';
import { CapacitorBarcodeScanner, CapacitorBarcodeScannerTypeHint } from '@capacitor/barcode-scanner';
import { NativeBiometric } from '@capgo/capacitor-native-biometric';
import type { Platform } from '../../svelte-ui/src/platform';

interface RustCorePlugin {
  version(): Promise<{ value: string }>;
  open(o: { key: string; sealed: string }): Promise<{ value: string }>;
  seal(o: { key: string; plain: string }): Promise<{ value: string }>;
  benchInRust(o: { n: number; size: number }): Promise<{ value: number }>;
}
const RustCore = registerPlugin<RustCorePlugin>('RustCore');

export const platform: Platform = {
  name: 'capacitor',
  rust: {
    version: async () => (await RustCore.version()).value,
    open: async (key, sealed) => (await RustCore.open({ key, sealed })).value,
    seal: async (key, plain) => (await RustCore.seal({ key, plain })).value,
    benchInRust: async (n, size) => (await RustCore.benchInRust({ n, size })).value,
  },
  async biometric(reason) {
    try {
      const a = await NativeBiometric.isAvailable({ useFallback: true });
      if (!a.isAvailable) return { ok: false, detail: `indisponible (code ${a.errorCode ?? '?'})` };
      await NativeBiometric.verifyIdentity({ reason, title: 'Escouade', useFallback: true });
      return { ok: true, detail: 'empreinte' };
    } catch (e) {
      return { ok: false, detail: String(e) };
    }
  },
  async scanQr() {
    try {
      const r = await CapacitorBarcodeScanner.scanBarcode({ hint: CapacitorBarcodeScannerTypeHint.QR_CODE });
      return r.ScanResult || null;
    } catch (e) {
      console.warn('scan', e);
      return null;
    }
  },
  push: {
    async register() {
      try {
        const perm = await PushNotifications.requestPermissions();
        if (perm.receive !== 'granted') return { token: null, detail: `permission ${perm.receive}` };
        const token = new Promise<{ token: string | null; detail: string }>((resolve) => {
          PushNotifications.addListener('registration', (t) => resolve({ token: t.value, detail: 'ok' }));
          PushNotifications.addListener('registrationError', (e) => resolve({ token: null, detail: `FCM : ${e.error}` }));
          setTimeout(() => resolve({ token: null, detail: 'pas de réponse de FCM en 20 s' }), 20_000);
        });
        await PushNotifications.register();
        return await token;
      } catch (e) {
        return { token: null, detail: String(e) };
      }
    },
    onOpened(cb) {
      PushNotifications.addListener('pushNotificationActionPerformed', (a) => {
        const data = (a.notification.data ?? {}) as Record<string, string>;
        cb({ sealed: data.sealed, raw: a });
      });
    },
  },
};
