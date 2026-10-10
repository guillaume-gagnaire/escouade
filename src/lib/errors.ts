import { t } from './i18n';

/**
 * The path of a file the backend found missing (`NOT_FOUND:<path>`, see `fsedit::NotFound`), null for any other
 * error. The rest of the line is the path, colons included.
 */
export const notFoundPath = (e: unknown): string | null => (typeof e === 'string' ? (/^NOT_FOUND:(.*)$/s.exec(e)?.[1] ?? null) : null);

/** An error of the backend as the window shows it: a code it sends is written here, any other text is its own. */
export function errorText(e: unknown): string {
  const path = notFoundPath(e);
  if (path !== null) return t('errors.notFound', { path });
  return String(e);
}
