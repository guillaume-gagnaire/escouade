# The seed of the plugins

What the installer carries (`bundle.resources` of `tauri.conf.json`) so that Escouade works from its first start, with no
network: for each entry of `plugins/seed.json`, the files the store published, the same bytes and the same signature
(`<id>/version.json`, `version.json.minisig`, `archive.tar.gz`, `plugin.wasm`), and the revocation list of the day
(`revocations.json` and its `.minisig`). See `docs/PLUGINS.md` §8.7.

Nothing but this file is committed:

- `release.yml` fills this folder with `store/scripts/seed.ts fetch`, which checks each signature and fails otherwise;
- a build of development may fill it with `sh scripts/seed-dev.sh`: modules built here, without a signature, which
  only a build of development takes. It writes its own `seed.json` beside them; `ESCOUADE_SEED_DIR` points at the folder.
