#!/bin/sh
# escouade-server as a systemd service, without Docker (https://escouade.dev/docs/server/).
#
#   curl -fsSLO https://github.com/guillaume-gagnaire/escouade/releases/latest/download/install.sh
#   less install.sh                     # read it first
#   sh install.sh                       # a service of this user (systemctl --user), started at boot (linger)
#   sudo sh install.sh --system         # a service of the system, run by the user `escouade` (made if missing)
#
#   sh install.sh --uninstall [--purge] # the service and the program gone; the data kept, unless --purge
#
# It downloads the archive of the release for this machine (Linux x64 or arm64), checks its SHA-256 and its signature
# (the key of Escouade's updater, below: with `minisign` if it is there, else `openssl`), puts the program in place,
# writes the unit of the service and starts it. Run again, it changes only what differs (another version, another
# unit); the data is never touched but by --purge.
#
# Options:
#   --version X.Y.Z   this version (else the latest release's)
#   --system          a service of the system (as root), run by --user (default: escouade)
#   --user NAME       with --system: the user that runs it
#   --no-start        installs without starting the service
#   --uninstall       removes the service and the program, keeps the data
#   --purge           with --uninstall: removes the data too (~/.escouade; with --system, the user and its home)
#   -h, --help
#
# For the tests only: ESCOUADE_INSTALL_URL (another address of the releases) and ESCOUADE_INSTALL_PUBKEY (another key,
# in base64 as tauri.conf.json holds one).
set -eu

REPO_URL=https://github.com/guillaume-gagnaire/escouade/releases
# The public key of the app's updater (plugins.updater.pubkey of src-tauri/tauri.conf.json), which signs the server's
# archives too: escouade-server checks its own updates with it.
UPDATER_PUBKEY=dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDU3NzMxM0UzRTI4MkZDRjYKUldUMi9JTGk0eE56Vi93TCs0dEUyL1hBOXEwOEU0RmYvUFgrbWpHa0tWNVFLR1lLdVltRXNvaHYK
SERVICE=escouade-server

url=${ESCOUADE_INSTALL_URL:-$REPO_URL}
pubkey=${ESCOUADE_INSTALL_PUBKEY:-$UPDATER_PUBKEY}
version=
system=0
user=escouade
start=1
uninstall=0
purge=0

say() { printf '%s\n' "$*"; }
die() {
  printf 'install.sh: %s\n' "$*" >&2
  exit 1
}
usage() { sed -n '2,/^set -eu/p' "$0" | sed '$d' | sed 's/^# \{0,1\}//'; }

while [ $# -gt 0 ]; do
  case $1 in
    --version) version=${2:?--version needs a version}; shift ;;
    --version=*) version=${1#*=} ;;
    --system) system=1 ;;
    --user) user=${2:?--user needs a name}; shift ;;
    --user=*) user=${1#*=} ;;
    --no-start) start=0 ;;
    --uninstall) uninstall=1 ;;
    --purge) purge=1 ;;
    -h | --help) usage; exit 0 ;;
    *) die "unknown option: $1 (--help)" ;;
  esac
  shift
done
version=${version#v}
[ "$purge" = 0 ] || [ "$uninstall" = 1 ] || die "--purge goes with --uninstall"
case $version in
  '' | [0-9]*.[0-9]*.[0-9]*) ;;
  *) die "--version: \"$version\" is not a version (X.Y.Z)" ;;
esac
case $user in
  *[!a-z0-9_-]* | '') die "--user: \"$user\" is not a user name" ;;
esac

[ "$(uname -s)" = Linux ] || die "escouade-server runs on Linux"
case $(uname -m) in
  x86_64 | amd64) arch=x64 ;;
  aarch64 | arm64) arch=arm64 ;;
  *) die "no escouade-server for $(uname -m) (Linux x64 and arm64 only)" ;;
esac
command -v systemctl >/dev/null 2>&1 && [ -d /run/systemd/system ] ||
  die "systemd is not running here: use the Docker image (https://escouade.dev/docs/server/)"

