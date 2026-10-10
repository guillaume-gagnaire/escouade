// What the site's tests share.

/** Every text of a catalog (or any tree of objects, lists and texts) by its path: `hero.line1`, `faq.items[2].q`… */
export function leaves(value: unknown, path = ''): Map<string, string> {
  const out = new Map<string, string>();
  if (typeof value === 'string') out.set(path, value);
  else if (Array.isArray(value)) value.forEach((v, i) => leaves(v, `${path}[${i}]`).forEach((s, p) => out.set(p, s)));
  else if (value && typeof value === 'object') {
    for (const [k, v] of Object.entries(value)) leaves(v, path ? `${path}.${k}` : k).forEach((s, p) => out.set(p, s));
  } else throw new Error(`${path}: neither a text, a list nor an object`);
  return out;
}

/** Text as Vue writes it in HTML. */
export const escapeText = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

/** What an HTML attribute says, with its entities decoded. */
export const unescapeAttr = (s: string) =>
  s
    .replace(/&quot;/g, '"')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&amp;/g, '&');

/** The attributes of each `<name …>` tag of `html`, by attribute name. */
export function tags(html: string, name: string): Record<string, string>[] {
  return [...html.matchAll(new RegExp(`<${name}(?:\\s[^>]*)?>`, 'g'))].map((m) =>
    Object.fromEntries([...m[0].matchAll(/([\w:-]+)="([^"]*)"/g)].map((a) => [a[1], unescapeAttr(a[2])])),
  );
}

/** The `<head>` of a page. */
export const headOf = (html: string) => html.slice(html.indexOf('<head'), html.indexOf('</head>'));
