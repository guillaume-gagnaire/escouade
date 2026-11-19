#!/bin/sh
# The image of escouade-server, tried as a user runs it (ESC-515, deploy/Dockerfile, https://escouade.dev/docs/server/):
# on a new volume, its root filesystem read-only and every capability dropped, it starts as its user without
# privileges, passes its HEALTHCHECK, keeps everything in /data, listens on 127.0.0.1 of the container alone, says in
# its log how to pair (never the code), pairs by `docker exec … escouade-server pair` and a client of the protocol,
# stops cleanly at `docker stop` (exit code 0), and starts again on the same volume, its device still paired.
#
#   sh scripts/server-image-test.sh <image> [--harness] [--measure]
#
#   --harness  installs Claude Code by its plugin's command (curl … | bash) in the volume, and checks it is there
#              after a restart (network; no account, no sign-in)
#   --measure  memory and processes at rest, then with an agent of the fake `claude` in a turn (tests/fixtures)
#
# DOCKER: the program (docker by default, or podman).
set -eu
image=${1:?usage: server-image-test.sh <image> [--harness] [--measure]}
shift
harness=0
measure=0
for a in "$@"; do
  case $a in
    --harness) harness=1 ;;
    --measure) measure=1 ;;
    *) echo "unknown option: $a" >&2; exit 2 ;;
  esac
done
docker=${DOCKER:-docker}
here=$(cd "$(dirname "$0")" && pwd)
name=escouade-image-test-$$
volume=$name-data
port=46480

fail() {
  echo "::error::$1"
  echo "--- the container's log"
  $docker logs "$name" 2>&1 | tail -60 || true
  exit 1
}
cleanup() {
  $docker rm -f "$name" >/dev/null 2>&1 || true
  $docker volume rm -f "$volume" >/dev/null 2>&1 || true
}
trap cleanup EXIT INT TERM

ex() { $docker exec "$name" "$@"; }
client() { $docker exec -i "$name" node --input-type=module - "$@" <"$here/server-image-client.mjs"; }

healthy() {
  for _ in $(seq 1 90); do
    if [ "$docker" = podman ]; then
      podman healthcheck run "$name" >/dev/null 2>&1 && return 0
    else
      [ "$($docker inspect -f '{{.State.Health.Status}}' "$name")" = healthy ] && return 0
    fi
    [ "$($docker inspect -f '{{.State.Running}}' "$name")" = true ] || fail "the container stopped as it started"
    sleep 2
  done
  fail "not healthy in 180 s"
}

echo "--- size: $($docker image inspect -f '{{.Size}}' "$image") bytes"
$docker volume create "$volume" >/dev/null
if [ "$measure" = 1 ]; then
  # The fake `claude` (tests/fixtures) as the volume's harness, and the machine's account of it as a user adds it from
  # the client: before the first start, as a real server's harness is installed before its agents.
  $docker run --rm -i -v "$volume:/data" --entrypoint sh "$image" -c '
    mkdir -p /data/.local/bin /data/.escouade
    cat > /data/.local/bin/claude && chmod 755 /data/.local/bin/claude
    printf "%s" "{\"accountsVersion\":1,\"accounts\":[{\"id\":\"principal\",\"name\":\"Principal\",\"engine\":\"claude\"}]}" > /data/.escouade/settings.json
  ' <"$here/../tests/fixtures/fake-claude.mjs"
fi
$docker run -d --name "$name" \
  --read-only --tmpfs /tmp:size=1g,mode=1777 \
  --cap-drop ALL --security-opt no-new-privileges \
  -v "$volume:/data" "$image" >/dev/null
healthy
echo "healthy"

# Its user, without privileges.
uid=$(ex id -u)
[ "$uid" != 0 ] || fail "it runs as root"
ex sh -c 'command -v sudo' >/dev/null 2>&1 && fail "sudo is in the image"
echo "runs as uid $uid ($(ex id -un)), no sudo"
# Its root filesystem is read-only: nothing is written outside /data (and /tmp).
ex sh -c 'touch /opt/escouade-server/x' 2>/dev/null && fail "/opt is writable"
ex sh -c 'touch /data/.write-test && rm /data/.write-test' || fail "/data is not writable"
ex test -s /data/.escouade/server-status.json || fail "nothing of Escouade in /data/.escouade"
version=$(ex escouade-server --version | sed 's/^escouade-server //')
echo "escouade-server $version, its data in /data/.escouade"

