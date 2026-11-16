import { describe, expect, it } from 'vitest';
import { docx, zip } from '../../../../tests/e2e/preview-files';
import { docxHtml, inflatedSize, MAX_INFLATED } from './docx';

const dom = (html: string) => {
  const el = document.createElement('div');
  el.innerHTML = html;
  return el;
};

describe('a Word file as its preview shows it', () => {
  it('keeps its titles, list, table, image and links, sanitized: a trapped link is text', async () => {
    const el = dom(await docxHtml(new Uint8Array(docx())));
    expect(el.querySelector('h1')?.textContent).toBe('Rapport d’essai');
    expect(el.querySelector('h2')?.textContent).toBe('Les étapes');
    expect([...el.querySelectorAll('ul li')].map((li) => li.textContent)).toEqual(['Première étape', 'Deuxième étape']);
    expect(el.querySelectorAll('table tr')).toHaveLength(2);
    expect(el.querySelector('img')?.getAttribute('src')).toMatch(/^data:image\/png;base64,/);
    const links = [...el.querySelectorAll('a')];
    expect(links.find((a) => a.textContent === 'le site')?.getAttribute('href')).toBe('https://example.invalid/');
    expect(links.find((a) => a.textContent === 'le lien piégé')?.getAttribute('href') ?? null).toBeNull();
    expect(el.querySelector('script')).toBeNull();
    expect(el.textContent).toContain("<script>document.title='pwned'</script>");
  });

  it('adds up what a zip inflates to, and refuses a file that is not one, or would inflate past its bound', async () => {
    expect(inflatedSize(new Uint8Array(zip({ a: 'abc', b: Buffer.alloc(10) })))).toBe(13);
    expect(inflatedSize(new Uint8Array([1, 2, 3]))).toBeNull();
    await expect(docxHtml(new Uint8Array(Buffer.from('pas un zip')))).rejects.toThrow(/pas un document Word lisible/);
    // A zip whose directory says more than the bound: not given to mammoth.
    const bomb = zip({ 'word/document.xml': 'x' });
    const at = bomb.lastIndexOf(Buffer.from([0x50, 0x4b, 0x01, 0x02]));
    bomb.writeUInt32LE(MAX_INFLATED + 1, at + 24);
    await expect(docxHtml(new Uint8Array(bomb))).rejects.toThrow(/se décompresse en plus de/);
  });
});
