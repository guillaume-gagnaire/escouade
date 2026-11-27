// The model, effort and mode an agent can be set to, as its harness offers them: the composer's chips and the canvas's
// card show the same lists.

import {
  capsOf,
  effortsOf,
  engineName,
  fromConfig,
  modelCost,
  modelMenu,
  modelsOf,
  modelTakes,
  modesOf,
  readingModels,
  searchesModels,
  speakerOf,
} from './engines';
import { t } from './i18n';
import { api } from './ipc';
import { EFFORTS, MODES, modeLabel, modeTitle, modelOptions } from './models';
import { app } from './state.svelte';
import type { Agent } from './types';

export type OptionChoice = {
  value: string;
  label: string;
  detail?: string;
  title?: string;
  disabled?: boolean;
  reason?: string;
  favorite?: boolean;
  group?: string;
};

export function modelChoicesOf(agent: Agent): OptionChoice[] {
  // A harness that lists many models, or those of its providers (OpenCode): the favorites first, then a group by family
  // (by provider); a favorite says its provider, out of its group.
  const choices: OptionChoice[] = searchesModels(agent)
    ? modelMenu(agent).map((m) => {
        const cost = modelCost(agent, m.id);
        const detail =
          (m.favorite ? m.provider : undefined) ?? cost?.label ?? (fromConfig(agent, m.id) ? t('composer.model.fromConfig') : undefined);
        return { value: m.id, label: m.label, detail, title: cost?.title, favorite: m.favorite, group: m.group };
      })
    : // Claude Code's aliases (`default`, `opus`…) are read for its own agents only: Cursor's Auto is `default` too.
      modelOptions((agent.engine ?? 'claude') === 'claude' ? app.models : [], modelsOf(agent)).map((o) => {
        // What it costs beside the others, when its harness bills by model (Copilot: `×3`).
        const cost = modelCost(agent, o.value);
        if (cost) return { ...o, detail: cost.label, title: cost.title };
        // Codex's model of its config.toml, which it does not list (ESC-186).
        if (fromConfig(agent, o.value)) return { ...o, detail: t('composer.model.fromConfig') };
        // Codex's stand-in while its account's list is being read (ESC-186).
        return o.value && readingModels(agent) ? { ...o, detail: t('composer.model.reading') } : o;
      });
  // A harness with no stand-in (OpenCode) while its account's list is being read: one greyed line that says so (ESC-186).
  return choices.length || !readingModels(agent) ? choices : [{ value: '', label: t('composer.model.reading'), disabled: true }];
}

/** Its harness's efforts for its model; none when the model takes none (Haiku). */
export function effortChoicesOf(agent: Agent): OptionChoice[] {
  if (!modelTakes(agent, agent.model, 'effort')) return [];
  return effortsOf(agent, EFFORTS).map((ef) => ({ value: ef.value, label: ef.label, detail: ef.title }));
}

/** The five modes keep their places: one its harness lacks is greyed, with why (choice 17 of the 2.0 design). */
export function modeChoicesOf(agent: Agent): OptionChoice[] {
  const caps = capsOf(agent.engine);
  const takes = new Set(modesOf(agent, MODES, agent.mode).map((m) => m.value));
  return MODES.map((md) => {
    // « Bypass » its harness has, not allowed in the settings (SEC-09).
    if (md.value === 'bypassPermissions' && !takes.has(md.value) && caps?.modes.some((m) => m.id === md.value))
      return { value: md.value, label: md.label, disabled: true, reason: t('composer.mode.bypassNotAllowed') };
    if (!takes.has(md.value)) {
      const harness = engineName(agent.engine);
      // A harness of one mode alone runs in it, whatever is asked (ESC-489): the others say so.
      const only = caps?.modes.length === 1 ? caps.modes[0].id : null;
      const reason = only
        ? t('composer.mode.only', { harness, mode: modeLabel(only) })
        : md.value === 'auto'
          ? t('composer.mode.missingAuto', { harness })
          : t('composer.mode.missing', { harness, mode: md.label });
      return { value: md.value, label: md.label, disabled: true, reason };
    }
    const unavailable = md.value === 'auto' && !modelTakes(agent, agent.model, 'auto');
    const meaning = caps?.modes.find((m) => m.id === md.value)?.meaning ?? modeTitle(md.value, speakerOf(agent.engine));
    return {
      value: md.value,
      label: md.label,
      detail: unavailable ? t('composer.mode.autoUnavailableDetail') : meaning,
      title: unavailable ? t('composer.mode.autoUnavailableTitle') : meaning,
      disabled: unavailable,
    };
  });
}

/** Sets the agent to it at once, and tells the core. */
export function setAgentOption(agentId: string, o: { model?: string; effort?: string; mode?: string }) {
  const a = app.agents[agentId];
  if (a) Object.assign(a, o);
  if (o.mode === 'bypassPermissions') app.toast(t('composer.mode.bypassToast', { engine: speakerOf(a?.engine) }), 'info');
  app.run(api.setAgentOptions(agentId, o));
}
