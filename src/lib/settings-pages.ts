// The register of the settings window (direction A of the 2.2 redesign, design/settings-2-2/PROPOSITION.md §2):
// its pages, each of Escouade or of a project, and its rows, each declared once with its page, its label, whether it
// is folded into « Avancé », and the keys it sets. What it sets is read here: the dot of a changed page, the search
// (R8), and the guarantee that no key is lost (each key of `Settings` and of a project is set by one page alone).
//
// The rows say too in which tab of the 2.0 window they are shown (`tab`), until the lot that moves them (R3 to R7):
// the 2.0 tabs read it for their dot of a change.

import { t, type Key } from './i18n';
import type { IconName } from './icons';
import type { BoardSettings, Settings } from './types';
import type { ProjectDraft } from './settings.svelte';

/** Who a page sets: Escouade, or the project chosen. */
export type SettingsScope = 'app' | 'project';

export type SettingsPage =
  // Escouade's, in the order of the bar.
  | 'general'
  | 'agents'
  | 'accounts'
  | 'services'
  | 'voice'
  | 'notifications'
  | 'plugins'
  | 'advanced'
  // At the foot of the bar.
  | 'about'
  // A project's, under its name.
  | 'project'
  | 'git'
  | 'launch'
  | 'board'
  | 'tickets'
  | 'team'
  | 'privacy'
  // Under the projects: what a new project starts with, the closed ones (the app's settings).
  | 'allProjects';

/** The tabs of the 2.0 window: still read (`legacyTab`), and shown until R2. */
export type SettingsTab =
  | 'app'
  | 'claude'
  | 'accounts'
  | 'notifications'
  | 'voice'
  | 'projects'
  | 'board'
  | 'integrations'
  | 'plugins'
  | 'mcp'
  | 'terminals'
  | 'network'
  | 'about';

/** A section of a tab that a link of the app asked for. */
export type SettingsSection = 'launch' | 'sources';

export interface PageInfo {
  id: SettingsPage;
  scope: SettingsScope;
  icon: IconName;
  label: string;
  desc: string;
}

/** A page whose label and description are read where shown, so that they follow a change of language. */
function page(id: SettingsPage, scope: SettingsScope, icon: IconName): PageInfo {
  return {
    id,
    scope,
    icon,
    get label() {
      return t(`settings.pages.${id}.label`);
    },
    get desc() {
      return t(`settings.pages.${id}.desc`);
    },
  };
}

/** The pages, in the order of the bar: Escouade's, « À propos », each project's, then « Tous les projets ». */
export const SETTINGS_PAGES: PageInfo[] = [
  page('general', 'app', 'sliders'),
  page('agents', 'app', 'agents'),
  page('accounts', 'app', 'user'),
  page('services', 'app', 'link'),
  page('voice', 'app', 'mic'),
  page('notifications', 'app', 'bell'),
  page('plugins', 'app', 'puzzle'),
  page('advanced', 'app', 'term'),
  page('about', 'app', 'info'),
  page('project', 'project', 'folder'),
  page('git', 'project', 'branch'),
  page('launch', 'project', 'play'),
  page('board', 'project', 'kanban'),
  page('tickets', 'project', 'import'),
  page('team', 'project', 'team'),
  page('privacy', 'project', 'lock'),
  page('allProjects', 'app', 'folder'),
];

export const pageInfo = (id: SettingsPage): PageInfo => SETTINGS_PAGES.find((p) => p.id === id)!;

/** What a project's draft holds, its board apart. */
export type ProjectKey = Exclude<keyof ProjectDraft, 'board'>;
export type BoardKey = keyof BoardSettings;

export interface RowInfo {
  /** Unique in the register. */
  id: string;
  page: SettingsPage;
  label: string;
  /** Folded into « Avancé » at the foot of its page. */
  advanced: boolean;
  /** The keys of the app's settings it sets. */
  settings: (keyof Settings)[];
  /** The keys of a project it sets, and of its board. */
  project: ProjectKey[];
  board: BoardKey[];
  /** The 2.0 tab it is shown in until its lot moves it. */
  tab: SettingsTab;
}

/** A label that is not a text of the catalogs (the name of a program). */
type Raw = { raw: string };

interface RowOptions {
  advanced?: boolean;
  settings?: (keyof Settings)[];
  project?: ProjectKey[];
  board?: BoardKey[];
}

