#!/bin/sh
# Compiles a `run.wasm` plugin under Linux, in a container, as a Windows or macOS machine does on its own: `escouade-plugin
# build` from this tree, its toolchain downloaded, twice in two folders. The hashes it prints must be those of the host
# (docs/PLUGINS.md §8.5.2). The tree is piped in, nothing is mounted, so that it works from Git Bash too.
#
#   sh scripts/plugin-build-linux.sh [folder [--first-party]]     (by default, the fixture of goldens.json)
set -e
here=$(cd "$(dirname "$0")" && pwd)
cd "$here/.."
folder=${1:-tests/fixtures/plugins/rust-build}
# A plugin of this repository, whose id is reserved, says `--first-party`.
hand=${2:-}
engine=${CONTAINER_ENGINE:-$(command -v podman || command -v docker)}
[ -n "$engine" ] || { echo "no podman nor docker here" >&2; exit 2; }
export MSYS_NO_PATHCONV=1
git ls-files -co --exclude-standard -- crates plugins src-tauri "$folder" | tar -cf - -T - |
  "$engine" run --rm -i docker.io/library/rust:1.98.0-bookworm sh -ec "
    mkdir -p /w/a && tar --no-same-owner -xf - -C /w/a
    cd /w/a/src-tauri
    RUSTUP_TOOLCHAIN=1.98.0 CARGO_TARGET_DIR=/tmp/cli cargo build --locked --quiet --bin escouade-plugin-cli
    cp -r /w/a /w/somewhere-else
    cd /
    for at in /w/a /w/somewhere-else; do
      ESCOUADE_DATA_DIR=/tmp/data /tmp/cli/debug/escouade-plugin-cli build \"\$at/$folder\" $hand --out /tmp/out --json
      rm -rf /tmp/data/cache
    done
    /tmp/cli/debug/escouade-plugin-cli --version | grep -E '^(rust|rustc-canon)'
  "
