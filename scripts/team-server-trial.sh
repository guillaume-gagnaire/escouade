#!/bin/bash
# escouade-team-server in a bare Debian container (ESC-586, https://escouade.dev/docs/team/install), as server-trial.sh
# tries escouade-server: it says its version, `init` creates the base and prints the token that creates a team (never
# written in the log), `serve` answers /healthz on 127.0.0.1 alone and refuses a request without the protocol's
# handshake, and it stops cleanly at SIGTERM (exit code 0). Run inside the container (the Release's job « Servers »):
#
#   docker run --rm -v "$PWD/src-tauri/target/release:/srv/bin:ro" -v "$PWD/scripts:/srv/scripts:ro" \
#     debian:bookworm-slim bash /srv/scripts/team-server-trial.sh /srv/bin/escouade-team-server [<version>]
set -u
bin=${1:?usage: team-server-trial.sh <escouade-team-server> [<version>]}
version=${2:-}
data=/tmp/team-data
port=48443
fail() {
  echo "::error::$1"
  echo "--- log"
  cat /tmp/team.log 2>/dev/null
  exit 1
}
said=$("$bin" --version) || fail "--version"
echo "$said"
[ -z "$version" ] || [ "$said" = "escouade-team-server $version" ] || fail "--version says « $said », not $version"
# The help creates nothing and listens nowhere.
(cd /tmp && "$bin" init --help >/dev/null) || fail "init --help"
[ ! -e /tmp/data ] || fail "init --help created ./data"
ESCOUADE_TEAM_DATA=$data "$bin" init >/tmp/init.txt 2>/tmp/init.err || fail "init: $(cat /tmp/init.err)"
token=$(awk 'NF == 1 && length($1) >= 20 { print $1; exit }' /tmp/init.txt)
[ -n "$token" ] || fail "init printed no token: $(cat /tmp/init.txt)"
[ -s "$data/team.db" ] || fail "init made no base in $data"
ESCOUADE_TEAM_DATA=$data ESCOUADE_TEAM_LISTEN=127.0.0.1:$port "$bin" serve >/tmp/team.log 2>&1 &
pid=$!
for _ in $(seq 1 120); do
  grep -q 'serving' /tmp/team.log && break
  kill -0 "$pid" 2>/dev/null || fail "escouade-team-server stopped as it started"
  sleep 0.5
done
grep -q 'serving' /tmp/team.log || fail "not serving in 60 s"
# One request over HTTP/1.1, its whole answer.
ask() {
  exec 3<>"/dev/tcp/127.0.0.1/$port" || fail "nothing listens on 127.0.0.1:$port"
  printf 'GET %s HTTP/1.1\r\nHost: 127.0.0.1:%s\r\nConnection: close\r\n\r\n' "$1" "$port" >&3
  timeout 5 cat <&3 | tr -d '\r'
  exec 3>&-
}
health=$(ask /healthz)
echo "$health" | head -1
echo "$health" | head -1 | grep -q '^HTTP/1.1 200' || fail "/healthz: $(echo "$health" | head -1)"
[ "$(echo "$health" | tail -1)" = ok ] || fail "/healthz did not say ok"
# A route of the protocol without its handshake is refused (426, « Mets Escouade à jour »).
old=$(ask /v1/devices | head -1)
echo "without the handshake: $old"
echo "$old" | grep -q '^HTTP/1.1 426' || fail "a request without Escouade-Protocol is not refused in 426: $old"
# On 127.0.0.1 alone.
hex=$(printf '%04X' "$port")
listening=$(awk -v p=":$hex" '$4 == "0A" && $2 ~ p"$" { print $2 }' /proc/net/tcp /proc/net/tcp6 2>/dev/null)
[ "$listening" = "0100007F:$hex" ] || fail "listens elsewhere than 127.0.0.1: $listening"
kill -TERM "$pid"
wait "$pid"
code=$?
[ "$code" -eq 0 ] || fail "exit code $code after SIGTERM"
grep -qF "$token" /tmp/team.log && fail "the token is in the log"
tail -3 /tmp/team.log
echo "escouade-team-server: init, served /healthz on 127.0.0.1, refused an old client, stopped at SIGTERM"
