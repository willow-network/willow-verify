"""Saves test/anchor/tags/: `willow_getBlockAnchor ["latest"]` on all seven chains and
`["finalized"]` on Ethereum, as Willow's public route serves them, with the light-client data that
verifies them. These are the fixtures of interface I5 (notes/verify-every-response-build-2026-11-11.md
section 1.3.2); the drop-in client and the verify page code against them.

    python3 save_tag_anchors.py

Writes checkpoint.json, bootstrap.json and updates.json (from a public beacon API, as
save_anchor_data.py's `auto`: the checkpoint two periods back), then one file per case:

    {"chainId", "request", "response", "savedAt", "expect", ["block"]}

- `request` is the JSON-RPC request; `response` the body Willow sent, which `Anchor.verify(chainId,
  n, hash, JSON.stringify(response), savedAt)` verifies, `n` being `response.result.blockNumber`.
- `expect` is what that returns: `path`, `level`, and `hash`, which is the block's hash as a
  public node serves it (`eth_getBlockByNumber`), not Willow's.
- `block`, on the L2s: that public node's header JSON (`transactions` dropped), which the
  eth-anchor tests serve as the L2 RPC.

Until the tags are served (willow-eth-anchor before W4 refuses them with -32602), each response is
the one Willow sends for the block the tag chooses: on Ethereum `[n]` for the light client's head
(`head`) and for its finalized block (`history`), asked again until Willow's head is that block;
on an L2 `[n]` for a block near the tip, served from the newest block m its sequencer signed, with
`blockNumber` set to m and the history hops back to n dropped. That is the response the tag gets
(`headers` is block m's alone, `proofs` empty); everything else is Willow's, unchanged.

Asks Willow's public route about a dozen times and the public nodes about as often. Env:
WILLOW_PUBLIC (default https://api.willow.tech/v1/rpc), BEACON_API, and <CHAIN>_RPC per chain.
"""

import json
import os
import sys
import time
import urllib.error
import urllib.request

WILLOW = os.environ.get("WILLOW_PUBLIC", "https://api.willow.tech/v1/rpc")
BEACON = os.environ.get("BEACON_API", "https://ethereum-beacon-api.publicnode.com")
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "anchor", "tags")
HISTORY = "0x0000F90827F1C53a10cb7A02335B175320002935"
# Each chain's public node, and how many blocks behind its tip to ask Willow for (about 5 seconds).
CHAINS = {
    1: ("ETH_RPC", "https://ethereum-rpc.publicnode.com", 0),
    8453: ("BASE_RPC", "https://base-rpc.publicnode.com", 3),
    10: ("OP_RPC", "https://optimism-rpc.publicnode.com", 3),
    130: ("UNICHAIN_RPC", "https://unichain-rpc.publicnode.com", 5),
    57073: ("INK_RPC", "https://rpc-gel.inkonchain.com", 5),
    42161: ("ARB_RPC", "https://arbitrum-one-rpc.publicnode.com", 20),
    4663: ("ROBINHOOD_RPC", "https://rpc.mainnet.chain.robinhood.com", 50),
}
UA = {"User-Agent": "willow-tests", "Accept": "application/json"}


def get(path):
    req = urllib.request.Request(BEACON + path, headers=UA)
    return json.load(urllib.request.urlopen(req, timeout=60))


