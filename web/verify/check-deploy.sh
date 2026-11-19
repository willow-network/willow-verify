#!/usr/bin/env bash
# Checks a deployed verifier page against the local build, fetching as Chrome does so an edge
# rewrite (e.g. Cloudflare's analytics beacon) cannot hide: every served file, decoded, matches
# web/verify-dist/SHA256SUMS.txt with nothing injected; hashed files are immutable; index.html,
# checkpoint.json, /verify/ and SHA256SUMS.txt revalidate with no-transform; a missing name is a
# 404 without a long cache. Also prints the proof status. Needs brotli and zstd; UA=... overrides
# the agent.
#   web/verify/check-deploy.sh https://api.willow.tech [dist dir]
set -euo pipefail
base="${1:?usage: check-deploy.sh <origin, e.g. https://api.willow.tech> [dist dir]}"
here="$(cd "$(dirname "$0")" && pwd)"
dist="${2:-$here/../verify-dist}"
for tool in curl shasum gzip brotli zstd; do
  command -v "$tool" >/dev/null || { echo "check-deploy.sh needs $tool on PATH"; exit 2; }
done
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fail=0
ok() { echo "ok    $*"; }
bad() { echo "FAIL  $*"; fail=1; }

ua="${UA:-Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/154.0.0.0 Safari/537.36}"
doc_cc="no-cache, no-transform"
asset_cc="public, max-age=31536000, immutable"
# Cloudflare's Web Analytics beacon, email obfuscation, Rocket Loader, anything under /cdn-cgi/.
injected='cloudflareinsights|cdn-cgi|cf-beacon|data-cfemail|__cf_email__|email-decode|rocket-loader'

header() { tr -d '\r' < "$tmp/h" | awk -v n="$1:" 'tolower($1)==n { $1=""; sub(/^ /, ""); print }'; }

# fetch <doc|asset> <path>: GET as a Chrome navigation or subresource; headers to $tmp/h, decoded body to $tmp/b.
fetch() {
  local enc
  local -a hdrs=(-A "$ua" -H 'Accept-Encoding: gzip, deflate, br, zstd' -H 'Accept-Language: en-US,en;q=0.9')
  if [ "$1" = doc ]; then
    hdrs+=(-H 'Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7'
      -H 'Upgrade-Insecure-Requests: 1' -H 'Sec-Fetch-Dest: document' -H 'Sec-Fetch-Mode: navigate'
      -H 'Sec-Fetch-Site: none' -H 'Sec-Fetch-User: ?1')
  else
    hdrs+=(-H 'Accept: */*' -H 'Sec-Fetch-Dest: empty' -H 'Sec-Fetch-Mode: cors' -H 'Sec-Fetch-Site: same-origin')
  fi
  curl -fsS -o "$tmp/raw" -D "$tmp/h" "${hdrs[@]}" "$base/verify/$2" || return 1
  enc="$(header content-encoding)"
  case "$enc" in
    "" | identity) cp "$tmp/raw" "$tmp/b" ;;
    gzip | x-gzip) gzip -dc < "$tmp/raw" > "$tmp/b" ;;
    br) brotli -dc < "$tmp/raw" > "$tmp/b" ;;
    zstd) zstd -dcq < "$tmp/raw" > "$tmp/b" ;;
    *) echo "unhandled Content-Encoding '$enc'" >&2; return 1 ;;
  esac
}

# checkpoint.json is the build's {root, slot}; the server adds the other light clients' starts to it every day
# (sepolia: a Sepolia beacon block, bnb: a BNB Smart Chain epoch block, berachain: a Berachain CometBFT header, which
# it writes every 6 hours), which no build makes.
# same_checkpoint <served file> <built file>: the served one is the built one, with only those starts added.
same_checkpoint() {
  local served built rest
  served="$(tr -d '\n' <"$1")"
  built="$(tr -d '\n' <"$2")"
  [ "$served" = "$built" ] && return 0
  rest="${served#"${built%\}},"}"
  [ "$rest" != "$served" ] || return 1
  [[ $rest =~ ^\"sepolia\":\{\"root\":\"0x[0-9a-f]{64}\",\"slot\":[0-9]+\},\"bnb\":\{\"number\":[0-9]+,\"hash\":\"0x[0-9a-f]{64}\",\"time\":[0-9]+\}(,\"berachain\":\{\"height\":[0-9]+,\"hash\":\"0x[0-9a-f]{64}\",\"time\":[0-9]+\})?\}$ ]]
}

# check <doc|asset> <path> <want sha256>
check() {
  local kind="$1" label="/verify/$2" sum="$3" want got enc hits cc
  if ! fetch "$kind" "$2"; then bad "$label: fetch or decode failed"; return; fi
  enc="$(header content-encoding)"
  got="$(shasum -a 256 < "$tmp/b" | cut -d' ' -f1)"
  if [ "$got" = "$sum" ]; then
    ok "$label content (${enc:-identity})"
  elif [ "$2" = checkpoint.json ] && same_checkpoint "$tmp/b" "$dist/checkpoint.json"; then
    ok "$label content (${enc:-identity}): the build's checkpoint, with the server's daily light-client starts"
  else
    bad "$label content (${enc:-identity}): $got, want $sum"
  fi
  hits="$(grep -a -o -iE "$injected" "$tmp/b" | sort -u | tr '\n' ' ' || true)"
  [ -z "$hits" ] && ok "$label nothing injected" || bad "$label INJECTED MARKUP: $hits"
  [ "$kind" = doc ] && want="$doc_cc" || want="$asset_cc"
  cc="$(header cache-control)"
  [ "$cc" = "$want" ] && ok "$label Cache-Control: $cc" || bad "$label Cache-Control: '$cc', want '$want'"
}

check doc SHA256SUMS.txt "$(shasum -a 256 < "$dist/SHA256SUMS.txt" | cut -d' ' -f1)"
check doc "" "$(awk '$2 == "index.html" { print $1 }' "$dist/SHA256SUMS.txt")"
# checkpoint.json and THIRD-PARTY-LICENSES.txt, like index.html, keep their names and revalidate.
while read -r sum name; do
  case "$name" in
    index.html | checkpoint.json | THIRD-PARTY-LICENSES.txt) check doc "$name" "$sum" ;;
    *) check asset "$name" "$sum" ;;
  esac
done < "$dist/SHA256SUMS.txt"
# Never a test build (CHECKPOINT=test), whose log verifier can replace a chain's pinned cutover.
glue="$(awk '$2 ~ /^verifier\.[0-9a-f]+\.js$/ { print $2 }' "$dist/SHA256SUMS.txt")"
if fetch asset "$glue" && ! grep -q setTestCutover "$tmp/b"; then
  ok "/verify/$glue: not a test build"
else
  bad "/verify/$glue: a test build (setTestCutover), or not fetched"
fi
missing="$(curl -sS -o /dev/null -D - -A "$ua" "$base/verify/app.000000000000.js" | tr -d '\r')"
if echo "$missing" | head -1 | grep -q ' 404' && ! echo "$missing" | grep -qi 'immutable'; then
  ok "missing hashed name: 404, not cached"
else
  bad "missing hashed name: $(echo "$missing" | head -1), $(echo "$missing" | grep -i cache-control || echo 'no Cache-Control')"
fi
echo "proof status: $(curl -sS -m 15 "$base/v1/rpc/health/proofs" 2>&1 | head -c 400 || echo unreachable)"
[ "$fail" = 0 ] && echo "all checks passed" || { echo "SOME CHECKS FAILED"; exit 1; }
