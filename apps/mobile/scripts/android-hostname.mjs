// SR1-09 (ESC-631): the Android app's WebView gets an origin of its own, `https://app.escouade.invalid`, in place of
// `https://localhost`, which any page served on port 443 of a machine of the tailnet has too. Capacitor has one
// `server.hostname` for both platforms, and iOS keeps `capacitor://localhost` (no browser makes that scheme): this
// hook of the Capacitor CLI (`capacitor:copy:after`, run by `cap copy` and `cap sync`) writes the hostname in the
// Android copy of the config alone. `.invalid` is reserved (RFC 6761): no real site, no DNS answer, ever has it.
//
// node scripts/android-hostname.mjs [path of the copied capacitor.config.json]
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

export const ANDROID_HOSTNAME = 'app.escouade.invalid';

/** The config `text` with the Android hostname: the rest as it was. */
export function withAndroidHostname(text) {
  const config = JSON.parse(text);
  config.server = { ...(config.server ?? {}), hostname: ANDROID_HOSTNAME };
  return JSON.stringify(config, null, 2) + '\n';
}

const here = dirname(fileURLToPath(import.meta.url));
const main = process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1];
if (main) {
  const platform = process.env.CAPACITOR_PLATFORM_NAME;
  const asked = process.argv[2];
  if (!asked && platform && platform !== 'android') process.exit(0);
  const path = asked ?? join(here, '..', 'android', 'app', 'src', 'main', 'assets', 'capacitor.config.json');
  if (!existsSync(path)) {
    console.error(`android-hostname: ${path} missing (run cap sync android first)`);
    process.exit(asked ? 1 : 0);
  }
  writeFileSync(path, withAndroidHostname(readFileSync(path, 'utf8')));
  console.log(`android-hostname: the Android WebView is served from https://${ANDROID_HOSTNAME}`);
}
