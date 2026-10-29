import sharp from 'sharp';

// The screenshots of a plugin (`store.screenshots`, docs/STORE.md §3.2): PNG, JPEG or WebP, told by their first
// bytes and not by their name, 1600 px wide and 1 MB each at most, 6 at most. Their size is read from their header
// before anything decodes them; then the store re-encodes them in their own format, which drops their metadata
// (place, camera, comments) and whatever a crafted file hides past its image.

export const SCREENSHOTS_MAX = 6;
export const SCREENSHOT_BYTES = 1024 * 1024;
export const SCREENSHOT_WIDTH = 1600;
/** The height a screenshot may have: a tall page stays readable, a strip of a million pixels does not pass. */
export const SCREENSHOT_HEIGHT = 4000;

export type ImageFormat = 'png' | 'jpeg' | 'webp';

export interface ImageInfo {
  format: ImageFormat;
  width: number;
  height: number;
}

/** The format and size of an image, from its header alone; null if it is none of PNG, JPEG or WebP, or its header is cut. */
export function imageInfo(b: Buffer): ImageInfo | null {
  if (b.length >= 24 && b.subarray(0, 8).equals(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]))) {
    if (b.toString('ascii', 12, 16) !== 'IHDR') return null;
    return { format: 'png', width: b.readUInt32BE(16), height: b.readUInt32BE(20) };
  }
  if (b.length >= 4 && b[0] === 0xff && b[1] === 0xd8) {
    // The frame's size is in its SOF segment (C0 to CF, but C4, C8 and CC), after the markers before it.
    let at = 2;
    while (at + 9 < b.length) {
      if (b[at] !== 0xff) return null;
      const marker = b[at + 1]!;
      if (marker === 0xff) {
        at++;
        continue;
      }
      const len = b.readUInt16BE(at + 2);
      if (marker >= 0xc0 && marker <= 0xcf && ![0xc4, 0xc8, 0xcc].includes(marker))
        return { format: 'jpeg', height: b.readUInt16BE(at + 5), width: b.readUInt16BE(at + 7) };
      at += 2 + len;
    }
    return null;
  }
  if (b.length >= 30 && b.toString('ascii', 0, 4) === 'RIFF' && b.toString('ascii', 8, 12) === 'WEBP') {
    const chunk = b.toString('ascii', 12, 16);
    if (chunk === 'VP8 ' && b[23] === 0x9d && b[24] === 0x01 && b[25] === 0x2a)
      return { format: 'webp', width: b.readUInt16LE(26) & 0x3fff, height: b.readUInt16LE(28) & 0x3fff };
    if (chunk === 'VP8L' && b[20] === 0x2f) {
      const bits = b.readUInt32LE(21);
      return { format: 'webp', width: (bits & 0x3fff) + 1, height: ((bits >> 14) & 0x3fff) + 1 };
    }
    if (chunk === 'VP8X') return { format: 'webp', width: b.readUIntLE(24, 3) + 1, height: b.readUIntLE(27, 3) + 1 };
    return null;
  }
  return null;
}

/** Re-encodes a screenshot that passed its checks in its own format, without its metadata. */
export async function reencode(b: Buffer, format: ImageFormat): Promise<Buffer> {
  const image = sharp(b, { limitInputPixels: SCREENSHOT_WIDTH * SCREENSHOT_HEIGHT, failOn: 'error' }).rotate();
  if (format === 'png') return image.png({ compressionLevel: 9 }).toBuffer();
  if (format === 'jpeg') return image.jpeg({ quality: 86, mozjpeg: true }).toBuffer();
  return image.webp({ quality: 86 }).toBuffer();
}
