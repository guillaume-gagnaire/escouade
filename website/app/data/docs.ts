// The documentation: its groups of pages, in the order of the sidebar. A page is a Markdown file per language,
// content/docs/<lang>/<slug>.md (see content/docs/README.md), the pages of a section in its folder (`plugins/manifest`);
// the names of the groups and of the pages in the sidebar are in the catalogs, a page's own title is the first line of
// its file. No page is called `agents`: on a system that ignores the case of file names, coding agents would take
// agents.md for an AGENTS.md of instructions.
import type { Lang } from './language';

export const DOC_GROUPS = [
  { id: 'start', pages: ['introduction', 'installation', 'harness', 'quick-start', 'interface'] },
  {
    id: 'agents',
    pages: [
      'managing-agents',
      'chat',
      'plan',
      'workflows',
      'shared-memory',
      'code-index',
      'overview',
      'notifications',
      'claude-code-view',
      'remote-control',
    ],
  },
  { id: 'code', pages: ['git', 'worktrees', 'branches', 'editor', 'terminals', 'launch'] },
  { id: 'board', pages: ['board', 'autopilot', 'testing', 'integrations', 'shared-board', 'docs-space', 'mcp'] },
  { id: 'usage', pages: ['accounts', 'usage', 'stats'] },
  {
    id: 'settings',
    pages: [
      'settings-application',
      'settings-claude-code',
      'settings-accounts',
      'settings-notifications',
      'settings-projects',
      'settings-board',
      'settings-integrations',
      'settings-terminals',
      'settings-network',
      'settings-voice',
      'settings-about',
    ],
  },
  {
    id: 'plugins',
    pages: [
      'plugins/installer',
      'plugins/manifest',
      'plugins/api',
      'plugins/events',
      'plugins/permission-rules',
      'plugins/themes',
      'plugins/sdk',
      'plugins/testing',
      'plugins/publishing',
    ],
  },
  { id: 'reference', pages: ['shortcuts', 'escouade-yml', 'data', 'updates', 'troubleshooting'] },
] as const;

export type DocGroupId = (typeof DOC_GROUPS)[number]['id'];
export type DocSlug = (typeof DOC_GROUPS)[number]['pages'][number];

/** Every page, in reading order: the sidebar's, from the first group to the last. */
export const DOC_SLUGS: readonly DocSlug[] = DOC_GROUPS.flatMap((g) => [...g.pages]);

export const isDocSlug = (s: string): s is DocSlug => (DOC_SLUGS as readonly string[]).includes(s);

/** Where the documentation's home, or one of its pages, lives in `lang` (a route, without the base URL). */
export const docPath = (lang: Lang, slug?: DocSlug) => `${lang === 'en' ? '/en' : ''}/docs/${slug ? `${slug}/` : ''}`;

/**
 * The route the router takes for a link of a page's content (`url`, resolved), when it leads to another page of the
 * documentation of this site (`origin`, under `base`). Null for any other link, which the browser follows itself:
 * another site, the rest of the site, or a section of the page shown (`current`, a route).
 */
export function docRoute(url: URL, origin: string, base: string, current: string): string | null {
  const root = base.endsWith('/') ? base : `${base}/`;
  if (url.origin !== origin || !url.pathname.startsWith(root)) return null;
  const path = `/${url.pathname.slice(root.length)}`;
  const page = (p: string) => p.replace(/\/$/, '');
  if (!/^\/(?:en\/)?docs\//.test(path) || page(path) === page(current)) return null;
  return path + url.hash;
}

/** The Markdown file of a page, from the website's folder: what the “edit this page” link opens on GitHub. */
export const docSource = (lang: Lang, slug: DocSlug) => `content/docs/${lang}/${slug}.md`;

/**
 * The slug of a route's parameter: one word, or the words of a page of a section (`plugins/manifest`). A catch-all
 * route gives an empty last word for the trailing slash of the site's addresses.
 */
export const slugOf = (param: string | string[] | undefined) =>
  (Array.isArray(param) ? param : [String(param ?? '')]).filter(Boolean).join('/');
