// A Word file (`.docx`) read in a tab of the VS Code view (ESC-473): converted to HTML by mammoth (BSD-2-Clause),
// loaded when the first one opens: its titles, paragraphs, lists, tables, links and images (as `data:` images, which
// name nothing), in read only. Its HTML is sanitized as any document's (`cleanHtml`), its links followed by the pane.
//
// A `.docx` is a zip: before mammoth reads it whole in memory, the sizes its central directory says are added up, and
// a file that would inflate past `MAX_INFLATED` is not opened (a zip bomb stays a file of a few kilobytes).

import mammoth from 'mammoth/mammoth.browser.min.js';
import { t } from '../../i18n';
import { fSize } from '../../format';
import { cleanHtml } from './sanitize';

/** What a Word file may inflate to, at most. */
export const MAX_INFLATED = 256 * 1024 * 1024;

/**
 * The total of the uncompressed sizes of the entries of a zip, from its central directory (null when it is not a zip
 * that can be read so). A ZIP64 entry, whose sizes are in its extra field, counts as too big.
 */
export function inflatedSize(b: Uint8Array): number | null {
  const v = new DataView(b.buffer, b.byteOffset, b.byteLength);
  // The end of central directory record: its signature in the last 64 KiB + 22 bytes.
  let eocd = -1;
  for (let i = b.length - 22; i >= Math.max(0, b.length - 22 - 0xffff); i--) {
    if (v.getUint32(i, true) === 0x06054b50) {
      eocd = i;
      break;
    }
  }
  if (eocd < 0) return null;
  const count = v.getUint16(eocd + 10, true);
  let at = v.getUint32(eocd + 16, true);
  let total = 0;
  for (let n = 0; n < count; n++) {
    if (at + 46 > b.length || v.getUint32(at, true) !== 0x02014b50) return null;
    const size = v.getUint32(at + 24, true);
    if (size === 0xffffffff) return Number.MAX_SAFE_INTEGER;
    total += size;
    at += 46 + v.getUint16(at + 28, true) + v.getUint16(at + 30, true) + v.getUint16(at + 32, true);
  }
  return total;
}

/** The HTML of the Word file of `bytes`, sanitized. */
export async function docxHtml(bytes: Uint8Array): Promise<string> {
  const inflated = inflatedSize(bytes);
  if (inflated === null) throw new Error(t('editor.preview.docx.notZip'));
  if (inflated > MAX_INFLATED) throw new Error(t('editor.preview.docx.tooBig', { max: fSize(MAX_INFLATED) }));
  const buffer = bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer;
  const { value } = await mammoth.convertToHtml({ arrayBuffer: buffer });
  return cleanHtml(value);
}
