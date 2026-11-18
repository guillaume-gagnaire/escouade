#!/usr/bin/env bash
# Puts Microsoft's ONNX Runtime where the bundles take it, for the systems where the conversation's voice (ESC-97)
# loads it at run time: Intel Macs (pyke builds no static library for them) and Linux (pyke's needs glibc 2.38; this
# one, 2.28). Bundled by src-tauri/tauri.ort-<os>.conf.json.
#   scripts/onnxruntime.sh macos → src-tauri/macos/onnxruntime/libonnxruntime.1.23.2.dylib
#   scripts/onnxruntime.sh linux → src-tauri/linux/onnxruntime/libonnxruntime.so.1.23.2
#
# From Microsoft's NuGet package Microsoft.ML.OnnxRuntime (nuget.org, where a published version can never change),
# since github.com/microsoft/onnxruntime answers 404 (2026-11-18): the package's SHA-256 is pinned, and so is each
# library's. The Linux one is byte for byte the library of the GitHub release that Escouade 2.1.3 shipped.
set -euo pipefail

VERSION=1.23.2
PACKAGE_SHA256=25fe172f7fcdf34f2b5b02c8b997f6b88abc282956849d4af4335af23d0c4e4a
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
case "${1:-}" in
  macos)
    IN_PACKAGE="runtimes/osx-x64/native/libonnxruntime.dylib"
    SHA256=8c9c78de65ea3786f987c0d980e9c1b13a3a5fbc6b3e2965ba05b450e6e4c054
    LIB="libonnxruntime.${VERSION}.dylib"
    ;;
  linux)
    IN_PACKAGE="runtimes/linux-x64/native/libonnxruntime.so"
    SHA256=13ab8084954fa4a47c777880180b90810d6020f021441395712b48a75b74c68b
    LIB="libonnxruntime.so.${VERSION}"
    ;;
  *)
    echo "usage: $0 macos|linux" >&2
    exit 2
    ;;
esac
DEST="$ROOT/src-tauri/$1/onnxruntime"

if [ -f "$DEST/$LIB" ]; then
  echo "$DEST/$LIB is there"
  exit 0
fi
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
curl -fsSL -o "$TMP/ort.nupkg" \
  "https://api.nuget.org/v3-flatcontainer/microsoft.ml.onnxruntime/${VERSION}/microsoft.ml.onnxruntime.${VERSION}.nupkg"
if command -v shasum >/dev/null; then sum="shasum -a 256"; else sum=sha256sum; fi
echo "$PACKAGE_SHA256  $TMP/ort.nupkg" | $sum -c -
unzip -q -o "$TMP/ort.nupkg" "$IN_PACKAGE" LICENSE -d "$TMP/x"
echo "$SHA256  $TMP/x/$IN_PACKAGE" | $sum -c -
mkdir -p "$DEST"
cp "$TMP/x/$IN_PACKAGE" "$DEST/$LIB"
cp "$TMP/x/LICENSE" "$DEST/LICENSE"
echo "$DEST/$LIB"
