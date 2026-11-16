import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { hashOf, inlineScripts, policyOf, withPolicy } from '../csp';
import { headOf, tags } from './helpers';

const OUT = fileURLToPath(new URL('../.output/public/', import.meta.url));

/** Every page the generation wrote, by its path under the output. */
function pages(dir = OUT): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
    e.isDirectory() ? pages(join(dir, e.name)) : e.name.endsWith('.html') ? [join(dir, e.name)] : [],
  );
}

/** The policy a page's `<meta>` gives, by directive. */
function policy(html: string): Map<string, string[]> {
  const meta = tags(headOf(html), 'meta').filter((m) => m['http-equiv'] === 'Content-Security-Policy');
  expect(meta).toHaveLength(1);
  return new Map(
    meta[0]!.content!.split(';').map((d) => {
      const [name, ...values] = d.trim().split(/\s+/);
      return [name!, values];
    }),
  );
}

describe('the content security policy of a page (SEC-16)', () => {
  it('names its own inline scripts by their hashes, and nothing else runs inline', () => {
    const html =
      '<html><head><meta charset="utf-8"><script type="importmap">{"imports":{}}</script>' +
      '<script>window.a=1</script><script type="module" src="/_nuxt/x.js"></script>' +
      '<script type="application/json" id="__NUXT_DATA__">[1]</script></head><body></body></html>';
    expect(inlineScripts(html)).toEqual(['{"imports":{}}', 'window.a=1']);
    const out = withPolicy(html);
    // First in the head, before the scripts it names.
    expect(out.indexOf('Content-Security-Policy')).toBeLessThan(out.indexOf('<script'));
    const p = policy(out);
    expect(p.get('script-src')).toEqual(["'self'", hashOf('{"imports":{}}'), hashOf('window.a=1')]);
    expect(p.get('script-src')).not.toContain("'unsafe-inline'");
    expect(p.get('default-src')).toEqual(["'self'"]);
    expect(p.get('object-src')).toEqual(["'none'"]);
    expect(p.get('base-uri')).toEqual(["'self'"]);
    expect(policyOf([])).toContain("script-src 'self';");
  });

  it('is in every generated page, and covers each of its inline scripts', () => {
    const all = pages();
    expect(all.length).toBeGreaterThan(100);
    for (const file of all) {
      const html = readFileSync(file, 'utf8');
      const p = policy(html);
      const allowed = p.get('script-src')!;
      for (const script of inlineScripts(html)) expect(allowed, `${file}: ${script.slice(0, 60)}`).toContain(hashOf(script));
      // Nothing of another host: the site loads its own files alone.
      for (const [directive, values] of p)
        for (const v of values) expect(v.startsWith("'") || v === 'data:', `${file}: ${directive} ${v}`).toBe(true);
    }
    // Every page of the site, read cold right after `nuxt generate`: past 5 s on a loaded Windows machine.
  }, 30_000);
});
