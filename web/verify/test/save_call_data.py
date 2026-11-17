"""Saves a state witness for verified calls on Ethereum (anchor-verify's call.rs tests): every account
and slot the calls touch at a block, as eth_getProof results plus code, the way the anchor service
builds them.

    python3 save_call_data.py <out.json> <block> <to>:<calldata> [<to>:<calldata> ...]

<block> is the execution block whose state root the light client verified (a finality update's
finalized block). The caller is the zero address; its account and the block's fee recipient (which
the EVM loads for every call) are included too.
"""

import json
import os
import sys
import urllib.request

EL = os.environ.get("EL_RPC", "https://eth.drpc.org")
ZERO = "0x" + "00" * 20


def rpc(method, params):
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode()
    req = urllib.request.Request(EL, data=body, headers={"Content-Type": "application/json", "User-Agent": "willow-tests"})
    out = json.load(urllib.request.urlopen(req, timeout=60))
    if "error" in out:
        raise SystemExit(f"{method}: {out['error']}")
    return out["result"]


def main():
    out, block, calls = sys.argv[1], hex(int(sys.argv[2])), sys.argv[3:]
    miner = rpc("eth_getBlockByNumber", [block, False])["miner"].lower()
    touched = {ZERO: set(), miner: set()}
    for c in calls:
        to, data = c.split(":")
        touched.setdefault(to.lower(), set())
        al = rpc("eth_createAccessList", [{"from": ZERO, "to": to, "data": data}, block])
        for e in al["accessList"]:
            touched.setdefault(e["address"].lower(), set()).update(k.lower() for k in e["storageKeys"])
    accounts = []
    for a, keys in sorted(touched.items()):
        p = rpc("eth_getProof", [a, sorted(keys), block])
        p["code"] = rpc("eth_getCode", [a, block])
        accounts.append(p)
    json.dump({"block": int(block, 16), "calls": [dict(zip(("to", "data"), c.split(":"))) for c in calls], "accounts": accounts},
              open(out, "w"), separators=(",", ":"))
    print(f"{len(accounts)} accounts, {sum(len(a['storageProof']) for a in accounts)} slots")


main()
