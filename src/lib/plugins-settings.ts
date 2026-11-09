// Réglages › Plugins (direction A, ESC-384; design/settings-2-2/index.html#a-plugins): what each installed plugin adds and
// where it is set, in one line, and on how many projects it is on. Read from the plugins as the core gives them.
import { t } from './i18n';
import { activeIn, categoriesOf, contributionsOf, type PluginView } from './plugins';
import type { Project } from './types';

/** What the line names, in this order: what the user meets in the app. Settings and secrets have their « Réglages ». */
const SHOWN = [
  'harnesses',
  'ticketSources',
  'forges',
  'themes',
  'launchEnvironments',
  'webhooks',
  'commands',
  'projectTemplates',
  'menus',
  'statusBar',
  'badges',
] as const;

/** What a plugin adds, with where it is set: its two first points, its description when it fills none. */
export function addsOf(p: PluginView, lang: string): string {
  const m = p.manifest;
  if (!m) return p.description;
  const contributes: Record<string, unknown> = {};
  for (const k of SHOWN) if (k in m.contributes) contributes[k] = m.contributes[k];
  // Its accounts alone (a source or a harness already says where they go).
  if (!Object.keys(contributes).length && m.contributes.accounts) contributes.accounts = m.contributes.accounts;
  const out = contributionsOf({ ...m, contributes: { ...contributes } }, lang);
  // A voice service is a plugin that wakes on `voice:<service>` (Deepgram, ElevenLabs…): it is chosen in Voix.
  if (m.activation.some((a) => a.startsWith('voice:'))) out.unshift(t('plugins.settingsPage.voice'));
  return out.slice(0, 2).join(' · ') || p.description;
}

/** A theme reaches the whole app; anything else is on project by project (§3.3). */
export const wholeApp = (p: PluginView) => {
  const cats = categoriesOf(p);
  return !p.manifest?.run && cats.length > 0 && cats.every((c) => c === 'themes');
};

/** « Actif sur N projets sur M », or « Pour toute l'app »; nothing with no project open. */
export function reachOf(p: PluginView, projects: Pick<Project, 'id' | 'private'>[]): string {
  if (wholeApp(p)) return t('plugins.settingsPage.wholeApp');
  if (!projects.length) return '';
  const on = projects.filter((project) => activeIn(p, project)).length;
  return t('plugins.settingsPage.active', { count: on, total: String(projects.length) });
}

/** Whether its page in the view has settings or secrets to enter: « Réglages » opens it there. */
export const hasSettings = (p: PluginView) =>
  ((p.manifest?.contributes.settings as unknown[] | undefined)?.length ?? 0) + p.secretKeys.length > 0;
