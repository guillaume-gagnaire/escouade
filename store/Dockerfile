# The store, as Coolify deploys it on store.escouade.dev: built and tested, then served by Nitro on port 3000.
# Built from the root of the repo (the store reads the site's tokens in website/app/assets/main.css, the manifest's schema
# in schemas/, and its compiler is built from crates/ and src-tauri/escouade-js/): `docker build -f store/Dockerfile .`.
# What the build sees is in Dockerfile.dockerignore.
#
# Coolify hands the application's variables to the build as well (it cannot be told not to): nothing here declares
# or needs one. The tests run without the database's variables, so that they never reach the store's base; and the
# final stage runs no command, so that no variable lands in the image's history.
#
# The compiler of the `run.js` plugins comes first (ESC-328, docs/STORE.md §3.3), built here from the repository: the
# image depends on no release of the app. A change of the store alone finds its stages in the cache.

# The pinned Rust of the repository (src-tauri/rust-toolchain.toml, here for good by RUSTUP_TOOLCHAIN: rustup downloads
# nothing) and what compiles for the plugins' host: wasm32-wasip1 and the pinned Javy, checked by the SHA-256 of its
# archive (`javy_asset` in crates/escouade-plugin-cli/src/wasm.rs).
FROM rust:1.98.0-bookworm@sha256:82150a52ec202c1b14d7817e14516c392bb7f5cfebd88f1ed531cb37ebd39922 AS toolchain
ENV RUSTUP_TOOLCHAIN=1.98.0 ESCOUADE_JAVY=/opt/javy
# libclang: bindgen, for QuickJS's bindings.
RUN apt-get update && apt-get install -y --no-install-recommends libclang-dev \
  && rustup target add wasm32-wasip1 \
  && curl -fsSL -o /tmp/javy.gz https://github.com/bytecodealliance/javy/releases/download/v9.1.0/javy-x86_64-linux-v9.1.0.gz \
  && echo "a68b122d48eb3dfc1b801d4e14c39271fde3638243d3272d206e376ac9189e39  /tmp/javy.gz" | sha256sum -c - \
  && gunzip -c /tmp/javy.gz > /opt/javy \
  && chmod 755 /opt/javy \
  && rm /tmp/javy.gz

# escouade-js, Escouade's Javy plugin: the same bytes as the one CI builds for the app (scripts/escouade-js.sh).
FROM toolchain AS engine
WORKDIR /repo
COPY scripts/escouade-js.sh scripts/
COPY src-tauri/escouade-js src-tauri/escouade-js
RUN sh scripts/escouade-js.sh

# The workspace's manifests alone, an empty crate for each: cargo needs them all to read the workspace and its lock.
# Run again when a crate changes, this stage still gives the same files unless a manifest changed.
FROM toolchain AS manifests
WORKDIR /repo
COPY src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/
COPY crates crates
COPY plugins plugins
RUN find . -type f ! -name Cargo.toml ! -name Cargo.lock -delete \
  && find . -name Cargo.toml -execdir sh -c 'mkdir -p src && : > src/lib.rs' \; \
  && echo 'fn main() {}' | tee src-tauri/src/main.rs crates/escouade-plugin-cli/src/main.rs > /dev/null

# The dependencies of escouade-plugin, in a layer of their own. Without the workspace's one codegen unit and full LTO, which
# would cost minutes at each change of the crate: this program starts Javy, nothing more.
FROM toolchain AS dependencies
ENV CARGO_PROFILE_RELEASE_LTO=off CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
WORKDIR /repo/src-tauri
COPY --from=manifests /repo /repo
RUN cargo build --release --locked -p escouade-plugin-cli

# escouade-plugin, without Tauri, GTK or WebKit: this crate alone, with the escouade-js it embeds.
FROM dependencies AS compiler
ENV ESCOUADE_JS_WASM=/opt/escouade-js.wasm
COPY --from=engine /repo/src-tauri/escouade-js/target/escouade-js.wasm /opt/escouade-js.wasm
COPY crates/escouade-plugin-cli ../crates/escouade-plugin-cli
RUN find ../crates/escouade-plugin-cli -type f -exec touch {} + \
  && cargo build --release --locked -p escouade-plugin-cli \
  && cp target/release/escouade-plugin-cli /opt/escouade-plugin \
  && /opt/escouade-plugin --version

