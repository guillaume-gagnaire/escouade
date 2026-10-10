// The French catalog, source of the keys: `t` accepts its paths, the English catalog must have the same.
// A zone belongs to one part of the interface (one task at a time writes it); `common` and `format` are shared.
import accounts from './accounts';
import board from './board';
import boardSettings from './boardSettings';
import branches from './branches';
import common from './common';
import composer from './composer';
import conv from './conv';
import editor from './editor';
import errors from './errors';
import format from './format';
import git from './git';
import integrations from './integrations';
import mcp from './mcp';
import nav from './nav';
import runs from './runs';
import settings from './settings';
import shell from './shell';
import stats from './stats';

export const fr = {
  common,
  format,
  editor,
  settings,
  integrations,
  boardSettings,
  board,
  stats,
  nav,
  git,
  conv,
  composer,
  runs,
  shell,
  errors,
  accounts,
  mcp,
  branches,
} as const;

export type Catalog = typeof fr;
