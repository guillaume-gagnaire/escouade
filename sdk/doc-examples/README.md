# The examples of the documentation

Every example of code in `website/content/docs/{fr,en}/plugins/` that is not taken from a plugin comes from these two plugins, which do the same thing in TypeScript (`ts/`) and in Rust (`rust/`). A page quotes their lines under their path, and tests hold the rest:

- `website/tests/docs.test.ts`: a block under a path is that file, or lines of it, as they are;
- `doc-examples.test.ts`: no block of TypeScript or Rust without a path, every fragment of a manifest valid, `ts/` typechecked;
- `sdk/scripts/template-ci.mjs` (CI): `rust/` passes `cargo test`, `cargo check --target wasm32-wasip1`, `escouade-plugin validate` and the bench, on the crates of crates.io; `ts/` is bundled, validated and benched.

They are not published. Change a line here, and copy it again into the pages.
