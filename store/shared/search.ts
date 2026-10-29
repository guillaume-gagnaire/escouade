// What the search compares and marks, on the server (server/utils/catalog.ts) and in the page that shows its results
// (the words found, marked): a text folded, accents and case aside.

/** A text without its accents, in lower case, its dashes and dots as spaces: what the search compares. */
export const fold = (s: string) =>
  s
    .normalize('NFD')
    .replace(/\p{M}/gu, '')
    .toLowerCase()
    .replace(/[-_.’']/g, ' ');

/** The folded words of a query. */
export const words = (s: string) => fold(s).split(/\s+/).filter(Boolean);

/**
 * The parts of `text` the query's words match, accents and case aside, to mark them (`<mark>`): the text cut in
 * pieces, every other one matched.
 */
export function highlight(text: string, q: string): string[] {
  const ws = words(q).filter((w) => w.length > 1);
  if (!ws.length) return [text];
  // Each character folded alone keeps the positions of the original.
  const chars = [...text];
  const folded = chars.map((c) => fold(c) || ' ');
  const flat = folded.join('');
  const starts: number[] = [];
  let pos = 0;
  for (const f of folded) {
    starts.push(pos);
    pos += f.length;
  }
  const marked = new Array(chars.length).fill(false);
  for (const w of ws) {
    let at = flat.indexOf(w);
    while (at >= 0) {
      for (let i = 0; i < chars.length; i++) if (starts[i]! >= at && starts[i]! < at + w.length) marked[i] = true;
      at = flat.indexOf(w, at + w.length);
    }
  }
  const out: string[] = [''];
  let on = false;
  chars.forEach((c, i) => {
    if (marked[i] !== on) {
      out.push('');
      on = marked[i];
    }
    out[out.length - 1] += c;
  });
  return out;
}
