import type { Logo } from '../../shared/marks';
import type { Text } from './checks';

// The plugins the app gives (docs/PLUGINS.md §4.3, INT-4's origin `builtin`): its harnesses, its Kanban sources and its
// code hosts, installed and updated with the app, signed with it. They have a page on the store with the mark
// « Escouade » (decided by the chief of the squad on 2026-10-29), and no version of their own: theirs is the app's.
// Nothing here is downloaded; « Ouvrir dans Escouade » opens their page in the app. Their ids are in the reserve
// `dev.escouade.*`, which the store grants to the app's repository alone (STORE_FIRST_PARTY_REPO, ESC-314): a plugin of
// that repository's `plugins/<id>/` is submitted as anyone's (decision 21), its link to its folder from its `path`, and
// listed as a plugin of the store. What is listed below is what the app gives, nothing more; `source` is the folder of
// one that left the core, `plugins/<short name>/` (docs/PLUGINS.md §8.6.1), which its page links to.

export interface Builtin {
  id: string;
  name: string;
  categories: string[];
  description: Text;
  /** The hosts its program reaches, as its manifest will declare them; none for a harness (the CLI's own). */
  network: string[];
  /** Whether it keeps the secrets of its accounts (a token, an API key). */
  secrets: boolean;
  /** Its page in Escouade's documentation. */
  docs: Text;
  /** Its tile (PLG-13): its brand's mark where the brand's published rules allow it without asking, else its
   * category's glyph (shared/marks.ts, docs/PLUGINS.md §3.1). */
  logo: Logo;
  /** Its folder in the app's repository, once it is a plugin of its own there (`plugins/<short name>`, INT-4's rule:
   * manifest, README, tests): its page links to it. None for those still in the core. */
  source?: string;
}

/** Where a folder of the app's repository is read. */
export const sourceUrl = (path: string) => `https://github.com/guillaume-gagnaire/escouade/tree/main/${path}`;

const t = (fr: string, en: string): Text => ({ fr, en });
const doc = (page: string): Text => t(`https://escouade.dev/docs/${page}`, `https://escouade.dev/en/docs/${page}`);

const harness = (id: string, name: string, fr: string, en: string): Builtin => ({
  id: `dev.escouade.${id}`,
  name,
  categories: ['harness'],
  description: t(fr, en),
  network: [],
  secrets: false,
  docs: doc('harness'),
  logo: id === 'opencode' ? { brand: 'opencode' } : { category: 'harness' },
});

