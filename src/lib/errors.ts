/**
 * The path of a file the backend found missing (`NOT_FOUND:<path>`, see `fsedit::NotFound`), null for any other
 * error. The rest of the line is the path, colons included. The editor shows such a file as missing, in its own words
 * (`editor`), never this code.
 */
export const notFoundPath = (e: unknown): string | null => (typeof e === 'string' ? (/^NOT_FOUND:(.*)$/s.exec(e)?.[1] ?? null) : null);
