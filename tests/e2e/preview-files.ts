// The files the e2e of the previews (ESC-473) open, made by the test: a PNG, a PDF of three pages, a Word file (titles, a
// list, a table, an image, a link), and the trapped ones: an SVG and a Markdown file with scripts and images of the web.

import zlib from 'node:zlib';

const CRC = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});

function crc32(b: Buffer): number {
  let c = 0xffffffff;
  for (const x of b) c = CRC[(c ^ x) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

/** A PNG of `w` × `h` pixels: a gradient, its top left quarter transparent. */
export function png(w: number, h: number): Buffer {
  const chunk = (type: string, data: Buffer) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const td = Buffer.concat([Buffer.from(type, 'ascii'), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(td));
    return Buffer.concat([len, td, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(w, 0);
  ihdr.writeUInt32BE(h, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  const raw = Buffer.alloc((w * 4 + 1) * h);
  for (let y = 0; y < h; y++) {
    raw[y * (w * 4 + 1)] = 0;
    for (let x = 0; x < w; x++) {
      const at = y * (w * 4 + 1) + 1 + x * 4;
      raw[at] = Math.round((x / w) * 255);
      raw[at + 1] = 120;
      raw[at + 2] = Math.round((y / h) * 255);
      raw[at + 3] = x < w / 2 && y < h / 2 ? 0 : 255;
    }
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', zlib.deflateSync(raw)),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

/**
 * A PDF of `pages` pages, each saying « Page n — Escouade aperçu » in Helvetica (a font it does not embed), its
 * catalogue with a JavaScript action at its opening (which no preview runs).
 */
export function pdf(pages: number): Buffer {
  const objs: string[] = [];
  const kids = Array.from({ length: pages }, (_, i) => `${4 + i * 2} 0 R`).join(' ');
  objs.push('<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /JavaScript /JS (app.alert\\("piege"\\)) >> >>');
  objs.push(`<< /Type /Pages /Kids [${kids}] /Count ${pages} >>`);
  objs.push('<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>');
  for (let i = 0; i < pages; i++) {
    const text = `BT /F1 28 Tf 72 700 Td (Page ${i + 1} - Escouade apercu) Tj ET`;
    objs.push(`<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 3 0 R >> >> /Contents ${5 + i * 2} 0 R >>`);
    objs.push(`<< /Length ${text.length} >>\nstream\n${text}\nendstream`);
  }
  let out = '%PDF-1.7\n';
  const offsets: number[] = [];
  objs.forEach((o, i) => {
    offsets.push(Buffer.byteLength(out, 'latin1'));
    out += `${i + 1} 0 obj\n${o}\nendobj\n`;
  });
  const xref = Buffer.byteLength(out, 'latin1');
  out += `xref\n0 ${objs.length + 1}\n0000000000 65535 f \n`;
  for (const off of offsets) out += `${String(off).padStart(10, '0')} 00000 n \n`;
  out += `trailer\n<< /Size ${objs.length + 1} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return Buffer.from(out, 'latin1');
}

/** A zip of `files`, stored (not compressed), as Word reads one. */
export function zip(files: Record<string, string | Buffer>): Buffer {
  const locals: Buffer[] = [];
  const central: Buffer[] = [];
  let offset = 0;
  for (const [name, content] of Object.entries(files)) {
    const data = Buffer.isBuffer(content) ? content : Buffer.from(content, 'utf8');
    const n = Buffer.from(name, 'utf8');
    const crc = crc32(data);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4);
    local.writeUInt32LE(crc, 14);
    local.writeUInt32LE(data.length, 18);
    local.writeUInt32LE(data.length, 22);
    local.writeUInt16LE(n.length, 26);
    locals.push(local, n, data);
    const c = Buffer.alloc(46);
    c.writeUInt32LE(0x02014b50, 0);
    c.writeUInt16LE(20, 4);
    c.writeUInt16LE(20, 6);
    c.writeUInt32LE(crc, 16);
    c.writeUInt32LE(data.length, 20);
    c.writeUInt32LE(data.length, 24);
    c.writeUInt16LE(n.length, 28);
    c.writeUInt32LE(offset, 42);
    central.push(c, n);
    offset += 30 + n.length + data.length;
  }
  const dir = Buffer.concat(central);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(Object.keys(files).length, 8);
  end.writeUInt16LE(Object.keys(files).length, 10);
  end.writeUInt32LE(dir.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, dir, end]);
}

const W = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"';
const R = 'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"';

/** A Word file: a title, a paragraph with a link of the web and a trapped one, a list, a table, an image. */
export function docx(): Buffer {
  const p = (text: string, style?: string) =>
    `<w:p>${style ? `<w:pPr><w:pStyle w:val="${style}"/></w:pPr>` : ''}<w:r><w:t xml:space="preserve">${text}</w:t></w:r></w:p>`;
  const li = (text: string) =>
    `<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>${text}</w:t></w:r></w:p>`;
  const cell = (text: string) => `<w:tc><w:p><w:r><w:t>${text}</w:t></w:r></w:p></w:tc>`;
  const image =
    '<w:p><w:r><w:drawing><wp:inline xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">' +
    '<wp:extent cx="1905000" cy="1428750"/><wp:docPr id="1" name="Schéma" descr="Le schéma"/>' +
    '<a:graphic xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">' +
    '<pic:pic xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:nvPicPr><pic:cNvPr id="0" name="image1.png"/><pic:cNvPicPr/></pic:nvPicPr>' +
    '<pic:blipFill><a:blip r:embed="rImg"/></pic:blipFill><pic:spPr/></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>';
  const body =
    p('Rapport d’essai', 'Heading1') +
    `<w:p><w:r><w:t xml:space="preserve">Voir </w:t></w:r><w:hyperlink r:id="rWeb"><w:r><w:t>le site</w:t></w:r></w:hyperlink>` +
    `<w:r><w:t xml:space="preserve"> et </w:t></w:r><w:hyperlink r:id="rJs"><w:r><w:t>le lien piégé</w:t></w:r></w:hyperlink>` +
    `<w:r><w:t xml:space="preserve"> &lt;script&gt;document.title='pwned'&lt;/script&gt;</w:t></w:r></w:p>` +
    p('Les étapes', 'Heading2') +
    li('Première étape') +
    li('Deuxième étape') +
    `<w:tbl><w:tr>${cell('Nom')}${cell('Valeur')}</w:tr><w:tr>${cell('pages')}${cell('3')}</w:tr></w:tbl>` +
    image;
  return zip({
    '[Content_Types].xml':
      '<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">' +
      '<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/>' +
      '<Default Extension="png" ContentType="image/png"/>' +
      '<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>',
    '_rels/.rels':
      '<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">' +
      '<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>',
    'word/document.xml': `<?xml version="1.0" encoding="UTF-8"?><w:document ${W} ${R}><w:body>${body}</w:body></w:document>`,
    'word/_rels/document.xml.rels':
      '<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">' +
      '<Relationship Id="rStyles" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>' +
      '<Relationship Id="rNum" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/>' +
      '<Relationship Id="rImg" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/image1.png"/>' +
      '<Relationship Id="rWeb" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.invalid/" TargetMode="External"/>' +
      '<Relationship Id="rJs" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="javascript:document.title=%27pwned%27" TargetMode="External"/>' +
      '</Relationships>',
    'word/styles.xml':
      `<?xml version="1.0" encoding="UTF-8"?><w:styles ${W}>` +
      '<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/></w:style>' +
      '<w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/></w:style></w:styles>',
    'word/numbering.xml':
      `<?xml version="1.0" encoding="UTF-8"?><w:numbering ${W}><w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:numFmt w:val="bullet"/></w:lvl></w:abstractNum>` +
      '<w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num></w:numbering>',
    'word/media/image1.png': png(40, 30),
  });
}

/**
 * An SVG with what would run in a page (a script, an event, a link of the web) and an image of the web: shown as an
 * image, none of them does anything.
 */
export const TRAPPED_SVG =
  '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="120" height="80" viewBox="0 0 120 80" onload="document.title=\'pwned-svg\'">' +
  '<script>document.title = "pwned-svg"; window.top.__pwned = true;</script>' +
  '<rect width="120" height="80" fill="#4a7"/><circle cx="60" cy="40" r="24" fill="#fff"/>' +
  '<image href="https://tracker.invalid/svg-pixel.png" width="1" height="1"/>' +
  '<a xlink:href="javascript:alert(1)"><text x="10" y="70">x</text></a></svg>\n';

/** A Markdown file with its traps: scripts, an event, an image of the web, a style over the app; and its good parts. */
export const TRAPPED_MD = [
  '# Guide de l’essai',
  '',
  'Un paragraphe avec un [lien vers le code](../src/app.ts#L2) et un [lien du web](https://example.invalid/page).',
  '',
  '![Le pixel de suivi](https://tracker.invalid/md-pixel.png)',
  '',
  '![Le logo local](../assets/photo.png)',
  '',
  '<script>document.title = "pwned-md"; window.__pwned = true;</script>',
  '',
  '<img src="x.png" onerror="document.title=\'pwned-md\'">',
  '',
  '<style>body { display: none !important; }</style>',
  '',
  '<div style="position:fixed;inset:0;background:red">recouvre</div>',
  '',
  '## La suite',
  '',
  '- un',
  '- deux',
  '',
  '| a | b |',
  '|---|---|',
  '| 1 | 2 |',
  '',
  '```ts',
  'const x = 1;',
  '```',
  '',
  ...Array.from({ length: 80 }, (_, i) => `Ligne de remplissage ${i + 1}.\n`),
  '## Fin',
  '',
  'Le bas du document.',
  '',
].join('\n');

/** A WAV of `seconds` seconds of a 440 Hz tone, 8 kHz, 16 bits, mono. */
export function wav(seconds: number): Buffer {
  const rate = 8000;
  const n = rate * seconds;
  const data = Buffer.alloc(n * 2);
  for (let i = 0; i < n; i++) data.writeInt16LE(Math.round(Math.sin((2 * Math.PI * 440 * i) / rate) * 8000), i * 2);
  const head = Buffer.alloc(44);
  head.write('RIFF', 0, 'ascii');
  head.writeUInt32LE(36 + data.length, 4);
  head.write('WAVEfmt ', 8, 'ascii');
  head.writeUInt32LE(16, 16);
  head.writeUInt16LE(1, 20);
  head.writeUInt16LE(1, 22);
  head.writeUInt32LE(rate, 24);
  head.writeUInt32LE(rate * 2, 28);
  head.writeUInt16LE(2, 32);
  head.writeUInt16LE(16, 34);
  head.write('data', 36, 'ascii');
  head.writeUInt32LE(data.length, 40);
  return Buffer.concat([head, data]);
}
