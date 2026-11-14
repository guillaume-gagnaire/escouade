// Écrit un google-services.json FACTICE pour un paquet Android : aucun projet Firebase n'existe derrière (le ticket
// interdit d'en créer un). Il suffit à compiler le SDK de FCM et à initialiser FirebaseApp, donc à recevoir un message
// injecté par adb (voir bench/android-push.mjs) ; l'inscription auprès de FCM (le jeton) échoue, et c'est attendu.
//
//   node mobile-study/shared/push/fake-google-services.mjs <paquet> <fichier de sortie>
import { writeFileSync } from 'node:fs';

const [pkg, out] = process.argv.slice(2);
if (!pkg || !out) throw new Error('usage : fake-google-services.mjs <paquet> <sortie>');
const json = {
  project_info: { project_number: '000000000000', project_id: 'esc445-factice-sans-projet', storage_bucket: 'esc445-factice-sans-projet.invalid' },
  client: [
    {
      client_info: { mobilesdk_app_id: '1:000000000000:android:0000000000000445', android_client_info: { package_name: pkg } },
      oauth_client: [],
      // 39 caractères, la forme d'une clé d'API ; elle n'ouvre rien.
      api_key: [{ current_key: 'AIzaSyFAKE-esc445-not-a-real-key-000000' }],
      services: { appinvite_service: { other_platform_oauth_client: [] } },
    },
  ],
  configuration_version: '1',
};
writeFileSync(out, JSON.stringify(json, null, 2) + '\n');
console.log(`google-services.json factice pour ${pkg} : ${out}`);