def post(url, method, params):
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode()
    req = urllib.request.Request(url, data=body, headers={**UA, "Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=60) as r:
        return json.load(r)


def node(chain, method, params):
    env, url, _ = CHAINS[chain]
    out = post(os.environ.get(env, url), method, params)
    if "error" in out:
        sys.exit(f"chain {chain} {method}: {out['error']}")
    return out["result"]


def willow(chain, params):
    """Willow's response body to willow_getBlockAnchor(params), and the request."""
    request = {"jsonrpc": "2.0", "id": 1, "method": "willow_getBlockAnchor", "params": params}
    time.sleep(1)  # a low rate on the public route
    return post(f"{WILLOW}/{chain}/public", request["method"], params), request


def rlp_items(b):
    """The items of an RLP list of strings (a header)."""
    def item(i):
        p = b[i]
        if p < 0x80:
            return i, i + 1
        if p < 0xB8:
            return i + 1, i + 1 + p - 0x80
        if p < 0xC0:
            n = p - 0xB7
            ln = int.from_bytes(b[i + 1:i + 1 + n], "big")
            return i + 1 + n, i + 1 + n + ln
        if p < 0xF8:
            return i + 1, i + 1 + p - 0xC0
        n = p - 0xF7
        ln = int.from_bytes(b[i + 1:i + 1 + n], "big")
        return i + 1 + n, i + 1 + n + ln
    start, end = item(0)
    out, i = [], start
    while i < end:
        s, e = item(i)
        out.append(b[s:e])
        i = e
    return out


def header_number_time(rlp_hex):
    f = rlp_items(bytes.fromhex(rlp_hex[2:]))
    return int.from_bytes(f[8], "big"), int.from_bytes(f[11], "big")


def public_block(chain, n):
    """Block n as the chain's public node serves it, asked again while it does not have it yet."""
    for _ in range(10):
        b = node(chain, "eth_getBlockByNumber", [hex(n), False])
        if b:
            b["transactions"] = []
            return b
        time.sleep(2)
    sys.exit(f"chain {chain}: the public node has no block {n}")


def save(name, value):
    with open(os.path.join(OUT, name), "w") as f:
        json.dump(value, f, separators=(",", ":"))


def light_client():
    fin = get("/eth/v1/beacon/light_client/finality_update")
    period = int(fin["data"]["signature_slot"]) // 8192
    u = get(f"/eth/v1/beacon/light_client/updates?start_period={period - 2}&count=1")[0]
    slot = int(u["data"]["finalized_header"]["beacon"]["slot"])
    root = get(f"/eth/v1/beacon/headers/{slot}")["data"]["root"]
    start = slot // 8192
    save("checkpoint.json", {"root": root, "slot": slot})
    save("bootstrap.json", get(f"/eth/v1/beacon/light_client/bootstrap/{root}"))
    # One period more than now: the fixtures' updates may be signed in the next one.
    save("updates.json", get(f"/eth/v1/beacon/light_client/updates?start_period={start}&count={period - start + 1}"))
    print(f"checkpoint {root} slot {slot}, periods {start}..{period}")


def refused_tag(body):
    return body.get("error", {}).get("code") == -32602


def ethereum():
    # Latest: the light client's head. Asked by number, again until Willow's head is that block.
    body, request = willow(1, ["latest"])
    if refused_tag(body):
        n = int(get("/eth/v1/beacon/light_client/finality_update")["data"]["attested_header"]["execution"]["block_number"])
        for _ in range(8):
            body, _ = willow(1, [hex(n)])
            r = body.get("result")
            if r is None:
                if body.get("error", {}).get("data", {}).get("status") != "waiting":
                    sys.exit(f"Ethereum {n}: {body}")
                time.sleep(4)
            elif r["proof"]["kind"] == "head":
                break
            else:
                # Willow's head is newer: ask for it.
                n = int(r["update"]["data"]["attested_header"]["execution"]["block_number"])
        else:
            sys.exit("Willow's head moved on each time")
    saved_at = int(time.time())
    r = body["result"]
    n = int(r["blockNumber"], 16)
    assert r["proof"]["kind"] == "head", r["proof"]["kind"]
    save("latest-1.json", {"chainId": 1, "request": request, "response": body, "savedAt": saved_at,
                           "expect": {"path": "head", "level": "signed",
                                      "hash": public_block(1, n)["hash"]}})
    print(f"Ethereum latest: block {n} (head)")
    # Finalized: the finalized block, by history under the head.
    body, request = willow(1, ["finalized"])
    if refused_tag(body):
        f = int(r["update"]["data"]["finalized_header"]["execution"]["block_number"])
        for _ in range(6):
            body, _ = willow(1, [hex(f)])
            got = body.get("result") or sys.exit(f"Ethereum {f}: {body}")
            now_f = int(got["update"]["data"]["finalized_header"]["execution"]["block_number"])
            if now_f == f and got["proof"]["kind"] == "history":
                break
            f = now_f
        else:
            sys.exit("Ethereum's finalized block moved on each time")
    saved_at = int(time.time())
    f = int(body["result"]["blockNumber"], 16)
    assert body["result"]["proof"]["kind"] == "history"
    save("finalized-1.json", {"chainId": 1, "request": request, "response": body, "savedAt": saved_at,
                              "expect": {"path": "history", "level": "final",
                                         "hash": public_block(1, f)["hash"]}})
    print(f"Ethereum finalized: block {f} (history)")


def l2(chain):
    body, request = willow(chain, ["latest"])
    if refused_tag(body):
        lag = CHAINS[chain][2]
        for _ in range(4):
            n = int(node(chain, "eth_blockNumber", []), 16) - lag
            body, _ = willow(chain, [hex(n)])
            if "result" in body:
                break
            print(f"chain {chain} block {n}: {body.get('error')}")
            time.sleep(5)
        else:
            sys.exit(f"chain {chain}: no anchor")
        r = body["result"]
        if "signed" not in r:
            sys.exit(f"chain {chain} block {n}: anchored by a claim, not a signed block; ask nearer the tip")
        m, _ = header_number_time(r["headers"][0])
        r["blockNumber"] = hex(m)
        r["headers"] = r["headers"][:1]
        r["proofs"] = []
    saved_at = int(time.time())
    r = body["result"]
    m, t = header_number_time(r["headers"][0])
    assert int(r["blockNumber"], 16) == m and len(r["headers"]) == 1 and not r["proofs"]
    block = public_block(chain, m)
    save(f"latest-{chain}.json", {"chainId": chain, "request": request, "response": body, "savedAt": saved_at,
                                  "expect": {"path": "sequencer", "level": "signed", "hash": block["hash"],
                                             "anchorTime": t},
                                  "block": block})
    print(f"chain {chain} latest: block {m}, {saved_at - t} s old")


def main():
    os.makedirs(OUT, exist_ok=True)
    light_client()
    ethereum()
    for chain in CHAINS:
        if chain != 1:
            l2(chain)


main()
