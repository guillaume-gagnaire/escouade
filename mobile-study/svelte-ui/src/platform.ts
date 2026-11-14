// Ce que la maquette Svelte demande au système, et que Tauri 2 ou Capacitor lui donne (`tauri/src/platform.ts`,
// `capacitor/src/platform.ts`). Le reste de l'interface est le même pour les deux.

/** Le Rust commun (`mobile-study/shared/core`) : Tauri l'appelle sans pont, Capacitor par un plugin natif (UniFFI). */
export interface RustCore {
  version(): Promise<string>;
  open(key: string, sealed: string): Promise<string>;
  seal(key: string, plain: string): Promise<string>;
  /** `n` allers-retours faits dans le Rust (µs). */
  benchInRust(n: number, size: number): Promise<number>;
}

export interface PushOpened {
  /** Le texte scellé du push (le relais ne voit que lui). */
  sealed?: string;
  raw: unknown;
}

export interface Platform {
  /** « tauri » ou « capacitor » : le nom des repères envoyés au faux serveur. */
  name: string;
  rust: RustCore;
  /** Le verrou biométrique : true si l'empreinte (ou le visage) a été reconnue. */
  biometric(reason: string): Promise<{ ok: boolean; detail: string }>;
  /** Le QR code de l'appairage, lu à l'appareil photo ; null si l'utilisateur a renoncé. */
  scanQr(): Promise<string | null>;
  push: {
    /** Demande la permission et inscrit l'app : le jeton (FCM, APNs), ou l'erreur rencontrée. */
    register(): Promise<{ token: string | null; detail: string }>;
    /** Une notification touchée, app ouverte par elle ou déjà ouverte. */
    onOpened(cb: (p: PushOpened) => void): void;
  };
}