function row(id: string, page: SettingsPage, tab: SettingsTab, label: Key | Raw, o: RowOptions = {}): RowInfo {
  return {
    id,
    page,
    tab,
    advanced: !!o.advanced,
    settings: o.settings ?? [],
    project: o.project ?? [],
    board: o.board ?? [],
    get label() {
      return typeof label === 'string' ? t(label) : label.raw;
    },
  };
}

// The app's settings: one row per control, in the order of its page (PROPOSITION.md §2 and annexe A).
const s = (...settings: (keyof Settings)[]) => ({ settings });

export const SETTINGS_ROWS: RowInfo[] = [
  // Général › Apparence, Langue, Mises à jour, Toi; Avancé.
  row('theme', 'general', 'app', 'settings.app.theme', s('theme')),
  row('colors', 'general', 'app', 'settings.app.colors', s('pluginTheme')),
  row('zoom', 'general', 'app', 'settings.app.zoom', s('uiZoom')),
  row('terminalFontSize', 'general', 'app', 'settings.app.terminalFontSize', s('terminalFontSize')),
  row('uiLanguage', 'general', 'app', 'settings.app.uiLanguage', s('language')),
  row('claudeLanguage', 'general', 'app', 'settings.app.claudeLanguage', s('claudeLanguage')),
  row('updateCheck', 'general', 'about', 'settings.about.check'),
  row('autoUpdate', 'general', 'about', 'settings.about.autoUpdate', s('autoUpdate')),
  row('betaUpdates', 'general', 'about', 'settings.about.betaUpdates', s('betaUpdates')),
  row('userName', 'general', 'app', 'share.settings.name', s('userName', 'userNameCleared')),
  row('telemetry', 'general', 'about', 'settings.about.telemetry', s('telemetry')),
  row('editorFontSize', 'general', 'app', 'settings.app.editorFontSize', { advanced: true, ...s('editorFontSize') }),
  row('userId', 'general', 'app', 'share.settings.id', { advanced: true, ...s('userId') }),

  // Agents › Nouveaux agents, Textes rédigés, Remote control, Piloter Escouade; Avancé.
  row('terminalView', 'agents', 'claude', 'settings.claude.terminalView', s('terminalView')),
  row('todoTools', 'agents', 'claude', 'settings.claude.todoTools', s('todoTools')),
  row('midTurnDelay', 'agents', 'claude', 'settings.claude.midTurnDelay', s('midTurnDelaySecs')),
  row('writer', 'agents', 'app', 'settings.app.writer', s('writerEngine')),
  row('writerModel', 'agents', 'app', 'settings.app.writerModel', s('writerModel')),
  row('remoteAll', 'agents', 'claude', 'settings.claude.remoteAll', s('remoteControlAll')),
  row('mcp', 'agents', 'mcp', 'mcp.section.enable', s('mcpEnabled')),
  row('mcpDeclared', 'agents', 'mcp', 'mcp.section.list'),
  row('idleStop', 'agents', 'claude', 'settings.claude.idleStop', { advanced: true, ...s('idleStopMinutes') }),
  row('mcpActivity', 'agents', 'mcp', 'mcp.section.title', { advanced: true }),

  // Comptes et harness › each harness, Quand un quota est épuisé; Avancé.
  row('harnesses', 'accounts', 'claude', 'settings.claude.allHarnesses'),
  row('accountsList', 'accounts', 'accounts', 'accounts.tab.list', s('accounts')),
  row('favoriteModels', 'accounts', 'claude', 'settings.claude.models.label', s('favoriteModels')),
  row('switchAt', 'accounts', 'accounts', 'accounts.quotas.switchAt', s('switchAt')),
  row('quotaPause', 'accounts', 'board', 'boardSettings.tab.quotaPause', s('quotaPause')),
  row('switchOnLimit', 'accounts', 'accounts', 'accounts.switchOnLimit.label', s('switchOnLimit')),
  row('autoResume', 'accounts', 'claude', 'settings.claude.autoResume', s('autoResume')),
  row('handoverOnLimit', 'accounts', 'accounts', 'handover.settings.onLimit', s('handoverOnLimit')),
  row('engineOrder', 'accounts', 'accounts', 'accounts.engineOrder.label', { advanced: true, ...s('engineOrder') }),
  row('handoverModel', 'accounts', 'accounts', 'handover.settings.model', { advanced: true, ...s('handoverModels') }),
  row('claudePath', 'accounts', 'claude', 'settings.claude.path', { advanced: true, ...s('claudePath') }),
  row('selfUpdate', 'accounts', 'claude', 'settings.claude.selfUpdate', { advanced: true, ...s('claudeSelfUpdate') }),
  row('quotaProjection', 'accounts', 'accounts', 'accounts.quotas.projection', { advanced: true, ...s('quotaProjection') }),

  // Services connectés › Hébergeurs de code, Sources de tickets, Synchronisation, Import automatique.
  row('forges', 'services', 'integrations', 'integrations.forges.title'),
  row('ticketAccounts', 'services', 'integrations', 'integrations.accounts.title'),
  row('sync', 'services', 'integrations', 'integrations.sync.title', s('integrations')),
  row('autoImport', 'services', 'integrations', 'integrations.autoImport.title', s('integrations')),

  // Voix › Dictée, Conversation.
  row('dictation', 'voice', 'voice', 'voice.settings.transcription', s('voice')),
  row('conversation', 'voice', 'voice', 'voice.speech.title', s('voice')),

  // Notifications › Me prévenir pour, Pull requests et déploiements, Canaux, Son.
  row('notifyFor', 'notifications', 'notifications', 'settings.notifications.notifyFor', s('notifyFor')),
  row('osNotifications', 'notifications', 'notifications', 'settings.notifications.system', s('osNotifications')),
  row('sound', 'notifications', 'notifications', 'settings.notifications.soundOn', s('sound')),

  // Plugins › Installés, Proposés par Escouade, N'installer que des versions vérifiées (in the view Plugins until R7).
  row('pluginsInstalled', 'plugins', 'plugins', 'settings.pages.plugins.label'),
  row('pluginsVerifiedOnly', 'plugins', 'plugins', 'plugins.store.verifiedOnly', s('pluginsVerifiedOnly')),

  // Avancé › Réseau, Terminaux, Pièces jointes, Recherche par le sens, Données locales.
  row('proxyUrl', 'advanced', 'network', 'settings.network.proxyUrl', s('proxyUrl')),
  row('noProxy', 'advanced', 'network', 'settings.network.exclusions', s('noProxy')),
  row('proxyTerminals', 'advanced', 'network', 'settings.network.proxyTerminals', s('proxyTerminals')),
  row('insecureTls', 'advanced', 'network', 'settings.network.insecureTls', s('insecureTls')),
  row('pwshPath', 'advanced', 'terminals', { raw: 'PowerShell 7' }, s('pwshPath')),
  row('bashPath', 'advanced', 'terminals', { raw: 'Git Bash' }, s('bashPath')),
  row('wslDistro', 'advanced', 'terminals', 'settings.terminals.wsl', s('wslDistro')),
  row('attachMax', 'advanced', 'claude', 'settings.claude.attachMax', s('attachMaxMb')),
  row('attachWarn', 'advanced', 'claude', 'settings.claude.attachWarn', s('attachWarnMb')),
  row('attachOrigin', 'advanced', 'claude', 'settings.claude.attachOrigin', s('attachOrigin')),
  row('attachBigText', 'advanced', 'claude', 'settings.claude.attachBigText', s('attachBigText')),
  row('attachKeep', 'advanced', 'claude', 'settings.claude.attachKeep', s('attachKeepDays')),
  row('vectors', 'advanced', 'projects', 'vectors.allTitle'),
  row('localData', 'advanced', 'about', 'settings.about.localData'),

  // À propos.
  row('version', 'about', 'about', { raw: 'Escouade' }),
  row('credits', 'about', 'about', 'settings.about.credits'),
  row('site', 'about', 'about', 'settings.about.site'),
  row('discord', 'about', 'about', 'settings.about.discord'),
  row('support', 'about', 'about', 'settings.about.support'),

  // A project's pages (annexe A).
  // Projet; Avancé › Fermer le projet.
  row('projectName', 'project', 'projects', 'settings.project.nameLabel', { project: ['name'] }),
  row('projectFolder', 'project', 'projects', 'common.folder'),
  row('projectColor', 'project', 'projects', 'common.color', { project: ['color'] }),
  row('tileLetters', 'project', 'projects', 'settings.project.lettersLabel', { project: ['tileLetters'] }),
  row('logo', 'project', 'projects', 'settings.project.logoLabel', { project: ['logo'] }),
  row('tileLabel', 'project', 'projects', 'settings.project.labelLabel', { project: ['tileLabel', 'tileLabelText'] }),
  row('projectEngine', 'project', 'projects', 'settings.project.engine', { project: ['engine'] }),
  row('projectAccount', 'project', 'projects', 'accounts.project.label', { project: ['account'] }),
  row('wizard', 'project', 'projects', 'wizard.reopen.label'),
  row('proposedPlugins', 'project', 'projects', 'plugins.proposed.title'),
  row('closeProject', 'project', 'projects', 'settings.project.close', { advanced: true }),

  // Git et worktrees.
  row('worktreePerAgent', 'git', 'projects', 'settings.project.worktreePerAgent', { project: ['worktreePerAgent'] }),
  row('worktreeAtFirstMessage', 'git', 'projects', 'settings.project.worktreeAtFirstMessage', {
    project: ['worktreeAtFirstMessage'],
  }),
  row('worktreeBase', 'git', 'projects', 'settings.project.worktreeBase', { project: ['worktreeBase'] }),
  row('worktreeDir', 'git', 'projects', 'settings.project.worktreeDir', { project: ['worktreeDir'] }),
  row('agentPrefix', 'git', 'projects', 'settings.project.agentPrefix', { project: ['agentBranchPrefix'] }),
  row('ticketPrefix', 'git', 'projects', 'settings.project.ticketPrefix', { project: ['ticketBranchPrefix'] }),
  row('copy', 'git', 'projects', 'settings.project.copy', { project: ['copy'] }),
  row('commit', 'git', 'projects', 'settings.project.commit', { project: ['commitMode'] }),
  row('docsBranch', 'git', 'projects', 'docs.branch.label', { project: ['docsBranch'] }),
  row('worktreeSetup', 'git', 'projects', 'settings.project.setupTitle', { project: ['worktreeSetup'] }),
  row('worktreeTeardown', 'git', 'projects', 'settings.project.teardownTitle', { project: ['worktreeTeardown'] }),
  row('leftovers', 'git', 'projects', 'settings.project.leftovers'),
  row('repos', 'git', 'projects', 'repos.settings.title'),

  // Lancement.
  row('pagesOpen', 'launch', 'projects', 'settings.project.pagesOpen', { project: ['pagesOpen'] }),
  row('runCommands', 'launch', 'projects', 'settings.project.launch', { project: ['runCommands'] }),

  // Kanban › Validation, Avant et après, Pilote auto, Agents des tickets, Workflow.
  row('action', 'board', 'board', 'boardSettings.tab.actionsTitle', { board: ['action'] }),
  row('target', 'board', 'board', 'boardSettings.tab.targetBranch', { board: ['target'] }),
  row('strategy', 'board', 'board', 'boardSettings.tab.strategy', { board: ['strategy'] }),
  row('draftPr', 'board', 'board', 'boardSettings.tab.draftPr', { board: ['draft'] }),
  row('reviewers', 'board', 'board', 'boardSettings.tab.reviewers', { board: ['reviewers'] }),
  row('labels', 'board', 'board', 'boardSettings.tab.labels', { board: ['labels'] }),
  row('replyThreads', 'board', 'board', 'boardSettings.tab.replyThreads', { board: ['replyThreads'] }),
  row('prUpdate', 'board', 'board', 'boardSettings.tab.prUpdate', { board: ['prUpdate'] }),
  row('testCommand', 'board', 'board', 'boardSettings.tab.testCommand', { board: ['testCommand'] }),
  row('testsFirst', 'board', 'board', 'boardSettings.tab.testsFirst', { board: ['testsFirst'] }),
  row('cleanup', 'board', 'board', 'boardSettings.tab.cleanup', { board: ['cleanup'] }),
  row('commitMessage', 'board', 'board', 'boardSettings.tab.commitMessage', { board: ['conventional'] }),
  row('onConflict', 'board', 'board', 'boardSettings.tab.onConflict', { board: ['conflict'] }),
  row('autoAssign', 'board', 'board', 'boardSettings.tab.autoAssign', { board: ['autopilot'] }),
  row('redTargetPause', 'board', 'board', 'boardSettings.tab.redTargetPause', { board: ['pauseOnRedTarget'] }),
  row('parallel', 'board', 'board', 'boardSettings.tab.parallel', { board: ['maxParallel'] }),
  row('model', 'board', 'board', 'boardSettings.tab.defaultModel', { board: ['model'] }),
  row('effort', 'board', 'board', 'boardSettings.tab.effort', { board: ['effort'] }),
  row('mode', 'board', 'board', 'boardSettings.tab.mode', { board: ['mode'] }),
  row('workflowsEnabled', 'board', 'projects', 'workflowSettings.enabled', { project: ['workflowsEnabled'] }),
  row('workflowsAutoRead', 'board', 'projects', 'workflowSettings.auto', { project: ['workflowsAutoRead'] }),
  row('workflowsInRepo', 'board', 'projects', 'workflowSettings.where', { project: ['workflowsInRepo'] }),
  row('workflowSteps', 'board', 'projects', 'workflowSettings.stepsTitle', { project: ['workflowFixes'] }),

  // Tickets importés › Sources liées, Correspondance des statuts.
  row('sources', 'tickets', 'integrations', 'integrations.modal.sources', { project: ['integrations'] }),
  row('mapping', 'tickets', 'integrations', 'integrations.mapping.title', { project: ['integrations'] }),

  // Équipe › Kanban partagé, Réglages partagés (.escouade.yml), Mémoire partagée.
  row('projectUserName', 'team', 'projects', 'share.settings.projectName', { project: ['userName'] }),
  row('shareNew', 'team', 'projects', 'share.settings.shareNew', { project: ['shareNew'] }),
  row('sharedFile', 'team', 'projects', 'share.settings.files'),
  row('memory', 'team', 'projects', 'memory.settings.title'),

  // Confidentialité.
  row('private', 'privacy', 'projects', 'settings.project.private', { project: ['private'] }),
  row('sharedOnLimit', 'privacy', 'projects', 'accounts.private.sharedOnLimit', { project: ['sharedOnLimit'] }),
  row('projectRemote', 'privacy', 'projects', 'settings.project.remote', { project: ['remoteControl'] }),

  // Tous les projets › Nouveaux projets, Projets fermés.
  row('privateProjects', 'allProjects', 'projects', 'settings.newProjects.private', s('privateProjects')),
  row('closedProjects', 'allProjects', 'projects', 'settings.closed.title'),
];