export const BUILTINS: Builtin[] = [
  {
    ...harness(
      'claude-code',
      'Claude Code',
      'Claude Code comme harness : la conversation, sa vue terminal, les permissions, le plan, les comptes et leur quota.',
      'Claude Code as a harness: the conversation, its terminal view, permissions, the plan, accounts and their quota.',
    ),
    // Out of the core since INT-10.5.
    source: 'plugins/claude',
  },
  {
    ...harness(
      'codex',
      'Codex',
      'Codex d’OpenAI comme harness : la conversation, sa vue terminal, les permissions, le plan et le coût par compte.',
      'OpenAI’s Codex as a harness: the conversation, its terminal view, permissions, the plan and the cost per account.',
    ),
    source: 'plugins/codex',
  },
  {
    ...harness(
      'copilot-cli',
      'Copilot CLI',
      'GitHub Copilot CLI comme harness : la conversation, sa vue terminal, les permissions et les crédits du compte.',
      'GitHub Copilot CLI as a harness: the conversation, its terminal view, permissions and the account’s credits.',
    ),
    // Out of the core since INT-10.3.
    source: 'plugins/copilot',
  },
  {
    ...harness(
      'cursor-cli',
      'Cursor CLI',
      'Cursor CLI comme harness, par ACP : la conversation, les permissions, les modèles et l’usage du compte.',
      'Cursor CLI as a harness, over ACP: the conversation, permissions, models and the account’s usage.',
    ),
    // The first harness to have left the core (INT-10.1).
    source: 'plugins/cursor',
  },
  {
    ...harness(
      'opencode',
      'OpenCode',
      'OpenCode comme harness : un serveur par compte, ses modèles (locaux compris), son budget par jour.',
      'OpenCode as a harness: one server per account, its models (local ones too), its daily budget.',
    ),
    source: 'plugins/opencode',
  },
  {
    ...harness(
      'mistral',
      'Mistral',
      'Mistral dans Escouade : Mistral Vibe comme harness, par ACP (la conversation, sa vue terminal, la clé et le coût par compte), et le service de voix, sur un compte Mistral partagé.',
      'Mistral in Escouade: Mistral Vibe as a harness, over ACP (the conversation, its terminal view, the key and the cost per account), and the voice service, on one shared Mistral account.',
    ),
    // Out of the core since INT-10.6; one plugin with the voice since INT-10.6 bis.
    source: 'plugins/mistral',
  },
  {
    ...harness(
      'antigravity-cli',
      'Antigravity CLI',
      'Antigravity CLI comme harness : la conversation, sa vue terminal, les permissions et les comptes Google.',
      'Antigravity CLI as a harness: the conversation, its terminal view, permissions and Google accounts.',
    ),
    // Out of the core since INT-10.8.
    source: 'plugins/antigravity',
  },
  {
    ...harness(
      'gemini-cli',
      'Gemini CLI',
      'Gemini CLI comme harness, par ACP : la conversation, sa vue terminal, les permissions et les comptes.',
      'Gemini CLI as a harness, over ACP: the conversation, its terminal view, permissions and accounts.',
    ),
    // Out of the core since INT-10.7.
    source: 'plugins/gemini',
  },
  {
    ...harness(
      'acp',
      'ACP',
      'Les harness qui parlent ACP, décrits par un plugin sans code (Goose, par exemple) : Escouade les lance, leur parle et montre leur conversation.',
      'The harnesses that speak ACP, described by a plugin without code (Goose, say): Escouade launches them, speaks to them and shows their conversation.',
    ),
    docs: doc('plugins/harnesses'),
    source: 'plugins/acp',
  },
  {
    id: 'dev.escouade.github',
    name: 'GitHub',
    categories: ['ticket-sources', 'forges'],
    description: t(
      'Les issues dans le Kanban, les PR, la CI et les déploiements de tes dépôts GitHub, sur un seul compte.',
      'Issues in the Kanban, pull requests, CI and deployments of your GitHub repositories, on one account.',
    ),
    network: ['api.github.com', '*.ghe.com'],
    secrets: true,
    docs: doc('integrations'),
    logo: { brand: 'github' },
    source: 'plugins/github',
  },
  {
    id: 'dev.escouade.gitlab',
    name: 'GitLab',
    categories: ['ticket-sources', 'forges'],
    description: t(
      'Les issues de GitLab dans le Kanban, les merge requests, la CI et les déploiements, sur gitlab.com ou ton instance.',
      'GitLab issues in the Kanban, merge requests, CI and deployments, on gitlab.com or your own instance.',
    ),
    network: ['gitlab.com'],
    secrets: true,
    docs: doc('integrations'),
    logo: { category: 'forge' },
    source: 'plugins/gitlab',
  },
  {
    id: 'dev.escouade.azure-devops',
    name: 'Azure DevOps',
    categories: ['ticket-sources', 'forges'],
    description: t(
      'Azure Boards dans le Kanban, les PR d’Azure Repos et leurs pipelines, sur le compte de ton organisation.',
      'Azure Boards in the Kanban, Azure Repos pull requests and their pipelines, on your organization’s account.',
    ),
    network: ['dev.azure.com', '*.dev.azure.com', '*.visualstudio.com'],
    secrets: true,
    docs: doc('integrations'),
    logo: { category: 'forge' },
    source: 'plugins/azure-devops',
  },
  {
    id: 'dev.escouade.bitbucket',
    name: 'Bitbucket',
    categories: ['forges'],
    description: t(
      'Les pull requests de Bitbucket et leurs pipelines, suivies sur la carte de l’agent.',
      'Bitbucket pull requests and their pipelines, followed on the agent’s card.',
    ),
    network: ['api.bitbucket.org'],
    secrets: true,
    docs: doc('integrations'),
    logo: { category: 'forge' },
    source: 'plugins/bitbucket',
  },
  {
    id: 'dev.escouade.gitea',
    name: 'Gitea / Forgejo',
    categories: ['forges'],
    description: t(
      'Les PR, la CI (statuts et Actions), les relectures et le merge de tes dépôts Gitea, Forgejo et Codeberg, sur un seul compte.',
      'Pull requests, CI (statuses and Actions), reviews and merging of your Gitea, Forgejo and Codeberg repositories, on one account.',
    ),
    network: ['codeberg.org', 'gitea.com'],
    secrets: true,
    docs: doc('integrations'),
    logo: { brand: 'gitea' },
    source: 'plugins/gitea',
  },
  ...(
    [
      [
        'jira',
        'Jira',
        '*.atlassian.net',
        'Les tickets de Jira Cloud dans le Kanban, leur état suivi dans les deux sens.',
        'Jira Cloud issues in the Kanban, their state followed both ways.',
      ],
      [
        'trello',
        'Trello',
        'api.trello.com',
        'Les cartes de Trello dans le Kanban, leurs listes comme colonnes.',
        'Trello cards in the Kanban, their lists as columns.',
      ],
      [
        'linear',
        'Linear',
        'api.linear.app',
        'Les issues de Linear dans le Kanban, leur état suivi dans les deux sens.',
        'Linear issues in the Kanban, their state followed both ways.',
      ],
      [
        'clickup',
        'ClickUp',
        'api.clickup.com',
        'Les tâches de ClickUp dans le Kanban, leur statut suivi dans les deux sens.',
        'ClickUp tasks in the Kanban, their status followed both ways.',
      ],
    ] as const
  ).map(([id, name, host, fr, en]): Builtin => ({
    id: `dev.escouade.${id}`,
    name,
    categories: ['ticket-sources'],
    description: t(fr, en),
    network: [host],
    secrets: true,
    docs: doc('integrations'),
    logo: { category: 'source' },
    // Trello (INT-7.1), Linear (INT-7.2), Jira (INT-7.3) and ClickUp (INT-7.4) have left the core.
    ...(id === 'trello' || id === 'linear' || id === 'jira' || id === 'clickup' ? { source: `plugins/${id}` } : {}),
  })),
];

/** Whether `id` is one of the plugins the app gives (not every `dev.escouade.*`: those of `plugins/<id>/` are the store's). */
export const isBuiltinId = (id: string) => BUILTINS.some((b) => b.id === id);
