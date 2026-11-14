// Le point d'entrée commun : chaque technique (Tauri 2, Capacitor) le monte avec ce qu'elle sait du système.
import { mount } from 'svelte';
import '../../../src/app.css';
import './mobile.css';
import App from './App.svelte';
import { setLang } from '../../../src/lib/i18n';
import type { Platform } from './platform';

export function start(platform: Platform) {
  setLang('fr');
  document.documentElement.dataset.theme = 'dark';
  return mount(App, { target: document.getElementById('app')!, props: { platform } });
}
