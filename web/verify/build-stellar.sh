#!/usr/bin/env bash
# Builds only the Stellar module (crates/stellar-verify-wasm) exactly as build.sh builds it: build.sh's own lines up to
# its module list (the toolchain check, SOURCES, stage() and wasm()) run as they are, then the one `wasm` call build.sh
# makes for Stellar. Writes stellar.js and stellar_bg.wasm (wasm-bindgen's names, which @willow-network/rpc ships in
# wasm/) to $1. Same machine and toolchain as build.sh (Dockerfile): the same bytes as build.sh's stellar.<hash>.*.
set -euo pipefail
dest="${1:?usage: build-stellar.sh <output directory>}"
dest="$(mkdir -p "$dest" && cd "$dest" && pwd)"
script="$(cd "$(dirname "$0")" && pwd)/build.sh"
head="$(sed -n '1,/^logs=zkvm\/logset-guest\/query-wasm$/p' "$script" | sed '$d')"
grep -q '^wasm() {' <<< "$head" && grep -q '^stage() {' <<< "$head" || { echo "build.sh has no stage()/wasm() before its module list" >&2; exit 1; }
eval "$head"
stellar=crates/stellar-verify-wasm
grep -q "^wasm \$stellar willow_stellar_verify_wasm stellar " "$script" || { echo "build.sh builds Stellar otherwise" >&2; exit 1; }
wasm $stellar willow_stellar_verify_wasm stellar "${STELLAR_TARGET_DIR:-${CARGO_TARGET_DIR:-$repo/$stellar/target}}" ""
cp "$tmp/stellar.js" "$tmp/stellar_bg.wasm" "$dest/"
(cd "$dest" && shasum -a 256 stellar.js stellar_bg.wasm)
