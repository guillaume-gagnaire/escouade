// `timer/*`, through the standard `setTimeout` and `clearTimeout`.
import type { Plugin } from '@escouade-dev/plugin';

// Other calls go on while this one waits.
export async function later(p: Plugin) {
  await new Promise((resolve) => setTimeout(resolve, 1000));
  p.log.info('one second later');
  return 'rested';
}

// A timeout on anything that waits.
export async function patient(p: Plugin) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const late = new Promise<never>((_, reject) => {
    timer = setTimeout(() => reject(new Error('timed out')), 2000);
  });
  try {
    return await Promise.race([p.secret('token'), late]);
  } finally {
    clearTimeout(timer);
  }
}