# What a `run.wasm` plugin compiles in (ESC-336, docs/STORE.md §3.3): a tree of its own that bubblewrap shows the compilation
# as its root, with neither Node nor the store. gcc links the `build.rs` and the macros of the closed list, which are native;
# the toolchain is the image's, the one `ESCOUADE_RUST_TOOLCHAIN` names (rustc 1.98.0 and wasm32-wasip1, no `rust-src`), so that
# a compilation downloads none. The certificates serve `cargo fetch`. `cc` is linked without /etc/alternatives: the sandbox's
# /etc is empty.
FROM debian:bookworm-slim@sha256:c67e92c8099ba22853ab4cfda7cfb494d253d6012ca82dee98ba8976772af018 AS sandbox
RUN apt-get update && apt-get install -y --no-install-recommends gcc libc6-dev ca-certificates bubblewrap \
  && ln -sf gcc /usr/bin/cc \
  && rm -rf /var/lib/apt/lists/* /var/cache/apt/*
COPY --from=toolchain /usr/local/rustup/toolchains/1.98.0-x86_64-unknown-linux-gnu /opt/rust
# The sandbox has no /proc, and rustc's `ld.lld`, which links `build.rs` and macros, looks for itself there: the same call,
# by a script. A module does not go through it: it is linked by `rust-lld` itself.
RUN printf '#!/bin/sh\nexec /opt/rust/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld -flavor gnu "$@"\n' \
  > /opt/rust/lib/rustlib/x86_64-unknown-linux-gnu/bin/gcc-ld/ld.lld
COPY --from=compiler /opt/escouade-plugin /usr/local/bin/escouade-plugin

# The store, on glibc as its final image.
FROM node:24-bookworm-slim AS build
# git: the tests of the local review diff two versions with it (scripts/plugin-review/, docs/STORE.md §4.2). busybox: a
# wget for the final image, which runs no command to get one.
RUN apt-get update && apt-get install -y --no-install-recommends git busybox-static
WORKDIR /repo
COPY store store
COPY schemas schemas
COPY plugins plugins
COPY website/app/assets/main.css website/app/assets/
COPY README.md LICENSE ./
WORKDIR /repo/store
# A failing test fails the deployment: the store online stays the last one that passed.
RUN npm ci && npm run build && env -u DATABASE_URL -u TEST_DATABASE_URL npm test && mkdir -p /out/archives

# glibc for Javy, which alpine's musl does not run; this image's util-linux gives `unshare`, which sandboxes a `run.js`, and
# bubblewrap comes from the stage above, for a `run.wasm`.
FROM node:24-bookworm-slim
WORKDIR /app
ENV NODE_ENV=production HOST=0.0.0.0 PORT=3000 ARCHIVE_DIR=/data/archives MIGRATIONS_DIR=/app/migrations
# The eleventh check's compiler (server/utils/compile.ts): a compilation is given these, and nothing else of the store's.
ENV STORE_ESCOUADE_PLUGIN=/usr/local/bin/escouade-plugin STORE_JAVY=/usr/local/bin/javy STORE_COMPILE_ROOT=/opt/sandbox STORE_RUST_TOOLCHAIN=/opt/rust
COPY --from=compiler /opt/escouade-plugin /usr/local/bin/escouade-plugin
COPY --from=toolchain /opt/javy /usr/local/bin/javy
COPY --from=sandbox /usr/bin/bwrap /usr/local/bin/bwrap
COPY --from=sandbox /usr/lib/x86_64-linux-gnu/libcap.so.2 /usr/lib/x86_64-linux-gnu/libcap.so.2
COPY --from=sandbox /usr /opt/sandbox/usr
COPY --from=sandbox /opt /opt/sandbox/opt
COPY --from=sandbox /etc/ssl /opt/sandbox/etc/ssl
# What asks /api/health, the image's own and Coolify's: this image has neither wget nor curl.
COPY --from=build /bin/busybox /usr/local/bin/wget
COPY --from=build /repo/store/.output ./.output
# The migrations are played at start, before /api/health says ok (docs/STORE.md, §1.3).
COPY --from=build /repo/store/server/db/migrations ./migrations
# The public keys of the store's signatures (store/keys/*.pub), read at start; never a private one (docs/STORE.md, §4.5).
COPY --from=build /repo/store/keys ./keys
# The archives' volume (Coolify mounts it here and backs it up every night): the server writes in it.
COPY --from=build --chown=node:node /out/archives /data/archives
USER node
VOLUME /data/archives
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=5s --start-period=60s CMD wget -qO /dev/null http://127.0.0.1:3000/api/health || exit 1
CMD ["node", ".output/server/index.mjs"]
