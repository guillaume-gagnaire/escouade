import { describe, expect, it } from 'vitest';
import { attachmentKind, guardFileDrops, readAttachment, sizeLabel } from './attachments';
import { setLang } from './i18n';

const file = (name: string, type = '', body: BlobPart = 'x') => new File([body], name, { type });

describe('attachmentKind', () => {
  it('recognises the images the API reads, PDFs and text files', () => {
    expect(attachmentKind(file('capture.png', 'image/png'))).toBe('image');
    expect(attachmentKind(file('photo.JPG'))).toBe('image');
    expect(attachmentKind(file('rapport.pdf', 'application/pdf'))).toBe('pdf');
    expect(attachmentKind(file('rapport.pdf'))).toBe('pdf');
    expect(attachmentKind(file('data.json', 'application/json'))).toBe('text');
    expect(attachmentKind(file('journal', 'text/plain'))).toBe('text');
    expect(attachmentKind(file('Dockerfile'))).toBe('text');
  });

  it('goes by the extension first, as Windows often gives no type or a wrong one', () => {
    expect(attachmentKind(file('notes.md'))).toBe('text');
    expect(attachmentKind(file('script.ps1'))).toBe('text');
    // The Windows registry maps .ts to MPEG transport streams.
    expect(attachmentKind(file('app.ts', 'video/mp2t'))).toBe('text');
    expect(attachmentKind(file('export.csv', 'application/vnd.ms-excel'))).toBe('text');
  });

  it('refuses the rest', () => {
    expect(attachmentKind(file('plan.docx', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'))).toBeNull();
    expect(attachmentKind(file('sources.zip', 'application/zip'))).toBeNull();
    expect(attachmentKind(file('setup.exe'))).toBeNull();
    expect(attachmentKind(file('scan.bmp', 'image/bmp'))).toBeNull();
    expect(attachmentKind(file('x.constructor'))).toBeNull();
  });
});

describe('readAttachment', () => {
  it('reads a text file as text, whatever its type', async () => {
    const a = await readAttachment(file('notes.md', '', '# Notes\nà faire'));
    expect(a).toMatchObject({ kind: 'text', name: 'notes.md', mediaType: 'text/plain', data: '# Notes\nà faire' });
  });

  it('reads a PDF as base64', async () => {
    const a = await readAttachment(file('rapport.pdf', '', '%PDF-1.4'));
    expect(a).toMatchObject({ kind: 'pdf', name: 'rapport.pdf', mediaType: 'application/pdf', data: btoa('%PDF-1.4') });
  });

  it('keeps an image preview', async () => {
    const a = await readAttachment(file('capture.png', 'image/png', new Uint8Array([137, 80, 78, 71])));
    expect(a.kind).toBe('image');
    expect(a.url).toMatch(/^data:image\/png;base64,/);
    expect(a.data).toBe(a.url!.slice(a.url!.indexOf(',') + 1));
  });

  it('says which files are supported when one is not', async () => {
    await expect(readAttachment(file('sources.zip', 'application/zip'))).rejects.toThrow(/sources\.zip.*images.*PDF.*texte/);
  });

  it('reads the text encodings Windows writes, and refuses binary content', async () => {
    // Excel's CSV exports on a French Windows: Windows-1252.
    const csv = await readAttachment(file('export.csv', '', new Uint8Array([0x63, 0xe9, 0x74, 0xe9, 0x80])));
    expect(csv.data).toBe('cété€');
    // PowerShell 5's Out-File: UTF-16 with its BOM.
    const log = await readAttachment(file('out.txt', '', new Uint8Array([0xff, 0xfe, 0x6f, 0, 0x6b, 0])));
    expect(log.data).toBe('ok');
    await expect(readAttachment(file('binaire.log', '', new Uint8Array([0x61, 0, 0x62])))).rejects.toThrow(/binaire\.log/);
  });

  it('reads as text a file of no known kind that Windows gives no type, when it is text', async () => {
    expect(await readAttachment(file('.env.local', '', 'PORT=3000'))).toMatchObject({ kind: 'text', data: 'PORT=3000' });
    expect(await readAttachment(file('Jenkinsfile', '', 'pipeline {}'))).toMatchObject({ kind: 'text' });
    const binary = file('data.bin', '', new Uint8Array([0x61, 0, 0x62]));
    await expect(readAttachment(binary)).rejects.toThrow(/data\.bin.*images.*PDF.*texte/);
    // Only known text files get the Windows-1252 reading: this is not UTF-8, so not text.
    await expect(readAttachment(file('blob.xyz', '', new Uint8Array([0xe9, 0x80, 0x41])))).rejects.toThrow(/blob\.xyz/);
    // A type from the system is trusted.
    await expect(readAttachment(file('sources.zip', 'application/zip', 'PK'))).rejects.toThrow(/sources\.zip/);
  });

  it('refuses a file over the size limit of its kind', async () => {
    const big = (name: string, kb: number) => {
      const f = file(name);
      Object.defineProperty(f, 'size', { value: kb * 1024 + 1 });
      return f;
    };
    await expect(readAttachment(big('capture.png', 5 * 1024))).rejects.toThrow('capture.png dépasse 5 Mo');
    // Within what a whole message may carry (18 MB, sent in base64 under the API's 32 MB).
    await expect(readAttachment(big('rapport.pdf', 18 * 1024))).rejects.toThrow('rapport.pdf dépasse 18 Mo');
    // More text would fill Claude's context.
    await expect(readAttachment(big('notes.md', 256))).rejects.toThrow('notes.md dépasse 256 Ko');
  });
});

describe('guardFileDrops', () => {
  const drag = (type: string, types: string[]) => {
    const e = new Event(type, { bubbles: true, cancelable: true });
    Object.defineProperty(e, 'dataTransfer', { value: { types, dropEffect: 'copy' } });
    return e as DragEvent;
  };

  it('keeps a file dropped outside the composer from replacing the app', () => {
    const stop = guardFileDrops(window);
    try {
      const over = drag('dragover', ['Files']);
      document.body.dispatchEvent(over);
      expect(over.defaultPrevented).toBe(true);
      expect(over.dataTransfer!.dropEffect).toBe('none');
      const drop = drag('drop', ['Files']);
      document.body.dispatchEvent(drop);
      expect(drop.defaultPrevented).toBe(true);
      // Dragging text into a field keeps working.
      const text = drag('drop', ['text/plain']);
      document.body.dispatchEvent(text);
      expect(text.defaultPrevented).toBe(false);
    } finally {
      stop();
    }
  });

  it('lets a drop zone accept the file', () => {
    const stop = guardFileDrops(window);
    const zone = document.createElement('div');
    document.body.append(zone);
    zone.addEventListener('dragover', (e) => e.preventDefault());
    try {
      const over = drag('dragover', ['Files']);
      zone.dispatchEvent(over);
      expect(over.dataTransfer!.dropEffect).toBe('copy');
    } finally {
      stop();
      zone.remove();
    }
  });
});

describe('the sizes and the refusals in English', () => {
  const big = (name: string, kb: number) => {
    const f = file(name);
    Object.defineProperty(f, 'size', { value: kb * 1024 + 1 });
    return f;
  };

  it('writes a size with the unit of each language', () => {
    expect(sizeLabel(5 * 1024 * 1024)).toBe('5 Mo');
    expect(sizeLabel(256 * 1024)).toBe('256 Ko');
    setLang('en');
    expect(sizeLabel(5 * 1024 * 1024)).toBe('5 MB');
    expect(sizeLabel(256 * 1024)).toBe('256 KB');
  });

  it('says in English why a file cannot be attached', async () => {
    setLang('en');
    await expect(readAttachment(file('sources.zip', 'application/zip', 'PK'))).rejects.toThrow(
      '“sources.zip” can’t be attached. Supported files are images (PNG, JPEG, GIF, WebP), PDFs and text files.',
    );
    await expect(readAttachment(big('capture.png', 5 * 1024))).rejects.toThrow('capture.png is over 5 MB');
    await expect(readAttachment(big('notes.md', 256))).rejects.toThrow('notes.md is over 256 KB');
    await expect(readAttachment(file('binary.log', '', new Uint8Array([0x61, 0, 0x62])))).rejects.toThrow(
      'binary.log isn’t a text file, so it can’t be attached.',
    );
  });
});
