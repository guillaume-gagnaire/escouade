import { defineZone } from '../types';

// Empty on purpose: the backend words its errors itself, in the language of the interface (`tr!`, src-tauri/src/i18n.rs),
// and the one it sends as a code (`NOT_FOUND:<path>`) is worded by the `editor` zone. This zone is kept for an error code
// the window would one day have to word.
export default defineZone('errors', {});
