import type { Logo } from '../../shared/marks';
import type { Text } from './checks';

// The plugins of Escouade's own hand (docs/PLUGINS.md §4.3): its harnesses, its Kanban sources and its code hosts,
// installed and updated with the app, signed with it, their ids `dev.escouade.*` reserved. They have a page on the
// store with the mark « Escouade » (decided by the chief of the squad on 2026-10-29), and no version of their own:
// theirs is the app's. Nothing here is downloaded; « Ouvrir dans Escouade » opens their page in the app.

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
}

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
  harness(
    'claude-code',
    'Claude Code',
    'Claude Code comme harness : la conversation, sa vue terminal, les permissions, le plan, les comptes et leur quota.',
    'Claude Code as a harness: the conversation, its terminal view, permissions, the plan, accounts and their quota.',
  ),
  harness(
    'codex',
    'Codex',
    'Codex d’OpenAI comme harness : la conversation, sa vue terminal, les permissions, le plan et le coût par compte.',
    'OpenAI’s Codex as a harness: the conversation, its terminal view, permissions, the plan and the cost per account.',
  ),
  harness(
    'copilot-cli',
    'Copilot CLI',
    'GitHub Copilot CLI comme harness : la conversation, sa vue terminal, les permissions et les crédits du compte.',
    'GitHub Copilot CLI as a harness: the conversation, its terminal view, permissions and the account’s credits.',
  ),
  harness(
    'cursor-cli',
    'Cursor CLI',
    'Cursor CLI comme harness, par ACP : la conversation, les permissions, les modèles et l’usage du compte.',
    'Cursor CLI as a harness, over ACP: the conversation, permissions, models and the account’s usage.',
  ),
  harness(
    'opencode',
    'OpenCode',
    'OpenCode comme harness : un serveur par compte, ses modèles (locaux compris), son budget par jour.',
    'OpenCode as a harness: one server per account, its models (local ones too), its daily budget.',
  ),
  harness(
    'mistral-vibe',
    'Mistral Vibe',
    'Mistral Vibe comme harness, par ACP : la conversation, sa vue terminal, la clé et le coût par compte.',
    'Mistral Vibe as a harness, over ACP: the conversation, its terminal view, the key and the cost per account.',
  ),
  harness(
    'antigravity-cli',
    'Antigravity CLI',
    'Antigravity CLI comme harness : la conversation, sa vue terminal, les permissions et les comptes Google.',
    'Antigravity CLI as a harness: the conversation, its terminal view, permissions and Google accounts.',
  ),
  harness(
    'gemini-cli',
    'Gemini CLI',
    'Gemini CLI comme harness, par ACP : la conversation, sa vue terminal, les permissions et les comptes.',
    'Gemini CLI as a harness, over ACP: the conversation, its terminal view, permissions and accounts.',
  ),
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
  },
  {
    id: 'dev.escouade.azure-devops',
    name: 'Azure DevOps',
    categories: ['ticket-sources', 'forges'],
    description: t(
      'Azure Boards dans le Kanban, les PR d’Azure Repos et leurs pipelines, sur le compte de ton organisation.',
      'Azure Boards in the Kanban, Azure Repos pull requests and their pipelines, on your organization’s account.',
    ),
    network: ['dev.azure.com'],
    secrets: true,
    docs: doc('integrations'),
    logo: { category: 'forge' },
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
  },
  {
    id: 'dev.escouade.gitea',
    name: 'Gitea et Forgejo',
    categories: ['forges'],
    description: t(
      'Les pull requests et les actions de ton instance Gitea ou Forgejo.',
      'Pull requests and actions of your Gitea or Forgejo instance.',
    ),
    network: [],
    secrets: true,
    docs: doc('integrations'),
    logo: { brand: 'gitea' },
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
  })),
];

export const isBuiltinId = (id: string) => id.startsWith('dev.escouade.');
