#!/bin/sh
# Backs the archives up to the bucket of the backups (docs/STORE.md, §1.4): Coolify's scheduled task runs it in the
# store's container every night at 3:30, as `backup-archives`. Its variables are Coolify's: RESTIC_REPOSITORY,
# RESTIC_PASSWORD, AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY (store/README.md). An archive never changes: restic
# only sends the new ones. A failure exits non-zero, which Coolify notifies on Discord.
set -eu

: "${RESTIC_REPOSITORY:?RESTIC_REPOSITORY is missing}"
: "${RESTIC_PASSWORD:?RESTIC_PASSWORD is missing}"
dir="${ARCHIVE_DIR:-/data/archives}"

# The first night, the repository does not exist yet.
if ! restic cat config >/dev/null 2>&1; then
  restic init
fi

restic backup --host store --tag archives "$dir"
# 30 nights kept, as for the database.
restic forget --host store --tag archives --keep-daily 30 --prune
restic check
