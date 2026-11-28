// First: the `<style>` elements created from now on carry the page's nonce (lib/style-nonce.ts).
import './lib/style-nonce-boot';
import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import { earlyAppearance } from './lib/appearance';
import { guardFileDrops } from './lib/attachments';
import { guardLinks } from './lib/links';
import { earlyPluginTheme } from './lib/plugin-theme.svelte';
import { markWebKit } from './lib/webkit';
import { app } from './lib/state.svelte';
import { seenAtStart, showWhatsUp } from './lib/whats-up';

// The app is a desktop window: keep the browser's own context menu out of the way,
// except in text fields where copy/paste matters.
window.addEventListener('contextmenu', (e) => {
  const t = e.target as HTMLElement;
  if (!t.closest('input, textarea, .md, .xterm')) e.preventDefault();
});

// Files are dropped on the composer only: anywhere else the WebView would open them.
guardFileDrops(window);

// A link never takes the window away from the app: the ones that go outside open in the browser.
guardLinks(document);

// What's up ?: what counts as seen is known at once (and kept at the very first start), before the settings are read.
const whatsUpSeen = seenAtStart();
// Its window comes once the interface knows its language (the settings'): its notes and its words are in that one. In
// a browser without the app (`?whats-up=apercu`), the browser's language at once. The page escouade-server serves
// (vite.web.config.ts) is the app: it waits as the window does, and never looks for Tauri (the test falls away there).
const inApp = !!import.meta.env.ESCOUADE_WEB || '__TAURI_INTERNALS__' in window;
const whatsUpReady = setInterval(() => {
  if (inApp && !app.ready) return;
  clearInterval(whatsUpReady);
  showWhatsUp(document.querySelector<HTMLTemplateElement>('template#whats-up'), whatsUpSeen);
}, 50);

// The theme the page kept, before the first render: the settings say it again a moment later.
earlyAppearance();
// And the colours of the plugin's theme it showed (Réglages › Général › Apparence), until the plugins are read.
earlyPluginTheme();

// Under WebKit (Linux, macOS), what crashes its web process is turned off (app.css, `:root.webkit`).
markWebKit();

mount(App, { target: document.getElementById('app')! });
