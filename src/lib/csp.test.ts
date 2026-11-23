// The content security policy of the window (`src-tauri/tauri.conf.json`), SEC-05 of the 2.2.0 audit: beside what it
// already refused (inline scripts, `eval`, what comes from elsewhere), no form sends anything, no `<base>` moves the
// page's addresses, no plugin of the old kind (`<object>`, `<embed>`) runs, and no page puts the window in a frame. A
// `<style>` element applies only with the nonce of the page (ESC-593, lib/style-nonce.ts); the `style` attributes Svelte
// writes stay allowed. tests/e2e/csp.spec.ts reads the policy the real window has.

import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const conf = JSON.parse(fs.readFileSync(path.join(ROOT, 'src-tauri/tauri.conf.json'), 'utf8')) as {
  app: { security: { csp: string; devCsp?: string; dangerousDisableAssetCspModification?: unknown } };
};
const directives = new Map(
  conf.app.security.csp.split(';').map((d) => {
    const [name, ...values] = d.trim().split(/\s+/);
    return [name!, values] as const;
  }),
);

describe('the content security policy of the window', () => {
  it('closes forms, the base address, objects and frames around it (SEC-05)', () => {
    for (const name of ['form-action', 'base-uri', 'object-src', 'frame-ancestors']) expect(directives.get(name), name).toEqual(["'none'"]);
  });

  it('applies a <style> element only with the nonce Tauri adds, and keeps the style attributes (SEC-05, ESC-593)', () => {
    expect(directives.get('style-src')).toEqual(["'self'"]);
    expect(directives.get('style-src-attr')).toEqual(["'unsafe-inline'"]);
    // A `style-src-elem` would take the place of `style-src`, where Tauri writes the nonce: there is none.
    expect(directives.has('style-src-elem')).toBe(false);
    // And Tauri is left to write it.
    expect(conf.app.security.dangerousDisableAssetCspModification).toBeUndefined();
    expect(conf.app.security.devCsp).toBeUndefined();
  });

  it('gives index.html the <style> that carries the nonce', () => {
    const html = fs.readFileSync(path.join(ROOT, 'index.html'), 'utf8');
    expect(html).toMatch(/<style id="escouade-style-nonce"><\/style>/);
  });

  it('lets the previews of the VS Code view play sound and video from the core’s bytes only, and load no image or media of the web (ESC-473)', () => {
    expect(directives.get('media-src')).toEqual(["'self'", 'blob:']);
    for (const name of ['img-src', 'media-src', 'connect-src', 'default-src'])
      expect(
        directives.get(name)!.filter((v) => /^(https?:|\*$)/.test(v) && !v.endsWith('.localhost')),
        name,
      ).toEqual([]);
  });

  it('lets the window reach a server’s tunnel on this computer’s loopback, nowhere else (ESC-537)', () => {
    const sockets = directives.get('connect-src')!.filter((v) => v.startsWith('ws'));
    expect(sockets).toEqual(['ws://127.0.0.1:*']);
  });

  it('still refuses inline scripts and eval', () => {
    const scripts = directives.get('script-src')!;
    expect(scripts).not.toContain("'unsafe-inline'");
    expect(scripts).not.toContain("'unsafe-eval'");
  });
});
