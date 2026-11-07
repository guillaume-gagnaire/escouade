// `proc/*`.
import type { Plugin } from '@escouade-dev/plugin';

// Runs a line of `permissions.commands` in a project: what it writes comes to the function, `exited` says how it ended.
export function status(p: Plugin, project: string) {
  return p.runProgram({ command: 'git', args: ['status', '--short'], dir: 'project', project }, (data, stream) =>
    p.log.info(`${stream}: ${data}`),
  );
}

// What it reads on its input; `true` ends that input.
export async function input(p: Plugin) {
  const program = await p.runProgram({ command: 'git', args: ['log', '--oneline'] });
  await program.write('yes\n', true);
  return program.exited;
}

export async function stop(p: Plugin) {
  const program = await p.runProgram({ command: 'git', args: ['log', '--oneline'] });
  await program.kill();
}
