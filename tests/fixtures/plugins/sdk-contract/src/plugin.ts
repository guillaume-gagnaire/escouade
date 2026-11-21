// A plugin written for @escouade-dev/plugin 0.5 that takes the contract of Escouade 2.2.5 (ESC-531): on a ticket that
// moves, the tunnel of its project, from the VPN profile the project chose (its fields, its password, its configuration on
// several lines), written in its temporary folder and run by programs whose entries have jokers, five at once at most
// (`limits.processes`). Git stands in for OpenVPN: every system of the CI has it.
//
// The bench plays its bundle in WASM (crates/escouade-plugins/src/plugins_bench_wasm_tests.rs); the SDK's fake Escouade
// plays this file (sdk/plugin/src/both.test.ts), which checks dist/plugin.mjs is the build of src/main.ts:
//
//   npx esbuild tests/fixtures/plugins/sdk-contract/src/main.ts --bundle --platform=node --format=esm --target=node20 \
//     --outfile=tests/fixtures/plugins/sdk-contract/dist/plugin.mjs
import { createPlugin, type Program } from '../../../../../sdk/plugin/src/index';

export const plugin = createPlugin();

/** One tunnel at a time: a fact that comes while one is under way waits for none. */
let underWay = false;

plugin.onEvent('ticket.column', async (fact, p) => {
  // A project's fact: the tunnel of that project.
  if (!fact.project) return;
  if (underWay) return p.log.info('a tunnel is under way');
  underWay = true;
  try {
    await tunnel(fact.project.id);
  } finally {
    underWay = false;
  }
});

async function tunnel(project: string) {
  const p = plugin;
  // The profile the project chose: only the user chooses it, a .escouade.yml may only propose one.
  const profile = await p.profiles.of(project, 'profile');
  if (!profile) return p.log.warn('no VPN profile for this project: no tunnel');
  const names = (await p.profiles.list('vpn')).map((r) => r.name);
  const fields = await p.profiles.get('vpn', profile);
  const settings = await p.settings(project);
  const ovpn = await p.secret('ovpn', { profile });
  const password = await p.secret('password', { profile });
  const notes = await p.secret('notes');
  // What it says of them, never a value.
  p.log.info(
    `profile ${names.join(', ')}: ${fields?.server} in ${fields?.proto}, ${ovpn?.split('\n').length ?? 0} lines of configuration, ` +
      `${password ? 'a' : 'no'} password, ${notes ? 'notes' : 'no notes'}, routes ${settings.routes}`,
  );
  await p.fs.write({ root: 'temp', path: 'tunnel.ovpn', text: ovpn ?? '' });

  // A program that lives, then one that names it by its pid ({pid}), a number ({int}), files of its two folders.
  const programs: Program[] = [];
  const run = async (args: string[]) => {
    const program = await p.runTyped({ command: 'git', args });
    programs.push(program);
    return program;
  };
  // Those three read their input until it closes.
  const batch = await run(['cat-file', '--batch']);
  const reading = [
    batch,
    await run(['column', '--mode=plain', '--width', String(batch.pid)]),
    await run(['column', '--mode=plain', '--padding', '2']),
  ];
  const checks = [await run(['hash-object', 'tunnel.ovpn']), await run(['hash-object', 'rules.txt'])];
  // Five at once: the sixth waits for an end.
  await Promise.all(checks.map((c) => c.exited));
  await run(['--no-pager', 'version']);
  for (const program of reading) await program.close();
  const codes = await Promise.all(programs.map((program) => program.exited));
  p.log.info(`tunnel checked: ${codes.join(' ')}`);
}