# Where it goes.
if [ "$system" = 1 ]; then
  [ "$(id -u)" = 0 ] || die "--system installs as root: sudo sh install.sh --system"
  home=$(getent passwd "$user" | cut -d: -f6 || true)
  [ -n "$home" ] || home=/var/lib/$user
  bin_link=/usr/local/bin/escouade-server
  unit_dir=/etc/systemd/system
  ctl() { systemctl "$@"; }
  as_user() { runuser -u "$user" -- env HOME="$home" "$@"; }
  pair_cmd="sudo -u $user -H escouade-server pair"
else
  [ "$(id -u)" != 0 ] || die "as root, install a service of the system: sh install.sh --system"
  user=$(id -un)
  home=$HOME
  bin_link=$home/.local/bin/escouade-server
  unit_dir=${XDG_CONFIG_HOME:-$home/.config}/systemd/user
  ctl() { systemctl --user "$@"; }
  as_user() { "$@"; }
  pair_cmd="escouade-server pair"
fi
dir=$home/.local/share/escouade-server
data=$home/.escouade
unit=$unit_dir/$SERVICE.service

# ---------- uninstall ----------

if [ "$uninstall" = 1 ]; then
  if [ "$system" = 0 ] && [ -z "${XDG_RUNTIME_DIR:-}" ]; then
    export XDG_RUNTIME_DIR=/run/user/$(id -u)
  fi
  if [ -f "$unit" ]; then
    ctl disable --now "$SERVICE" >/dev/null 2>&1 || true
    rm -f "$unit"
    ctl daemon-reload || true
    say "The service $SERVICE is stopped and removed."
  fi
  rm -rf "$dir" "$dir.new" "$dir.old"
  if [ -L "$bin_link" ] || [ -f "$bin_link" ]; then rm -f "$bin_link"; fi
  say "The program is removed."
  if [ "$purge" = 1 ]; then
    if [ "$system" = 1 ] && id "$user" >/dev/null 2>&1; then
      pkill -u "$user" >/dev/null 2>&1 || true
      userdel -r "$user" >/dev/null 2>&1 || userdel "$user"
      rm -rf "$home"
      say "The user $user and its home ($home: Escouade's data, the harnesses and their sign-ins) are removed."
    else
      rm -rf "$data"
      say "Escouade's data ($data) is removed. The harnesses' own folders (~/.claude…) are theirs: left as they are."
    fi
  else
    say "Escouade's data is kept in $data (sh install.sh --uninstall --purge removes it)."
  fi
  exit 0
fi

# ---------- tools ----------

if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL --proto '=https,http' --retry 3 -o "$2" "$1"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -q -O "$2" "$1"; }
else
  die "curl or wget is needed"
fi
for tool in tar sha256sum base64 sed; do
  command -v "$tool" >/dev/null 2>&1 || die "$tool is needed"
done
if command -v minisign >/dev/null 2>&1; then
  verifier=minisign
elif command -v openssl >/dev/null 2>&1 && openssl list -digest-algorithms 2>/dev/null | grep -qi blake2b512; then
  verifier=openssl
else
  die "minisign or openssl (3 or newer) is needed to check the signature: apt install minisign (or openssl)"
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM

