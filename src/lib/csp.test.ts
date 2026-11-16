// The content security policy of the window (`src-tauri/tauri.conf.json`), SEC-05 of the 2.2.0 audit: beside what it
// already refused (inline scripts, `eval`, what comes from elsewhere), no form sends anything, no `<base>` moves the
// page's addresses, no plugin of the old kind (`<object>`, `<embed>`) runs, and no page puts the window in a frame.
// Inline styles stay allowed: the interface and the VS Code view write them (another work, docs/TODO-2.3.md).

import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';

const ROOT = path.resolve(import.meta.dirname, '..', '..');
const conf = JSON.parse(fs.readFileSync(path.join(ROOT, 'src-tauri/tauri.conf.json'), 'utf8')) as {
  app: { security: { csp: string } };
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

  it('lets the previews of the VS Code view play sound and video from the core’s bytes only, and load no image or media of the web (ESC-473)', () => {
    expect(directives.get('media-src')).toEqual(["'self'", 'blob:']);
    for (const name of ['img-src', 'media-src', 'connect-src', 'default-src'])
      expect(
        directives.get(name)!.filter((v) => /^(https?:|\*$)/.test(v) && !v.endsWith('.localhost')),
        name,
      ).toEqual([]);
  });

  it('still refuses inline scripts and eval', () => {
    const scripts = directives.get('script-src')!;
    expect(scripts).not.toContain("'unsafe-inline'");
    expect(scripts).not.toContain("'unsafe-eval'");
  });
});
