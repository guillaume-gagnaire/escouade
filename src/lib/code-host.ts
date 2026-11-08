// A project's code host (ESC-390): the sidebar's « PR » tab and the board's « Environnements » show only when the
// project's remote is on a host whose plugin is on, and that host reads pull requests, deployments.

import type { CodeHost } from './types';

/** The host has the tab: it reads pull requests (`pullRequests`), deployments (`deployments`). None while not read. */
export const hasTab = (host: CodeHost | null | undefined, what: 'pullRequests' | 'deployments') => !!host?.[what];