# The API on 127.0.0.1 of the container alone (nothing published, nothing reachable from outside).
hex=$(printf '%04X' "$port")
listening=$(ex sh -c "awk '\$4 == \"0A\" { print \$2 }' /proc/net/tcp /proc/net/tcp6" | grep ":$hex$" || true)
[ "$listening" = "0100007F:$hex" ] || fail "the API listens elsewhere than 127.0.0.1: $listening"
echo "the API listens on 127.0.0.1:$port of the container alone"

# A new server says how to pair, and never a code.
$docker logs "$name" 2>&1 | grep -q 'docker exec <container> escouade-server pair' || fail "the log does not say how to pair"
code=$(ex escouade-server pair | grep -Eo '[A-Z2-9]{4}-[A-Z2-9]{4}' | head -1)
[ -n "$code" ] || fail "pair printed no code"
$docker logs "$name" 2>&1 | grep -q "$code" && fail "the code is in the log"
ex grep -rqs "$code" /data/.escouade && fail "the code is written in /data"
token=$(client pair "$code") || fail "the pairing was refused"
[ -n "$token" ] || fail "no token"
$docker logs "$name" 2>&1 | grep -q "$token" && fail "the token is in the log"
client hello "$version" "$token" | grep -q '"t":"welcome"' || fail "the token is refused"
client hello "0.0.1" "$token" | grep -q '"code":"version"' || fail "another version is not refused"
# The device's token is kept in the file of the secrets, its owner's alone.
[ "$(ex stat -c %a /data/.escouade/secrets.json)" = 600 ] || fail "the secrets are not in a file of their owner alone"
echo "paired by docker exec, the device's token passes the handshake"

if [ "$harness" = 1 ]; then
  ex bash -c 'curl -fsSL https://claude.ai/install.sh | bash' >/dev/null 2>&1 || fail "Claude Code's installer failed"
  ex claude --version || fail "claude is not on the PATH"
  where=$(ex sh -c 'readlink -f "$(command -v claude)"')
  case $where in /data/*) ;; *) fail "claude is installed out of the volume: $where" ;; esac
  [ "$(ex npm config get prefix)" = /data/.npm-global ] || fail "npm -g does not install into the volume"
  echo "Claude Code installed by its plugin's command, in the volume ($where); npm -g goes to /data/.npm-global"
fi

if [ "$measure" = 1 ]; then
  stats() { $docker stats --no-stream --format '{{.MemUsage}} {{.PIDs}}' "$name"; }
  echo "--- at rest: $(stats)"
  ex sh -c 'git init -q /data/projects/demo && cd /data/projects/demo && echo demo > README.md && git add . && git -c user.name=t -c user.email=t@example.org commit -qm init'
  agent=$(client agent "$version" "$token" /data/projects/demo 'flood 40s') || fail "no agent"
  echo "agent $agent in a turn of the fake claude"
  for _ in $(seq 1 8); do
    sleep 4
    s=$(stats)
    echo "    in a turn: $s"
  done
  echo "--- at rest again (the turn over): $(sleep 15; stats)"
fi

# A clean stop: SIGTERM, the turns cut, the state written, exit code 0.
$docker stop -t 60 "$name" >/dev/null
[ "$($docker inspect -f '{{.State.ExitCode}}' "$name")" = 0 ] || fail "exit code $($docker inspect -f '{{.State.ExitCode}}' "$name") after docker stop"
$docker logs "$name" 2>&1 | grep -q 'SIGTERM: stopping' || fail "the stop is not in the log"
$docker logs "$name" 2>&1 | grep -q 'escouade-server: stopped' || fail "the stop did not end"
echo "stopped cleanly (exit code 0)"

# Started again on the same volume: healthy, the device still paired.
$docker start "$name" >/dev/null
healthy
client hello "$version" "$token" | grep -q '"t":"welcome"' || fail "the device is no longer paired after a restart"
ex escouade-server devices | grep -q 'Image test' || fail "devices does not list the device"
if [ "$harness" = 1 ]; then
  ex claude --version >/dev/null || fail "Claude Code is gone after a restart"
fi
# An update is a new image: the command refuses here.
ex escouade-server update --to 9.9.9 >/dev/null 2>&1 && fail "update ran in the image"
echo "started again on the same volume: healthy, the device still paired; update refused in the image"
echo "escouade-server's image: all checks passed"
