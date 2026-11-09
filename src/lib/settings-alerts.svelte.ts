// The amber dots of the settings bar (R8 of the 2.2 redesign, design/settings-2-2/PROPOSITION.md §2, principle 6):
// a page that has a problem says so beside its name, its reason in the tooltip. A harness without an account (Comptes et
// harness), a token expired or unread (Services connectés), a plugin stopped or disabled by Escouade (Plugins), a project
// that no account of its harness serves (the project, and its page « Projet » where its account is chosen).

import { accountsTabEngines, engineOfProject, hasAccount, projectsWithoutAccount } from './accounts';
import { engineName } from './engines';
import { t } from './i18n';
import { api } from './ipc';
import { pluginsApi, type PluginRun, type PluginView } from './plugins';
import { plugins } from './plugins-view.svelte';
import { errorText } from './refusal';
import { app } from './state.svelte';
import type { AccountView, ForgeAccountView, ForgesView } from './types';

/** What makes the dots: the state of the app, and what the window reads when it opens. */
export interface AlertsInput {
  /** The harnesses offered in Comptes et harness, and whether each has an account. */
  harnesses: { name: string; hasAccount: boolean }[];
  /** The projects no account of their harness serves, with its name. */
  orphans: { id: string; harness: string }[];
  forges: Pick<ForgeAccountView, 'label' | 'status'>[];
  /** The ticket systems' accounts. */
  ticketAccounts: Pick<AccountView, 'label' | 'connected' | 'unread'>[];
  plugins: Pick<PluginView, 'id' | 'name' | 'state' | 'choices' | 'declarative'>[];
  /** Each plugin's process, as last read. */
  runs: Record<string, PluginRun['status']>;
}

/** Why a plugin stands still, or nothing when it is meant to (switched off, no program) or runs. */
function pluginProblem(p: AlertsInput['plugins'][number], run: PluginRun['status'] | undefined): string | null {
  if (p.state?.kind === 'invalid') return t('plugins.state.invalid');
  if (p.state?.kind === 'disabled') return t('plugins.state.disabled');
  if (!p.choices?.enabled || p.declarative || run?.phase !== 'stopped') return null;
  const stop = run.stop;
  if (stop?.kind === 'crashed') return t('plugins.state.crashed', { count: stop.times ?? 3 });
  if (stop?.kind === 'memory') return t('plugins.state.memory');
  if (stop?.kind === 'refused') return t('plugins.state.refused', { reason: stop.reason ?? '' });
  return null;
}

/**
 * The reasons of each entry of the bar that has a problem: by page of Escouade (`accounts`), by project (`project:<id>`)
 * and by page of a project (`<id>:project`), as the bar keys its entries.
 */
export function alertsOf(input: AlertsInput): Record<string, string[]> {
  const out: Record<string, string[]> = {};
  const add = (key: string, reason: string) => (out[key] ??= []).push(reason);
  for (const h of input.harnesses) if (!h.hasAccount) add('accounts', t('settings.nav.alerts.noAccount', { harness: h.name }));
  for (const o of input.orphans) {
    const reason = t('settings.nav.alerts.projectNoAccount', { harness: o.harness });
    add(`project:${o.id}`, reason);
    add(`${o.id}:project`, reason);
  }
  for (const f of input.forges) {
    if (f.status?.health === 'expired') add('services', t('integrations.forges.expired', { label: f.label }));
    if (f.status?.health === 'unread') add('services', t('settings.nav.alerts.unread', { label: f.label }));
  }
  for (const a of input.ticketAccounts) if (a.connected && a.unread) add('services', t('settings.nav.alerts.unread', { label: a.label }));
  for (const p of input.plugins) {
    const problem = pluginProblem(p, input.runs[p.id]);
    if (problem) add('plugins', t('settings.nav.alerts.plugin', { name: p.name, state: problem }));
  }
  return out;
}

/** What the window reads when it opens: the code hosts' accounts as last checked, each plugin's process. */
class SettingsAlerts {
  forges = $state<ForgeAccountView[]>([]);
  runs = $state<Record<string, PluginRun['status']>>({});

  /** The code hosts' view, as Services connectés reads it again (its check finds a token expired). */
  takeForges(v: ForgesView | null | undefined) {
    if (v) this.forges = v.accounts;
  }

  async load() {
    api.forges().then(
      (v) => this.takeForges(v),
      (e) => console.warn(errorText(e)),
    );
    // As the app read them (the Plugins page and view read them again); their processes, now. The dots wait for no
    // list that cannot be read.
    if (!plugins.loaded) {
      try {
        await plugins.load();
      } catch (e) {
        console.warn(errorText(e));
        return;
      }
    }
    const running = plugins.list.filter((p) => p.state?.kind === 'ready' && p.choices?.enabled && !p.declarative);
    const read = await Promise.all(
      running.map((p) =>
        pluginsApi.run(p.id).then(
          // A process not read (nothing answered) is no dot.
          (r) => (r?.status ? ([p.id, r.status] as const) : null),
          () => null,
        ),
      ),
    );
    this.runs = Object.fromEntries(read.filter((r) => r !== null));
  }

  /** The dots, as the app stands now. */
  get all(): Record<string, string[]> {
    return alertsOf({
      harnesses: accountsTabEngines().map((e) => ({ name: engineName(e), hasAccount: hasAccount(e) })),
      orphans: projectsWithoutAccount().map((p) => ({ id: p.id, harness: engineName(engineOfProject(p)) })),
      forges: this.forges,
      ticketAccounts: app.accounts,
      plugins: plugins.list,
      runs: this.runs,
    });
  }
}

export const settingsAlerts = new SettingsAlerts();
