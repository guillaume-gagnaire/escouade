#!/bin/sh
# The way back from the 2.3 to the 2.2 (ESC-586): what the 2.3 writes, read by the code of the 2.2.
#
#   sh scripts/compat-2.2.sh [<the 2.2's ref, origin/main by default>]
#
# 1. This checkout's test `compat22_tests` (src-tauri/src/compat22_tests.rs) writes, by the 2.3's own code, a data
#    folder and a repository with what the 2.2 does not know (ESCOUADE_COMPAT22_OUT).
# 2. The 2.2's sources (`git archive <ref>`) go to a folder of their own, never a worktree nor a branch of this one;
#    tests/compat-2.2/read_on_main.rs is put there as one of its modules, and its test reads the folder of 1 with the
#    2.2's code: no file set aside at the start, each file read, what the 2.3 wrote found in it. What the 2.2 has and
#    the 2.3 did not write is printed as a warning (« warning: »): it would be lost on the way back.
#
# COMPAT_DIR: the folder of the run (a new temporary one by default, removed at the end unless KEEP=1). The two builds
# share this checkout's target folder (CARGO_TARGET_DIR), so the crates of the lock build once.
set -eu
ref=${1:-origin/main}
root=$(cd "$(dirname "$0")/.." && pwd)
work=${COMPAT_DIR:-$(mktemp -d)}
out=$work/written
main=$work/main
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$root/src-tauri/target}"
cleanup() { [ "${KEEP:-0}" = 1 ] || rm -rf "$main"; }
trap cleanup EXIT INT TERM

echo "== 1. what the 2.3 writes ($(git -C "$root" rev-parse --short HEAD))"
rm -rf "$out"
(cd "$root/src-tauri" && ESCOUADE_COMPAT22_OUT="$out" cargo test --locked -p escouade --lib compat22_tests -- --nocapture)
[ -s "$out/expect.json" ] || { echo "compat-2.2.sh: the 2.3 wrote nothing in $out" >&2; exit 1; }

echo "== 2. read by the 2.2 ($ref, $(git -C "$root" rev-parse --short "$ref"))"
rm -rf "$main"
mkdir -p "$main"
git -C "$root" archive --format=tar "$ref" | tar -x -C "$main"
# What its build reads beside the sources: the window's files (tauri's build checks them), the JS plugins' engine.
mkdir -p "$main/dist"
[ -d "$root/dist" ] && cp -R "$root/dist/." "$main/dist/"
if [ -d "$root/src-tauri/escouade-js/target" ]; then
  mkdir -p "$main/src-tauri/escouade-js/target"
  cp "$root/src-tauri/escouade-js/target/"escouade-js.wasm* "$main/src-tauri/escouade-js/target/" 2>/dev/null || true
fi
cp "$root/tests/compat-2.2/read_on_main.rs" "$main/src-tauri/src/compat23_read.rs"
printf '\n#[cfg(test)]\nmod compat23_read;\n' >>"$main/src-tauri/src/lib.rs"
(cd "$main/src-tauri" && ESCOUADE_COMPAT23_IN="$out" cargo test --locked -p escouade --lib compat23_read -- --nocapture) | tee "$work/read.log"
grep -q 'the 2.2 reads what the 2.3 wrote' "$work/read.log" || { echo "compat-2.2.sh: the 2.2's test did not run" >&2; exit 1; }
echo "== the 2.2 reads what the 2.3 writes"
