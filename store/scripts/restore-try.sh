#!/bin/sh
# Tries a restore of the store on this machine (store/README.md, « Essayer la restauration »), from Coolify's
# backups: the dump of store-db into an EMPTY base, then the backup of the archives' volume into a new folder, and
# says what came back. Run it at the start, then once a quarter, and note the date in docs/STORE.md (§1.4).
#
#   store/scripts/restore-try.sh <dump> <url of an empty base> [<volume backup> <new folder>]
#
# <dump>: pg_dump's custom format (.dmp, Coolify's) or a plain SQL file. <volume backup>: the archive of the volume
# /data/archives (.tar.gz, .tgz or .tar). Needs psql, pg_restore and tar.
set -eu

dump=${1:?the dump file}
url=${2:?the url of an empty base}
volume=${3:-}
out=${4:-}

tables=$(psql "$url" -v ON_ERROR_STOP=1 -tAc "select count(*) from information_schema.tables where table_schema not in ('pg_catalog', 'information_schema')")
if [ "$tables" != "0" ]; then
  echo "restore-try: the base is not empty ($tables tables): give it an empty one" >&2
  exit 1
fi

if pg_restore --list "$dump" >/dev/null 2>&1; then
  pg_restore --no-owner --no-privileges --exit-on-error -d "$url" "$dump"
else
  psql "$url" -v ON_ERROR_STOP=1 -q -f "$dump"
fi

echo "== the database"
psql "$url" -v ON_ERROR_STOP=1 -c "select schemaname as schema, relname as table, n_live_tup as rows from pg_stat_user_tables order by 1, 2" \
  -c "select count(*) as migrations from drizzle.__drizzle_migrations"

if [ -n "$volume" ]; then
  : "${out:?a new folder for the archives}"
  if [ -e "$out" ] && [ -n "$(ls -A "$out")" ]; then
    echo "restore-try: $out is not empty" >&2
    exit 1
  fi
  # An archive of a backup is read as it is: no member may leave the folder.
  if tar -tf "$volume" | grep -Eq '^/|(^|/)\.\.(/|$)'; then
    echo "restore-try: $volume holds a path that leaves its folder" >&2
    exit 1
  fi
  mkdir -p "$out"
  tar -xf "$volume" -C "$out" --no-same-owner
  echo "== the archives"
  echo "$(find "$out" -type f | wc -l) files restored in $out"
  (cd "$out" && find . -type f -exec sha256sum {} + | sort -k 2)
fi
