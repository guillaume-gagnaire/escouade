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
import integrations from './integrations';
import mcp from './mcp';
import memory from './memory';
import codeindex from './codeindex';
import nav from './nav';
import plan from './plan';
import prs from './prs';
import repos from './repos';
import runs from './runs';
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

export const fr = {
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
  shell,
  errors,
  accounts,
  handover,
  mcp,
  memory,
  codeindex,
  branches,
  plan,
  prs,
  repos,
  subagents,
  bgtasks,
  wizard,
  voice,
  workflowSettings,
  share,
  workflow,
  plugins,
} as const;

export type Catalog = typeof fr;
