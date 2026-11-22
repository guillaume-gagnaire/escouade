// The French catalog, source of the keys: `t` accepts its paths, the English catalog must have the same.
// A zone belongs to one part of the interface (one task at a time writes it); `common` and `format` are shared.
import accounts from './accounts';
import bgtasks from './bgtasks';
import board from './board';
import boardSettings from './boardSettings';
import branches from './branches';
import common from './common';
import composer from './composer';
import conv from './conv';
import docs from './docs';
import editor from './editor';
import errors from './errors';
import format from './format';
import git from './git';
import handover from './handover';
import heat from './heat';
import features from './features';
import integrations from './integrations';
import inbox from './inbox';
import mcp from './mcp';
import memory from './memory';
import backups from './backups';
import onboarding from './onboarding';
import discover from './discover';
import inboxViews from './inboxViews';
import vectors from './vectors';
import nav from './nav';
import plan from './plan';
import prs from './prs';
import envs from './envs';
import repos from './repos';
import runs from './runs';
import scheduled from './scheduled';
import overseer from './overseer';
import devpage from './devpage';
import settings from './settings';
import share from './share';
import shell from './shell';
import stats from './stats';
import subagents from './subagents';
import team from './team';
import teamBoard from './teamBoard';
import remote from './remote';
import servers from './servers';
import phone from './phone';
import spaces from './spaces';
import moves from './moves';
import teamProjects from './teamProjects';
import teamDocs from './teamDocs';
import teamCosts from './teamCosts';
import teamPlugins from './teamPlugins';
import wizard from './wizard';
import voice from './voice';
import plugins from './plugins';
import workflow from './workflow';
import workflowSettings from './workflowSettings';

export const fr = {
  inbox,
  common,
  format,
  editor,
  settings,
  integrations,
  boardSettings,
  board,
  docs,
  stats,
  nav,
  git,
  conv,
  composer,
  runs,
  devpage,
  shell,
  errors,
  accounts,
  handover,
  heat,
  features,
  mcp,
  memory,
  backups,
  onboarding,
  discover,
  inboxViews,
  vectors,
  branches,
  plan,
  prs,
  envs,
  repos,
  subagents,
  bgtasks,
  wizard,
  voice,
  workflowSettings,
  share,
  workflow,
  plugins,
  scheduled,
  overseer,
  team,
  teamBoard,
  remote,
  servers,
  phone,
  spaces,
  moves,
  teamProjects,
  teamDocs,
  teamCosts,
  teamPlugins,
} as const;

export type Catalog = typeof fr;
