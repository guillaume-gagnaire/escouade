// Le Rust commun (mobile-study/shared/core) vu du JavaScript : le turbo-module généré par uniffi-bindgen-react-native
// (modules/escouade-core), appelé de façon synchrone par JSI.
import * as core from 'react-native-escouade-core';

export const coreVersion = (): string => core.coreVersion();
export const open = (key: string, sealed: string): string => core.open(key, sealed);
export const seal = (key: string, plain: string): string => core.seal(key, plain);
export const benchInRust = (n: number, size: number): number => Number(core.benchRoundtrips(n, size));
