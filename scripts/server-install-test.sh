#!/bin/sh
# deploy/install.sh tried on a machine with systemd, as root (ESC-515): a fake release served on 127.0.0.1 (the archive
# of scripts/server-package.sh, its SHA-256, its signature by a key of the test's own, made here and never kept), then:
#
# - a service of the system (--system): the program checked and put in place, the user `escouade` made, the service
#   up and healthy; run again, nothing changes (the same process); an archive whose signature does not hold changes
#   nothing; `escouade-server update` checks the next archive, stops the server at rest and systemd starts it again;
#   the automatic update (ESC-702): off, the version is said and nothing changes; on, the server finds a signed archive of
#   its channel by itself, checks it, puts it in place, stops at rest and systemd starts it again (the archive of test
#   holds the same program: the restart then says that the version did not take, and it is not tried again);
#   --uninstall keeps the data, --uninstall --purge removes it;
# - its listening (ESC-706): by default on the socket /run/escouade-server/server.sock (RuntimeDirectory, 0700, the
#   service's user's; the socket 0600; said by server-status.json and `status --json`; answered there, refused to
#   another user); `--port N` puts it on a port (for a proxy, or the fallback); run again without --socket or --port,
#   the listening is kept and said; `--socket` brings it back;
# - the same with openssl alone (no minisign);
# - a service of a user (systemctl --user, linger), installed and removed, on its socket %t/escouade/server.sock; a
#   beta installed puts it on the beta channel.
#
#   sudo sh scripts/server-install-test.sh <escouade-server, a build of development>
#
# The build of development takes the test's key (ESCOUADE_SERVER_UPDATE_PUBKEY; a release never does). Needs systemd
# running, GNU tar, minisign, openssl, python3 (the fake release's server), curl.
set -eu
bin=${1:?usage: server-install-test.sh <escouade-server>}
[ "$(id -u)" = 0 ] || { echo "as root" >&2; exit 2; }
here=$(cd "$(dirname "$0")" && pwd)
install=$here/../deploy/install.sh
work=$(mktemp -d)
http=
fail() {
  echo "::error::$1"
  journalctl -u escouade-server --no-pager -n 40 2>/dev/null || true
  exit 1
}
cleanup() {
  [ -z "$http" ] || kill "$http" 2>/dev/null || true
  sh "$install" --uninstall --system --purge >/dev/null 2>&1 || true
  rm -rf "$work"
}
trap cleanup EXIT INT TERM

version=$("$bin" --version | sed 's/^escouade-server //')
case $(uname -m) in x86_64) arch=x64 ;; aarch64) arch=arm64 ;; *) fail "arch $(uname -m)" ;; esac

# The test's key, and the Tauri updater's format: the minisign files in base64.
minisign -G -W -p "$work/key.pub" -s "$work/key.sec" >/dev/null
pubkey=$(base64 -w0 "$work/key.pub")
release() { # <version>: the archive of that version in the fake release, signed
  d=$work/release/download/v$1
  mkdir -p "$d"
  ESCOUADE_NO_WEB_PAGE=1 sh "$here/server-package.sh" "$bin" "$1" "$arch" "$d" >/dev/null
  minisign -S -s "$work/key.sec" -m "$d/escouade-server-$1-linux-$arch.tar.gz" -x "$work/sig" >/dev/null
  base64 -w0 "$work/sig" >"$d/escouade-server-$1-linux-$arch.tar.gz.sig"
}
release "$version"
release 9.9.9
mkdir -p "$work/release/latest/download"
printf '{"version":"%s"}\n' "$version" >"$work/release/latest/download/latest.json"
# 9.9.8: an archive changed after it was signed (its SHA-256 written again, as a mirror that lies would).
release 9.9.8
bad=$work/release/download/v9.9.8/escouade-server-9.9.8-linux-$arch.tar.gz
printf 'x' >>"$bad"
(cd "$(dirname "$bad")" && sha256sum "$(basename "$bad")" >"$(basename "$bad").sha256")

python3 -u -m http.server 0 --bind 127.0.0.1 --directory "$work/release" >"$work/http.log" 2>&1 &
http=$!
for _ in $(seq 1 50); do
  port=$(sed -n 's/.*port \([0-9]*\).*/\1/p' "$work/http.log" | head -1)
  [ -n "$port" ] && break
  sleep 0.2
done
[ -n "${port:-}" ] || fail "the fake release is not served"
url=http://127.0.0.1:$port
export ESCOUADE_INSTALL_URL="$url" ESCOUADE_INSTALL_PUBKEY="$pubkey"
pid() { systemctl show -p MainPID --value escouade-server; }

