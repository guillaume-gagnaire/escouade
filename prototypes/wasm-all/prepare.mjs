#!/usr/bin/env node
// Makes the `generated/` folders of the guest from the repository: the real kit (but `serve`), the real plugins Linear and
// Codex. Each change is printed: it is the whole of what their port costs (docs/PLUGINS.md §8.9). They are copies, not
// `#[path]`s to the repository, so that a source's path holds no `..`, as a plugin in plugins/<id>/ has none.
import { cpSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, '..', '..');

const turn = (run) => `
/// The host's turn: the plugin kept at the first one, then what came.
#[export_name = "escouade-turn"]
pub extern "C" fn turn() {
    static START: std::sync::Once = std::sync::Once::new();
    START.call_once(|| {
        run();
    });
    ${run};
}
`;

function make(to, from, changes) {
  rmSync(to, { recursive: true, force: true });
  mkdirSync(to, { recursive: true });
  for (const [file, list] of Object.entries(changes)) {
    let text = readFileSync(join(from, file), 'utf8').replace(/\r\n/g, '\n');
    for (const [a, b] of list) {
      if (!(typeof a === 'string' ? text.includes(a) : a.test(text))) throw new Error(`${file}: ${a} is no longer there`);
      text = text.replace(a, b);
      console.log(`${file}: ${String(a).slice(0, 70)}`);
    }
    writeFileSync(join(to, file), text);
  }
}

make(join(here, 'guest', 'kit', 'src', 'generated'), join(repo, 'crates', 'escouade-plugin-kit', 'src'), {
  'errors.rs': [],
  'forge.rs': [],
  'hosts.rs': [],
  'http.rs': [],
  'i18n.rs': [],
  'text.rs': [],
});

make(join(here, 'guest', 'linear', 'generated'), join(repo, 'plugins', 'linear', 'src'), {
  'lib.rs': [[/$/, turn('escouade_plugin_kit::serve::turn()')]],
});

const codex = join(repo, 'plugins', 'codex');
cpSync(join(codex, 'escouade-plugin.json'), join(here, 'guest', 'codex', 'escouade-plugin.json'));
make(join(here, 'guest', 'codex', 'generated'), join(codex, 'src'), {
  'lib.rs': [
    [
      /pub fn run\(\) -> i32 \{[\s\S]*?\n\}\n/,
      `pub fn run() -> i32 {
    let mut plugin = Plugin::default();
    escouade_turn::host(json!({ "harnesses": ["codex"] }), move |m, params| plugin.handle(m, params));
    0
}
`,
    ],
    ['use escouade_plugin_protocol::{serve, Refusal};', 'use escouade_plugin_protocol::Refusal;'],
    [/$/, turn('escouade_turn::sync_turn()')],
  ],
  'client.rs': [
    ['u64::from(std::process::id()) << 20', 'escouade_turn::instance_id() << 20'],
    [
      'home.and_then(|h| std::fs::read_to_string(h.join("config.toml")).ok())',
      'home.and_then(|h| escouade_turn::fs_read(&h.join("config.toml")))',
    ],
  ],
  'translate.rs': [],
  'words.rs': [],
  'prices.rs': [],
});
