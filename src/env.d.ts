// The constants of a build (vite.config.ts, vite.web.config.ts).
interface ImportMetaEnv {
  /** The browser's build (vite.web.config.ts, BC-1): `true` there, absent elsewhere. */
  readonly ESCOUADE_WEB?: boolean;
}
