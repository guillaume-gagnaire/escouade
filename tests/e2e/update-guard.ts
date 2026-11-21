// The guard of the end-to-end tests against the real updates of Escouade (ESC-543): an app of the tests never reads the real
// latest.json, never downloads a real version, never runs an installer. A real installer would install in the folder of the
// app of the machine's user (`%LOCALAPPDATA%\Escouade`), which its Restart Manager would close first. The fixture reads the
// app's log at the end of every test (`updateTrouble`): what it says there fails the test.

/** An address on this machine: a test's server, the only one a test's app may get an update from (`updates::test_server`). */
export function onThisMachine(address: string): boolean {
  let host: string;
  try {
    host = new URL(address).hostname.replace(/^\[|\]$/g, '').toLowerCase();
  } catch {
    return false;
  }
  if (host === 'localhost' || host === '::1') return true;
  return /^127\.\d{1,3}\.\d{1,3}\.\d{1,3}$/.test(host);
}

/**
 * What the app's log (`app.log`) says of an update that does not belong in a test: a check that read an address not on this
 * machine (the real latest.json), an install started. An installer the app refused (a test server's latest.json that
 * points elsewhere) is not one: nothing went for it. Empty when there is nothing.
 */
export function updateTrouble(log: string): string[] {
  const found: string[] = [];
  for (const line of log.split(/\r?\n/)) {
    const read = /update check reads (.+)$/.exec(line);
    if (read && read[1].split(', ').some((a) => !onThisMachine(a.trim()))) found.push(line);
    else if (/installing Escouade /.test(line)) found.push(line);
  }
  return found;
}
