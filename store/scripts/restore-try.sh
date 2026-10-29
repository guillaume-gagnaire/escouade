#!/bin/sh
# Tries a restore of the store on this machine (store/README.md, « Essayer la restauration »): the database's dump
# into an EMPTY base, and the last snapshot of the archives into a new folder, then says what came back. Run it at
# the start, then once a quarter, and note the date in docs/STORE.md.
#
#   store/scripts/restore-try.sh <dump> <url of an empty base> <new folder for the archives>
#
# <dump>: a backup of Coolify (pg_dump's custom format, .dmp) or a plain SQL file. The archives come from the
# restic repository of RESTIC_REPOSITORY / RESTIC_PASSWORD (and AWS_* for the bucket); without RESTIC_REPOSITORY,
# only the database is tried. Needs psql, pg_restore and restic.
set -eu

dump=${1:?the dump file}
url=${2:?the url of an empty base}
out=${3:?a new folder for the archives}

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

if [ -n "${RESTIC_REPOSITORY:-}" ]; then
  if [ -e "$out" ] && [ -n "$(ls -A "$out")" ]; then
    echo "restore-try: $out is not empty" >&2
    exit 1
  fi
  echo "== the archives"
  restic snapshots --host store --tag archives --latest 1
  restic restore latest --host store --tag archives --target "$out" --verify
  echo "$(find "$out" -type f | wc -l) files restored in $out"
fi
