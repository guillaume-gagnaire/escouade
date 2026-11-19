#!/bin/sh
# The archive of escouade-server for a release (ESC-515, https://escouade.dev/docs/server/): the program, the seed of its
# plugins and the licence, in one folder `escouade-server/`, as `escouade-server update`, deploy/install.sh and
# deploy/Dockerfile read it; and its SHA-256 beside it. The Release signs it afterwards (the updater's key: `.sig`).
#
#   sh scripts/server-package.sh <escouade-server> <version> <x64|arm64> <out folder> [<seed folder>]
#
# The same bytes for the same files (sorted, owned by 0, dated by SOURCE_DATE_EPOCH or the commit), GNU tar.
set -eu
bin=${1:?usage: server-package.sh <escouade-server> <version> <x64|arm64> <out> [<seed>]}
version=${2:?version}
arch=${3:?x64 or arm64}
out=${4:?out folder}
seed=${5:-}
case $arch in x64 | arm64) ;; *) echo "server-package.sh: arch x64 or arm64, not $arch" >&2; exit 1 ;; esac
[ -f "$bin" ] || { echo "server-package.sh: no program at $bin" >&2; exit 1; }
root=$(cd "$(dirname "$0")/.." && pwd)
epoch=${SOURCE_DATE_EPOCH:-$(git -C "$root" log -1 --format=%ct 2>/dev/null || date +%s)}
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT INT TERM
mkdir -p "$stage/escouade-server" "$out"
cp "$bin" "$stage/escouade-server/escouade-server"
chmod 0755 "$stage/escouade-server/escouade-server"
cp "$root/LICENSE" "$stage/escouade-server/LICENSE"
if [ -n "$seed" ]; then
  [ -f "$seed/revocations.json" ] || { echo "server-package.sh: $seed is not a seed (store/scripts/seed.ts fetch)" >&2; exit 1; }
  mkdir -p "$stage/escouade-server/seed"
  cp -R "$seed"/. "$stage/escouade-server/seed/"
  rm -f "$stage/escouade-server/seed/README.md"
fi
name=escouade-server-$version-linux-$arch.tar.gz
out=$(cd "$out" && pwd)
# The modes in the archive whatever the disk keeps (an NTFS has no exec bit): the folders and the program 0755, the rest
# 0644.
tarred() { tar --sort=name --owner=0 --group=0 --numeric-owner --mtime="@$epoch" "$@"; }
(
  cd "$stage"
  tarred --mode=u=rwX,go=rX --exclude=escouade-server/escouade-server -cf "$stage/a.tar" escouade-server
  tarred --mode=0755 -rf "$stage/a.tar" escouade-server/escouade-server
)
gzip -n -9 <"$stage/a.tar" >"$out/$name"
(cd "$out" && sha256sum "$name" >"$name.sha256")
echo "$out/$name"
cat "$out/$name.sha256"
