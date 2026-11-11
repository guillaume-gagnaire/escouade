#!/bin/sh
# The system libraries of the Linux app (WebKitGTK, tray, AppImage, sound, whisper.cpp) on a GitHub runner: the CI's
# Linux jobs and the Release. libasound2-dev: the microphone (cpal); libclang-dev and cmake: whisper.cpp, built by
# whisper-rs.
#
# The runner's mirrors fail in turns, one runner and one day at a time:
# - 2026-11-04, a stalled mirror held the step 28 minutes;
# - 2026-11-06, Azure's mirror crawled through the big packages for the 45 minutes of a job: apt only leaves a mirror
#   that fails, not a slow one;
# - 2026-11-11, with Canonical's put first for that, a Release's runner could not reach it: each of the ~140 packages
#   waited 20 s before falling back, and the step ran past an hour twice.
# So each order of the mirrors is tried for 9 minutes at most, 27 for the three, under the 30 of a Linux job (a normal
# run takes one or two): the runner's own first (Azure's), then Canonical's first, then the runner's again. dpkg is
# set right after an attempt cut in its middle. The same day the runners' DNS failed now and then, Azure's mirror too:
# then only waiting helps, and a job ends at its limit rather than past an hour.
set -u

PACKAGES="libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf libasound2-dev libclang-dev cmake"
MIRRORS=/etc/apt/apt-mirrors.txt
OPTS="-o Acquire::Retries=2 -o Acquire::http::Timeout=20 -o Acquire::https::Timeout=20 -o DPkg::Lock::Timeout=60"

attempt() {
  echo "::group::apt ($1)"
  [ -f "$MIRRORS" ] && cat "$MIRRORS"
  # shellcheck disable=SC2086 # the options and the packages are words
  timeout 180 sudo DEBIAN_FRONTEND=noninteractive apt-get $OPTS update &&
    timeout 360 sudo DEBIAN_FRONTEND=noninteractive apt-get $OPTS install -y $PACKAGES
  status=$?
  echo "::endgroup::"
  [ "$status" -eq 0 ] && return 0
  echo "::warning::apt failed or ran past its time ($1): trying another order of the mirrors"
  sudo dpkg --configure -a || true
  return 1
}

# Azure's mirror (priority 1 on GitHub's runners) behind the others, or back in front.
azure_last() { [ ! -f "$MIRRORS" ] || sudo sed -i -E 's/^(http:\/\/azure\.archive\S*\s+priority:)1/\14/' "$MIRRORS"; }
azure_first() { [ ! -f "$MIRRORS" ] || sudo sed -i -E 's/^(http:\/\/azure\.archive\S*\s+priority:)4/\11/' "$MIRRORS"; }

resolves() { getent ahosts "$1" >/dev/null 2>&1; }

# From 14:00 UTC on 2026-11-11 the runners' resolver failed for every mirror of Ubuntu, Azure's included
# (actions/runner-images#15296 and #15297). A runner that resolves none of them takes public resolvers, and kernel.org's
# mirror of Ubuntu last in its list (outside *.ubuntu.com): apt checks the archive's signature whatever the mirror.
public_dns() {
  if resolves azure.archive.ubuntu.com || resolves archive.ubuntu.com; then return 0; fi
  echo "::warning::this runner resolves no mirror of Ubuntu: public resolvers, and kernel.org's mirror last"
  sudo mkdir -p /etc/systemd/resolved.conf.d
  printf '%s\n' '[Resolve]' 'DNS=1.1.1.1 1.0.0.1 8.8.8.8 8.8.4.4' |
    sudo tee /etc/systemd/resolved.conf.d/escouade-ci-dns.conf >/dev/null
  sudo systemctl restart systemd-resolved || true
  if [ -f "$MIRRORS" ] && ! grep -q mirrors.edge.kernel.org "$MIRRORS"; then
    printf 'http://mirrors.edge.kernel.org/ubuntu/\tpriority:5\n' | sudo tee -a "$MIRRORS" >/dev/null
  fi
}

public_dns
# `dns`: only that, for a job whose apt runs elsewhere (Playwright's browsers in the store's job).
[ "${1:-}" = dns ] && exit 0

attempt "the runner's order" && exit 0
azure_last
attempt "Canonical's mirror first" && exit 0
azure_first
attempt "the runner's order, again" && exit 0
echo "::error::the system libraries of the Linux app could not be installed from any mirror"
exit 1
