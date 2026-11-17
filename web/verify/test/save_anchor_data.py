"""Saves the Ethereum light-client data and block anchors the browser tests serve (test/anchor/), from a
public beacon API and execution RPC, as Willow's anchor service would answer them.

    python3 save_anchor_data.py <checkpoint root | auto> <block> [<block> ...]

`auto` picks the finalized header of the update two periods back (the page pins one that old). Writes
checkpoint.json (root, slot), bootstrap.json and updates.json (the willow_lightClient* results), for
each block (or `h`, `h-<k>`: the head, k blocks behind it) history-<n>.json, its willow_getBlockAnchor
result (`history` or `head`, against the newest finality update), and sepolia.json, a Sepolia finality
update (another chain's validators, for the tests). Blocks older than 8,191 blocks need the anchor
service's `beacon` answers (Willow's anchor service), saved as beacon-<n>.json.
"""

import json
import os
import sys
import urllib.request

BEACON = os.environ.get("BEACON_API", "https://ethereum-beacon-api.publicnode.com")
SEPOLIA = os.environ.get("SEPOLIA_BEACON_API", "https://ethereum-sepolia-beacon-api.publicnode.com")
EL = os.environ.get("EL_RPC", "https://ethereum-rpc.publicnode.com")
HISTORY = "0x0000F90827F1C53a10cb7A02335B175320002935"
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "anchor")


def get(path, base=BEACON):
    req = urllib.request.Request(base + path, headers={"Accept": "application/json", "User-Agent": "willow-tests"})
    return json.load(urllib.request.urlopen(req, timeout=60))


def rpc(method, params):
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode()
    req = urllib.request.Request(EL, data=body, headers={"Content-Type": "application/json", "User-Agent": "willow-tests"})
    out = json.load(urllib.request.urlopen(req, timeout=60))
    if "error" in out:
        raise SystemExit(f"{method}: {out['error']}")
    return out["result"]


def save(name, value):
    with open(os.path.join(OUT, name), "w") as f:
        json.dump(value, f, separators=(",", ":"))


def main():
    os.makedirs(OUT, exist_ok=True)
    fin = get("/eth/v1/beacon/light_client/finality_update")
    period = int(fin["data"]["signature_slot"]) // 8192
    if sys.argv[1] == "auto":
        u = get(f"/eth/v1/beacon/light_client/updates?start_period={period - 2}&count=1")[0]
        root = None
        slot = int(u["data"]["finalized_header"]["beacon"]["slot"])
        root = get(f"/eth/v1/beacon/headers/{slot}")["data"]["root"]
    else:
        root = sys.argv[1]
        slot = int(get(f"/eth/v1/beacon/headers/{root}")["data"]["header"]["message"]["slot"])
    boot = get(f"/eth/v1/beacon/light_client/bootstrap/{root}")
    start = slot // 8192
    updates = get(f"/eth/v1/beacon/light_client/updates?start_period={start}&count={period - start}")
    save("checkpoint.json", {"root": root, "slot": slot})
    save("bootstrap.json", boot)
    save("updates.json", updates)
    head = int(fin["data"]["attested_header"]["execution"]["block_number"])
    # "h" is the head's own block, "h-20" twenty blocks behind it (signed, not final yet).
    blocks = [head - int(b[2:] or 0) if b.startswith("h") else int(b) for b in sys.argv[2:]]
    for n in blocks:
        if n == head:
            proof = {"kind": "head"}
        elif head - 8191 <= n < head:
            p = rpc("eth_getProof", [HISTORY, [hex(n % 8191)], hex(head)])
            proof = {"kind": "history", "proof": p}
        else:
            print(f"block {n}: not within 8,191 blocks of {head}; skipped")
            continue
        save(f"history-{n}.json", {"blockNumber": hex(n), "update": fin, "proof": proof})
        print(f"block {n}: {proof['kind']} against {head}")
    save("sepolia.json", {"update": get("/eth/v1/beacon/light_client/finality_update", SEPOLIA)})
    print(f"checkpoint {root} slot {slot}, periods {start}..{period}, head {head}")


main()
