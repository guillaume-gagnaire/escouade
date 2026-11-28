#!/bin/sh
# The archive of a server of Escouade for a release, and its SHA-256 beside it; the Release signs it afterwards (the
# updater's key: `.sig`). The program's name says which (ESC-515, ESC-586):
#
# - escouade-server (https://escouade.dev/docs/server/): the program, the seed of its plugins and the licence, in one
#   folder `escouade-server/`, as `escouade-server update`, deploy/install.sh and deploy/Dockerfile read it;
# - escouade-team-server (https://escouade.dev/docs/team/install): the program, the licence and the examples of
#   crates/escouade-team-server/deploy (systemd, compose, Caddy), in `escouade-team-server/`, as its Dockerfile reads it.
#
#   sh scripts/server-package.sh <program> <version> <x64|arm64> <out folder> [<seed folder>]
#
# escouade-server carries the page of the browser's client (BC-1): built before it (npm run build:web, then cargo build
# with ESCOUADE_WEB_DIST=dist-web), as the Release does. A program without it is refused, unless ESCOUADE_NO_WEB_PAGE=1
# (the CI's archives of a debug build, which try the rest).
#
# The same bytes for the same files (sorted, owned by 0, dated by SOURCE_DATE_EPOCH or the commit), GNU tar.
set -eu
bin=${1:?usage: server-package.sh <program> <version> <x64|arm64> <out> [<seed>]}
version=${2:?version}
arch=${3:?x64 or arm64}
out=${4:?out folder}
seed=${5:-}
case $arch in x64 | arm64) ;; *) echo "server-package.sh: arch x64 or arm64, not $arch" >&2; exit 1 ;; esac
[ -f "$bin" ] || { echo "server-package.sh: no program at $bin" >&2; exit 1; }
product=$(basename "$bin")
case $product in escouade-server | escouade-team-server) ;; *) echo "server-package.sh: escouade-server or escouade-team-server, not $product" >&2; exit 1 ;; esac
if [ "$product" = escouade-server ] && [ "${ESCOUADE_NO_WEB_PAGE:-0}" != 1 ]; then
  grep -aq "escouade-web-page:" "$bin" || {
    echo "server-package.sh: $bin carries no browser page (npm run build:web, then cargo build with ESCOUADE_WEB_DIST=dist-web)" >&2
    exit 1
  }
fi
root=$(cd "$(dirname "$0")/.." && pwd)
epoch=${SOURCE_DATE_EPOCH:-$(git -C "$root" log -1 --format=%ct 2>/dev/null || date +%s)}
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT INT TERM
mkdir -p "$stage/$product" "$out"
cp "$bin" "$stage/$product/$product"
chmod 0755 "$stage/$product/$product"
cp "$root/LICENSE" "$stage/$product/LICENSE"
if [ -n "$seed" ]; then
  [ "$product" = escouade-server ] || { echo "server-package.sh: only escouade-server carries a seed" >&2; exit 1; }
  [ -f "$seed/revocations.json" ] || { echo "server-package.sh: $seed is not a seed (store/scripts/seed.ts fetch)" >&2; exit 1; }
  mkdir -p "$stage/$product/seed"
  cp -R "$seed"/. "$stage/$product/seed/"
  rm -f "$stage/$product/seed/README.md"
fi
if [ "$product" = escouade-team-server ]; then
  mkdir -p "$stage/$product/deploy"
  cp "$root/crates/escouade-team-server/deploy"/* "$stage/$product/deploy/"
fi
name=$product-$version-linux-$arch.tar.gz
out=$(cd "$out" && pwd)
# The modes in the archive whatever the disk keeps (an NTFS has no exec bit): the folders and the program 0755, the rest
# 0644.
tarred() { tar --sort=name --owner=0 --group=0 --numeric-owner --mtime="@$epoch" "$@"; }
(
  cd "$stage"
  tarred --mode=u=rwX,go=rX --exclude="$product/$product" -cf "$stage/a.tar" "$product"
  tarred --mode=0755 -rf "$stage/a.tar" "$product/$product"
)
gzip -n -9 <"$stage/a.tar" >"$out/$name"
(cd "$out" && sha256sum "$name" >"$name.sha256")
echo "$out/$name"
cat "$out/$name.sha256"
