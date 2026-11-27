// The templates of the agents' pages in their shelter, in the real app (ESC-674, docs/AGENT-PAGES.md § 7): `review`,
// `diagram` (the embedded Mermaid, `securityLevel: 'strict'`, under the shelter's guard that refuses code made from text)
// and `compare`, each rendered by the core from data whose every text is a trap, opened by a debug build's command (the MCP
// tool comes with AP-2), at the width of a page beside the conversation at 1440 and at 1024, in French and English, dark and
// light. What each drew is read in Escouade's world (the read bridge, ESC-668): the trap stays text, the anchors are there.
// Captures of each go to the test's output folder, and to ESCOUADE_CAPTURES when it is set.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import type { Page } from '@playwright/test';
import { expect, test } from './fixture';

type Invoke = (cmd: string, args?: unknown) => Promise<unknown>;

async function call<T>(page: Page, cmd: string, args: unknown): Promise<T> {
  await page.waitForFunction(() => '__TAURI_INTERNALS__' in window, null, { timeout: 30_000 });
  return page.evaluate(([c, a]) => (window as unknown as { __TAURI_INTERNALS__: { invoke: Invoke } }).__TAURI_INTERNALS__.invoke(c, a), [
    cmd,
    args,
  ] as const) as Promise<T>;
}

const TRAP = '<img src=x onerror=alert(1)><b>piège</b>';
const shot = (name: string) =>
  fs.readFileSync(path.join(path.dirname(fileURLToPath(import.meta.url)), '../../design/agent-pages/captures', name)).toString('base64');

function review(lang: string) {
  const fr = lang === 'fr';
  return {
    subject: { ticket: 'POR-41', branch: 'ticket/por-41', target: 'main', loop: '2/5' },
    facts: { files: 3, added: 214, removed: 38, tests: { passed: 48, total: 48 } },
    criteria: [
      {
        text: fr ? 'La liste montre 25 adhérents par page' : 'The list shows 25 members a page',
        state: 'met',
        md: 'Vérifié par `users.spec.ts`.',
      },
      { text: TRAP, state: 'met', md: `**${TRAP}**` },
      {
        text: fr ? 'Le filtre et le tri gardés' : 'The filter and the sort kept',
        state: 'partial',
        md: 'Le filtre est dans l’adresse (`?q=`) ; le **tri** revient à « Nom ».',
      },
    ],
    changes: [
      {
        path: 'src/routes/adherents/+page.svelte',
        added: 96,
        removed: 21,
        diff: '@@ -1,3 +1,4 @@\n import { page } from "$app/state";\n-let rows = all;\n+let rows = $derived(slice(all, n));\n+let n = $state(25);',
      },
      { path: 'src/lib/pagination.ts', added: 58, status: 'added' },
      { path: `tests/${TRAP}.spec.ts`, added: 44, removed: 9 },
    ],
    risks: [{ md: fr ? 'Le tri n’est pas gardé dans l’adresse.' : 'The sort is not kept in the address.', level: 'medium' }],
    sections: [{ id: 'perf', title: fr ? 'Performances' : 'Performance', md: TRAP }],
  };
}

const diagram = {
  kind: 'mermaid',
  source: `flowchart LR\n  A[Agent] -->|show_page| B{Escouade}\n  B --> C["${TRAP.replace(/"/g, '#quot;')}"]\n  B --> D[À toi]`,
  md: `La *légende* et ${TRAP}`,
};

const compare = {
  options: [
    {
      id: 'a',
      title: 'Les repères',
      md: 'Côte à côte, à parts égales.',
      images: ['shots/a.png'],
      pros: ['Le pilotage se lit'],
      cons: ['Étroit à 1024'],
    },
    { id: 'b', title: TRAP, images: [{ dark: 'shots/b.png', light: 'shots/b-clair.png' }], pros: [TRAP] },
    { id: 'c', title: 'L’onglet', md: 'Dans le panneau de droite.' },
  ],
  criteria: ['La conversation reste entière', TRAP],
  recommend: 'a',
};
const images: [string, string][] = [
  ['shots/a.png', shot('a-revue.png')],
  ['shots/b.png', shot('b-revue.png')],
  ['shots/b-clair.png', shot('b-revue-clair.png')],
];
const choice = {
  key: 'a-31',
  title: 'Fusionner POR-41 ?',
  options: [
    { id: 'a', label: 'Fusionner dans main', description: 'Le tri devient un ticket à part.' },
    { id: 'b', label: 'Renvoyer à pagination', description: 'Garder aussi le tri dans l’adresse.' },
    { id: 'c', label: 'Attendre' },
  ],
  recommend: 'a',
};

// The page's place beside the conversation, direction A: at 1440 × 900, then at 1024 × 728.
const sizes = [
  { name: '1440', bounds: { x: 60, y: 40, width: 540, height: 760 } },
  { name: '1024', bounds: { x: 60, y: 40, width: 380, height: 600 } },
];

