// La part de code reprise d'Escouade, maquette par maquette : les lignes écrites pour elle (hors fichiers générés),
// celles de l'étude qu'elle partage avec une autre (l'interface Svelte), et celles d'Escouade qu'elle importe telles
// quelles. Une ligne compte si elle n'est ni vide ni un commentaire seul.
//
//   node mobile-study/bench/code-share.mjs   (écrit results/code-share.json)
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const study = join(here, '..');
const repo = join(study, '..');

function lines(file) {
  const text = readFileSync(file, 'utf8');
  let n = 0;
  let inBlock = false;
  for (const raw of text.split('\n')) {
    const l = raw.trim();
    if (inBlock) {
      if (l.includes('*/')) inBlock = false;
      continue;
    }
    if (!l || l.startsWith('//') || l.startsWith('#') || l.startsWith('<!--')) continue;
    if (l.startsWith('/*') || l.startsWith('/**')) {
      if (!l.includes('*/')) inBlock = true;
      continue;
    }
    n++;
  }
  return n;
}
const sum = (base, files) => Object.fromEntries(files.map((f) => [f, lines(join(base, f))]));
const total = (o) => Object.values(o).reduce((s, v) => s + v, 0);

// Ce que l'interface Svelte importe d'Escouade (le catalogue des textes à part : il est entier, mais la maquette n'en
// lit que quelques clés).
const escouadeUi = sum(repo, ['src/lib/markdown.ts', 'src/lib/diff.ts', 'src/components/DiffView.svelte', 'src/app.css']);
const svelteShared = sum(study, [
  'svelte-ui/src/App.svelte',
  'svelte-ui/src/Message.svelte',
  'svelte-ui/src/server.ts',
  'svelte-ui/src/platform.ts',
  'svelte-ui/src/texts.ts',
  'svelte-ui/src/main.ts',
  'svelte-ui/src/mobile.css',
]);

const techs = {
  tauri: {
    own: sum(study, ['tauri/src/platform.ts', 'tauri/src/main.ts', 'tauri/src-tauri/src/lib.rs', 'tauri/src-tauri/src/main.rs', 'tauri/vite.config.ts', 'tauri/index.html']),
    shared: svelteShared,
    escouade: escouadeUi,
    escouadeTexts: 'le catalogue entier (t())',
  },
  capacitor: {
    own: sum(study, [
      'capacitor/src/platform.ts',
      'capacitor/src/main.ts',
      'capacitor/android/app/src/main/java/dev/escouade/study/capacitor/RustCorePlugin.kt',
      'capacitor/android/app/src/main/java/dev/escouade/study/capacitor/MainActivity.java',
      'capacitor/plugins/rust-core/ios/Sources/RustCorePlugin/RustCorePlugin.swift',
      'capacitor/plugins/rust-core/Package.swift',
      'capacitor/vite.config.ts',
      'capacitor/index.html',
    ]),
    shared: svelteShared,
    escouade: escouadeUi,
    escouadeTexts: 'le catalogue entier (t())',
  },
  'react-native': {
    own: sum(study, [
      'react-native/App.tsx',
      'react-native/index.ts',
      'react-native/src/Message.tsx',
      'react-native/src/server.ts',
      'react-native/src/rust.ts',
      'react-native/src/theme.ts',
      'react-native/src/escouade.ts',
      'react-native/src/rune-shim.ts',
      'react-native/metro.config.js',
    ]),
    shared: {},
    escouade: sum(repo, ['src/lib/diff.ts']),
    escouadeTexts: 'le catalogue entier (t())',
  },
  flutter: {
    own: sum(study, [
      'flutter/lib/main.dart',
      'flutter/lib/message.dart',
      'flutter/lib/server.dart',
      'flutter/lib/texts.dart',
      'flutter/lib/theme.dart',
      'flutter/rust/src/api/core.rs',
    ]),
    shared: {},
    escouade: {},
    escouadeTexts: '8 clés exportées en JSON (shared/texts/export-texts.mjs)',
  },
};

const out = {};
for (const [k, v] of Object.entries(techs)) {
  const own = total(v.own);
  const shared = total(v.shared);
  const esc = total(v.escouade);
  out[k] = {
    ownLines: own,
    sharedStudyLines: shared,
    escouadeLines: esc,
    reusedPct: Math.round(((shared + esc) / (own + shared + esc)) * 100),
    escouadeFiles: Object.keys(v.escouade),
    texts: v.escouadeTexts,
    files: { ...v.own, ...v.shared },
  };
  console.log(k, `propres ${own}, partagées ${shared}, Escouade ${esc} → ${out[k].reusedPct} % repris`);
}
writeFileSync(join(study, 'results/code-share.json'), JSON.stringify(out, null, 2) + '\n');
