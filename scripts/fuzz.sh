#!/bin/sh
# The fuzzing of the team server and its client (TS-18, fuzz/Cargo.toml): every target, or those named, for so many
# seconds each. Linux or macOS, with cargo-fuzz (`cargo install cargo-fuzz`); the nightly comes from fuzz/rust-toolchain.toml.
#
#   sh scripts/fuzz.sh                 every target, 20 s each (the CI's short pass)
#   sh scripts/fuzz.sh 3600 journal    one target for an hour
#
# Each target starts from its committed seeds (fuzz/seeds/<target>, never written to) and grows its corpus in
# fuzz/corpus/<target> (not committed). What allocates past 256 MiB at once, or holds more than 2 GiB, fails, as does
# an input that runs over 20 s: what a peer gives must never make a device or the server allocate without a bound. A
# failure leaves its input in fuzz/artifacts/<target>/ and its stack on the output; this script then fails too.
set -eu

seconds=${1:-20}
if [ $# -gt 0 ]; then shift; fi
cd "$(dirname "$0")/../fuzz"
targets=${*:-$(ls fuzz_targets | sed 's/\.rs$//')}

# Built once, all of them (the run of each then starts at once).
cargo fuzz build --fuzz-dir .

failed=""
for t in $targets; do
  mkdir -p "corpus/$t"
  echo "== $t ($seconds s)"
  if ! cargo fuzz run --fuzz-dir . "$t" "corpus/$t" "seeds/$t" -- \
      -max_total_time="$seconds" -rss_limit_mb=2048 -malloc_limit_mb=256 -timeout=20 -max_len=8192 \
      -print_final_stats=1; then
    failed="$failed $t"
  fi
done

if [ -n "$failed" ]; then
  echo "fuzzing failed:$failed (inputs in fuzz/artifacts/)" >&2
  exit 1
fi
echo "fuzzing: every target held"
