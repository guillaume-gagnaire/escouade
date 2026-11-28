/**
 * The phone's dictation (M9, `docs/MOBILE.md` § 3.5): the system's, on the device when it allows (Android's recognition
 * sheet, iOS's speech recognizer), through the app's plugin. What it heard goes into the field, to read again before it
 * goes; the audio never reaches Escouade's core. The keyboard's own microphone works too, without the app.
 *
 * The photos joined to a message: a picture of the phone, made smaller (2048 px at most, JPEG) so that it goes in the
 * message itself, as an image of the desktop's composer does (`send_message`'s attachments).
 */

import { locale } from '$shared/lib/i18n';
import type { Attachment } from '$shared/lib/types';
import { EscouadeMobile, type EscouadeMobilePlugin } from '@escouade/capacitor-mobile';

type Voice = Pick<EscouadeMobilePlugin, 'dictate' | 'stopDictation' | 'openFile'>;
let plugin: Voice = EscouadeMobile;

/** A test's plugin in place of the app's; the app's again with none. */
export function useVoice(fake: Voice | null): void {
  plugin = fake ?? EscouadeMobile;
}

/** What the system heard, or null (cancelled, nothing heard, no dictation on this phone). */
export async function dictate(): Promise<string | null> {
  try {
    const { text } = await plugin.dictate({ lang: locale.ui === 'en' ? 'en-US' : 'fr-FR' });
    return text.trim() || null;
  } catch {
    return null;
  }
}

export async function stopDictation(): Promise<void> {
  await plugin.stopDictation().catch(() => {});
}

/** A file's bytes opened by the system's app that reads it; rejects with its reason. */
export async function openWithSystem(name: string, mediaType: string, bytes: Uint8Array): Promise<void> {
  let bin = '';
  for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  await plugin.openFile({ name, mediaType, data: btoa(bin) });
}

/** The longest side of a photo once made smaller. */
export const PHOTO_SIDE = 2048;
/** The most a photo weighs in the message (the API's limit for an image is 5 MB, base64 aside). */
export const PHOTO_MAX = 3.5 * 1024 * 1024;

/** A photo of the phone as an attachment of the message, made smaller; null when it cannot be read or stays too heavy. */
export async function photoAttachment(file: File): Promise<(Attachment & { url: string }) | null> {
  let bitmap: ImageBitmap;
  try {
    bitmap = await createImageBitmap(file);
  } catch {
    return null;
  }
  const scale = Math.min(1, PHOTO_SIDE / Math.max(bitmap.width, bitmap.height));
  const canvas = document.createElement('canvas');
  canvas.width = Math.max(1, Math.round(bitmap.width * scale));
  canvas.height = Math.max(1, Math.round(bitmap.height * scale));
  canvas.getContext('2d')?.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
  bitmap.close();
  const url = canvas.toDataURL('image/jpeg', 0.85);
  const data = url.slice(url.indexOf(',') + 1);
  if ((data.length * 3) / 4 > PHOTO_MAX) return null;
  const name = (file.name || 'photo').replace(/\.[^.]+$/, '') + '.jpg';
  return { name, mediaType: 'image/jpeg', data, url };
}
