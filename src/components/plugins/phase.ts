// Where a plugin stands, in words: its state as loaded first (switched off, disabled by Escouade, its manifest broken, no
// program), then its process's (ESC-203: running, restarted, stopped after three crashes…).
import { t } from '../../lib/i18n';
import type { PluginRun, PluginView } from '../../lib/plugins';

export function stateText(p: PluginView): string | null {
  if (p.state.kind === 'invalid') return t('plugins.state.invalid');
  if (p.state.kind === 'disabled') return t('plugins.state.disabled');
  if (!p.choices.enabled) return t('plugins.state.off');
  if (p.declarative) return t('plugins.state.declarative');
  return null;
}

export function phaseText(status: PluginRun['status'], p: PluginView): string {
  const loaded = stateText(p);
  if (loaded) return loaded;
  if (status.phase === 'stopped') {
    const stop = status.stop;
    if (stop?.kind === 'crashed') return t('plugins.state.crashed', { count: stop.times ?? 3 });
    if (stop?.kind === 'memory') return t('plugins.state.memory');
    if (stop?.kind === 'refused') return t('plugins.state.refused', { reason: stop.reason ?? '' });
    return t('plugins.state.stopped');
  }
  return t(`plugins.state.${status.phase}`);
}