test('the templates are drawn by the core in their shelter: the agent’s text stays text, Mermaid draws without network', async ({
  app,
}) => {
  test.skip(process.platform !== 'win32', 'WebView2: the read bridge and the captures of the end-to-end test');
  test.setTimeout(600_000);
  const { page } = app;
  await page.setViewportSize({ width: 1440, height: 900 }).catch(() => {});
  const out = process.env.ESCOUADE_CAPTURES ?? test.info().outputPath('captures');
  fs.mkdirSync(out, { recursive: true });
  const read = (p: string, selector: string | null, text = false) => call<string>(page, 'agentpage_read', { page: p, selector, text });

  let n = 1;
  for (const size of sizes)
    for (const lang of ['fr', 'en'])
      for (const theme of ['dark', 'light']) {
        for (const template of ['review', 'diagram', 'compare'] as const) {
          const p = `p-${++n}`;
          const data = template === 'review' ? review(lang) : template === 'diagram' ? diagram : compare;
          const anchors = await call<{ key: string; selector: string }[]>(page, 'agentpage_open_template', {
            page: p,
            template,
            title: `${TRAP} · POR-41`,
            data,
            lang,
            theme,
            choice: template === 'diagram' ? null : choice,
            images: template === 'compare' ? images : null,
            bounds: size.bounds,
          });
          const where = `${template} ${lang} ${theme} ${size.name}`;
          // Drawn, with its script: the page's own world says so in its attributes.
          await expect
            .poll(() => read(p, 'html', false).catch(() => ''), { timeout: 30_000, message: where })
            .toContain(`data-theme="${theme}"`);
          const html = await read(p, 'main');
          const text = await read(p, 'main', true);
          // The trap is text, written out, never an element.
          expect(html, where).not.toMatch(/<img src="?x/);
          expect(html, where).not.toContain('<b>piège</b>');
          if (template !== 'diagram') expect(text, where).toContain(TRAP);
          expect(await read(p, 'title', true), where).toContain(TRAP);
          // The words of the language.
          if (template === 'review') {
            expect(text, where).toContain(lang === 'fr' ? "Critères d'acceptation" : 'Acceptance criteria');
            expect(text, where).toContain(lang === 'fr' ? 'Ton verdict' : 'Your verdict');
            expect(text, where).toContain(lang === 'fr' ? 'À moitié' : 'Partly');
            for (const key of ['criteria.2', 'changes.0', 'risks.0', 'sections.perf', 'verdict.a'])
              expect(
                anchors.map((a) => a.key),
                where,
              ).toContain(key);
            expect(await read(p, '#crit-3', true), where).toContain(
              lang === 'fr' ? 'Le filtre et le tri gardés' : 'The filter and the sort kept',
            );
            expect(await read(p, '#option-a'), where).toContain(lang === 'fr' ? 'Recommandé' : 'Recommended');
            expect(await read(p, '#change-1'), where).toContain('<span class="add">+let n = $state(25);</span>');
          }
          if (template === 'diagram') {
            // Mermaid drew it, in strict mode: its nodes carry their ids from the source, the trap is a label's text.
            await expect
              .poll(async () => (await read(p, '#diagram')).match(/data-state="(drawn|failed)"/)?.[1] ?? '', {
                timeout: 30_000,
                message: where,
              })
              .toBe('drawn');
            const svg = await read(p, '#diagram svg');
            expect(svg, where).toContain('<svg');
            for (const node of ['A', 'B', 'C', 'D']) expect(svg, `${where} node ${node}`).toContain(`data-node="${node}"`);
            expect(svg, where).not.toContain('<foreignObject');
            // Strict: a label's markup is taken away or shown, never run.
            expect(svg, where).not.toMatch(/<img[\s>]/);
            expect(svg, where).not.toMatch(/\sonerror=/);
            expect(await read(p, '#legend', true), where).toContain(TRAP);
          }
          if (template === 'compare') {
            expect(
              anchors.map((a) => a.key),
              where,
            ).toEqual(expect.arrayContaining(['options.a', 'options.1', 'criteria.1']));
            const plate = await read(p, '#option-b');
            expect(plate, where).toContain(`data-theme-shown="${theme}"`);
            expect(await read(p, '#option-a'), where).toContain('class="seal"');
            // Its images are served from its version, and drawn.
            expect(await read(p, '#option-a .shot img')).toContain('src="shots/a.png"');
          }
          await page.waitForTimeout(template === 'compare' ? 1200 : 400);
          await call(page, 'agentpage_capture', { page: p, path: path.join(out, `${template}-${lang}-${theme}-${size.name}`) });
          await call(page, 'agentpage_close', { page: p });
        }
      }
  test.info().annotations.push({ type: 'captures', description: out });
});

test('data out of its schema is refused by the core with every problem, nothing opens', async ({ app }) => {
  const { page } = app;
  const refused = await call<string>(page, 'agentpage_open_template', {
    page: 'p-90',
    template: 'review',
    title: 'Revue',
    data: { subject: { ticket: 'POR 41' }, criteria: [{ text: 'x', state: 'done' }], verdict: 'fusionner', html: '<script>' },
    lang: 'en',
    theme: 'dark',
    choice: null,
    images: null,
    bounds: null,
  }).catch((e: unknown) => String(e));
  expect(refused).toContain('subject.ticket');
  expect(refused).toContain('criteria.0.state: one of: met, partial, unmet, unknown');
  expect(refused).toContain('verdict: unknown field');
  expect(refused).toContain('html: unknown field');
  expect(await call(page, 'agentpage_view', { page: 'p-90' })).toBeNull();
});
