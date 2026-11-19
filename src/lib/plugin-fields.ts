import { fNum } from './format';
import { t } from './i18n';
import type { Manifest, ManifestSetting, ManifestText } from './plugins';

// The page of a plugin's settings (ESC-526, issue #40, docs/PLUGINS-CONTRACT-STUDY.md §4): its fields put in the groups its
// manifest declares (`contributes.groups`), settings and secrets mixed in the order of `fields`; a field in no group comes
// after, under « Réglages » then « Secrets », as before. A field on several lines (`multiline`) and the import of a file
// (`import`): the file is read once, in UTF-8, 64 KiB at most; its content fills the field, its path is never known here.

/** The most a field on several lines holds, in bytes (64 KiB): over it, refused, nothing cut. */
export const FIELD_MAX = 64 * 1024;

/** A secret as the manifest declares it (`contributes.secrets`). */
export interface ManifestSecret {
  key: string;
  title?: ManifestText;
  description?: ManifestText;
  multiline?: boolean;
  lines?: number;
  import?: string[];
}

/** A setting, with what a field on several lines adds. */
export type FieldSetting = ManifestSetting & { multiline?: boolean; lines?: number; import?: string[] };

/** A group of the manifest (`contributes.groups`). */
export interface ManifestGroup {
  id: string;
  title?: ManifestText;
  description?: ManifestText;
  folded?: boolean;
  fields: string[];
}

/** A field of the page: a setting or a secret, by its key. */
export type Field = { kind: 'setting'; key: string; setting: FieldSetting } | { kind: 'secret'; key: string; secret: ManifestSecret };

/** What the page shows, in its order: the groups, then the settings and the secrets in no group. */
export interface Layout {
  groups: { group: ManifestGroup; fields: Field[] }[];
  settings: FieldSetting[];
  secrets: ManifestSecret[];
}

/** The page's fields: `secretKeys` are the secrets it shows (`contributes.secrets`, then those of `permissions.secrets`). */
export function layoutOf(manifest: Manifest | null, secretKeys: string[]): Layout {
  const c = manifest?.contributes ?? {};
  // A setting of each project is not on the plugin's page: each project sets it in its own (ESC-530).
  const settings = ((c.settings as FieldSetting[] | undefined) ?? []).filter((s) => s?.scope !== 'project');
  const declared = (c.secrets as ManifestSecret[] | undefined) ?? [];
  const secrets: ManifestSecret[] = secretKeys.map((key) => declared.find((s) => s.key === key) ?? { key });
  const groups = ((c.groups as ManifestGroup[] | undefined) ?? []).filter((g) => g && Array.isArray(g.fields));
  const placed = new Set<string>();
  const shown = groups
    .map((group) => ({
      group,
      fields: group.fields.flatMap((key): Field[] => {
        if (placed.has(key)) return [];
        const setting = settings.find((s) => s.key === key);
        const secret = secrets.find((s) => s.key === key);
        const field: Field | null = setting ? { kind: 'setting', key, setting } : secret ? { kind: 'secret', key, secret } : null;
        if (!field) return [];
        placed.add(key);
        return [field];
      }),
    }))
    .filter((g) => g.fields.length);
  return {
    groups: shown,
    settings: settings.filter((s) => !placed.has(s.key)),
    secrets: secrets.filter((s) => !placed.has(s.key)),
  };
}

/** The lines a field on several lines shows: `lines` from 3 to 20, 6 when not said. */
export function visibleLines(lines: number | undefined): number {
  if (typeof lines !== 'number' || !Number.isFinite(lines)) return 6;
  return Math.min(20, Math.max(3, Math.round(lines)));
}

/** The bytes of a text in UTF-8. */
export const bytesOf = (text: string) => new TextEncoder().encode(text).length;

/** Why a text does not fit a field (over 64 KiB), or null. */
export function tooBig(text: string): string | null {
  return bytesOf(text) > FIELD_MAX ? t('plugins.settings.tooBig') : null;
}

/** The lines of a text, as its numbered field counts them (an empty text: one). */
export function lineCount(text: string): number {
  return text.replace(/\r?\n$/, '').split(/\r?\n/).length;
}

/** « 5,1 Ko »: a size of a few kilobytes with one decimal (« 0,1 Ko » at least, never « 0 Ko »). */
export function sizeLabel(bytes: number): string {
  return t('format.bytes.kilobytes', { n: fNum(Math.max(0.1, bytes / 1024), 1) });
}

/** What a secret on several lines kept says: « Gardé · 84 lignes · 5,1 Ko ». */
export function keptLabel(size: { lines: number; bytes: number }, inFile: boolean): string {
  return t(inFile ? 'plugins.settings.keptSizeInFile' : 'plugins.settings.keptSize', {
    lines: t('plugins.settings.lines', { count: size.lines }),
    size: sizeLabel(size.bytes),
  });
}

/** The `accept` of the file dialog: the extensions the field takes. */
export const acceptOf = (exts: string[]) => exts.join(',');

/** The bytes of a file (FileReader where `arrayBuffer` is missing, as in jsdom). */
function bufferOf(file: Blob): Promise<ArrayBuffer> {
  if (typeof file.arrayBuffer === 'function') return file.arrayBuffer();
  return new Promise((resolve, reject) => {
    const r = new FileReader();
    r.onload = () => resolve(r.result as ArrayBuffer);
    r.onerror = () => reject(r.error);
    r.readAsArrayBuffer(file);
  });
}

/**
 * Reads a file the user chose for a field that imports one, once: its name must end with one of `exts` (the dialog's filter
 * is a help, not a check), it holds 64 KiB at most, and it is text in UTF-8 (no NUL). Answers its text, or why it was refused;
 * nothing of where it was is kept, nor its name.
 */
export async function readImport(file: Blob & { name?: string }, exts: string[]): Promise<{ text: string } | { error: string }> {
  const name = (file.name ?? '').toLowerCase();
  if (exts.length && !exts.some((e) => name.endsWith(e.toLowerCase())))
    return { error: t('plugins.settings.importKind', { exts: exts.join(', ') }) };
  if (file.size > FIELD_MAX) return { error: t('plugins.settings.tooBig') };
  const bytes = new Uint8Array(await bufferOf(file));
  if (bytes.length > FIELD_MAX) return { error: t('plugins.settings.tooBig') };
  let text: string;
  try {
    text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    return { error: t('plugins.settings.notText') };
  }
  if (text.includes('\u0000')) return { error: t('plugins.settings.notText') };
  return { text };
}
