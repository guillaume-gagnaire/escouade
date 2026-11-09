// The register of the settings window (direction A of the 2.2 redesign, design/settings-2-2/PROPOSITION.md §2):
// its pages, each of Escouade or of a project, and its rows, each declared once with its page, its label, whether it
// is folded into « Avancé », and the keys it sets. What it sets is read here: the dot of a changed page, the search
// (R8), and the guarantee that no key is lost (each key of `Settings` and of a project is set by one page alone).

import { t, tIn, type Key, type Lang } from './i18n';
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
  /** Its label in a language, whatever the interface's (the search reads both). */
  labelIn(lang: Lang): string;
  /** What it does, in a language: its label's `…Desc` or `…Hint` in the catalog, else nothing. */
  descIn(lang: Lang): string;
  /** Folded into « Avancé » at the foot of its page. */
  advanced: boolean;
  /** The keys of the app's settings it sets. */
  settings: (keyof Settings)[];
  /** The keys of a project it sets, and of its board. */
  project: ProjectKey[];
  board: BoardKey[];
}

/** A label that is not a text of the catalogs (the name of a program). */
type Raw = { raw: string };

interface RowOptions {
  advanced?: boolean;
  settings?: (keyof Settings)[];
  project?: ProjectKey[];
  board?: BoardKey[];
}

function row(id: string, page: SettingsPage, label: Key | Raw, o: RowOptions = {}): RowInfo {
  return {
    id,
    page,
    advanced: !!o.advanced,
    settings: o.settings ?? [],
    project: o.project ?? [],
    board: o.board ?? [],
    get label() {
      return typeof label === 'string' ? t(label) : label.raw;
    },
    labelIn: (lang) => (typeof label === 'string' ? tIn(lang, label) : label.raw),
    descIn: (lang) => (typeof label === 'string' ? descOf(lang, label) : ''),
  };
}

/** The text that says what a row does, found beside its label in the catalog (`betaUpdates` → `betaUpdatesDesc`). */
function descOf(lang: Lang, label: Key): string {
  return ['Desc', 'Hint']
    .map((end) => {
      const key = `${label}${end}` as Key;
      const text = (tIn as (l: Lang, k: Key) => string)(lang, key);
      return text === key ? '' : text;
    })
    .filter(Boolean)
    .join(' ');
}

// The app's settings: one row per control, in the order of its page (PROPOSITION.md §2 and annexe A).
const s = (...settings: (keyof Settings)[]) => ({ settings });

