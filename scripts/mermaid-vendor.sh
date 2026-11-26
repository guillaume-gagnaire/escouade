#!/bin/sh
# Refreshes the Mermaid embedded in the diagram template of the agents' pages (ESC-674, docs/AGENT-PAGES.md § 7.2):
# `sh scripts/mermaid-vendor.sh 12.3.0` takes dist/mermaid.min.js of that npm version, gzips it into
# src-tauri/src/agentpage_templates/mermaid.min.js.gz with its LICENSE, and prints its SHA-256: write it, with the version,
# in MERMAID_SHA256 and MERMAID_VERSION of src-tauri/src/agentpage_templates.rs (a test checks them). Take a version
# published two weeks ago at least, and read its changes: it runs in the shelter, without network, `securityLevel: 'strict'`.
set -e
version=${1:?the version of mermaid, as 12.3.0}
here=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
(cd "$work" && npm pack "mermaid@$version" --silent >/dev/null && tar xzf "mermaid-$version.tgz")
out="$here/src-tauri/src/agentpage_templates"
gzip -9 -n -c "$work/package/dist/mermaid.min.js" > "$out/mermaid.min.js.gz"
cp "$work/package/LICENSE" "$out/MERMAID-LICENSE"
(sha256sum "$work/package/dist/mermaid.min.js" 2>/dev/null || shasum -a 256 "$work/package/dist/mermaid.min.js") | cut -d' ' -f1
