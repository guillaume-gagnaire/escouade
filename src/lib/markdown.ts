// Markdown rendering for Claude messages: marked (GFM) → DOMPurify, then lazy shiki highlighting.

import DOMPurify from 'dompurify';
import { Marked, type Tokens } from 'marked';
import { openUrl } from '@tauri-apps/plugin-opener';
import { locale, t } from './i18n';

const escapeHtml = (s: string) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

const marked = new Marked({
  gfm: true,
  breaks: false,
  renderer: {
    // Raw HTML in a reply is shown as text: it could otherwise be styled over the app's own UI.
    html({ text }: Tokens.HTML | Tokens.Tag) {
      return escapeHtml(text);
    },
    code({ text, lang }: Tokens.Code) {
      const l = (lang ?? '').trim().split(/\s+/)[0] ?? '';
      return (
        `<pre data-lang="${escapeHtml(l)}"><div class="code-head"><span>${escapeHtml(l || t('conv.markdown.plainText'))}</span>` +
        `<button class="copy-btn" type="button">${escapeHtml(t('common.copy'))}</button></div><code>${escapeHtml(text)}</code></pre>`
      );
    },
  },
});

DOMPurify.addHook('afterSanitizeAttributes', (node) => {
  if (node.tagName === 'A') node.setAttribute('rel', 'noopener noreferrer');
});

const CACHE_SIZE = 500;
const cached = new Map<string, string>();

/** Renders sanitized HTML. `cache` is off for streaming text, whose intermediate states are never reused. */
export function renderMarkdown(text: string, cache = true): string {
  // The code blocks carry texts (« Copier »): a rendering is reused only in the language it was made in. Reading the
  // language here also makes whoever renders follow a change of it.
  const key = `${locale.ui}:${text}`;
  const hit = cached.get(key);
  if (hit !== undefined) {
    // Least-recently-used order: a hit moves to the end.
    cached.delete(key);
    cached.set(key, hit);
    return hit;
  }
  const raw = marked.parse(text, { async: false }) as string;
  const html = DOMPurify.sanitize(raw, { ADD_ATTR: ['data-lang'], FORBID_TAGS: ['style', 'form', 'input'], FORBID_ATTR: ['style'] });
  if (cache && text.length < 50_000) {
    cached.set(key, html);
    if (cached.size > CACHE_SIZE) cached.delete(cached.keys().next().value!);
  }
  return html;
}

// ---------- syntax highlighting ----------

const LANGS: Record<string, () => Promise<any>> = {
  ts: () => import('shiki/langs/typescript.mjs'),
  typescript: () => import('shiki/langs/typescript.mjs'),
  tsx: () => import('shiki/langs/tsx.mjs'),
  js: () => import('shiki/langs/javascript.mjs'),
  javascript: () => import('shiki/langs/javascript.mjs'),
  jsx: () => import('shiki/langs/jsx.mjs'),
  json: () => import('shiki/langs/json.mjs'),
  jsonc: () => import('shiki/langs/jsonc.mjs'),
  rust: () => import('shiki/langs/rust.mjs'),
  rs: () => import('shiki/langs/rust.mjs'),
  python: () => import('shiki/langs/python.mjs'),
  py: () => import('shiki/langs/python.mjs'),
  bash: () => import('shiki/langs/bash.mjs'),
  sh: () => import('shiki/langs/bash.mjs'),
  shell: () => import('shiki/langs/bash.mjs'),
  zsh: () => import('shiki/langs/bash.mjs'),
  powershell: () => import('shiki/langs/powershell.mjs'),
  ps1: () => import('shiki/langs/powershell.mjs'),
  css: () => import('shiki/langs/css.mjs'),
  scss: () => import('shiki/langs/scss.mjs'),
  html: () => import('shiki/langs/html.mjs'),
  xml: () => import('shiki/langs/xml.mjs'),
  svelte: () => import('shiki/langs/svelte.mjs'),
  vue: () => import('shiki/langs/vue.mjs'),
  markdown: () => import('shiki/langs/markdown.mjs'),
  md: () => import('shiki/langs/markdown.mjs'),
  yaml: () => import('shiki/langs/yaml.mjs'),
  yml: () => import('shiki/langs/yaml.mjs'),
  toml: () => import('shiki/langs/toml.mjs'),
  sql: () => import('shiki/langs/sql.mjs'),
  go: () => import('shiki/langs/go.mjs'),
  java: () => import('shiki/langs/java.mjs'),
  kotlin: () => import('shiki/langs/kotlin.mjs'),
  c: () => import('shiki/langs/c.mjs'),
  cpp: () => import('shiki/langs/cpp.mjs'),
  csharp: () => import('shiki/langs/csharp.mjs'),
  cs: () => import('shiki/langs/csharp.mjs'),
  php: () => import('shiki/langs/php.mjs'),
  ruby: () => import('shiki/langs/ruby.mjs'),
  swift: () => import('shiki/langs/swift.mjs'),
  diff: () => import('shiki/langs/diff.mjs'),
  dockerfile: () => import('shiki/langs/dockerfile.mjs'),
  ini: () => import('shiki/langs/ini.mjs'),
  graphql: () => import('shiki/langs/graphql.mjs'),
  prisma: () => import('shiki/langs/prisma.mjs'),
};