# `file` signed as the Tauri updater signs (`sig`: the minisign signature file, in base64), by the key `pubkey`.
verify() {
  file=$1
  sig=$2
  printf '%s' "$pubkey" | base64 -d >"$tmp/key.txt" 2>/dev/null || return 1
  base64 -d <"$sig" >"$tmp/sig.txt" 2>/dev/null || return 1
  key_line=$(sed -n 2p "$tmp/key.txt")
  if [ "$verifier" = minisign ]; then
    minisign -V -q -P "$key_line" -m "$file" -x "$tmp/sig.txt" >/dev/null 2>&1
    return
  fi
  # openssl: the signature (ED: Ed25519 over the BLAKE2b-512 of the file), then the global one over it and the trusted
  # comment; the key's id checked against the signature's.
  printf '%s' "$key_line" | base64 -d >"$tmp/pk.bin" 2>/dev/null || return 1
  sed -n 2p "$tmp/sig.txt" | base64 -d >"$tmp/s.bin" 2>/dev/null || return 1
  [ "$(head -c 2 "$tmp/pk.bin")" = Ed ] && [ "$(head -c 2 "$tmp/s.bin")" = ED ] || return 1
  [ "$(head -c 10 "$tmp/pk.bin" | tail -c 8 | od -An -tx1)" = "$(head -c 10 "$tmp/s.bin" | tail -c 8 | od -An -tx1)" ] || return 1
  { printf '\060\052\060\005\006\003\053\145\160\003\041\000'; tail -c 32 "$tmp/pk.bin"; } >"$tmp/pk.der"
  openssl pkey -pubin -inform DER -in "$tmp/pk.der" -out "$tmp/pk.pem" 2>/dev/null || return 1
  tail -c 64 "$tmp/s.bin" >"$tmp/s.raw"
  openssl dgst -blake2b512 -binary "$file" >"$tmp/h.bin" || return 1
  openssl pkeyutl -verify -pubin -inkey "$tmp/pk.pem" -rawin -in "$tmp/h.bin" -sigfile "$tmp/s.raw" >/dev/null 2>&1 || return 1
  comment=$(sed -n 3p "$tmp/sig.txt")
  comment=${comment#trusted comment: }
  sed -n 4p "$tmp/sig.txt" | base64 -d >"$tmp/g.bin" 2>/dev/null || return 1
  { cat "$tmp/s.raw"; printf '%s' "$comment"; } >"$tmp/g.msg"
  openssl pkeyutl -verify -pubin -inkey "$tmp/pk.pem" -rawin -in "$tmp/g.msg" -sigfile "$tmp/g.bin" >/dev/null 2>&1
}

# ---------- the version and its archive ----------

if [ -z "$version" ]; then
  fetch "$url/latest/download/latest.json" "$tmp/latest.json" || die "the latest release is not readable at $url"
  version=$(sed -n 's/.*"version"[[:space:]]*:[[:space:]]*"v\{0,1\}\([0-9][0-9A-Za-z.-]*\)".*/\1/p' "$tmp/latest.json" | head -1)
  [ -n "$version" ] || die "no version in latest.json"
fi
name=escouade-server-$version-linux-$arch.tar.gz

installed=
if [ -x "$dir/escouade-server" ]; then
  installed=$("$dir/escouade-server" --version 2>/dev/null | sed -n 's/^escouade-server //p' || true)
fi
changed=0
if [ "$installed" = "$version" ]; then
  say "escouade-server $version is already installed in $dir."
else
  say "Downloading $name…"
  base=$url/download/v$version
  fetch "$base/$name" "$tmp/$name" || die "$base/$name is not readable"
  fetch "$base/$name.sha256" "$tmp/$name.sha256" || die "$base/$name.sha256 is not readable"
  fetch "$base/$name.sig" "$tmp/$name.sig" || die "$base/$name.sig is not readable"
  expected=$(cut -d' ' -f1 <"$tmp/$name.sha256")
  actual=$(sha256sum "$tmp/$name" | cut -d' ' -f1)
  [ -n "$expected" ] && [ "$expected" = "$actual" ] || die "SHA-256 refused: the archive is not the release's. Nothing is changed."
  verify "$tmp/$name" "$tmp/$name.sig" || die "signature refused ($verifier): the archive does not come from Escouade's release. Nothing is changed."
  say "SHA-256 and signature checked ($verifier)."
  mkdir -p "$tmp/x"
  tar -xzf "$tmp/$name" -C "$tmp/x"
  src=$tmp/x
  [ -f "$src/escouade-server" ] || src=$tmp/x/escouade-server
  [ -f "$src/escouade-server" ] || die "the archive holds no escouade-server"
  if [ "$system" = 1 ] && ! id "$user" >/dev/null 2>&1; then
    useradd --system --create-home --home-dir "$home" --shell /bin/bash "$user"
    say "User $user made (home $home)."
  fi
  mkdir -p "$(dirname "$dir")"
  rm -rf "$dir.new"
  cp -R "$src" "$dir.new"
  rm -rf "$dir.old"
  [ ! -d "$dir" ] || mv "$dir" "$dir.old"
  mv "$dir.new" "$dir"
  changed=1
  say "escouade-server $version put in $dir."
fi
if [ "$system" = 1 ]; then
  id "$user" >/dev/null 2>&1 || useradd --system --create-home --home-dir "$home" --shell /bin/bash "$user"
  mkdir -p "$home/.local/share" "$home/.local/bin"
  # The program is its user's, as the app is: `escouade-server update`, run by it, puts the next version in place.
  chown -R "$user:" "$home/.local"
fi
mkdir -p "$(dirname "$bin_link")"
if [ "$(readlink "$bin_link" 2>/dev/null || true)" != "$dir/escouade-server" ]; then
  ln -sfn "$dir/escouade-server" "$bin_link"
fi

# ---------- the service ----------

mkdir -p "$unit_dir"
if [ "$system" = 1 ]; then
  cat >"$tmp/unit" <<EOF
# Written by install.sh (https://escouade.dev/docs/server/); written again by the next run.
[Unit]
Description=Escouade server (escouade-server)
Documentation=https://escouade.dev/docs/server/
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$user
Group=$(id -gn "$user")
WorkingDirectory=$home
ExecStart=$dir/escouade-server run
# The clean stop: SIGTERM to the server alone, which stops its agents itself (each resumes its session).
KillMode=mixed
TimeoutStopSec=90
# Started again after a crash, and after \`escouade-server update\` stops it on its new version.
Restart=always
RestartSec=3
UMask=0077
NoNewPrivileges=yes
PrivateTmp=yes
ProtectSystem=full
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF
else
  cat >"$tmp/unit" <<EOF
# Written by install.sh (https://escouade.dev/docs/server/); written again by the next run.
[Unit]
Description=Escouade server (escouade-server)
Documentation=https://escouade.dev/docs/server/

[Service]
Type=simple
ExecStart=$dir/escouade-server run
# The clean stop: SIGTERM to the server alone, which stops its agents itself (each resumes its session).
KillMode=mixed
TimeoutStopSec=90
# Started again after a crash, and after \`escouade-server update\` stops it on its new version.
Restart=always
RestartSec=3
UMask=0077
NoNewPrivileges=yes

[Install]
WantedBy=default.target
EOF
fi
if ! cmp -s "$tmp/unit" "$unit" 2>/dev/null; then
  cp "$tmp/unit" "$unit"
  changed=1
  say "Service written: $unit"
fi

if [ "$system" = 0 ]; then
  # A service of the user runs while they are logged in, or always with linger: a server runs H24.
  if [ "$(loginctl show-user "$user" -p Linger --value 2>/dev/null || true)" != yes ]; then
    loginctl enable-linger "$user" 2>/dev/null ||
      say "WARNING: linger is not on, so the service stops when you log out: sudo loginctl enable-linger $user"
  fi
  [ -n "${XDG_RUNTIME_DIR:-}" ] || export XDG_RUNTIME_DIR=/run/user/$(id -u)
fi
ctl daemon-reload
if [ "$start" = 1 ]; then
  ctl enable "$SERVICE" >/dev/null 2>&1
  if [ "$changed" = 1 ] || ! ctl is-active --quiet "$SERVICE"; then
    ctl restart "$SERVICE"
  fi
  ok=0
  for _ in $(seq 1 120); do
    if as_user "$dir/escouade-server" health --data-dir "$data" >/dev/null 2>&1; then
      ok=1
      break
    fi
    sleep 1
  done
  [ "$ok" = 1 ] || die "the service did not come up in 120 s: journalctl $([ "$system" = 1 ] || printf -- '--user ')-u $SERVICE"
  say "The service $SERVICE runs ($(as_user "$dir/escouade-server" health --data-dir "$data"))."
else
  say "Not started (--no-start): $([ "$system" = 1 ] && printf 'systemctl' || printf 'systemctl --user') enable --now $SERVICE"
fi

if [ "$system" = 0 ]; then
  case ":$PATH:" in
    *":$(dirname "$bin_link"):"*) ;;
    *) say "Add $(dirname "$bin_link") to your PATH to run escouade-server by its name." ;;
  esac
fi
cat <<EOF

Next:
  1. A pairing code for your computer:   $pair_cmd
  2. On your computer, a tunnel to the server's API (it listens on 127.0.0.1 of the server only):
       ssh -N -L 46480:127.0.0.1:46480 <this server>
     then, in Escouade, « Se connecter à un serveur » (Connect to a server), and the code.
  3. Updates: escouade-server update (it checks the signature and waits for the agents' turns).

The accounts the agents use (Claude Code and the others) are signed in on this server and their sign-ins live here,
in $home: the security of this server (its disk, its SSH access, its backups) becomes theirs.
Data: $data (back it up while the service is stopped).
EOF
