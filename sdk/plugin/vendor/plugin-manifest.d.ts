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
    | 'account';
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
}

export const API: number;
export const RESERVED_PREFIX: string;
export const ESCOUADE_DIR: string;
export function validate(manifest: unknown, options?: ValidateOptions): Validation;
export function validateText(text: string, options?: ValidateOptions): Validation;
export function message(error: ManifestError, lang?: 'fr' | 'en'): string;