export const SETTINGS_ROWS: RowInfo[] = [
  // Général › Apparence, Langue, Mises à jour, Toi; Avancé.
  row('theme', 'general', 'settings.app.theme', s('theme')),
  row('colors', 'general', 'settings.app.colors', s('pluginTheme')),
  row('zoom', 'general', 'settings.app.zoom', s('uiZoom')),
  row('terminalFontSize', 'general', 'settings.app.terminalFontSize', s('terminalFontSize')),
  row('uiLanguage', 'general', 'settings.app.uiLanguage', s('language')),
  row('claudeLanguage', 'general', 'settings.app.claudeLanguage', s('claudeLanguage')),
  row('updateCheck', 'general', 'settings.about.check'),
  row('autoUpdate', 'general', 'settings.about.autoUpdate', s('autoUpdate')),
  row('betaUpdates', 'general', 'settings.about.betaUpdates', s('betaUpdates')),
  row('userName', 'general', 'share.settings.name', s('userName', 'userNameCleared')),
  row('telemetry', 'general', 'settings.about.telemetry', s('telemetry')),
  row('editorFontSize', 'general', 'settings.app.editorFontSize', { advanced: true, ...s('editorFontSize') }),
  row('userId', 'general', 'share.settings.id', { advanced: true, ...s('userId') }),

  // Agents › Nouveaux agents, Textes rédigés, Remote control, Piloter Escouade; Avancé.
  row('terminalView', 'agents', 'settings.claude.terminalView', s('terminalView')),
  row('todoTools', 'agents', 'settings.claude.todoTools', s('todoTools')),
  row('midTurnDelay', 'agents', 'settings.claude.midTurnDelay', s('midTurnDelaySecs')),
  row('writer', 'agents', 'settings.app.writer', s('writerEngine')),
  row('writerModel', 'agents', 'settings.app.writerModel', s('writerModel')),
  row('remoteAll', 'agents', 'settings.claude.remoteAll', s('remoteControlAll')),
  row('mcp', 'agents', 'mcp.section.enable', s('mcpEnabled')),
  row('mcpDeclared', 'agents', 'mcp.section.list'),
  row('idleStop', 'agents', 'settings.claude.idleStop', { advanced: true, ...s('idleStopMinutes') }),
  row('mcpActivity', 'agents', 'mcp.activity.title', { advanced: true }),

  // Comptes et harness › each harness, Quand un quota est épuisé; Avancé.
  row('harnesses', 'accounts', 'settings.claude.allHarnesses'),
  row('accountsList', 'accounts', 'accounts.tab.list', s('accounts')),
  row('favoriteModels', 'accounts', 'settings.claude.models.label', s('favoriteModels')),
  row('switchAt', 'accounts', 'accounts.quotas.switchAt', s('switchAt')),
  row('quotaPause', 'accounts', 'accounts.quotas.pauseAt', s('quotaPause')),
  row('switchOnLimit', 'accounts', 'accounts.switchOnLimit.label', s('switchOnLimit')),
  row('autoResume', 'accounts', 'settings.claude.autoResume', s('autoResume')),
  row('handoverOnLimit', 'accounts', 'handover.settings.onLimit', s('handoverOnLimit')),
  row('engineOrder', 'accounts', 'accounts.engineOrder.label', { advanced: true, ...s('engineOrder') }),
  row('handoverModel', 'accounts', 'handover.settings.modelName', { advanced: true, ...s('handoverModels') }),
  row('claudePath', 'accounts', 'settings.claude.path', { advanced: true, ...s('claudePath') }),
  row('selfUpdate', 'accounts', 'settings.claude.selfUpdate', { advanced: true, ...s('claudeSelfUpdate') }),
  row('quotaProjection', 'accounts', 'accounts.quotas.projection', { advanced: true, ...s('quotaProjection') }),

  // Services connectés › Hébergeurs de code, Sources de tickets, Synchronisation, Import automatique.
  row('forges', 'services', 'integrations.forges.title'),
  row('ticketAccounts', 'services', 'integrations.accounts.title'),
  row('sync', 'services', 'integrations.sync.title', s('integrations')),
  row('autoImport', 'services', 'integrations.autoImport.title', s('integrations')),

  // Voix › Dictée, Conversation.
  row('dictation', 'voice', 'voice.settings.transcription', s('voice')),
  row('conversation', 'voice', 'voice.speech.title', s('voice')),

  // Notifications › Me prévenir pour, Pull requests et déploiements, Canaux, Son.
  row('notifyFor', 'notifications', 'settings.notifications.notifyFor', s('notifyFor')),
  row('osNotifications', 'notifications', 'settings.notifications.systemName', s('osNotifications')),
  row('sound', 'notifications', 'settings.notifications.soundOn', s('sound')),

  // Plugins › Installés, Proposés par Escouade, Sécurité (N'installer que des versions vérifiées).
  row('pluginsInstalled', 'plugins', 'plugins.installed.title'),
  row('pluginsSeed', 'plugins', 'plugins.installed.seedTitle'),
  row('pluginsVerifiedOnly', 'plugins', 'plugins.installed.verifiedOnly', s('pluginsVerifiedOnly')),

  // Avancé › Réseau, Terminaux, Pièces jointes, Recherche par le sens, Données locales.
  row('proxyUrl', 'advanced', 'settings.network.proxyUrl', s('proxyUrl')),
  row('noProxy', 'advanced', 'settings.network.exclusions', s('noProxy')),
  row('proxyTerminals', 'advanced', 'settings.network.proxyTerminals', s('proxyTerminals')),
  row('insecureTls', 'advanced', 'settings.network.insecureTls', s('insecureTls')),
  row('pwshPath', 'advanced', { raw: 'PowerShell 7' }, s('pwshPath')),
  row('bashPath', 'advanced', { raw: 'Git Bash' }, s('bashPath')),
  row('wslDistro', 'advanced', 'settings.terminals.wsl', s('wslDistro')),
  row('attachMax', 'advanced', 'settings.claude.attachMax', s('attachMaxMb')),
  row('attachWarn', 'advanced', 'settings.claude.attachWarn', s('attachWarnMb')),
  row('attachOrigin', 'advanced', 'settings.claude.attachOrigin', s('attachOrigin')),
  row('attachBigText', 'advanced', 'settings.claude.attachBigText', s('attachBigText')),
  row('attachKeep', 'advanced', 'settings.claude.attachKeep', s('attachKeepDays')),
  row('vectors', 'advanced', 'vectors.label'),
  row('localData', 'advanced', 'settings.about.localData'),

  // À propos.
  row('version', 'about', { raw: 'Escouade' }),
  row('credits', 'about', 'settings.about.credits'),
  row('site', 'about', 'settings.about.site'),
  row('discord', 'about', 'settings.about.discord'),
  row('support', 'about', 'settings.about.support'),

  // A project's pages (annexe A).
  // Projet; Avancé › Fermer le projet.
  row('projectName', 'project', 'settings.project.nameLabel', { project: ['name'] }),
  row('projectFolder', 'project', 'common.folder'),
  row('projectColor', 'project', 'common.color', { project: ['color'] }),
  row('tileLetters', 'project', 'settings.project.lettersLabel', { project: ['tileLetters'] }),
  row('logo', 'project', 'settings.project.logoLabel', { project: ['logo'] }),
  row('tileLabel', 'project', 'settings.project.labelLabel', { project: ['tileLabel', 'tileLabelText'] }),
  row('projectEngine', 'project', 'settings.project.engine', { project: ['engine'] }),
  row('projectAccount', 'project', 'accounts.project.label', { project: ['account'] }),
  row('wizard', 'project', 'wizard.reopen.label'),
  row('proposedPlugins', 'project', 'plugins.proposed.label'),
  row('closeProject', 'project', 'settings.project.close', { advanced: true }),

  // Git et worktrees.
  row('worktreePerAgent', 'git', 'settings.project.worktreePerAgent', { project: ['worktreePerAgent'] }),
  row('worktreeAtFirstMessage', 'git', 'settings.project.worktreeAtFirstMessage', {
    project: ['worktreeAtFirstMessage'],
  }),
  row('worktreeBase', 'git', 'settings.project.worktreeBase', { project: ['worktreeBase'] }),
  row('worktreeDir', 'git', 'settings.project.worktreeDir', { project: ['worktreeDir'] }),
  row('agentPrefix', 'git', 'settings.project.agentPrefix', { project: ['agentBranchPrefix'] }),
  row('ticketPrefix', 'git', 'settings.project.ticketPrefix', { project: ['ticketBranchPrefix'] }),
  row('copy', 'git', 'settings.project.copy', { project: ['copy'] }),
  row('commit', 'git', 'settings.project.commit', { project: ['commitMode'] }),
  row('docsBranch', 'git', 'docs.branch.label', { project: ['docsBranch'] }),
  row('worktreeSetup', 'git', 'settings.project.setupTitle', { project: ['worktreeSetup'] }),
  row('worktreeTeardown', 'git', 'settings.project.teardownTitle', { project: ['worktreeTeardown'] }),
  row('leftovers', 'git', 'settings.project.leftovers'),
  row('repos', 'git', 'repos.settings.title'),

  // Lancement.
  row('pagesOpen', 'launch', 'settings.project.pagesOpen', { project: ['pagesOpen'] }),
  row('runCommands', 'launch', 'settings.project.launch', { project: ['runCommands'] }),

  // Kanban › Validation, Avant et après, Pilote auto, Agents des tickets, Workflow.
  row('action', 'board', 'boardSettings.tab.actionsTitle', { board: ['action'] }),
  row('target', 'board', 'boardSettings.tab.targetBranch', { board: ['target'] }),
  row('strategy', 'board', 'boardSettings.tab.strategy', { board: ['strategy'] }),
  row('draftPr', 'board', 'boardSettings.tab.draftPr', { board: ['draft'] }),
  row('reviewers', 'board', 'boardSettings.tab.reviewers', { board: ['reviewers'] }),
  row('labels', 'board', 'boardSettings.tab.labels', { board: ['labels'] }),
  row('replyThreads', 'board', 'boardSettings.tab.replyThreads', { board: ['replyThreads'] }),
  row('prUpdate', 'board', 'boardSettings.tab.prUpdate', { board: ['prUpdate'] }),
  row('testCommand', 'board', 'boardSettings.tab.testCommand', { board: ['testCommand'] }),
  row('testsFirst', 'board', 'boardSettings.tab.testsFirst', { board: ['testsFirst'] }),
  row('cleanup', 'board', 'boardSettings.tab.cleanup', { board: ['cleanup'] }),
  row('commitMessage', 'board', 'boardSettings.tab.commitMessage', { board: ['conventional'] }),
  row('onConflict', 'board', 'boardSettings.tab.onConflict', { board: ['conflict'] }),
  row('autoAssign', 'board', 'boardSettings.tab.autoAssign', { board: ['autopilot'] }),
  row('redTargetPause', 'board', 'boardSettings.tab.redTargetPause', { board: ['pauseOnRedTarget'] }),
  row('parallel', 'board', 'boardSettings.tab.parallel', { board: ['maxParallel'] }),
  row('model', 'board', 'common.model', { board: ['model'] }),
  row('effort', 'board', 'boardSettings.tab.effort', { board: ['effort'] }),
  row('mode', 'board', 'boardSettings.tab.mode', { board: ['mode'] }),
  row('workflowsEnabled', 'board', 'workflowSettings.enabled', { project: ['workflowsEnabled'] }),
  row('workflowsAutoRead', 'board', 'workflowSettings.auto', { project: ['workflowsAutoRead'] }),
  row('workflowsInRepo', 'board', 'workflowSettings.where', { project: ['workflowsInRepo'] }),
  row('workflowSteps', 'board', 'workflowSettings.stepsTitle', { project: ['workflowFixes'] }),

  // Tickets importés › Sources liées, Correspondance des statuts.
  row('sources', 'tickets', 'integrations.modal.sources', { project: ['integrations'] }),
  row('mapping', 'tickets', 'integrations.mapping.title', { project: ['integrations'] }),

  // Équipe › Kanban partagé, Réglages partagés (.escouade.yml), Mémoire partagée.
  row('projectUserName', 'team', 'share.settings.projectName', { project: ['userName'] }),
  row('shareNew', 'team', 'share.settings.shareNew', { project: ['shareNew'] }),
  row('sharedFile', 'team', 'share.settings.files'),
  row('memory', 'team', 'memory.settings.title'),

  // Confidentialité.
  row('private', 'privacy', 'settings.project.private', { project: ['private'] }),
  row('sharedOnLimit', 'privacy', 'accounts.private.sharedOnLimit', { project: ['sharedOnLimit'] }),
  row('projectRemote', 'privacy', 'settings.project.remote', { project: ['remoteControl'] }),
  row('projectPlugins', 'privacy', 'plugins.projects.title'),

  // Tous les projets › Nouveaux projets, Projets fermés.
  row('privateProjects', 'allProjects', 'settings.newProjects.private', s('privateProjects')),
  row('closedProjects', 'allProjects', 'settings.closed.title'),
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
