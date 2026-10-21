#!/usr/bin/env bash
# Builds the browser verifier into web/verify-dist/ (served at /verify/): the log verifier
# (zkvm/logset-guest/query-wasm, SP1), the transaction and receipt verifier
# (zkvm/logset-guest/items-wasm, Merkle-Patricia proofs) and the block confirmation
# (zkvm/logset-guest/anchor-wasm, Ethereum's light client), each its own wasm module.
# Needs: the repo toolchain with the wasm32-unknown-unknown target, and wasm-bindgen-cli 0.2.103
# (cargo install wasm-bindgen-cli --version 0.2.103). wasm-opt is used when it is on PATH. Every
# file but index.html gets a content hash in its name, so they can be cached forever;
# SHA256SUMS.txt lists them all. Target directories: LOGS_TARGET_DIR / ITEMS_TARGET_DIR /
# ANCHOR_TARGET_DIR, else CARGO_TARGET_DIR, else each crate's own target/.
#
# The bytes are the same from any checkout. Cargo hashes a local dependency's absolute path into
# the build when the dependency sits outside the workspace root, and that hash reaches the binary
# (in TypeIds), so each crate builds in a staged copy of SOURCES whose root is its workspace root:
# every local path is then relative to it.
# The page's starting point on Ethereum (ETH_CHECKPOINT) is refreshed at each build from two
# independent beacon APIs (checkpoint.py; BEACON_APIS names them): the page refuses one over 30
# days old, so a build serves 30 days at most. CHECKPOINT=<root>:<slot> pins one instead, and
# CHECKPOINT=test the saved one the browser tests serve (test/anchor/checkpoint.json).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
# VERIFY_DIST names another output directory (it is emptied first).
out="${VERIFY_DIST:-$here/../verify-dist}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

[ "$(wasm-bindgen --version)" = "wasm-bindgen 0.2.103" ] || { echo "need wasm-bindgen-cli 0.2.103" >&2; exit 1; }
repo="$(cd "$here/../.." && pwd)"
# What the three crates build from, relative to the repo.
SOURCES=(rust-toolchain.toml crates/network/src/beacon crates/receipts-trie zkvm/anchor-guest/core
  zkvm/beacon-cl-guest/core zkvm/window-anchor-guest/core zkvm/logset-guest/agg-core
  zkvm/logset-guest/anchor-verify zkvm/logset-guest/anchor-wasm zkvm/logset-guest/blockcheck
  zkvm/logset-guest/blockproof zkvm/logset-guest/chains zkvm/logset-guest/core
  zkvm/logset-guest/fixtures zkvm/logset-guest/items-wasm zkvm/logset-guest/query
  zkvm/logset-guest/query-verify zkvm/logset-guest/query-wasm zkvm/logset-guest/vkeys)

# stage <crate, relative to the repo> <dir>: SOURCES copied into <dir>, which becomes the crate's
# workspace root, with the crate's lock file and release profile. Its other local crates stay
# outside the workspace, as they are in the repo.
stage() {
  local rel="$1" dir="$2"
  mkdir -p "$dir"
  (cd "$repo" && rsync -aR --delete --exclude target/ "${SOURCES[@]}" "$dir/")
  cp "$dir/$rel/Cargo.lock" "$dir/Cargo.lock"
  python3 -I - "$dir" "$rel" <<'EOF'
import json, pathlib, re, sys, tomllib
root, rel = pathlib.Path(sys.argv[1]), sys.argv[2]
manifest = root / rel / "Cargo.toml"
text = manifest.read_text()
cfg = tomllib.loads(text)
if cfg.get("workspace") != {} or "patch" in cfg or "replace" in cfg:
    sys.exit(f"{rel}/Cargo.toml: expected an empty [workspace] and no [patch] or [replace]")
manifest.write_text(re.sub(r"(?m)^\[workspace\]\n", "", text, count=1))
resolver = {"2015": "1", "2018": "1", "2021": "2", "2024": "3"}[cfg["package"].get("edition", "2015")]
out = ["[workspace]", f"members = {json.dumps([rel])}", 'exclude = ["crates", "zkvm"]', f'resolver = "{resolver}"']
for name, keys in cfg.get("profile", {}).items():
    out += ["", f"[profile.{name}]"]
    for k, v in keys.items():
        if isinstance(v, dict):
            sys.exit(f"{rel}/Cargo.toml: [profile.{name}.{k}] is not supported here")
        out.append(f"{k} = {json.dumps(v)}")
(root / "Cargo.toml").write_text("\n".join(out) + "\n")
EOF
}

