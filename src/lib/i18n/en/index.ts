// The English catalog: each zone is checked against the French one where it is written (`defineZone`), and the
// whole of it here, so that a zone left out fails to compile too.
import { defineCatalog } from '../types';
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
import teamBreakers from './teamBreakers';
import remote from './remote';
import servers from './servers';
import phone from './phone';
import spaces from './spaces';
import mobile from './mobile';
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

export const en = defineCatalog({
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
  teamBreakers,
  remote,
  servers,
  phone,
  spaces,
  mobile,
  moves,
  teamProjects,
  teamDocs,
  teamCosts,
  teamPlugins,
});