# ---------- a service of the system ----------

sh "$install" --system >"$work/out" 2>&1 || { cat "$work/out"; fail "install.sh --system failed"; }
grep -q 'SHA-256 and signature checked (minisign)' "$work/out" || { cat "$work/out"; fail "the signature was not checked by minisign"; }
systemctl is-active --quiet escouade-server || fail "the service does not run"
systemctl is-enabled --quiet escouade-server || fail "the service does not start at boot"
[ "$(stat -c %U /var/lib/escouade/.local/share/escouade-server/escouade-server)" = escouade ] || fail "the program is not its user's"
[ "$(readlink /usr/local/bin/escouade-server)" = /var/lib/escouade/.local/share/escouade-server/escouade-server ] || fail "no link in /usr/local/bin"
[ "$(ps -o user= -p "$(pid)")" = escouade ] || fail "the server does not run as escouade"
runuser -u escouade -- env HOME=/var/lib/escouade escouade-server health || fail "health"
grep -q 'sudo -u escouade -H escouade-server pair' "$work/out" || fail "install.sh does not say how to pair"
grep -q 'security of this server' "$work/out" || fail "install.sh does not say where the sign-ins live"
echo "installed as a service of the system, run by escouade, healthy"

# Its socket (ESC-706): the default, in a folder of its user's alone.
sock=/run/escouade-server/server.sock
unit=/etc/systemd/system/escouade-server.service
grep -q "ExecStart=.* run --socket $sock" "$unit" || { cat "$unit"; fail "the service does not listen on its socket"; }
grep -q '^RuntimeDirectory=escouade-server' "$unit" && grep -q '^RuntimeDirectoryMode=0700' "$unit" || fail "no RuntimeDirectory 0700"
grep -q "Listening: the Unix socket $sock" "$work/out" || { cat "$work/out"; fail "install.sh does not say the socket"; }
grep -q "ssh -N -L 46480:$sock" "$work/out" || { cat "$work/out"; fail "install.sh does not say the tunnel to the socket"; }
[ -S "$sock" ] || fail "no socket at $sock"
[ "$(stat -c '%a %U' "$sock")" = "600 escouade" ] || fail "the socket is $(stat -c '%a %U' "$sock"), not 600 escouade"
[ "$(stat -c '%a %U' /run/escouade-server)" = "700 escouade" ] || fail "its folder is $(stat -c '%a %U' /run/escouade-server)"
grep -q "\"socket\": \"$sock\"" /var/lib/escouade/.escouade/server-status.json || fail "server-status.json does not say the socket"
runuser -u escouade -- env HOME=/var/lib/escouade escouade-server status --json | grep -q "\"socket\":\"$sock\"" || fail "status --json does not say the socket"
runuser -u escouade -- env HOME=/var/lib/escouade escouade-server health | grep -q "socket $sock" || fail "health does not say the socket"
# The WebSocket of the API opens there (101), and to no other user.
upgrade() { # <curl options…>: the HTTP code of an upgrade to the API's WebSocket
  curl -s -o /dev/null -m 3 -w '%{http_code}' -H 'Connection: Upgrade' -H 'Upgrade: websocket' -H 'Sec-WebSocket-Version: 13'     -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' "$@" || true
}
code=$(runuser -u escouade -- sh -c "$(command -v curl) -s -o /dev/null -m 3 -w %{http_code} -H 'Connection: Upgrade' -H 'Upgrade: websocket' -H 'Sec-WebSocket-Version: 13' -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' --unix-socket $sock http://127.0.0.1/remote" || true)
[ "$code" = 101 ] || fail "the API's WebSocket does not open on the socket ($code)"
code=$(runuser -u nobody -- sh -c "$(command -v curl) -s -o /dev/null -m 3 -w %{http_code} --unix-socket $sock http://127.0.0.1/remote" 2>/dev/null || true)
[ "$code" = 000 ] || fail "another user reached the socket ($code)"
ss -ltn 2>/dev/null | grep -q ':46480 ' && fail "a port listens besides the socket"
echo "its socket: $sock, 0600 in a folder 0700 of escouade's, said by its status, answered there and to no other user"

first=$(pid)
sh "$install" --system >"$work/out" 2>&1 || { cat "$work/out"; fail "install.sh run again failed"; }
grep -q "already installed" "$work/out" || { cat "$work/out"; fail "run again, it installed again"; }
[ "$(pid)" = "$first" ] || fail "run again, it restarted the service"
grep -q "Listening kept as the service there does: the Unix socket $sock" "$work/out" || { cat "$work/out"; fail "run again, the listening kept is not said"; }
echo "run again: nothing changed (the same process $first), the socket kept and said"

