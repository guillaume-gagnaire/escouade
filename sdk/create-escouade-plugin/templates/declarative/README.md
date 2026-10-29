# {{name}}

A declarative plugin of [Escouade](https://escouade.dev): a theme, described in `escouade-plugin.json`. Nothing runs.

```sh
npm install
npm run validate    # the manifest, field by field
npm run bench       # the conformance bench of Escouade (for a declarative plugin, its manifest)
npm run archive     # the .tar.gz and its SHA-256, in packs/
```

To try it in Escouade, load this folder in Settings › Plugins (developer mode). The manifest's fields: https://escouade.dev/en/docs/plugins/manifest
