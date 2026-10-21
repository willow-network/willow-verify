"""The page's starting point on Ethereum (ETH_CHECKPOINT), for build.sh: the first block of the
sync-committee period two before the newest finalized one, so the committee that signed Willow's
daily anchor is always one the page reaches. Read from two independent beacon APIs, which must
serve the same light-client bootstrap for it, whose header must hash to the root. Prints
<root>:<slot>.

    BEACON_APIS="<url> <url>" python3 checkpoint.py
"""

import hashlib
import json
import os
import sys
import urllib.error
import urllib.request

APIS = os.environ.get(
    "BEACON_APIS", "https://ethereum-beacon-api.publicnode.com https://lodestar-mainnet.chainsafe.io"
).split()
SLOTS_PER_PERIOD = 8192


def get(api, path):
    req = urllib.request.Request(f"{api}{path}", headers={"Accept": "application/json", "User-Agent": "willow-verify-build"})
    with urllib.request.urlopen(req, timeout=60) as r:
        return json.load(r)


def header_root(h):
    """hash_tree_root of a BeaconBlockHeader: five fields in eight leaves."""
    u64 = lambda v: int(v).to_bytes(8, "little") + bytes(24)
    b32 = lambda v: bytes.fromhex(v[2:])
    leaves = [u64(h["slot"]), u64(h["proposer_index"]), b32(h["parent_root"]), b32(h["state_root"]), b32(h["body_root"])]
    leaves += [bytes(32)] * 3
    while len(leaves) > 1:
        leaves = [hashlib.sha256(leaves[i] + leaves[i + 1]).digest() for i in range(0, len(leaves), 2)]
    return "0x" + leaves[0].hex()


if len(APIS) < 2:
    sys.exit("checkpoint.py: name two independent beacon APIs")
finalized = int(get(APIS[0], "/eth/v1/beacon/headers/finalized")["data"]["header"]["message"]["slot"])
slot = (finalized // SLOTS_PER_PERIOD - 2) * SLOTS_PER_PERIOD
# The period's first slot is an epoch's; when it is empty, the next epoch's first slot.
for _ in range(32):
    try:
        found = get(APIS[0], f"/eth/v1/beacon/headers/{slot}")["data"]
        break
    except urllib.error.HTTPError as e:
        if e.code != 404:
            raise
        slot += 32
else:
    sys.exit("checkpoint.py: no block at the start of the period's first epochs")
root = found["root"]
if not found.get("canonical", False) or slot >= finalized:
    sys.exit(f"checkpoint.py: block {root} at slot {slot} is not canonical and finalized")
headers = [get(api, f"/eth/v1/beacon/light_client/bootstrap/{root}")["data"]["header"]["beacon"] for api in APIS]
if any(h != headers[0] for h in headers[1:]):
    sys.exit(f"checkpoint.py: the beacon APIs serve different bootstraps for {root}")
if header_root(headers[0]) != root or int(headers[0]["slot"]) != slot:
    sys.exit(f"checkpoint.py: the bootstrap's header is not block {root} at slot {slot}")
print(f"{root}:{slot}")
