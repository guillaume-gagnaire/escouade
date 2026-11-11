// The plugins of the first hand a test finds installed when it says nothing of them (ESC-407): those that bring a harness,
// a ticket source, a code host or a voice service, as their manifests in plugins/ declare them. A settings page lists only what an
// installed plugin brings; a test of a plugin uninstalled gives its own `plugins_list`.
import type { Manifest, PluginView } from '../lib/plugins';
import antigravity from '../../plugins/antigravity/escouade-plugin.json';
import azure from '../../plugins/azure-devops/escouade-plugin.json';
import bitbucket from '../../plugins/bitbucket/escouade-plugin.json';
import claude from '../../plugins/claude/escouade-plugin.json';
import clickup from '../../plugins/clickup/escouade-plugin.json';
import codex from '../../plugins/codex/escouade-plugin.json';
import copilot from '../../plugins/copilot/escouade-plugin.json';
import cursor from '../../plugins/cursor/escouade-plugin.json';
import deepgram from '../../plugins/deepgram/escouade-plugin.json';
import elevenlabs from '../../plugins/elevenlabs/escouade-plugin.json';
import gemini from '../../plugins/gemini/escouade-plugin.json';
import gitea from '../../plugins/gitea/escouade-plugin.json';
import github from '../../plugins/github/escouade-plugin.json';
import goose from '../../plugins/goose/escouade-plugin.json';
import gitlab from '../../plugins/gitlab/escouade-plugin.json';
import groq from '../../plugins/groq/escouade-plugin.json';
import jira from '../../plugins/jira/escouade-plugin.json';
import linear from '../../plugins/linear/escouade-plugin.json';
import mistral from '../../plugins/mistral/escouade-plugin.json';
import openai from '../../plugins/openai/escouade-plugin.json';
import opencode from '../../plugins/opencode/escouade-plugin.json';
import trello from '../../plugins/trello/escouade-plugin.json';

/** An installed plugin of the first hand, on, as `plugins_list` gives it. */
export function installedPlugin(manifest: unknown): PluginView {
  // As the file has it: some fields left out (a harness asks no permission).
  const m = manifest as Omit<Manifest, 'permissions' | 'name'> & { name: unknown; permissions?: Partial<Manifest['permissions']> };
  return {
    id: m.id,
    name: typeof m.name === 'string' ? m.name : m.id,
    version: m.version,
    description: '',
    origin: { kind: 'store' } as PluginView['origin'],
    fingerprint: 'sha256:test',
    folder: '',
    declarative: false,
    state: { kind: 'ready' },
    ignored: [],
    manifest: {
      ...m,
      activation: m.activation ?? [],
      contributes: m.contributes ?? {},
      permissions: { events: [], network: [], secrets: [], commands: [], decide: false, ...m.permissions },
    } as Manifest,
    installedAt: '2026-11-11T00:00:00Z',
    choices: { enabled: true, projects: {}, revoked: [], settings: {} },
    asks: [],
    secretKeys: [],
    update: null,
    commands: [],
    commandsApproved: true,
    verified: true,
    withdrawn: null,
    inStore: null,
  };
}

/** The manifests of the harnesses, sources, code hosts and voice services of the first hand. */
export const FIRST_HAND = {
  antigravity,
  azure,
  bitbucket,
  claude,
  clickup,
  codex,
  copilot,
  cursor,
  deepgram,
  elevenlabs,
  gemini,
  gitea,
  github,
  gitlab,
  goose,
  groq,
  jira,
  linear,
  mistral,
  openai,
  opencode,
  trello,
};

/** Every one of them installed; `without` names those uninstalled. */
export function firstHandPlugins(...without: (keyof typeof FIRST_HAND)[]): PluginView[] {
  return Object.entries(FIRST_HAND)
    .filter(([k]) => !without.includes(k as keyof typeof FIRST_HAND))
    .map(([, m]) => installedPlugin(m));
}
