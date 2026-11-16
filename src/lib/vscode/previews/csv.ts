// A CSV or TSV file as a table (ESC-473): its rows, read as RFC 4180 says (quoted fields, `""` for a quote, line
// breaks in a quoted field), its separator guessed from its first line when it is not a `.tsv` (`,`, `;` or a tab).

/** The separator of a table: a tab for a `.tsv`, else the most frequent of `,`, `;` and tab on its first line. */
export function separatorOf(text: string, tsv: boolean): string {
  if (tsv) return '\t';
  const first = text.split(/\r?\n/, 1)[0] ?? '';
  let best = ',';
  let most = -1;
  for (const c of [',', ';', '\t']) {
    const n = first.split(c).length - 1;
    if (n > most) {
      most = n;
      best = c;
    }
  }
  return best;
}

/** The rows of `text`, at most `max` (the rest is counted: `more`). */
export function parseTable(text: string, sep: string, max = 5000): { rows: string[][]; more: number } {
  const rows: string[][] = [];
  let row: string[] = [];
  let field = '';
  let quoted = false;
  let more = 0;
  const end = () => {
    row.push(field);
    field = '';
  };
  const push = () => {
    end();
    if (rows.length < max) rows.push(row);
    else more++;
    row = [];
  };
  const body = text.charCodeAt(0) === 0xfeff ? text.slice(1) : text;
  for (let i = 0; i < body.length; i++) {
    const c = body[i];
    if (quoted) {
      if (c === '"') {
        if (body[i + 1] === '"') {
          field += '"';
          i++;
        } else quoted = false;
      } else field += c;
    } else if (c === '"' && field === '') quoted = true;
    else if (c === sep) end();
    else if (c === '\n' || c === '\r') {
      if (c === '\r' && body[i + 1] === '\n') i++;
      push();
    } else field += c;
  }
  if (field !== '' || row.length) push();
  return { rows, more };
}
