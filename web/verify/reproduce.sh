#!/usr/bin/env bash
# Builds the page from this source and verifies that every file Willow serves at <origin>/verify/
# is byte for byte the file this build makes. Usage: web/verify/reproduce.sh [origin], by default
# https://api.willow.tech.
#
# The build pins the Ethereum checkpoint the served page carries (ETH_CHECKPOINT in its app.js),
# since each build otherwise reads a fresh one. It runs on x86-64 Linux, the machine type Willow
# builds on (see Dockerfile): directly when this machine is one and has the pinned toolchain,
# wasm-bindgen 0.2.129 and Ubuntu's clang 18.1.3 (with llvm-ar) for wasm32, else in the Dockerfile's
# image (DOCKER=1 forces that).
set -euo pipefail
origin="${1:-https://api.willow.tech}"
here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

curl -fsS "$origin/verify/" -o "$work/index.html"
app="$(grep -o 'app\.[0-9a-f]*\.js' "$work/index.html" | head -1)"
[ -n "$app" ] || { echo "no app.js named in $origin/verify/" >&2; exit 1; }
curl -fsS "$origin/verify/$app" -o "$work/app.js"
checkpoint="$(grep -o "ETH_CHECKPOINT = { root: '0x[0-9a-f]*', slot: [0-9]* }" "$work/app.js" \
  | sed -E "s/.*root: '(0x[0-9a-f]*)', slot: ([0-9]*).*/\1:\2/")"
[ -n "$checkpoint" ] || { echo "no ETH_CHECKPOINT in $app" >&2; exit 1; }
echo "The served page carries Ethereum checkpoint $checkpoint."

host="$(cd "$repo" && rustc -vV 2>/dev/null | sed -n 's/^host: //p' || true)"
# The OpenVM module names the clang that compiled its C code, so another clang makes other bytes.
cc="$("${WASM_CC:-clang}" --version 2>/dev/null | head -1 || true)"
if [ "$host" = x86_64-unknown-linux-gnu ] && [ -z "${DOCKER:-}" ] \
  && [ "$cc" = "Ubuntu clang version 18.1.3 (1ubuntu1)" ]; then
  echo "Building on this machine..."
  CHECKPOINT="$checkpoint" VERIFY_DIST="$work/dist" bash "$here/build.sh" >"$work/build.log" 2>&1 \
    || { tail -40 "$work/build.log" >&2; exit 1; }
else
  echo "Building in Docker, as x86-64 Linux (the first run takes several minutes)..."
  image="$(docker build -q --platform linux/amd64 -t willow-verify-build "$here" 2>"$work/build.log")" \
    || { tail -40 "$work/build.log" >&2; exit 1; }
  # One build cache per image, so no build from another image's compilers is reused.
  image="${image#sha256:}"
  docker run --rm --platform linux/amd64 -v "$repo:/src:ro" -v "$work:/work" \
    -v "willow-verify-cache-${image:0:12}:/cache" -e CARGO_TARGET_DIR=/cache/target -e CHECKPOINT="$checkpoint" \
    -e VERIFY_DIST=/work/dist -e OWNER="$(id -u):$(id -g)" willow-verify-build \
    sh -c 'bash /src/web/verify/build.sh >/work/build.log 2>&1; s=$?; chown -R "$OWNER" /work; exit $s' \
    || { tail -40 "$work/build.log" >&2; exit 1; }
fi

# checkpoint.json is the build's {root, slot}; the server adds the other light clients' starts to it every day
# (sepolia: a Sepolia beacon block, bnb: a BNB Smart Chain epoch block), which no build makes.
# same_checkpoint <served file> <built file>: the served one is the built one, with only those starts added.
same_checkpoint() {
  local served built rest
  served="$(tr -d '\n' <"$1")"
  built="$(tr -d '\n' <"$2")"
  [ "$served" = "$built" ] && return 0
  rest="${served#"${built%\}},"}"
  [ "$rest" != "$served" ] || return 1
  [[ $rest =~ ^\"sepolia\":\{\"root\":\"0x[0-9a-f]{64}\",\"slot\":[0-9]+\},\"bnb\":\{\"number\":[0-9]+,\"hash\":\"0x[0-9a-f]{64}\",\"time\":[0-9]+\}\}$ ]]
}

# Every file the build makes, fetched as served and hashed here.
bad=0
while read -r sum name; do
  curl -fsS "$origin/verify/$name" -o "$work/served" || { echo "missing   $name"; bad=1; continue; }
  if [ "$(shasum -a 256 "$work/served" | cut -d' ' -f1)" = "$sum" ]; then
    echo "same      $name"
  elif [ "$name" = checkpoint.json ] && same_checkpoint "$work/served" "$work/dist/checkpoint.json"; then
    echo "same      $name (the build's checkpoint, with the server's daily light-client starts)"
  else
    echo "DIFFERENT $name"; bad=1
  fi
done < "$work/dist/SHA256SUMS.txt"
# And nothing served that the build does not make.
curl -fsS "$origin/verify/SHA256SUMS.txt" -o "$work/served-sums"
if ! cmp -s "$work/served-sums" "$work/dist/SHA256SUMS.txt"; then
  echo "DIFFERENT SHA256SUMS.txt"; bad=1
fi
if [ "$bad" = 0 ]; then
  echo "Every file Willow serves at $origin/verify/ is the file this source builds."
else
  echo "The served page is not what this source builds." >&2
  exit 1
fi
