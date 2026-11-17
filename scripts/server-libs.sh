#!/bin/sh
# The libraries escouade-server links (ESC-514, docs/CRATES.md § 14): none of the desktop's. Built without the feature
# `desktop`, it links neither WebKitGTK nor GTK (Tauri's), nor D-Bus (single-instance's), nor ALSA (cpal's: the voice's
# feature `devices`, the desktop's), nor X11 or Wayland: it runs on a machine with no desktop at all.
#
#   sh scripts/server-libs.sh src-tauri/target/debug/escouade-server
set -eu
bin=${1:?usage: server-libs.sh <escouade-server>}
libs=$(ldd "$bin")
echo "$libs"
bad=$(printf '%s\n' "$libs" | grep -Ei 'libwebkit2gtk|libjavascriptcore|libgtk|libgdk|libsoup|libappindicator|libayatana|libdbus|libasound|libX11|libxcb|libwayland|libpulse|libglib|libgobject' || true)
if [ -n "$bad" ]; then
  echo "::error::escouade-server links libraries of the desktop:"
  printf '%s\n' "$bad"
  exit 1
fi
echo "escouade-server links no library of the desktop"
