// The types of plugin-manifest.mjs: the checks of a plugin's manifest, escouade-plugin.json (ESC-173).

/** A field of a manifest that is wrong, by its path (`contributes.settings[0].key`; empty for the whole file). */
export interface ManifestError {
  path: string;
  code:
    | 'required'
    | 'type'
    | 'empty'
    | 'id'
    | 'reserved'
    | 'version'
    | 'api'
    | 'range'
    | 'url'
    | 'path'
    | 'key'
    | 'enum'
    | 'duplicate'
    | 'account'
    | 'file'
    | 'png'
    | 'heavy'
    | 'square'
    | 'small';
  /** The type, or the values, the field takes, for `type` and `enum`. */
  expected?: string;
}

export interface Validation {
  ok: boolean;
  errors: ManifestError[];
  /** The fields Escouade does not know, ignored. */
  ignored: string[];
}

export interface ValidateOptions {
  /** A plugin the app itself gives, which may take the ids and the programs of Escouade's own. */
  builtin?: boolean;
  /** The bytes of a file of the plugin's folder by its path, null when it is none: to read its `icon`. */
  file?: (path: string) => Uint8Array | null | undefined;
}

export const API: number;
export const RESERVED_PREFIX: string;
export const ESCOUADE_DIR: string;
export const ICON_MIN: number;
export const ICON_BYTES: number;
/** An icon's path as the folder's files are named: `/` between its parts, without `./` in front. */
export function iconPath(path: string): string;
/** What is wrong with an icon's bytes, from its header alone. */
export function iconProblems(bytes: Uint8Array): ('png' | 'heavy' | 'square' | 'small')[];
export function validate(manifest: unknown, options?: ValidateOptions): Validation;
export function validateText(text: string, options?: ValidateOptions): Validation;
export function message(error: ManifestError, lang?: 'fr' | 'en'): string;
