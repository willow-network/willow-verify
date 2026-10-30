#!/usr/bin/env bash
# Builds the browser verifier into web/verify-dist/ (served at /verify/): the log verifier
# (zkvm/logset-guest/query-wasm, with SP1's verifier), its OpenVM proof check
# (zkvm/logset-guest-openvm/verify-wasm: OpenVM's verifier cannot share SP1's dependency graph,
# so it is a module of its own that the log verifier calls for each OpenVM proof), the
# transaction and receipt verifier (zkvm/logset-guest/items-wasm, Merkle-Patricia proofs) and
# the block confirmation (zkvm/logset-guest/anchor-wasm, Ethereum's light client), each its own
# wasm module.
# Needs: the repo toolchain with the wasm32-unknown-unknown target, wasm-bindgen-cli 0.2.129
# (cargo install wasm-bindgen-cli --version 0.2.129), and a C compiler and archiver for wasm32
# (OpenVM's verifier links zstd's C code): WASM_CC / WASM_AR, by default clang and llvm-ar (Apple's
# clang has no wasm32 target; Homebrew's llvm does). wasm-opt is used when it is on PATH. Every
# file but index.html gets a content hash in its name, so they can be cached forever;
# SHA256SUMS.txt lists them all. Target directories: LOGS_TARGET_DIR / OPENVM_TARGET_DIR /
# ITEMS_TARGET_DIR / ANCHOR_TARGET_DIR, else CARGO_TARGET_DIR, else each crate's own target/.
#
# The bytes are the same from any checkout. Cargo hashes a local dependency's absolute path into
# the build when the dependency sits outside the workspace root, and that hash reaches the binary
# (in TypeIds), so each crate builds in a staged copy of SOURCES whose root is its workspace root:
# every local path is then relative to it.
# The page's starting point on Ethereum (ETH_CHECKPOINT) is refreshed at each build from two
# independent beacon APIs (checkpoint.py; BEACON_APIS names them): the page refuses one over 30
# days old, so a build serves 30 days at most. CHECKPOINT=<root>:<slot> pins one instead, and
# CHECKPOINT=test the saved one the browser tests serve (test/anchor/checkpoint.json).
# CHECKPOINT=test is also the test build: only its log verifier has query-wasm's test-cutover
# feature (Verifier.setTestCutover, a test cutover in place of a chain's pinned first OpenVM block,
# for test/real-openvm.mjs), and every other build stops if its log verifier has it.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
# VERIFY_DIST names another output directory (it is emptied first).
out="${VERIFY_DIST:-$here/../verify-dist}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
test_build=
[ "${CHECKPOINT:-}" != test ] || test_build=1

[ "$(wasm-bindgen --version)" = "wasm-bindgen 0.2.129" ] || { echo "need wasm-bindgen-cli 0.2.129" >&2; exit 1; }
repo="$(cd "$here/../.." && pwd)"
# What the four crates build from, relative to the repo. The OpenVM pins are the minted program
# keys (program-keys.txt: MINT.txt's key tables, without the record's provenance), the verifying
# keys (vk/) and the ELF hashes the trust module reads.
SOURCES=(rust-toolchain.toml crates/network/src/beacon crates/receipts-trie zkvm/anchor-guest/core
  zkvm/beacon-cl-guest/core zkvm/window-anchor-guest/core zkvm/logset-guest/agg-core
  zkvm/logset-guest/anchor-verify zkvm/logset-guest/anchor-wasm zkvm/logset-guest/blockcheck
  zkvm/logset-guest/blockproof zkvm/logset-guest/chains zkvm/logset-guest/core
  zkvm/logset-guest/fixtures zkvm/logset-guest/items-wasm zkvm/logset-guest/query
  zkvm/logset-guest/query-verify zkvm/logset-guest/query-wasm zkvm/logset-guest/vkeys
  zkvm/logset-guest-openvm/verify zkvm/logset-guest-openvm/verify-wasm
  zkvm/logset-guest-openvm/pins/program-keys.txt zkvm/logset-guest-openvm/pins/vk
  zkvm/logset-guest-openvm/pins/logset-leaf.sha256 zkvm/logset-guest-openvm/pins/logset-idxagg.sha256
  zkvm/logset-guest-openvm/pins/logset-fold.sha256)

# stage <crate, relative to the repo> <dir>: SOURCES copied into <dir>, which becomes the crate's
# workspace root, with the crate's lock file, release profile and git [patch]es (a [patch] only
# applies at the root). Its other local crates stay outside the workspace, as they are in the repo.
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
if cfg.get("workspace") != {} or "replace" in cfg:
    sys.exit(f"{rel}/Cargo.toml: expected an empty [workspace] and no [replace]")
patches = cfg.get("patch", {})
for source, crates in patches.items():
    for name, spec in crates.items():
        if set(spec) != {"git", "rev"}:
            sys.exit(f"{rel}/Cargo.toml: [patch] {name} must be a git rev, not {spec}")
text = re.sub(r"(?ms)^\[patch\.[^\n]*\n.*?(?=^\[(?!patch)|\Z)", "", text)
manifest.write_text(re.sub(r"(?m)^\[workspace\]\n", "", text, count=1))
resolver = {"2015": "1", "2018": "1", "2021": "2", "2024": "3"}[cfg["package"].get("edition", "2015")]
out = ["[workspace]", f"members = {json.dumps([rel])}", 'exclude = ["crates", "zkvm"]', f'resolver = "{resolver}"']
for name, keys in cfg.get("profile", {}).items():
    out += ["", f"[profile.{name}]"]
    for k, v in keys.items():
        if isinstance(v, dict):
            sys.exit(f"{rel}/Cargo.toml: [profile.{name}.{k}] is not supported here")
        out.append(f"{k} = {json.dumps(v)}")
