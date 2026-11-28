// The phone's app (MO-1): the app's tokens (src/app.css) then the phone's, the system's theme, language and safe areas,
// a connection to each space paired, the back of the system, and the shell.
import { mount } from 'svelte';
import '$shared/app.css';
import './mobile.css';
import { applyTheme } from '$shared/lib/theme';
import App from './App.svelte';
import { applyAppearance, followSystemTheme, systemTheme } from './lib/appearance';
import { barsFor, onBackButton, onForeground, platformOf } from './lib/native';
import { back, followHistory, goTab, tabOf } from './lib/nav.svelte';
import { allSpaces } from './lib/paired';
import { onSpacesChanged } from './lib/pairing.svelte';
import { pauseSpaces, resumeSpaces, startSpaces } from './lib/phone-spaces.svelte';

const platform = platformOf();
const theme = systemTheme();
applyAppearance(theme, platform);
barsFor(theme);
followSystemTheme((next) => {
  applyAppearance(next, platform);
  barsFor(next);
});
// The views over every space take Escouade's teal (src/lib/theme.ts), as « Partout » on the desktop.
applyTheme(null);

startSpaces(allSpaces());
// A space paired or removed (MO-4): the connections opened again.
onSpacesChanged(() => startSpaces(allSpaces()));
onForeground((active) => (active ? resumeSpaces() : pauseSpaces()));

const asked = tabOf(location.hash);
if (asked) goTab(asked);
onBackButton(back);
followHistory();

mount(App, { target: document.getElementById('app')! });
