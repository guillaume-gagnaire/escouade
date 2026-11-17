#!/bin/bash
# escouade-server in a bare Debian container, no display (ESC-514, docs/REMOTE.md « Le mode serveur »): it starts, serves
# the remote API on 127.0.0.1 alone, makes a pairing code, refuses a second server on its data folder, and stops cleanly
# at SIGTERM, its state written. Run inside the container (the CI's job « Server (Linux) »):
#
#   docker run --rm -v "$PWD/src-tauri/target/debug:/srv/bin:ro" -v "$PWD/scripts:/srv/scripts:ro" debian:trixie-slim \
#     bash /srv/scripts/server-trial.sh /srv/bin/escouade-server
set -u
bin=${1:?usage: server-trial.sh <escouade-server>}
data=/tmp/escouade-data
port=46480
fail() {
  echo "::error::$1"
  echo "--- log"
  cat /tmp/server.log 2>/dev/null
  exit 1
}
[ -z "${DISPLAY:-}" ] && [ -z "${WAYLAND_DISPLAY:-}" ] || fail "a display is set: not the trial asked for"
"$bin" --version || fail "--version"
"$bin" run --data-dir "$data" --port "$port" >/tmp/server.log 2>&1 &
pid=$!
for _ in $(seq 1 240); do
  grep -q 'ready: remote API on ws://127.0.0.1:' /tmp/server.log && break
  kill -0 "$pid" 2>/dev/null || fail "escouade-server stopped as it started"
  sleep 0.5
done
grep -q 'ready: remote API' /tmp/server.log || fail "not ready in 120 s"
# The WebSocket of the remote API opens on 127.0.0.1.
exec 3<>"/dev/tcp/127.0.0.1/$port" || fail "nothing listens on 127.0.0.1:$port"
printf 'GET /remote HTTP/1.1\r\nHost: 127.0.0.1:%s\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n' "$port" >&3
status=$(timeout 5 head -1 <&3 | tr -d '\r')
exec 3>&-
echo "the WebSocket: $status"
[ "$status" = "HTTP/1.1 101 Switching Protocols" ] || fail "the WebSocket did not open: $status"
# On 127.0.0.1 alone: the one listening socket of its port is the loopback's (0100007F).
hex=$(printf '%04X' "$port")
listening=$(awk -v p=":$hex" '$4 == "0A" && $2 ~ p"$" { print $2 }' /proc/net/tcp /proc/net/tcp6 2>/dev/null)
echo "listening: $listening"
[ "$listening" = "0100007F:$hex" ] || fail "listens elsewhere than 127.0.0.1: $listening"
# A pairing code, its file its owner's alone.
code=$("$bin" pair --data-dir "$data" | grep -Eo '[A-Z2-9]{4}-[A-Z2-9]{4}' | head -1)
[ -n "$code" ] || fail "pair printed no code"
[ "$(stat -c %a "$data/remote-pairing.json")" = 600 ] || fail "the pairing file is readable by others"
grep -q "$code" "$data"/*.json /tmp/server.log && fail "the code is written in a file or the log"
"$bin" devices --data-dir "$data" || fail "devices"
# A second server on the same folder: refused, and says why.
"$bin" run --data-dir "$data" --port $((port + 1)) >/tmp/second.log 2>&1 && fail "a second server started on the folder"
grep -Eq 'un seul à la fois|one at a time' /tmp/second.log || fail "the refusal does not say why: $(cat /tmp/second.log)"
# SIGTERM: a clean stop, its state written, its exit code 0.
kill -TERM "$pid"
wait "$pid"
code=$?
[ "$code" -eq 0 ] || fail "exit code $code after SIGTERM"
grep -q 'SIGTERM: stopping' /tmp/server.log || fail "the stop is not in the log"
grep -q 'escouade-server: stopped' /tmp/server.log || fail "the stop did not end"
[ -s "$data/state.json" ] || fail "no state written"
tail -4 /tmp/server.log
echo "escouade-server: started, served on 127.0.0.1, paired, stopped at SIGTERM, with no display"