/**
 * The keys of the app's settings no page of the window sets: set elsewhere, by a gesture of the app (the widths of
 * the bars, dragged), asked once (`nameAsked`), or fixed (`mcpPort`). « Rien d'autre ne quitte la fenêtre »: these
 * were already out of it.
 */
export const SETTINGS_ELSEWHERE: (keyof Settings)[] = ['railWidth', 'sideWidth', 'nameAsked', 'mcpPort'];

/** The keys of a board set by the app alone: the prefix and the number of its next ticket. */
export const BOARD_ELSEWHERE: BoardKey[] = ['prefix', 'nextNumber'];

/** The rows of a page, in their order. */
export const rowsOf = (id: SettingsPage): RowInfo[] => SETTINGS_ROWS.filter((r) => r.page === id);

/**
 * The page an id of the 2.0 window leads to (the links of the app until each lot moves its own, R6 at the latest),
 * with the section it asked for: a project's launch, a project's sources.
 */
export function legacyTab(tab: SettingsTab, section?: SettingsSection): SettingsPage {
  if (tab === 'projects') return section === 'launch' ? 'launch' : 'project';
  if (tab === 'integrations') return section === 'sources' ? 'tickets' : 'services';
  return LEGACY[tab];
}

const LEGACY: Record<Exclude<SettingsTab, 'projects' | 'integrations'>, SettingsPage> = {
  app: 'general',
  // « Accueil : aucun harness » and « Réglages › Harness » lead to the harnesses (annexe B).
  claude: 'accounts',
  accounts: 'accounts',
  notifications: 'notifications',
  voice: 'voice',
  board: 'board',
  plugins: 'plugins',
  mcp: 'agents',
  terminals: 'advanced',
  network: 'advanced',
  about: 'about',
};

/** The 2.0 tab a page is shown in until R2 (its first rows' tab, for a page made of several). */
export function tabOfPage(id: SettingsPage): SettingsTab {
  return TAB_OF[id];
}

const TAB_OF: Record<SettingsPage, SettingsTab> = {
  general: 'app',
  agents: 'claude',
  accounts: 'accounts',
  services: 'integrations',
  voice: 'voice',
  notifications: 'notifications',
  plugins: 'plugins',
  advanced: 'network',
  about: 'about',
  project: 'projects',
  git: 'projects',
  launch: 'projects',
  board: 'board',
  tickets: 'integrations',
  team: 'projects',
  privacy: 'projects',
  allProjects: 'projects',
};
