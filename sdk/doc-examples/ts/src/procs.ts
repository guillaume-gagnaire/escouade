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

// Runs `git --version` and answers what it wrote and how it ended: `data` is text, a piece of the output, not a line.
export async function version(p: Plugin) {
  let stdout = '';
  let stderr = '';
  const program = await p.runProgram({ command: 'git', args: ['--version'] }, (data, stream) => {
    if (stream === 'stderr') stderr += data;
    else stdout += data;
  });
  const code = await program.exited;
  return { stdout: stdout.trimEnd(), stderr, code };
}
