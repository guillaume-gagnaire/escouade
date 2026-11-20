#!/bin/sh
# deploy/install.sh tried on a machine with systemd, as root (ESC-515): a fake release served on 127.0.0.1 (the archive
# of scripts/server-package.sh, its SHA-256, its signature by a key of the test's own, made here and never kept), then:
#
# - a service of the system (--system): the program checked and put in place, the user `escouade` made, the service
#   up and healthy; run again, nothing changes (the same process); an archive whose signature does not hold changes
#   nothing; `escouade-server update` checks the next archive, stops the server at rest and systemd starts it again;
#   --uninstall keeps the data, --uninstall --purge removes it;
# - the same with openssl alone (no minisign);
# - a service of a user (systemctl --user, linger), installed and removed.
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
  sh "$here/server-package.sh" "$bin" "$1" "$arch" "$d" >/dev/null
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

first=$(pid)
sh "$install" --system >"$work/out" 2>&1 || { cat "$work/out"; fail "install.sh run again failed"; }
grep -q "already installed" "$work/out" || { cat "$work/out"; fail "run again, it installed again"; }
[ "$(pid)" = "$first" ] || fail "run again, it restarted the service"
echo "run again: nothing changed (the same process $first)"

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

useradd --create-home --shell /bin/sh esc-user
uid=$(id -u esc-user)
loginctl enable-linger esc-user
for _ in $(seq 1 50); do [ -S "/run/user/$uid/bus" ] && break; sleep 0.2; done
as_user() {
  runuser -u esc-user -- env HOME=/home/esc-user XDG_RUNTIME_DIR="/run/user/$uid" \
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
as_user sh "$install" >"$work/out" 2>&1 && grep -q "already installed" "$work/out" || fail "run again as a user, it installed again"
as_user sh "$install" --uninstall >/dev/null || fail "uninstall as a user"
[ -d /home/esc-user/.escouade ] || fail "the user's data was removed"
as_user sh "$install" --uninstall --purge >/dev/null
[ -e /home/esc-user/.escouade ] && fail "--purge left the user's data"
loginctl disable-linger esc-user
userdel -r esc-user >/dev/null 2>&1 || true
echo "a service of a user: installed (linger), healthy, idempotent, uninstalled"
echo "install.sh: all checks passed"
