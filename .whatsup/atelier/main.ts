// The atelier's entry: the tests' fake backend answers the interface's commands from a named state (etats.ts).
import '../../src/app.css';
import { mount } from 'svelte';
import App from '../../src/App.svelte';
import { setLang } from '../../src/lib/i18n';
import { app } from '../../src/lib/state.svelte';
import type { InitialState, UiEvent } from '../../src/lib/types';
import { CLAUDE_ENGINE, fakeBackend, resetApp, SETTINGS } from '../../src/test/ipc';
import { ETATS } from './etats';

const q = new URLSearchParams(location.search);
const nom = q.get('etat') || 'defaut';
const etat = ETATS[nom] ?? ETATS.defaut;
const theme = q.get('theme') === 'sombre' ? 'dark' : 'light';
// ?lang=en: the interface in English, as a note's English captures show it.
const lang = q.get('lang') === 'en' ? 'en' : 'fr';

let channel: { onmessage: (e: UiEvent) => void } | null = null;
const initial: InitialState = {
  projects: etat.projects,
  agents: etat.agents ?? [],
  tickets: etat.tickets ?? [],
  ui: { activeProject: etat.projects[0]?.id ?? null, view: 'project', selectedAgent: {}, layout: '', ...etat.ui },
  settings: { ...SETTINGS, theme, ...etat.settings },
  usage: { todayCost: 4.12, accounts: [], current: { claude: 'principal' } },
  git: etat.git ?? {},
  shells: [],
  terminals: [],
  claudeFound: true,
  version: '2.2.16',
  models: [],
  engines: [CLAUDE_ENGINE],
};
fakeBackend({
  subscribe: (args: any) => {
    channel = args.channel;
    return initial;
  },
  get_conversation: (a: any) => etat.conversations?.[a.agentId ?? a.id] ?? [],
  git_branches: () => ['main'],
  ...etat.handlers,
});
resetApp();
app.ready = false;
setLang(lang);
document.documentElement.dataset.theme = theme;
mount(App, { target: document.getElementById('app')! });

// Drawn: the state's own gestures (clicks, events), then two frames.
const frame = () => new Promise((r) => requestAnimationFrame(() => r(null)));
(async () => {
  for (let i = 0; i < 100 && !app.ready; i++) await new Promise((r) => setTimeout(r, 50));
  await etat.apres?.({ emit: (e: UiEvent) => channel?.onmessage(e) });
  await new Promise((r) => setTimeout(r, 400));
  await frame();
  await frame();
  (window as any).__whatsupPret = true;
})();