# wasm <crate, relative to the repo> <library name> <output name> <target dir>: build, bind,
# optimise into $tmp.
wasm() {
  local rel="$1" lib="$2" name="$3" target="$4"
  local dir="$target/stage" sysroot
  stage "$rel" "$dir"
  sysroot="$(cd "$dir" && rustc --print sysroot)"
  # No local paths in the binaries. When several prefixes match, rustc applies the last.
  (cd "$dir" && CARGO_TARGET_DIR="$target" \
    RUSTFLAGS="--remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo --remap-path-prefix=$sysroot=/rustc --remap-path-prefix=$target=/target --remap-path-prefix=$dir=/willow" \
    cargo build --release --locked --target wasm32-unknown-unknown)
  # No name section: its symbol hashes depend on where the checkout is, and it is a quarter of the file.
  wasm-bindgen --target web --no-typescript --remove-name-section --out-name "$name" --out-dir "$tmp" \
    "$target/wasm32-unknown-unknown/release/$lib.wasm"
  if command -v wasm-opt >/dev/null; then
    wasm-opt -O3 --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
      "$tmp/${name}_bg.wasm" -o "$tmp/${name}_bg.wasm"
  fi
}
logs=zkvm/logset-guest/query-wasm
items=zkvm/logset-guest/items-wasm
anchor=zkvm/logset-guest/anchor-wasm
wasm $logs willow_logs_verify_wasm verifier "${LOGS_TARGET_DIR:-${CARGO_TARGET_DIR:-$repo/$logs/target}}"
wasm $items willow_items_verify_wasm items "${ITEMS_TARGET_DIR:-${CARGO_TARGET_DIR:-$repo/$items/target}}"
wasm $anchor willow_anchor_verify_wasm anchor "${ANCHOR_TARGET_DIR:-${CARGO_TARGET_DIR:-$repo/$anchor/target}}"

h() { shasum -a 256 "$1" | cut -c1-12; }
# hashed <src> <stem> <ext> [placeholder=value ...]: substitute, then name by content.
hashed() {
  local src="$1" stem="$2" ext="$3"; shift 3
  cp "$src" "$tmp/x.$ext"
  for kv in "$@"; do
    sed "s|${kv%%=*}|${kv#*=}|g" "$tmp/x.$ext" > "$tmp/y" && mv "$tmp/y" "$tmp/x.$ext"
  done
  local name="$stem.$(h "$tmp/x.$ext").$ext"
  mv "$tmp/x.$ext" "$out/$name"
  echo "$name"
}

rm -rf "$out" && mkdir -p "$out"
wasm=$(hashed "$tmp/verifier_bg.wasm" verifier wasm)
glue=$(hashed "$tmp/verifier.js" verifier js)
iwasm=$(hashed "$tmp/items_bg.wasm" items wasm)
iglue=$(hashed "$tmp/items.js" items js)
awasm=$(hashed "$tmp/anchor_bg.wasm" anchor wasm)
aglue=$(hashed "$tmp/anchor.js" anchor js)
worker=$(hashed "$here/worker.js" worker js "__ITEMS_GLUE__=$iglue" "__ITEMS_WASM__=$iwasm" "__ANCHOR_GLUE__=$aglue" "__ANCHOR_WASM__=$awasm" "__GLUE__=$glue" "__WASM__=$wasm")
case "${CHECKPOINT:-}" in
  "") checkpoint="$(python3 "$here/checkpoint.py")" ;;
  test) checkpoint="$(python3 -c 'import json, sys; c = json.load(open(sys.argv[1])); print(c["root"] + ":" + str(c["slot"]))' "$here/test/anchor/checkpoint.json")" ;;
  *) checkpoint="$CHECKPOINT" ;;
esac
cp_root="${checkpoint%%:*}" cp_slot="${checkpoint##*:}"
[[ "$cp_root" =~ ^0x[0-9a-f]{64}$ && "$cp_slot" =~ ^[0-9]+$ ]] || { echo "not a checkpoint: $checkpoint" >&2; exit 1; }
sed "s|^const ETH_CHECKPOINT = .*|const ETH_CHECKPOINT = { root: '$cp_root', slot: $cp_slot };|" "$here/app.js" > "$tmp/app.js"
grep -q "^const ETH_CHECKPOINT = { root: '$cp_root', slot: $cp_slot };" "$tmp/app.js" || { echo "no ETH_CHECKPOINT in app.js" >&2; exit 1; }
echo "Checkpoint $cp_root at slot $cp_slot"
app=$(hashed "$tmp/app.js" app js "__WORKER__=$worker")
# Willow's fonts (fonts/, from Fontsource; licenses alongside).
fonts=()
subs=()
for f in sans sansmedium sansbold mono serif serifitalic; do
  name=$(hashed "$here/fonts/$f.woff2" "$f" woff2)
  fonts+=("$name")
  subs+=("__FONT_$(echo "$f" | tr a-z A-Z)__=$name")
done
style=$(hashed "$here/style.css" style css "${subs[@]}")
icon=$(hashed "$here/icon.svg" icon svg)
cp "$here/index.html" "$tmp/index.html"
for kv in "__APP__=$app" "__STYLE__=$style" "__ICON__=$icon" "${subs[@]}"; do
  sed "s|${kv%%=*}|${kv#*=}|g" "$tmp/index.html" > "$tmp/y" && mv "$tmp/y" "$tmp/index.html"
done
mv "$tmp/index.html" "$out/index.html"
(cd "$out" && shasum -a 256 index.html "$wasm" "$glue" "$iwasm" "$iglue" "$awasm" "$aglue" "$worker" "$app" "$style" "$icon" "${fonts[@]}" > SHA256SUMS.txt)
ls -l "$out"
