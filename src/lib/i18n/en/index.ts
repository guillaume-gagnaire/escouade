// The English catalog: each zone is checked against the French one where it is written (`defineZone`), and the
// whole of it here, so that a zone left out fails to compile too.
import { defineCatalog } from '../types';
import accounts from './accounts';
import bgtasks from './bgtasks';
import board from './board';
import boardSettings from './boardSettings';
import branches from './branches';
import fetchZone from './fetch';
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
import integrations from './integrations';
import inbox from './inbox';
import mcp from './mcp';
import memory from './memory';
import inboxViews from './inboxViews';
import vectors from './vectors';
import nav from './nav';
import plan from './plan';
import workspace from './workspace';
import prs from './prs';
import envs from './envs';
import repos from './repos';
import runs from './runs';
import devpage from './devpage';
import settings from './settings';
import share from './share';
import shell from './shell';
import stats from './stats';
import subagents from './subagents';
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
  mcp,
  memory,
  inboxViews,
  vectors,
  branches,
  fetch: fetchZone,
  plan,
  workspace,
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
});
