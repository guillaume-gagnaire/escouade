// Model / effort / permission-mode catalogs used by the composer and the settings.

import { t } from './i18n';
import type { ModelInfo } from './types';

/** The aliases offered: Claude Code runs the latest model of the family it knows. */
export const MODELS = [
  { value: 'fable', label: 'Fable' },
  { value: 'opus', label: 'Opus' },
  { value: 'sonnet', label: 'Sonnet' },
  { value: 'haiku', label: 'Haiku' },
];

/** A choice whose label and title are read when shown, so that they follow the language of the interface. */
function choice(value: string, label: () => string, title: () => string): { value: string; label: string; title: string } {
  return {
    value,
    get label() {
      return label();
    },
    get title() {
      return title();
    },
  };
}

export const EFFORTS = [
  choice(
    'low',
    () => t('composer.effort.low.label'),
    () => t('composer.effort.low.title'),
  ),
  choice(
    'medium',
    () => t('composer.effort.medium.label'),
    () => t('composer.effort.medium.title'),
  ),
  choice(
    'high',
    () => t('composer.effort.high.label'),
    () => t('composer.effort.high.title'),
  ),
  choice(
    'xhigh',
    () => t('composer.effort.xhigh.label'),
    () => t('composer.effort.xhigh.title'),
  ),
  choice(
    'max',
    () => t('composer.effort.max.label'),
    () => t('composer.effort.max.title'),
  ),
];

export const MODES = [
  choice(
    'auto',
    () => t('composer.mode.auto.label'),
    () => t('composer.mode.auto.title'),
  ),
  choice(
    'default',
    () => t('composer.mode.default.label'),
    () => t('composer.mode.default.title'),
  ),
  choice(
    'plan',
    () => t('composer.mode.plan.label'),
    () => t('composer.mode.plan.title'),
  ),
  choice(
    'acceptEdits',
    () => t('composer.mode.acceptEdits.label'),
    () => t('composer.mode.acceptEdits.title'),
  ),
  choice(
    'bypassPermissions',
    () => t('composer.mode.bypassPermissions.label'),
    () => t('composer.mode.bypassPermissions.title'),
  ),
];

/**
 * "sonnet" → "Sonnet 5.5" once Claude Code has told which model the alias runs (`catalog`),
 * "claude-sonnet-5" → "Sonnet 5".
 */
export function modelLabel(model: string, catalog: ModelInfo[] = []): string {
  const m = model.toLowerCase();
  const known = MODELS.find((x) => x.value === m);
  if (!known) return displayModel(model);
  const id = resolveAlias(m, catalog);
  return id ? displayModel(id) : known.label;
}

/** The aliases offered, each labelled with the version Claude Code runs for it. */
export function modelOptions(catalog: ModelInfo[]): { value: string; label: string }[] {
  return MODELS.map((m) => ({ value: m.value, label: modelLabel(m.value, catalog) }));
}

/**
 * The full id Claude Code runs for an alias: the one it reports for it, else the newest of the
 * family it lists (it lists some, like Fable, only by their full ids).
 */
function resolveAlias(alias: string, catalog: ModelInfo[]): string | undefined {
  const own = catalog.find((x) => x.value.toLowerCase() === alias && x.resolvedModel);
  if (own) return own.resolvedModel;
  let best: { id: string; major: number; minor: number } | undefined;
  for (const x of catalog) {
    const p = parseModel(x.resolvedModel);
    if (p?.family !== alias || p.major === undefined) continue;
    const minor = p.minor ?? 0;
    if (!best || p.major > best.major || (p.major === best.major && minor > best.minor)) {
      best = { id: x.resolvedModel, major: p.major, minor };
    }
  }
  return best?.id;
}

/** A trailing date (`-20250514`) is no part of the version. */
function parseModel(id: string): { family: string; major?: number; minor?: number } | null {
  const m = id.toLowerCase().match(/(fable|opus|sonnet|haiku)[-_]?(?:(\d{1,2})(?!\d))?(?:[-_.](\d{1,2})(?!\d))?/);
  if (!m) return null;
  return { family: m[1], major: m[2] ? Number(m[2]) : undefined, minor: m[3] ? Number(m[3]) : undefined };
}

/** "claude-opus-5-5" → "Opus 5.5", "claude-haiku-4-5-20251001" → "Haiku 4.5". */
export function displayModel(id: string): string {
  const p = parseModel(id);
  if (!p) return id;
  const name = p.family[0].toUpperCase() + p.family.slice(1);
  if (p.major === undefined) return name;
  return p.minor !== undefined ? `${name} ${p.major}.${p.minor}` : `${name} ${p.major}`;
}

export function supportsEffort(model: string): boolean {
  return !model.toLowerCase().includes('haiku');
}

export function supportsAuto(model: string): boolean {
  return !model.toLowerCase().includes('haiku');
}