let highlighterP: Promise<any> | null = null;

function highlighter() {
  highlighterP ??= (async () => {
    const [{ createHighlighterCore }, { createJavaScriptRegexEngine }, theme] = await Promise.all([
      import('shiki/core'),
      import('shiki/engine/javascript'),
      import('shiki/themes/vitesse-dark.mjs'),
    ]);
    return createHighlighterCore({ themes: [theme.default], langs: [], engine: createJavaScriptRegexEngine() });
  })();
  return highlighterP;
}

async function highlightCode(code: string, lang: string): Promise<string | null> {
  const loader = LANGS[lang.toLowerCase()];
  if (!loader) return null;
  const hl = await highlighter();
  const mod = await loader();
  const id = (Array.isArray(mod.default) ? mod.default : [mod.default]).at(-1)?.name;
  if (!hl.getLoadedLanguages().includes(id)) await hl.loadLanguage(mod.default);
  return hl.codeToHtml(code, { lang: id, theme: 'vitesse-dark' });
}

/** Highlights the not-yet-highlighted code blocks inside `root`. */
export async function highlightWithin(root: HTMLElement) {
  const blocks = root.querySelectorAll<HTMLPreElement>('pre[data-lang]:not([data-hl])');
  for (const pre of blocks) {
    pre.dataset.hl = '1';
    const code = pre.querySelector('code');
    const lang = pre.dataset.lang ?? '';
    if (!code || !lang) continue;
    try {
      const html = await highlightCode(code.textContent ?? '', lang);
      if (html && code.isConnected) {
        const wrap = document.createElement('div');
        wrap.innerHTML = html;
        const shikiPre = wrap.firstElementChild;
        if (shikiPre) {
          const inner = document.createElement('code');
          inner.className = 'shiki';
          inner.innerHTML = shikiPre.querySelector('code')?.innerHTML ?? '';
          code.replaceWith(inner);
        }
      }
    } catch (e) {
      console.warn('highlight failed', e);
    }
  }
}

/** Delegated click handling for rendered markdown: copy buttons and external links. */
export function handleMarkdownClick(e: MouseEvent) {
  const target = e.target as HTMLElement;
  const copy = target.closest('.copy-btn');
  if (copy) {
    const code = copy.closest('pre')?.querySelector('code');
    if (code) {
      navigator.clipboard.writeText(code.textContent ?? '');
      copy.textContent = t('conv.markdown.copied');
      setTimeout(() => (copy.textContent = t('common.copy')), 1200);
    }
    e.preventDefault();
    return;
  }
  const a = target.closest('a');
  if (a?.getAttribute('href')) {
    e.preventDefault();
    const href = a.getAttribute('href')!;
    if (/^(https?:|mailto:)/i.test(href)) openUrl(href).catch(() => {});
  }
}