for source, crates in patches.items():
    out += ["", f"[patch.{json.dumps(source)}]"]
    for name, spec in crates.items():
        out.append(f'{name} = {{ git = {json.dumps(spec["git"])}, rev = {json.dumps(spec["rev"])} }}')
(root / "Cargo.toml").write_text("\n".join(out) + "\n")
EOF
}

# wasm <crate, relative to the repo> <library name> <output name> <target dir> <cargo features, or
# ""> [rustc flag ...]: build, bind, optimise into $tmp.
wasm() {
  local rel="$1" lib="$2" name="$3" target="$4" features="$5"
  shift 5
  local dir="$target/stage" sysroot cargo_home="${CARGO_HOME:-$HOME/.cargo}"
  stage "$rel" "$dir"
  sysroot="$(cd "$dir" && rustc --print sysroot)"
  # No local paths in the binaries, from rustc or from C (zstd's, in the OpenVM module). When
  # several prefixes match, rustc applies the last.
  (cd "$dir" && CARGO_TARGET_DIR="$target" \
    CC_wasm32_unknown_unknown="${WASM_CC:-clang}" AR_wasm32_unknown_unknown="${WASM_AR:-llvm-ar}" \
    CFLAGS_wasm32_unknown_unknown="-ffile-prefix-map=$cargo_home=/cargo -ffile-prefix-map=$target=/target -ffile-prefix-map=$dir=/willow" \
    RUSTFLAGS="--remap-path-prefix=$cargo_home=/cargo --remap-path-prefix=$sysroot=/rustc --remap-path-prefix=$target=/target --remap-path-prefix=$dir=/willow${*:+ $*}" \
    cargo build --release --locked --target wasm32-unknown-unknown ${features:+--features "$features"})
  # No name section: its symbol hashes depend on where the checkout is, and it is a quarter of the file.
  wasm-bindgen --target web --no-typescript --remove-name-section --out-name "$name" --out-dir "$tmp" \
    "$target/wasm32-unknown-unknown/release/$lib.wasm"
  if command -v wasm-opt >/dev/null; then
    wasm-opt -O3 --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
      "$tmp/${name}_bg.wasm" -o "$tmp/${name}_bg.wasm"
  fi
}
logs=zkvm/logset-guest/query-wasm
openvm=zkvm/logset-guest-openvm/verify-wasm
items=zkvm/logset-guest/items-wasm
anchor=zkvm/logset-guest/anchor-wasm
wasm $logs willow_logs_verify_wasm verifier "${LOGS_TARGET_DIR:-${CARGO_TARGET_DIR:-$repo/$logs/target}}" \
  "${test_build:+test-cutover}"
# The test cutover is in the test build's log verifier and in no other build's.
if [ -n "$test_build" ]; then
  grep -q setTestCutover "$tmp/verifier.js" || { echo "the test build's log verifier has no setTestCutover" >&2; exit 1; }
elif grep -a -q -i settestcutover "$tmp/verifier.js" "$tmp/verifier_bg.wasm"; then
  echo "this build is not CHECKPOINT=test, but its log verifier has the test cutover (setTestCutover)" >&2
  exit 1
fi
# getrandom 0.3 (in OpenVM's graph) takes its JS backend only with this cfg as well as its feature.
wasm $openvm willow_logs_verify_openvm_wasm openvm "${OPENVM_TARGET_DIR:-${CARGO_TARGET_DIR:-$repo/$openvm/target}}" "" \
  '--cfg getrandom_backend="wasm_js"'
wasm $items willow_items_verify_wasm items "${ITEMS_TARGET_DIR:-${CARGO_TARGET_DIR:-$repo/$items/target}}" ""
wasm $anchor willow_anchor_verify_wasm anchor "${ANCHOR_TARGET_DIR:-${CARGO_TARGET_DIR:-$repo/$anchor/target}}" ""

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
owasm=$(hashed "$tmp/openvm_bg.wasm" openvm wasm)
oglue=$(hashed "$tmp/openvm.js" openvm js)
iwasm=$(hashed "$tmp/items_bg.wasm" items wasm)
iglue=$(hashed "$tmp/items.js" items js)
awasm=$(hashed "$tmp/anchor_bg.wasm" anchor wasm)
aglue=$(hashed "$tmp/anchor.js" anchor js)
worker=$(hashed "$here/worker.js" worker js "__ITEMS_GLUE__=$iglue" "__ITEMS_WASM__=$iwasm" "__ANCHOR_GLUE__=$aglue" "__ANCHOR_WASM__=$awasm" "__OPENVM_GLUE__=$oglue" "__OPENVM_WASM__=$owasm" "__GLUE__=$glue" "__WASM__=$wasm")
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
(cd "$out" && shasum -a 256 index.html "$wasm" "$glue" "$owasm" "$oglue" "$iwasm" "$iglue" "$awasm" "$aglue" "$worker" "$app" "$style" "$icon" "${fonts[@]}" > SHA256SUMS.txt)
ls -l "$out"
