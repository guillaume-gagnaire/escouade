#!/bin/sh
# The team server's image, tried as a user runs it (ESC-586, crates/escouade-team-server/Dockerfile): on a new volume,
# its root filesystem read-only and every capability dropped, `init` creates the base and prints the token that
# creates a team, `serve` answers /healthz on the port published on 127.0.0.1 of the host, stops cleanly at
# `docker stop` (exit code 0) and starts again on the same volume, its base kept.
#
#   sh scripts/team-server-image-test.sh <image>
#
# DOCKER: the program (docker by default, or podman); PORT: the port of the host (48443 by default).
set -eu
image=${1:?usage: team-server-image-test.sh <image>}
docker=${DOCKER:-docker}
port=${PORT:-48443}
name=escouade-team-image-test-$$
volume=$name-data

fail() {
  echo "::error::$1"
  echo "--- the container's log"
  $docker logs "$name" 2>&1 | tail -40 || true
  exit 1
}
cleanup() {
  $docker rm -f "$name" >/dev/null 2>&1 || true
  $docker volume rm -f "$volume" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

hardened="--read-only --tmpfs /tmp --cap-drop ALL --security-opt no-new-privileges"
# shellcheck disable=SC2086 # the options are words
token=$($docker run --rm $hardened -v "$volume:/data" "$image" init | awk 'NF == 1 && length($1) >= 20 { print $1; exit }')
[ -n "$token" ] || fail "init printed no token"
echo "init: a token of ${#token} characters"

healthy() {
  for _ in $(seq 1 60); do
    if curl -fsS "http://127.0.0.1:$port/healthz" 2>/dev/null | grep -qx ok; then return 0; fi
    [ "$($docker inspect -f '{{.State.Running}}' "$name")" = true ] || fail "the container stopped as it started"
    sleep 1
  done
  fail "/healthz did not answer ok in 60 s"
}
start() {
  # shellcheck disable=SC2086 # the options are words
  $docker run -d --name "$name" $hardened -v "$volume:/data" -p "127.0.0.1:$port:8443" "$image" >/dev/null
  healthy
}

start
user=$($docker exec "$name" id -un)
[ "$user" = escouade-team ] || fail "runs as $user, not escouade-team"
$docker exec "$name" test -s /data/team.db || fail "no base in /data"
$docker logs "$name" 2>&1 | grep -qF "$token" && fail "the token is in the log"
$docker stop -t 30 "$name" >/dev/null
code=$($docker inspect -f '{{.State.ExitCode}}' "$name")
[ "$code" = 0 ] || fail "exit code $code after docker stop"
$docker rm "$name" >/dev/null
# Again on the same volume: the base is kept.
start
$docker exec "$name" test -s /data/team.db || fail "the base is gone after a restart"
echo "the image of escouade-team-server: init, healthy on 127.0.0.1:$port, stopped cleanly, started again on its volume"
