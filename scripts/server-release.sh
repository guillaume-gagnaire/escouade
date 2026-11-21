#!/bin/sh
# A server's part of a release, for one architecture (ESC-515, ESC-586): escouade-server or escouade-team-server, as the
# program's name says. The Release runs it on each runner, and it runs the same way « à blanc » (a dry run) on a machine
# of development, with a key of its own and nothing published.
#
#   sh scripts/server-release.sh <program> <version> <x64|arm64> <seed folder, or "" for the team server> <out folder>
#
# 1. The archive and its SHA-256 (scripts/server-package.sh), and, for escouade-server, install.sh beside them (x64's
#    run alone).
# 2. Its signature (`.sig`), by the Tauri CLI with TAURI_SIGNING_PRIVATE_KEY and its password, as the app's updater
#    files are signed; then checked with the updater's public key (src-tauri/tauri.conf.json), or RELEASE_PUBKEY for a
#    dry run's own key: a signature that does not hold stops the release.
# 3. The image of this architecture on the archive (deploy/Dockerfile for escouade-server, the team server's own
#    crates/escouade-team-server/Dockerfile from its archive): with IMAGE (ghcr.io/<repository>/<program>) and PUSH=1,
#    pushed by its digest alone, written to <out>/<program>-<arch>.digest (the Release's last job tags the two digests
#    together once the release is published, scripts/image-tags.mjs); without, built here and named
#    <program>:<version>.
#
# DOCKER: docker (by default) or podman; TAURI_CLI: the CLI's version (package-lock.json's by default).
set -eu
bin=${1:?usage: server-release.sh <program> <version> <x64|arm64> <seed> <out>}
version=${2:?version}
arch=${3:?x64 or arm64}
seed=${4-}
out=${5:?out folder}
product=$(basename "$bin")
root=$(cd "$(dirname "$0")/.." && pwd)
docker=${DOCKER:-docker}
case $product in
  escouade-server)
    [ -n "$seed" ] || { echo "server-release.sh: escouade-server needs the seed of its plugins" >&2; exit 1; }
    dockerfile=$root/deploy/Dockerfile
    build_args=
    ;;
  escouade-team-server)
    dockerfile=$root/crates/escouade-team-server/Dockerfile
    build_args="--build-arg PROGRAM=archive"
    ;;
  *) echo "server-release.sh: escouade-server or escouade-team-server, not $product" >&2; exit 1 ;;
esac
: "${TAURI_SIGNING_PRIVATE_KEY:?the signing key of the updater (TAURI_SIGNING_PRIVATE_KEY)}"
json() { node -p "JSON.parse(require('fs').readFileSync(process.argv[1], 'utf8'))$2" "$1"; }
cli=${TAURI_CLI:-}
[ -n "$cli" ] || cli=$(json "$root/package-lock.json" "['packages']['node_modules/@tauri-apps/cli'].version")

mkdir -p "$out"
sh "$root/scripts/server-package.sh" "$bin" "$version" "$arch" "$out" "$seed" >/dev/null
archive=$(cd "$out" && pwd)/$product-$version-linux-$arch.tar.gz
[ "$product" != escouade-server ] || [ "$arch" != x64 ] || cp "$root/deploy/install.sh" "$out/install.sh"

npx --yes "@tauri-apps/cli@$cli" signer sign "$archive" >/dev/null
[ -s "$archive.sig" ] || { echo "::error::no signature for $archive" >&2; exit 1; }
node "$root/scripts/minisign-verify.mjs" "$archive" "$archive.sig" ${RELEASE_PUBKEY:+"$RELEASE_PUBKEY"}

platform=linux/amd64
[ "$arch" = x64 ] || platform=linux/arm64
context=$(mktemp -d)
trap 'rm -rf "$context"' EXIT INT TERM
cp "$archive" "$context/"
if [ "${PUSH:-0}" = 1 ]; then
  : "${IMAGE:?the name of the image in the registry}"
  # shellcheck disable=SC2086 # the build arguments are words, or none
  "$docker" buildx build --platform "$platform" -f "$dockerfile" --build-arg "VERSION=$version" $build_args \
    --provenance=false --metadata-file "$context/metadata.json" \
    --output "type=image,name=$IMAGE,push-by-digest=true,name-canonical=true,push=true" "$context"
  json "$context/metadata.json" "['containerimage.digest']" >"$out/$product-$arch.digest"
  echo "image $IMAGE@$(cat "$out/$product-$arch.digest") pushed by its digest"
else
  # podman keeps the HEALTHCHECK in the format of Docker alone (an OCI image of its drops it).
  format=
  [ "$(basename "$docker")" != podman ] || format="--format docker"
  # shellcheck disable=SC2086 # the options are words, or none
  "$docker" build $format --platform "$platform" -f "$dockerfile" --build-arg "VERSION=$version" $build_args \
    -t "$product:$version" "$context"
  echo "image $product:$version built ($platform), not pushed"
fi
ls -l "$out"