# --port: the service on a port of 127.0.0.1 (a proxy, or the fallback of an SSH that refuses forwards to a socket).
sh "$install" --system --port 46490 >"$work/out" 2>&1 || { cat "$work/out"; fail "install.sh --port failed"; }
grep -q 'ExecStart=.* run --port 46490$' "$unit" || { cat "$unit"; fail "the unit is not on the port"; }
grep -q "Listening: the port 46490 of 127.0.0.1" "$work/out" || { cat "$work/out"; fail "install.sh does not say the port"; }
runuser -u escouade -- env HOME=/var/lib/escouade escouade-server health | grep -q 'API on 127.0.0.1:46490' || fail "unhealthy on the port"
runuser -u escouade -- env HOME=/var/lib/escouade escouade-server status --json | grep -q '"socket"' && fail "status --json says a socket on a port"
[ -e "$sock" ] && fail "the socket is still there on a port"
code=$(upgrade http://127.0.0.1:46490/remote)
[ "$code" = 101 ] || fail "the API's WebSocket does not open on the port ($code)"
on_port=$(pid)
sh "$install" --system >"$work/out" 2>&1 || { cat "$work/out"; fail "install.sh run again on the port failed"; }
grep -q "Listening kept as the service there does: the port 46490 of 127.0.0.1" "$work/out" || { cat "$work/out"; fail "run again, the port is not kept"; }
[ "$(pid)" = "$on_port" ] || fail "run again on the port, it restarted the service"
# --socket: back on the socket.
sh "$install" --system --socket >"$work/out" 2>&1 || { cat "$work/out"; fail "install.sh --socket failed"; }
grep -q "run --socket $sock" "$unit" || fail "--socket did not bring the socket back"
runuser -u escouade -- env HOME=/var/lib/escouade escouade-server health | grep -q "socket $sock" || fail "unhealthy back on the socket"
first=$(pid)
echo "--port 46490: on the port, kept when run again; --socket: back on the socket"

sh "$install" --system --version 9.9.8 >"$work/out" 2>&1 && fail "an archive whose signature does not hold was taken"
grep -q 'signature refused' "$work/out" || { cat "$work/out"; fail "the refusal does not say the signature"; }
[ "$(pid)" = "$first" ] && [ "$(/usr/local/bin/escouade-server --version)" = "escouade-server $version" ] || fail "a refused archive changed something"
echo "a changed archive: refused, nothing changed"

# The update: checked, put in place, the server stopped at rest and started again by systemd.
runuser -u escouade -- env HOME=/var/lib/escouade ESCOUADE_SERVER_UPDATE_URL="$url" ESCOUADE_SERVER_UPDATE_PUBKEY="$pubkey" \
  escouade-server update --to 9.9.9 >"$work/out" 2>&1 || { cat "$work/out"; fail "escouade-server update failed"; }
cat "$work/out"
grep -q 'Signature' "$work/out" || fail "update did not check the signature"
grep -Eq 'systemd' "$work/out" || fail "update did not see systemd start it again"
[ "$(pid)" != "$first" ] || fail "the server was not restarted"
[ -f /var/lib/escouade/.local/share/escouade-server/escouade-server.old ] || fail "the old program is not kept"
runuser -u escouade -- env HOME=/var/lib/escouade escouade-server health || fail "unhealthy after the update"
runuser -u escouade -- env HOME=/var/lib/escouade ESCOUADE_SERVER_UPDATE_URL="$url" ESCOUADE_SERVER_UPDATE_PUBKEY="$pubkey" \
  escouade-server update --to 9.9.8 >"$work/out" 2>&1 && fail "update took an archive whose signature does not hold"
grep -q 'signature refused' "$work/out" || { cat "$work/out"; fail "update does not say the signature"; }
echo "escouade-server update: signature checked, restarted by systemd; a changed archive refused"

# The automatic update (ESC-702): the service given the fake release and a look every 2 s (a build of development only).
release 9.9.7
mkdir -p /etc/systemd/system/escouade-server.service.d
cat >/etc/systemd/system/escouade-server.service.d/test.conf <<EOF
[Service]
Environment=ESCOUADE_SERVER_UPDATE_URL=$url
Environment=ESCOUADE_SERVER_UPDATE_PUBKEY=$pubkey
Environment=ESCOUADE_SERVER_UPDATE_EVERY=2
EOF
systemctl daemon-reload
systemctl restart escouade-server
as_escouade() { runuser -u escouade -- env HOME=/var/lib/escouade "$@"; }
healthy() { for _ in $(seq 1 60); do as_escouade escouade-server health >/dev/null 2>&1 && return 0; sleep 1; done; return 1; }
healthy || fail "the service did not come back with its update's settings"
journal() { journalctl -u escouade-server -o cat --no-pager --since "@$since"; }
# Off: the version found is said, nothing changes.
as_escouade escouade-server config automatic off >"$work/out" 2>&1 || { cat "$work/out"; fail "config automatic off"; }
grep -q 'Automatic update: off' "$work/out" || { cat "$work/out"; fail "config does not say the automatic off"; }
since=$(date +%s)
before=$(pid)
printf '{"version":"9.9.7"}\n' >"$work/release/latest/download/latest.json"
for _ in $(seq 1 30); do journal | grep -q '9.9.7 is published (the automatic update is off)' && break; sleep 1; done
journal | grep -q '9.9.7 is published (the automatic update is off)' || { journal | tail -20; fail "the automatic off did not say the version"; }
sleep 5
[ "$(pid)" = "$before" ] || fail "the automatic off restarted the server"
grep -q '9.9.7' /var/lib/escouade/.escouade/server-status.json || fail "server-status.json does not say the version found"
echo "the automatic update off: 9.9.7 said, in the log and server-status.json, nothing changed"
# On: found, checked, put in place, restarted at rest by systemd.
since=$(date +%s)
as_escouade escouade-server config automatic on >"$work/out" 2>&1 || { cat "$work/out"; fail "config automatic on"; }
for _ in $(seq 1 90); do [ "$(pid)" != "$before" ] && [ -n "$(pid)" ] && [ "$(pid)" != 0 ] && break; sleep 1; done
[ "$(pid)" != "$before" ] || { journal | tail -30; fail "the automatic update did not restart the server"; }
healthy || { journal | tail -30; fail "unhealthy after the automatic update"; }
sleep 3
journal >"$work/journal"
for line in '9.9.7 found on the stable channel' '9.9.7: signature checked' '9.9.7 put in' 'at rest, stopping to start again at 9.9.7' 'did not take'; do
  grep -q "$line" "$work/journal" || { tail -40 "$work/journal"; fail "the log does not say: $line"; }
done
after=$(pid)
sleep 8
[ "$(pid)" = "$after" ] || fail "a version that did not take was tried again by itself (a loop of restarts)"
# A changed archive: refused by the automatic update too, nothing changed, never downloaded again.
since=$(date +%s)
gets() { grep -c "GET /download/v9.9.8/escouade-server-9.9.8-linux-$arch.tar.gz " "$work/http.log" || true; }
got=$(gets)
printf '{"version":"9.9.8"}\n' >"$work/release/latest/download/latest.json"
for _ in $(seq 1 30); do journal | grep -q 'signature refused' && break; sleep 1; done
journal | grep -q 'download refused: signature refused' || { journal | tail -20; fail "the automatic update did not refuse a changed archive"; }
[ "$(pid)" = "$after" ] || fail "a refused archive restarted the server"
sleep 6
[ "$(gets)" = "$((got + 1))" ] || fail "a refused archive was downloaded again ($got, then $(gets))"
rm -rf /etc/systemd/system/escouade-server.service.d
systemctl daemon-reload
# The release says its own version again, for what follows.
printf '{"version":"%s"}\n' "$version" >"$work/release/latest/download/latest.json"
echo "the automatic update: found, checked, put in place, restarted at rest by systemd, never looped; a changed archive refused"

sh "$install" --uninstall --system >"$work/out" 2>&1 || fail "uninstall failed"
systemctl cat escouade-server >/dev/null 2>&1 && fail "the service is still there"
[ -e /usr/local/bin/escouade-server ] && fail "the program is still there"
[ -s /var/lib/escouade/.escouade/state.json ] || fail "uninstall removed the data"
echo "uninstalled, the data kept"

# ---------- with openssl alone ----------

mkdir -p "$work/nominisign"
# The service's user runs through this PATH too (health, as escouade): a folder it may enter.
chmod 755 "$work" "$work/nominisign"
for t in /usr/bin/* /bin/*; do
  case $(basename "$t") in minisign) ;; *) [ -e "$work/nominisign/$(basename "$t")" ] || ln -s "$t" "$work/nominisign/$(basename "$t")" 2>/dev/null || true ;; esac
done
PATH=$work/nominisign:/usr/sbin:/sbin sh "$install" --system >"$work/out" 2>&1 || { cat "$work/out"; fail "install.sh with openssl failed"; }
grep -q 'SHA-256 and signature checked (openssl)' "$work/out" || { cat "$work/out"; fail "not checked by openssl"; }
PATH=$work/nominisign:/usr/sbin:/sbin sh "$install" --system --version 9.9.8 >"$work/out" 2>&1 && fail "openssl took a changed archive"
echo "openssl alone: the signature checked, a changed archive refused"
sh "$install" --uninstall --system --purge >/dev/null
id escouade >/dev/null 2>&1 && fail "--purge left the user"
[ -e /var/lib/escouade ] && fail "--purge left the data"
echo "uninstalled with --purge: the user and its data gone"

# ---------- a service of a user ----------

# A user of the test, as a run cut short may have left it.
userdel -r esc-user >/dev/null 2>&1 || true
useradd --create-home --shell /bin/sh esc-user
uid=$(id -u esc-user)
loginctl enable-linger esc-user
for _ in $(seq 1 50); do [ -S "/run/user/$uid/bus" ] && break; sleep 0.2; done
# The runner's own XDG_CONFIG_HOME (/home/runner/.config) and TMPDIR would follow runuser: the user gets its own, as a
# login of its own would.
as_user() {
  runuser -u esc-user -- env HOME=/home/esc-user XDG_RUNTIME_DIR="/run/user/$uid" \
    XDG_CONFIG_HOME=/home/esc-user/.config XDG_DATA_HOME=/home/esc-user/.local/share XDG_STATE_HOME=/home/esc-user/.local/state \
    XDG_CACHE_HOME=/home/esc-user/.cache TMPDIR=/tmp \
    DBUS_SESSION_BUS_ADDRESS="unix:path=/run/user/$uid/bus" ESCOUADE_INSTALL_URL="$url" ESCOUADE_INSTALL_PUBKEY="$pubkey" "$@"
}
# The checkout is under the runner's home, which another user may not enter: the user runs a copy, as one downloaded.
cp "$install" "$work/install.sh"
chmod 644 "$work/install.sh"
install=$work/install.sh
as_user sh "$install" >"$work/out" 2>&1 || { cat "$work/out"; fail "install.sh as a user failed"; }
as_user systemctl --user is-active --quiet escouade-server || fail "the user's service does not run"
[ -f /home/esc-user/.config/systemd/user/escouade-server.service ] || fail "no unit of the user"
as_user /home/esc-user/.local/bin/escouade-server health || fail "the user's server is not healthy"
grep -q 'Restart=always' /home/esc-user/.config/systemd/user/escouade-server.service || fail "the unit does not restart"
# Its socket (ESC-706): %t/escouade/server.sock, the user's runtime folder.
grep -q 'ExecStart=.* run --socket %t/escouade/server.sock$' /home/esc-user/.config/systemd/user/escouade-server.service ||
  fail "the user's service does not listen on its socket"
usock=/run/user/$uid/escouade/server.sock
[ -S "$usock" ] || fail "no socket at $usock"
[ "$(stat -c '%a %U' "$usock")" = "600 esc-user" ] || fail "the user's socket is $(stat -c '%a %U' "$usock")"
[ "$(stat -c '%a %U' "/run/user/$uid/escouade")" = "700 esc-user" ] || fail "the user's socket folder is not 700"
as_user /home/esc-user/.local/bin/escouade-server status --json | grep -q "\"socket\":\"$usock\"" || fail "the user's status --json does not say its socket"
as_user sh "$install" >"$work/out" 2>&1 && grep -q "already installed" "$work/out" || fail "run again as a user, it installed again"
# A beta installed: the beta channel, kept in the server's settings (ESC-702).
release 9.9.6-beta.1
as_user sh "$install" --version 9.9.6-beta.1 >"$work/out" 2>&1 || { cat "$work/out"; fail "install.sh --version 9.9.6-beta.1"; }
grep -q 'Channel: beta' "$work/out" || { cat "$work/out"; fail "a beta installed does not say its channel"; }
grep -q '"betaUpdates": true' /home/esc-user/.escouade/settings.json || fail "a beta installed is not on the beta channel"
as_user /home/esc-user/.local/bin/escouade-server config | grep -q 'Channel: beta' || fail "config does not say the beta channel"
as_user sh "$install" --uninstall >/dev/null || fail "uninstall as a user"
[ -d /home/esc-user/.escouade ] || fail "the user's data was removed"
as_user sh "$install" --uninstall --purge >/dev/null
[ -e /home/esc-user/.escouade ] && fail "--purge left the user's data"
loginctl disable-linger esc-user
# Its systemd of the user ended first: userdel refuses a user that still runs something.
loginctl terminate-user esc-user >/dev/null 2>&1 || true
sleep 2
userdel -r esc-user >/dev/null 2>&1 || true
echo "a service of a user: installed (linger), healthy, idempotent, uninstalled"
echo "install.sh: all checks passed"
